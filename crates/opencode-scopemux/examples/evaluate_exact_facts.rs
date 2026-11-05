//! SCOPE-014 evaluation harness.
//!
//! Compares three retrieval arms on a fixed, held-out fixture workspace:
//!   1. heuristic ScopeMux only,
//!   2. ScopeMux + LSP-derived exact facts,
//!   3. generic repository-local retrieval.
//!
//! Run with the native feature and prebuilt artifacts:
//!
//! ```bash
//! SCOPEMUX_PREBUILT_DIR=<native bundle> \
//!   cargo run -p opencode-scopemux --features native --example evaluate_exact_facts
//! ```
//!
//! The fixture is synthetic and fixed before measurement. Results are printed as a
//! markdown summary plus one JSON line per measurement so the card can cite them.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use opencode_lsp::{LspClientRegistry, LspExactFactProvider, LspServerConfig};
use opencode_retrieval::{
    ExactFactOutcome, ExactFactProvider, ExactFactRequest, GenericRepositoryProvider,
    RetrievalProvider,
};
use opencode_scopemux::ScopemuxProvider;
use opencode_types::{
    MapRole, RetrievalCandidate, RetrievalRepresentation, RetrievalRequest, TaskStage,
};

struct Task {
    seed: &'static str,
    objective: &'static str,
    expected_file: &'static str,
}

const TASKS: &[Task] = &[
    Task {
        seed: "src/lib.rs",
        objective: "render_report",
        expected_file: "src/render.rs",
    },
    Task {
        seed: "src/lib.rs",
        objective: "area_total",
        expected_file: "src/shapes.rs",
    },
    Task {
        seed: "src/report.rs",
        objective: "render_label",
        expected_file: "src/render.rs",
    },
];

fn write_fixture(root: &Path) {
    let src = root.join("src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"scope014-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    std::fs::write(
        src.join("lib.rs"),
        "pub mod render;\npub mod shapes;\n\npub fn run() -> String {\n    render::render_report()\n}\n\npub fn total(items: &[Box<dyn shapes::Shape>]) -> f64 {\n    items.iter().map(|item| item.area()).sum()\n}\n",
    )
    .unwrap();
    std::fs::write(
        src.join("shapes.rs"),
        "pub trait Shape {\n    fn area(&self) -> f64;\n}\n\npub struct Circle {\n    pub radius: f64,\n}\n\npub struct Square {\n    pub side: f64,\n}\n\nimpl Shape for Circle {\n    fn area(&self) -> f64 {\n        3.14159 * self.radius * self.radius\n    }\n}\n\nimpl Shape for Square {\n    fn area(&self) -> f64 {\n        self.side * self.side\n    }\n}\n",
    )
    .unwrap();
    std::fs::write(
        src.join("render.rs"),
        "pub fn render_report() -> String {\n    format!(\"total: {}\", render_label())\n}\n\npub fn render_label() -> &'static str {\n    \"report\"\n}\n",
    )
    .unwrap();
    // Distractor file: present so precision has something it can wrongly include.
    std::fs::write(
        src.join("report.rs"),
        "use crate::render::render_label;\n\npub fn describe() -> &'static str {\n    render_label()\n}\n",
    )
    .unwrap();
}

fn request(root: &Path, task: &Task) -> RetrievalRequest {
    RetrievalRequest {
        objective: task.objective.to_string(),
        stage: TaskStage::Implementing,
        workspace_root: root.to_string_lossy().to_string(),
        seed_files: vec![task.seed.to_string()],
        seed_symbols: Vec::new(),
        changed_files: Vec::new(),
        role: MapRole::Project,
        representation: RetrievalRepresentation::Search,
        token_budget: Some(20),
        task_id: Some(format!("scope014-{}", task.objective)),
        reopen_reason: None,
        plan_nodes: Vec::new(),
    }
}

fn token_estimate(candidate: &RetrievalCandidate) -> usize {
    candidate.estimated_tokens.unwrap_or_else(|| {
        candidate
            .snippet
            .as_ref()
            .map_or(0, |snippet| snippet.len() / 4)
    })
}

#[derive(Default)]
struct Measurement {
    arm: &'static str,
    task: &'static str,
    candidates: usize,
    tokens: usize,
    precision: f64,
    recall: f64,
    success: bool,
    exact_edges: usize,
    fallback_notes: usize,
    latency_ms: f64,
    cached_facts: usize,
    fact_calls: usize,
}

struct RecordingFacts {
    inner: Arc<dyn ExactFactProvider>,
    calls: AtomicUsize,
    cached: AtomicUsize,
}

impl RecordingFacts {
    fn new(inner: Arc<dyn ExactFactProvider>) -> Self {
        Self {
            inner,
            calls: AtomicUsize::new(0),
            cached: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl ExactFactProvider for RecordingFacts {
    async fn exact_facts(&self, request: ExactFactRequest) -> ExactFactOutcome {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let outcome = self.inner.exact_facts(request).await;
        if let ExactFactOutcome::Exact(fact) = &outcome {
            if fact.cached {
                self.cached.fetch_add(1, Ordering::SeqCst);
            }
        }
        outcome
    }
}

async fn measure(
    arm: &'static str,
    task: &Task,
    provider: &dyn RetrievalProvider,
    root: &Path,
    recording: Option<&RecordingFacts>,
) -> Measurement {
    let req = request(root, task);
    let calls_before = recording
        .map(|recording| recording.calls.load(Ordering::SeqCst))
        .unwrap_or(0);
    let cached_before = recording
        .map(|recording| recording.cached.load(Ordering::SeqCst))
        .unwrap_or(0);

    let start = Instant::now();
    let response = provider.retrieve(&req).await.expect("retrieval should run");
    let latency_ms = start.elapsed().as_secs_f64() * 1000.0;

    let candidates = response.candidates;
    let total = candidates.len();
    let expected = root.join(task.expected_file);
    let expected = expected
        .canonicalize()
        .unwrap_or(expected)
        .to_string_lossy()
        .to_string();
    let relevant = candidates
        .iter()
        .filter(|candidate| {
            let path = PathBuf::from(
                candidate
                    .path
                    .strip_prefix("file://")
                    .unwrap_or(&candidate.path),
            );
            path.canonicalize().unwrap_or(path).to_string_lossy() == expected
        })
        .count();
    let success = relevant > 0;
    let tokens = candidates.iter().map(token_estimate).sum();
    let exact_edges = candidates
        .iter()
        .filter(|candidate| candidate.provenance.contains("exact definition"))
        .count();

    Measurement {
        arm,
        task: task.objective,
        candidates: total,
        tokens,
        precision: if total == 0 {
            0.0
        } else {
            relevant as f64 / total as f64
        },
        recall: if success { 1.0 } else { 0.0 },
        success,
        exact_edges,
        fallback_notes: response
            .provider_notes
            .iter()
            .filter(|note| note.contains("unavailable"))
            .count(),
        latency_ms,
        cached_facts: recording
            .map(|recording| recording.cached.load(Ordering::SeqCst) - cached_before)
            .unwrap_or(0),
        fact_calls: recording
            .map(|recording| recording.calls.load(Ordering::SeqCst) - calls_before)
            .unwrap_or(0),
    }
}

fn percentile(values: &[f64], p: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let index = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    sorted[index]
}

fn rust_analyzer_available() -> bool {
    std::process::Command::new("rust-analyzer")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

async fn lsp_fact_provider(root: &Path) -> Option<Arc<RecordingFacts>> {
    if !rust_analyzer_available() {
        return None;
    }
    let registry = Arc::new(LspClientRegistry::new());
    registry.set_request_timeout(Duration::from_secs(60)).await;
    registry
        .configure(
            root.to_path_buf(),
            vec![LspServerConfig {
                id: "rust-analyzer".to_string(),
                command: "rust-analyzer".to_string(),
                args: Vec::new(),
                extensions: vec![".rs".to_string()],
                env: Default::default(),
                initialization_options: None,
                disabled: false,
            }],
        )
        .await;
    let adapter: Arc<dyn ExactFactProvider> = Arc::new(LspExactFactProvider::new(registry));
    Some(Arc::new(RecordingFacts::new(adapter)))
}

#[tokio::main]
async fn main() {
    let dir = tempfile::tempdir().expect("temp fixture");
    write_fixture(dir.path());
    let root = dir.path().to_path_buf();

    let mut measurements = Vec::new();

    // Arm 1: heuristic ScopeMux only.
    let heuristic = ScopemuxProvider::new();
    for task in TASKS {
        measurements.push(measure("heuristic", task, &heuristic, &root, None).await);
    }

    // Arm 2: ScopeMux + exact LSP facts. Run twice per task so the second pass
    // exercises the fact cache.
    let recording = lsp_fact_provider(&root).await;
    match &recording {
        Some(recording) => {
            let provider = ScopemuxProvider::with_exact_facts(Some(recording.clone()));
            for task in TASKS {
                measurements.push(
                    measure("heuristic+exact", task, &provider, &root, Some(recording)).await,
                );
            }
            for task in TASKS {
                measurements.push(
                    measure(
                        "heuristic+exact(cache)",
                        task,
                        &provider,
                        &root,
                        Some(recording),
                    )
                    .await,
                );
            }
        }
        None => {
            eprintln!("rust-analyzer not available; skipping the exact-facts arm");
        }
    }

    // Arm 3: generic repository-local retrieval.
    let generic = GenericRepositoryProvider::new();
    for task in TASKS {
        measurements.push(measure("generic", task, &generic, &root, None).await);
    }

    println!("# SCOPE-014 evaluation\n");
    println!(
        "| arm | task | candidates | tokens | precision | recall | success | exact edges | fallbacks | latency ms | fact calls | cached |"
    );
    println!("|---|---|---:|---:|---:|---:|---|---:|---:|---:|---:|---:|");
    for m in &measurements {
        println!(
            "| {} | {} | {} | {} | {:.2} | {:.2} | {} | {} | {} | {:.1} | {} | {} |",
            m.arm,
            m.task,
            m.candidates,
            m.tokens,
            m.precision,
            m.recall,
            m.success,
            m.exact_edges,
            m.fallback_notes,
            m.latency_ms,
            m.fact_calls,
            m.cached_facts
        );
    }

    println!("\n## Summary\n");
    for arm in [
        "heuristic",
        "heuristic+exact",
        "heuristic+exact(cache)",
        "generic",
    ] {
        let rows: Vec<&Measurement> = measurements.iter().filter(|m| m.arm == arm).collect();
        if rows.is_empty() {
            continue;
        }
        let successes = rows.iter().filter(|m| m.success).count();
        let tokens: usize = rows.iter().map(|m| m.tokens).sum();
        let latencies: Vec<f64> = rows.iter().map(|m| m.latency_ms).collect();
        let exact: usize = rows.iter().map(|m| m.exact_edges).sum();
        let fallbacks: usize = rows.iter().map(|m| m.fallback_notes).sum();
        let fact_calls: usize = rows.iter().map(|m| m.fact_calls).sum();
        let cached: usize = rows.iter().map(|m| m.cached_facts).sum();
        let precision = rows.iter().map(|m| m.precision).sum::<f64>() / rows.len() as f64;
        let recall = rows.iter().map(|m| m.recall).sum::<f64>() / rows.len() as f64;
        println!(
            "- **{arm}**: success {successes}/{}, avg precision {:.2}, avg recall {:.2}, tokens {}, exact edges {}, fallbacks {}, p50 {:.1}ms, p95 {:.1}ms, fact calls {}, cache hits {}, cost/success {:.1} tokens",
            rows.len(),
            precision,
            recall,
            tokens,
            exact,
            fallbacks,
            percentile(&latencies, 0.50),
            percentile(&latencies, 0.95),
            fact_calls,
            cached,
            if successes == 0 { 0.0 } else { tokens as f64 / successes as f64 }
        );
        let json_rows: Vec<String> = rows
            .iter()
            .map(|m| {
                format!(
                    "{{\"arm\":\"{}\",\"task\":\"{}\",\"candidates\":{},\"tokens\":{},\"precision\":{:.3},\"recall\":{:.3},\"success\":{},\"exact_edges\":{},\"fallbacks\":{},\"latency_ms\":{:.2},\"fact_calls\":{},\"cached\":{}}}",
                    m.arm, m.task, m.candidates, m.tokens, m.precision, m.recall, m.success, m.exact_edges, m.fallback_notes, m.latency_ms, m.fact_calls, m.cached_facts
                )
            })
            .collect();
        for row in json_rows {
            println!("{row}");
        }
    }
}

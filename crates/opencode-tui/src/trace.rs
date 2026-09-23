//! Runtime tracing for diagnosing TUI stalls.
//!
//! Tracing is enabled by default and writes to
//! `dirs::data_local_dir()/opencode/traces/tui.log` (macOS:
//! `~/Library/Application Support/opencode/traces/tui.log`), so a freeze is
//! captured without any environment setup. Override the path with
//! `OPENCODE_TUI_TRACE`; set it to `0`, `false`, `off`, or an empty string to
//! disable tracing.
//!
//! Every line carries the active session id once it is known, so a trace can be
//! attributed to a session with `grep session=ses_... traces/tui.log`. The file
//! is truncated at startup (one trace per run) and capped at `MAX_TRACE_BYTES`
//! so a long-running session cannot fill the disk.
//!
//! A background sampler thread writes one line per second with counters for
//! event-loop iterations, `session.updated` deliveries, sync attempts and
//! cumulative sync duration, draws and cumulative draw duration, and key
//! events. Individual sync calls and long loop gaps are logged inline.
//!
//! The sampler runs on its own thread so the counters are still captured while
//! the main event loop is blocked, which is exactly the case this tracing is
//! meant to observe.

use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// Stop appending once a trace grows past this size (roughly a full day of
/// one-sample-per-second output).
const MAX_TRACE_BYTES: u64 = 8 * 1024 * 1024;

static PATH: OnceLock<Option<String>> = OnceLock::new();
static STARTED: OnceLock<Instant> = OnceLock::new();
static INIT: OnceLock<()> = OnceLock::new();
static STOPPED: AtomicBool = AtomicBool::new(false);
static SESSION: Mutex<Option<String>> = Mutex::new(None);

static LOOP_ITERATIONS: AtomicU64 = AtomicU64::new(0);
static SESSION_UPDATED: AtomicU64 = AtomicU64::new(0);
static SYNCS: AtomicU64 = AtomicU64::new(0);
static SYNC_NS: AtomicU64 = AtomicU64::new(0);
static DRAWS: AtomicU64 = AtomicU64::new(0);
static DRAW_NS: AtomicU64 = AtomicU64::new(0);
static KEYS: AtomicU64 = AtomicU64::new(0);
static STARVATION_EVENTS: AtomicU64 = AtomicU64::new(0);
static STARVATION_NS: AtomicU64 = AtomicU64::new(0);

fn default_path() -> Option<String> {
    let dir = dirs::data_local_dir()?.join("opencode").join("traces");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join("tui.log").to_string_lossy().to_string())
}

fn explicit_path(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case("0")
        || trimmed.eq_ignore_ascii_case("false")
        || trimmed.eq_ignore_ascii_case("off")
    {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn resolve_path() -> Option<String> {
    match std::env::var("OPENCODE_TUI_TRACE") {
        Ok(value) => explicit_path(&value),
        Err(_) => default_path(),
    }
}

fn path() -> Option<&'static str> {
    PATH.get_or_init(resolve_path).as_deref()
}

pub fn enabled() -> bool {
    path().is_some()
}

/// The path the trace is written to, for display in the TUI or CLI.
pub fn trace_path() -> Option<String> {
    path().map(|p| p.to_string())
}

fn started() -> Instant {
    *STARTED.get_or_init(Instant::now)
}

fn elapsed_ms() -> f64 {
    started().elapsed().as_secs_f64() * 1000.0
}

fn ms(ns: u64) -> f64 {
    ns as f64 / 1_000_000.0
}

fn current_session() -> Option<String> {
    SESSION
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

pub fn line(args: std::fmt::Arguments<'_>) {
    if STOPPED.load(Ordering::Relaxed) {
        return;
    }
    let Some(path) = path() else {
        return;
    };
    let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    else {
        return;
    };

    let _ = match current_session() {
        Some(session) => writeln!(
            file,
            "[{:>10.3}ms] session={} {}",
            elapsed_ms(),
            session,
            args
        ),
        None => writeln!(file, "[{:>10.3}ms] {}", elapsed_ms(), args),
    };

    if file
        .metadata()
        .map(|m| m.len() >= MAX_TRACE_BYTES)
        .unwrap_or(false)
    {
        STOPPED.store(true, Ordering::Relaxed);
    }
}

/// Record the active session id, emitting a marker line when it changes.
pub fn sync_session(session_id: Option<&str>) {
    let new_value = session_id.map(str::to_string);
    {
        let mut guard = SESSION
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if *guard == new_value {
            return;
        }
        *guard = new_value.clone();
    }

    match new_value {
        Some(id) => line(format_args!("SESSION set id={}", id)),
        None => line(format_args!("SESSION clear")),
    }
}

/// Start the background sampler. Safe to call more than once.
pub fn init() {
    let Some(path) = path() else {
        return;
    };
    INIT.get_or_init(|| {
        // Fresh trace per run.
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path);

        line(format_args!(
            "=== trace start pid={} epoch_ms={} path={} ===",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0),
            path,
        ));

        std::thread::spawn(|| {
            let mut prev = Counters::load();
            let mut last = Instant::now();
            loop {
                std::thread::sleep(Duration::from_millis(1000));
                let now = Counters::load();
                let dt_ms = last.elapsed().as_secs_f64() * 1000.0;
                line(format_args!(
                    "SAMPLE dt={:>5.0}ms loop={:>4} updated={:>4} syncs={:>4} sync_ms={:>8.1} \
                     draws={:>4} draw_ms={:>7.1} keys={:>3} starv={:>3} starv_ms={:>8.1}",
                    dt_ms,
                    now.loop_iterations - prev.loop_iterations,
                    now.session_updated - prev.session_updated,
                    now.syncs - prev.syncs,
                    ms(now.sync_ns - prev.sync_ns),
                    now.draws - prev.draws,
                    ms(now.draw_ns - prev.draw_ns),
                    now.keys - prev.keys,
                    now.starvation_events - prev.starvation_events,
                    ms(now.starvation_ns - prev.starvation_ns),
                ));
                prev = now;
                last = Instant::now();
            }
        });
    });
}

pub fn record_iteration() {
    LOOP_ITERATIONS.fetch_add(1, Ordering::Relaxed);
}

pub fn record_session_updated() {
    SESSION_UPDATED.fetch_add(1, Ordering::Relaxed);
}

pub fn record_key() {
    KEYS.fetch_add(1, Ordering::Relaxed);
}

pub fn record_draw(duration: Duration) {
    DRAWS.fetch_add(1, Ordering::Relaxed);
    DRAW_NS.fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);
}

pub fn record_sync(get_session: Duration, get_messages: Duration, messages: usize) {
    let total = get_session + get_messages;
    SYNCS.fetch_add(1, Ordering::Relaxed);
    SYNC_NS.fetch_add(total.as_nanos() as u64, Ordering::Relaxed);
    line(format_args!(
        "SYNC get_session={:.1}ms get_messages={:.1}ms total={:.1}ms messages={}",
        get_session.as_secs_f64() * 1000.0,
        get_messages.as_secs_f64() * 1000.0,
        total.as_secs_f64() * 1000.0,
        messages,
    ));
}

pub fn record_starvation(gap: Duration) {
    STARVATION_EVENTS.fetch_add(1, Ordering::Relaxed);
    STARVATION_NS.fetch_add(gap.as_nanos() as u64, Ordering::Relaxed);
    line(format_args!(
        "STARVATION loop_gap={:.1}ms",
        gap.as_secs_f64() * 1000.0
    ));
}

struct Counters {
    loop_iterations: u64,
    session_updated: u64,
    syncs: u64,
    sync_ns: u64,
    draws: u64,
    draw_ns: u64,
    keys: u64,
    starvation_events: u64,
    starvation_ns: u64,
}

impl Counters {
    fn load() -> Self {
        Self {
            loop_iterations: LOOP_ITERATIONS.load(Ordering::Relaxed),
            session_updated: SESSION_UPDATED.load(Ordering::Relaxed),
            syncs: SYNCS.load(Ordering::Relaxed),
            sync_ns: SYNC_NS.load(Ordering::Relaxed),
            draws: DRAWS.load(Ordering::Relaxed),
            draw_ns: DRAW_NS.load(Ordering::Relaxed),
            keys: KEYS.load(Ordering::Relaxed),
            starvation_events: STARVATION_EVENTS.load(Ordering::Relaxed),
            starvation_ns: STARVATION_NS.load(Ordering::Relaxed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_path_parses_disable_and_override() {
        assert_eq!(explicit_path(""), None);
        assert_eq!(explicit_path("0"), None);
        assert_eq!(explicit_path("false"), None);
        assert_eq!(explicit_path("off"), None);
        assert_eq!(
            explicit_path("  /tmp/tui-trace.log  "),
            Some("/tmp/tui-trace.log".to_string())
        );
    }
}

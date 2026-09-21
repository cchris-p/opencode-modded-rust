//! Env-gated runtime tracing for diagnosing TUI stalls.
//!
//! Enabled by setting `OPENCODE_TUI_TRACE` to a file path. When set, a
//! background sampler thread writes one line per second with counters for
//! event-loop iterations, `session.updated` deliveries, sync attempts and
//! cumulative sync duration, draws and cumulative draw duration, and key
//! events. Individual sync calls and long loop gaps are logged inline.
//!
//! The sampler runs on its own thread so the counters are still captured while
//! the main event loop is blocked, which is exactly the case this tracing is
//! meant to observe.

use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

static PATH: OnceLock<Option<String>> = OnceLock::new();
static STARTED: OnceLock<Instant> = OnceLock::new();
static INIT: OnceLock<()> = OnceLock::new();

static LOOP_ITERATIONS: AtomicU64 = AtomicU64::new(0);
static SESSION_UPDATED: AtomicU64 = AtomicU64::new(0);
static SYNCS: AtomicU64 = AtomicU64::new(0);
static SYNC_NS: AtomicU64 = AtomicU64::new(0);
static DRAWS: AtomicU64 = AtomicU64::new(0);
static DRAW_NS: AtomicU64 = AtomicU64::new(0);
static KEYS: AtomicU64 = AtomicU64::new(0);
static STARVATION_EVENTS: AtomicU64 = AtomicU64::new(0);
static STARVATION_NS: AtomicU64 = AtomicU64::new(0);

fn path() -> Option<&'static str> {
    PATH.get_or_init(|| {
        std::env::var("OPENCODE_TUI_TRACE")
            .ok()
            .filter(|value| !value.is_empty())
    })
    .as_deref()
}

pub fn enabled() -> bool {
    path().is_some()
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

pub fn line(args: std::fmt::Arguments<'_>) {
    let Some(path) = path() else {
        return;
    };
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(file, "[{:>10.3}ms] {}", elapsed_ms(), args);
    }
}

/// Start the background sampler. Safe to call more than once.
pub fn init() {
    if !enabled() {
        return;
    }
    INIT.get_or_init(|| {
        line(format_args!(
            "=== trace start pid={} epoch_ms={} path={} ===",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0),
            path().unwrap_or(""),
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

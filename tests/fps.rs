// Performance regression guard – launches a live gled instance with the
// frame-rate limiter raised via `GLED_FPS_LIMIT`, parses the `frame_stats`
// debug log from stdout and asserts that gled sustains a healthy non-vsync
// frame rate while reporting the worst-case frame time (the proxy for
// `Surface::get_current_texture` stalls).
//
// The floor is deliberately set well below the steady-state rate observed on a
// modest integrated GPU (~180 fps median) but far above any vsync-locked rate
// (~60 fps). If vsync ever regressed — the original bug, ~20 ms stalls on
// `Surface::get_current_texture` — the median would collapse to the monitor
// refresh rate and this test would fail. The full UI render cost makes much
// higher rates GPU-bound rather than vsync-bound, so this guards the fix
// rather than chasing an unreachable absolute number.
//
// Performance is only meaningful on an optimized build, so this test runs the
// release binary by default. Build it first with `cargo build --release`, or
// point `GLED_BIN` at another binary. On Wayland-only sessions the surface may
// expose no non-vsync present mode, so we force the X11/XWayland backend where
// `Immediate` is available; override with `WINIT_UNIX_BACKEND` if needed.

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

/// Minimum sustained frame rate gled must reach. Set as a no-vsync regression
/// floor: comfortably below the observed steady state (~180 fps median on an
/// integrated GPU) yet well above any vsync-locked rate (~60 fps).
const TARGET_FPS: f32 = 120.0;
/// FPS limiter value handed to gled – high enough not to cap the measurement.
const FPS_LIMIT: u32 = 5000;
/// How long to sample frame statistics after warmup.
const SAMPLE_DURATION: Duration = Duration::from_secs(8);
/// Initial samples to discard while the window/GPU pipeline warms up.
const WARMUP_SAMPLES: usize = 2;

#[derive(Debug, Clone, Copy)]
struct FrameStats {
    fps: f32,
    max_frame_ms: f32,
}

/// Parse a line such as
/// `... frame_stats: fps=812.3 max_frame_ms=3.412`.
fn parse_frame_stats(line: &str) -> Option<FrameStats> {
    let rest = &line[line.find("frame_stats:")?..];
    let value_after = |key: &str| -> Option<f32> {
        rest.split(key)
            .nth(1)?
            .split_whitespace()
            .next()?
            .parse()
            .ok()
    };
    Some(FrameStats {
        fps: value_after("fps=")?,
        max_frame_ms: value_after("max_frame_ms=")?,
    })
}

// ── Gled process (RAII – always killed on drop) ───────────────────────────────

struct Gled {
    child: Child,
    stats: Receiver<FrameStats>,
}

impl Gled {
    fn start() -> Self {
        let bin = std::env::var("GLED_BIN").unwrap_or_else(|_| "./target/release/gled".to_string());
        let mut command = Command::new(&bin);
        command
            .env("GLED_FPS_LIMIT", FPS_LIMIT.to_string())
            .env("RUST_LOG", "gled=debug")
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        // Prefer the X11/XWayland backend, where the `Immediate` (no-vsync)
        // present mode is available. Respect an explicit user override.
        if std::env::var_os("WINIT_UNIX_BACKEND").is_none() {
            command.env("WINIT_UNIX_BACKEND", "x11");
        }
        let mut child = command.spawn().unwrap_or_else(|_| {
            panic!("Failed to start {bin} — run `cargo build --release` first (or set GLED_BIN)")
        });

        // `tracing` (and therefore the `frame_stats` debug line) is written to
        // stdout, so that is the pipe we parse.
        let pipe = child.stdout.take().expect("stdout piped");
        let (tx, stats) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(pipe).lines().map_while(Result::ok) {
                if let Some(sample) = parse_frame_stats(&line)
                    && tx.send(sample).is_err()
                {
                    break;
                }
            }
        });

        Gled { child, stats }
    }
}

impl Drop for Gled {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        println!("[✓] gled terminated");
    }
}

#[test]
fn test_fps_no_vsync_regression() {
    let gled = Gled::start();

    // Wait for the first frame_stats sample (gled startup + first window second).
    let first = gled
        .stats
        .recv_timeout(Duration::from_secs(30))
        .expect("No frame_stats received — does the environment have a display/GPU?");
    println!(
        "[✓] gled started, first sample: fps={:.1} max_frame_ms={:.3}",
        first.fps, first.max_frame_ms
    );

    // Discard warmup samples.
    for _ in 0..WARMUP_SAMPLES {
        let _ = gled.stats.recv_timeout(Duration::from_secs(5));
    }

    // Collect samples for the sampling window.
    let mut samples = Vec::new();
    let deadline = Instant::now() + SAMPLE_DURATION;
    while Instant::now() < deadline {
        if let Ok(sample) = gled.stats.recv_timeout(Duration::from_secs(5)) {
            println!(
                "    sample: fps={:.1} max_frame_ms={:.3}",
                sample.fps, sample.max_frame_ms
            );
            samples.push(sample);
        }
    }

    assert!(
        !samples.is_empty(),
        "No frame_stats samples collected during the measurement window"
    );

    let best_fps = samples.iter().map(|s| s.fps).fold(0.0_f32, f32::max);
    let median_fps = {
        let mut fps: Vec<f32> = samples.iter().map(|s| s.fps).collect();
        fps.sort_by(|a, b| a.partial_cmp(b).expect("fps is finite"));
        fps[fps.len() / 2]
    };
    let worst_frame_ms = samples
        .iter()
        .map(|s| s.max_frame_ms)
        .fold(0.0_f32, f32::max);

    println!(
        "── result: best_fps={best_fps:.1} median_fps={median_fps:.1} worst_frame_ms={worst_frame_ms:.3} (target {TARGET_FPS} fps)"
    );

    assert!(
        median_fps >= TARGET_FPS,
        "Sustained frame rate too low: median {median_fps:.1} fps < {TARGET_FPS} fps target \
         (best {best_fps:.1} fps, worst frame {worst_frame_ms:.3} ms — likely a surface/vsync stall)"
    );
}

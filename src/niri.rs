//! Keep the show running when niri takes the window off screen.
//!
//! A Wayland compositor stops pacing a window it does not show and drips
//! frame callbacks at ~1 Hz instead, which would pin animations, Art-Net and
//! DMX output to that rate. The cadence watchdog in `App::logic` detects the
//! drip as a fallback, but it can only start once the first dripped frame
//! arrives - up to a second of ~1 fps output on every hide.
//!
//! niri's IPC event stream reports layout changes within milliseconds, so
//! this thread flips [`eframe::SKIP_PAINTING`] the moment the window leaves
//! the screen, and clears it the moment it is back, without waiting for a
//! user interaction. Events are only change notifications: visibility is
//! always recomputed from authoritative `Windows`/`Workspaces` replies, so
//! unknown event variants or payload drift cannot break detection. Without
//! `NIRI_SOCKET` (any other platform or compositor) the thread never starts
//! and the watchdog remains the only line of defence.

use std::collections::HashSet;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use serde_json::Value;

/// At most one visibility recheck per interval; events in between are folded
/// into the next check. Well under the watchdog's 150 ms starve threshold,
/// and cheap: two tiny JSON replies over a local Unix socket.
const CHECK_INTERVAL: Duration = Duration::from_millis(100);
/// The event stream read timeout doubles as the recheck ticker.
const READ_TIMEOUT: Duration = Duration::from_millis(50);

static CONTEXT: OnceLock<egui::Context> = OnceLock::new();

/// Hand the root context to the visibility thread so it can request a repaint
/// when the window is back on screen without any user interaction.
pub fn set_context(ctx: &egui::Context) {
    let _ = CONTEXT.set(ctx.clone());
}

pub fn start_thread() {
    let Some(socket) = std::env::var_os("NIRI_SOCKET") else {
        return;
    };
    std::thread::Builder::new()
        .name("niri:visibility".to_owned())
        .spawn(move || watch_loop(socket.into()))
        .expect("Could not spawn niri visibility thread");
}

fn watch_loop(socket: PathBuf) {
    // Survives reconnects: whether the current painting pause was armed by
    // this watcher. Only our own pauses are auto-resumed - if the cadence
    // watchdog paused painting instead (a compositor that throttles a window
    // whose workspace is active, e.g. one scrolled just out of view),
    // resuming on the workspace signal alone would fight the watchdog in a
    // pause loop. That pause stays until user interaction, exactly as it
    // would without the watcher.
    let mut paused_by_us = false;
    let mut seen_visible = false;
    loop {
        match UnixStream::connect(&socket) {
            Ok(stream) => watch_stream(stream, &socket, &mut paused_by_us, &mut seen_visible),
            Err(err) => {
                tracing::debug!("niri socket unavailable ({err}); watchdog remains the fallback")
            }
        }
        // The compositor may be restarting; try again quietly.
        std::thread::sleep(Duration::from_secs(5));
    }
}

fn watch_stream(
    mut events: UnixStream,
    socket: &Path,
    paused_by_us: &mut bool,
    seen_visible: &mut bool,
) {
    let mut reader = BufReader::new(events.try_clone().expect("socket clone"));
    if events.set_read_timeout(Some(READ_TIMEOUT)).is_err() {
        return;
    }
    if events.write_all(b"\"EventStream\"\n").is_err() {
        return;
    }

    // Pausing only arms after the window has been seen on screen once: a
    // window that never presented a frame (freshly started off screen) is
    // indistinguishable from a hidden one, and pausing it would prevent the
    // first present, so it could never show up at all. Until then the
    // cadence watchdog covers the window.
    let mut line = String::new();
    let mut last_check = Instant::now();
    loop {
        match reader.read_line(&mut line) {
            Ok(0) => return,       // EOF: niri went away, reconnect
            Ok(_) => line.clear(), // a change happened somewhere; fall through to the check
            Err(err)
                if err.kind() == std::io::ErrorKind::WouldBlock
                    || err.kind() == std::io::ErrorKind::TimedOut => {}
            Err(_) => return,
        }
        if last_check.elapsed() < CHECK_INTERVAL {
            continue;
        }
        last_check = Instant::now();
        let Some(visible) = compute_visible(socket) else {
            continue;
        };
        if visible {
            if !*seen_visible {
                *seen_visible = true;
                tracing::debug!("niri watcher online; window on screen");
            } else if *paused_by_us {
                tracing::debug!("niri reports the window back on screen - resuming painting");
                *paused_by_us = false;
                eframe::SKIP_PAINTING.store(false, Ordering::Relaxed);
                if let Some(ctx) = CONTEXT.get() {
                    ctx.request_repaint();
                }
            }
        } else if *seen_visible && !eframe::skip_painting() {
            tracing::debug!(
                "niri reports the window off screen - pausing painting, output continues"
            );
            *paused_by_us = true;
            eframe::SKIP_PAINTING.store(true, Ordering::Relaxed);
        }
    }
}

/// Visibility from authoritative state, never from event payloads: one of our
/// windows (matched by pid) sits on an active workspace. A window scrolled
/// out of its workspace's view keeps that workspace active, so it stays
/// "visible" here - the compositor keeps pacing it, and the cadence watchdog
/// covers the rare compositor that still throttles it. `None` means the
/// compositor did not answer, or none of our windows is mapped yet (startup)
/// - keep the current state rather than flapping. The tile position is
/// deliberately ignored: niri 26.04 reports it as null even for windows that
/// are plainly on screen.
fn compute_visible(socket: &Path) -> Option<bool> {
    let windows = query(socket, "\"Windows\"")?;
    let workspaces = query(socket, "\"Workspaces\"")?;
    let active: HashSet<u64> = workspaces
        .pointer("/Ok/Workspaces")?
        .as_array()?
        .iter()
        .filter(|workspace| workspace["is_active"].as_bool() == Some(true))
        .filter_map(|workspace| workspace["id"].as_u64())
        .collect();
    let pid = u64::from(std::process::id());
    Some(
        windows
            .pointer("/Ok/Windows")?
            .as_array()?
            .iter()
            .filter(|window| window["pid"].as_u64() == Some(pid))
            .any(|window| {
                window["workspace_id"]
                    .as_u64()
                    .is_some_and(|workspace_id| active.contains(&workspace_id))
            }),
    )
}

fn query(socket: &Path, request: &str) -> Option<Value> {
    let mut stream = UnixStream::connect(socket).ok()?;
    stream.write_all(request.as_bytes()).ok()?;
    stream.write_all(b"\n").ok()?;
    let mut reply = String::new();
    BufReader::new(stream).read_line(&mut reply).ok()?;
    serde_json::from_str(&reply).ok()
}

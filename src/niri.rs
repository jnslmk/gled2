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
//! user interaction. A recheck runs the moment an event lands, so the flag
//! follows the compositor by about one socket round trip; bursts fold into a
//! pending recheck flushed by the read timeout, and a sustained flood never
//! rechecks faster than [`CHECK_INTERVAL`], bounding the worst case near
//! 100 ms. Events are only change notifications: visibility is
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

/// Fold window for event bursts: an event rechecks immediately unless one
/// ran within this window, in which case it only arms a pending recheck -
/// so a quiet compositor reaches the flag in one recheck while a flood
/// never queries faster than ~10/s. Well under the watchdog's 150 ms starve
/// threshold, and cheap: two tiny JSON replies over a local Unix socket.
const CHECK_INTERVAL: Duration = Duration::from_millis(100);
/// The event stream read timeout doubles as the flush ticker for a recheck
/// pended by [`CHECK_INTERVAL`], bounding the burst worst case at one tick
/// plus the queries. There is no idle polling: every state input emits an
/// event, so quiet time queries nothing at all.
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
    let mut pending = false;
    loop {
        let event = match reader.read_line(&mut line) {
            Ok(0) => return, // EOF: niri went away, reconnect
            Ok(_) => {
                line.clear(); // a change happened somewhere; recheck below
                true
            }
            Err(err)
                if err.kind() == std::io::ErrorKind::WouldBlock
                    || err.kind() == std::io::ErrorKind::TimedOut => false,
            Err(_) => return,
        };
        // An event outside the fold window rechecks at once, so the flag
        // lands one recheck after the compositor reports the change;
        // inside it the event only arms a pending recheck, which the next
        // read timeout flushes - a flood then never queries faster than
        // the fold window, and quiet time never queries at all. The check
        // covers every event that queued while it ran, so it clears the
        // pending flag.
        if event {
            if last_check.elapsed() < CHECK_INTERVAL {
                pending = true;
                continue;
            }
        } else if !pending {
            continue;
        }
        pending = false;
        last_check = Instant::now();
        let Some(visible) = compute_visible(socket) else {
            // `None` has two causes: the compositor did not answer, or it
            // answered and none of our windows is mapped yet. One probe
            // query tells them apart: if it also fails, the compositor is
            // unreachable and the state that raised this check is still
            // unknown - re-arm the pending recheck so the next read
            // timeout retries instead of waiting for a further event. A
            // successful probe means startup without a mapped window (or
            // reply drift); that state only changes with an event, so
            // waiting for one is correct and polling cannot help.
            pending = query(socket, "\"Workspaces\"").is_none();
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixListener;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
    use std::thread;

    /// A stand-in niri compositor: answers every query connection so our
    /// window (same pid) sits on an inactive workspace, and timestamps each
    /// query's arrival. With `answering` cleared it accepts queries but
    /// never replies, which is indistinguishable to `query` from a
    /// compositor that went away. The event stream is a socket pair, so
    /// event lines need no accept-ordering and dropping the test end EOFs
    /// the watcher. The blocking accept loop outlives the test; the process
    /// reaps it.
    struct FakeNiri {
        /// Query endpoint path handed to `watch_stream`.
        socket: PathBuf,
        /// Test end of the event pair: write event lines, drop for EOF.
        events: UnixStream,
        /// Arrival time of each query connection, in accept order.
        queries: Receiver<Instant>,
        /// Whether queries are answered; cleared to simulate an
        /// unreachable compositor.
        answering: Arc<AtomicBool>,
    }

    impl FakeNiri {
        /// Binds the query endpoint and returns itself plus the watcher's
        /// event stream end.
        fn spawn() -> (Self, UnixStream) {
            let dir =
                std::env::temp_dir().join(format!("gled-niri-test-{}", std::process::id()));
            std::fs::create_dir_all(&dir).expect("temp dir");
            let socket = dir.join("niri.sock");
            let _ = std::fs::remove_file(&socket);
            let listener = UnixListener::bind(&socket).expect("bind fake niri socket");
            let (sender, queries) = channel();
            let answering = Arc::new(AtomicBool::new(true));
            let windows = format!(
                r#"{{"Ok":{{"Windows":[{{"pid":{},"workspace_id":1}}]}}}}"#,
                std::process::id()
            );
            let workspaces = r#"{"Ok":{"Workspaces":[{"id":1,"is_active":false}]}}"#;
            let answering_loop = Arc::clone(&answering);
            thread::spawn(move || {
                for stream in listener.incoming() {
                    let mut stream = stream.expect("accept query");
                    let mut request = String::new();
                    BufReader::new(&stream)
                        .read_line(&mut request)
                        .expect("read query");
                    sender.send(Instant::now()).expect("send query time");
                    if !answering_loop.load(Ordering::Relaxed) {
                        continue;
                    }
                    let reply = if request.contains("Windows") {
                        &windows
                    } else {
                        workspaces
                    };
                    stream.write_all(reply.as_bytes()).expect("write reply");
                    stream.write_all(b"\n").expect("write reply newline");
                }
            });
            let (events, watcher_events) = UnixStream::pair().expect("event stream pair");
            (
                Self {
                    socket,
                    events,
                    queries,
                    answering,
                },
                watcher_events,
            )
        }
    }

    /// The latency contract: an event outside the fold window rechecks at
    /// once, an event inside it rechecks via the read-timeout flush, quiet
    /// time queries nothing, and an unreachable compositor is retried on
    /// the read timeout instead of waiting for a further event. Fails
    /// against the old ticker-gated loop, which re-polled every ~100-150 ms
    /// even without events.
    #[test]
    fn watcher_recheck_contract() {
        let (mut niri, watcher_events) = FakeNiri::spawn();
        let socket = niri.socket.clone();
        let watcher = thread::spawn(move || {
            let mut paused_by_us = false;
            // Arming allowed from the start, like a window that has been
            // on screen: the invisibility answers below must pause.
            let mut seen_visible = true;
            watch_stream(
                watcher_events,
                &socket,
                &mut paused_by_us,
                &mut seen_visible,
            );
            (paused_by_us, seen_visible)
        });
        // Outlast the startup fold window so the first event takes the
        // immediate path, not the pending flush.
        thread::sleep(Duration::from_millis(150));

        let sent = Instant::now();
        niri.events.write_all(b"{}\n").expect("send event");
        // READ_TIMEOUT is 50 ms and the flush path cannot fire before one
        // full read timeout, so a first query inside 45 ms proves the
        // immediate path; the previous 140 ms bound also passed a folded
        // recheck.
        niri.queries.recv_timeout(Duration::from_millis(45)).expect("immediate Windows");
        niri.queries.recv_timeout(Duration::from_millis(150)).expect("immediate Workspaces");
        assert!(sent.elapsed() < Duration::from_millis(45), "first recheck not immediate");

        // An event within the fold window must recheck via the next read
        // timeout: within READ_TIMEOUT plus queries and scheduler slack.
        niri.events.write_all(b"{}\n").expect("send folded event");
        niri.queries.recv_timeout(Duration::from_millis(200)).expect("flushed Windows");
        niri.queries.recv_timeout(Duration::from_millis(200)).expect("flushed Workspaces");

        // Quiet time queries nothing - the old loop would have re-polled
        // within ~150 ms.
        assert_eq!(
            niri.queries.recv_timeout(Duration::from_millis(400)),
            Err(RecvTimeoutError::Timeout),
            "idle period must not query"
        );

        // An unreachable compositor must self-heal: a failed check re-arms
        // the pending recheck via its probe query, so retries arrive on the
        // read timeout with no further events - the old idle tick's job.
        niri.answering.store(false, Ordering::Relaxed);
        niri.events.write_all(b"{}\n").expect("send event to dead compositor");
        for _ in 0..4 {
            niri.queries
                .recv_timeout(Duration::from_millis(150))
                .expect("unreachable retries not seen");
        }
        niri.answering.store(true, Ordering::Relaxed);
        // The pending recheck heals within a read timeout of the
        // compositor answering. A retry whose probe slipped in after the
        // store found the compositor reachable and consumed the pending
        // flag - waiting for events is that path's documented contract -
        // so one more event re-arms the check deterministically either way.
        niri.events.write_all(b"{}\n").expect("send event to heal");
        niri.queries.recv_timeout(Duration::from_millis(200)).expect("healed Windows");
        niri.queries.recv_timeout(Duration::from_millis(200)).expect("healed Workspaces");

        // EOF ends the watcher; both checks saw the window off screen, so
        // the pause must be armed. The flag store that goes with it is not
        // asserted: the global is process-shared with other tests in this
        // binary, so it is reset instead - `paused_by_us` is only set in
        // the branch that stores the flag.
        let dir = niri.socket.parent().expect("socket parent").to_owned();
        drop(niri);
        let (paused_by_us, seen_visible) = watcher.join().expect("watcher thread");
        assert!(seen_visible);
        assert!(paused_by_us);
        eframe::SKIP_PAINTING.store(false, Ordering::Relaxed);
        let _ = std::fs::remove_dir_all(dir);
    }
}

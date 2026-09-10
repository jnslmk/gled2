//! Regression guard for the hidden-window stutter: hiding the gled window
//! must pause painting promptly via the niri visibility watcher (well before
//! the compositor's ~1 Hz drip pins the show), and showing it again must
//! resume painting without any user interaction.
//!
//! Needs a live niri session; skips everywhere else. `#[ignore]`d like the
//! fps guard because it moves the desktop's focused workspace around. Run
//! with `cargo test --test visibility -- --ignored` (build the release
//! binary first, or point `GLED_BIN` at another binary).

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

/// Detection is one IPC event plus one immediate recheck (two tiny socket
/// queries); an event burst delays the recheck by at most one read timeout
/// (50 ms), a sustained flood by at most the 100 ms fold window. The budget
/// leaves room for scheduling jitter.
const PAUSE_BUDGET: Duration = Duration::from_millis(500);
const RESUME_BUDGET: Duration = Duration::from_millis(500);

struct Gled {
    child: Child,
    lines: mpsc::Receiver<(Instant, String)>,
}

impl Gled {
    fn start() -> Self {
        let bin = std::env::var("GLED_BIN").unwrap_or_else(|_| "./target/release/gled".to_string());
        let mut command = Command::new(&bin);
        command
            .env("RUST_LOG", "gled=debug")
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = command.spawn().unwrap_or_else(|err| {
            panic!(
                "Failed to start {bin} - run `cargo build --release` first (or set GLED_BIN): {err}"
            )
        });
        let pipe = child.stdout.take().expect("stdout piped");
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(pipe).lines().map_while(Result::ok) {
                if sender.send((Instant::now(), line)).is_err() {
                    break;
                }
            }
        });
        Gled {
            child,
            lines: receiver,
        }
    }

    /// Time from `action_at` until a log line containing `needle` shows up,
    /// or `None` if `budget` elapses first.
    fn wait_for(&self, needle: &str, action_at: Instant, budget: Duration) -> Option<Duration> {
        while let Some(remaining) = (action_at + budget).checked_duration_since(Instant::now()) {
            match self.lines.recv_timeout(remaining) {
                Ok((at, line)) if line.contains(needle) => {
                    return Some(at.duration_since(action_at));
                }
                Ok(_) => continue,
                Err(_) => break,
            }
        }
        None
    }
}

impl Drop for Gled {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn niri(args: &[&str]) -> Vec<u8> {
    let output = Command::new("niri")
        .args(["msg"])
        .args(args)
        .output()
        .expect("niri msg failed");
    assert!(output.status.success(), "niri msg {args:?} failed");
    output.stdout
}

/// Whether the window sits on an active workspace. niri 26.04 reports the
/// tile position as null even for on-screen windows, so the workspace state
/// is the only usable signal - the same one the watcher consumes.
fn on_active_workspace(window_id: u64) -> Option<bool> {
    let windows: serde_json::Value = serde_json::from_slice(&niri(&["-j", "windows"])).ok()?;
    let workspaces: serde_json::Value =
        serde_json::from_slice(&niri(&["-j", "workspaces"])).ok()?;
    let window = windows
        .as_array()?
        .iter()
        .find(|window| window["id"].as_u64() == Some(window_id))?;
    let workspace_id = window["workspace_id"].as_u64()?;
    Some(workspaces.as_array()?.iter().any(|workspace| {
        workspace["id"].as_u64() == Some(workspace_id)
            && workspace["is_active"].as_bool() == Some(true)
    }))
}

/// The gled window of `pid`, or `None` while the window is not mapped yet.
fn window_id_for_pid(pid: u32) -> Option<u64> {
    let windows: serde_json::Value = serde_json::from_slice(&niri(&["-j", "windows"])).ok()?;
    windows
        .as_array()?
        .iter()
        .find(|window| window["pid"].as_u64() == Some(u64::from(pid)))
        .and_then(|window| window["id"].as_u64())
}

#[test]
#[ignore = "moves the focused workspace around; needs a live niri session"]
fn test_visibility_watcher_pauses_and_resumes_promptly() {
    if std::env::var_os("NIRI_SOCKET").is_none() {
        println!("[skip] not a niri session");
        return;
    }

    let gled = Gled::start();
    gled.wait_for("frame_stats", Instant::now(), Duration::from_secs(30))
        .expect("gled did not start reporting frame statistics");
    let mut window_id = None;
    for _ in 0..50 {
        if let Some(id) = window_id_for_pid(gled.child.id()) {
            window_id = Some(id);
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    let window_id = window_id.expect("gled window did not appear in niri");

    // Bring the window on screen first: a freshly mapped window can sit
    // outside the visible view, and the watcher only judges visibility
    // after it has seen the window on screen once.
    for _ in 0..30 {
        niri(&["action", "focus-window", "--id", &window_id.to_string()]);
        if on_active_workspace(window_id) == Some(true) {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    assert_eq!(
        on_active_workspace(window_id),
        Some(true),
        "gled window never appeared on screen"
    );

    // Hide: move focus to any other workspace. Painting must pause on the
    // watcher's IPC detection, not on the compositor's ~1 Hz first drip.
    let workspaces: serde_json::Value =
        serde_json::from_slice(&niri(&["-j", "workspaces"])).expect("workspace list");
    let other = workspaces
        .as_array()
        .and_then(|list| {
            list.iter()
                .find(|workspace| workspace["is_active"].as_bool() != Some(true))
        })
        .and_then(|workspace| workspace["idx"].as_u64())
        .expect("no other workspace to hide to");

    let hide_at = Instant::now();
    niri(&["action", "focus-workspace", &other.to_string()]);
    let pause = gled
        .wait_for("pausing painting", hide_at, PAUSE_BUDGET)
        .expect("hiding the window did not pause painting in time");
    println!("[✓] painting paused {pause:?} after hiding");

    // Show: focus the window again. Painting must resume without any user
    // interaction.
    let show_at = Instant::now();
    niri(&["action", "focus-window", "--id", &window_id.to_string()]);
    let resume = gled
        .wait_for("resuming painting", show_at, RESUME_BUDGET)
        .expect("showing the window did not resume painting without interaction");
    println!("[✓] painting resumed {resume:?} after showing");
}

fn windows_for_pid(_workspaces: &serde_json::Value, _window_id: u64) -> Option<u64> {
    None
}

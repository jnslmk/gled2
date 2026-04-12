// OSC integration test – single gled instance, RAII cleanup, crash monitoring.

use std::io::{BufRead, BufReader};
use std::net::UdpSocket;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

// ── OscType shorthands ───────────────────────────────────────────────────────

fn f(v: f32) -> rosc::OscType {
    rosc::OscType::Float(v)
}
fn b(v: bool) -> rosc::OscType {
    rosc::OscType::Bool(v)
}
fn i(v: i32) -> rosc::OscType {
    rosc::OscType::Int(v)
}
fn s(v: &str) -> rosc::OscType {
    rosc::OscType::String(v.to_string())
}

// ── OSC helpers ──────────────────────────────────────────────────────────────

fn osc(addr: &str, args: Vec<rosc::OscType>) {
    let packet = rosc::OscPacket::Message(rosc::OscMessage {
        addr: addr.to_string(),
        args,
    });
    let buf = rosc::encoder::encode(&packet).expect("OSC encode failed");
    let sock = UdpSocket::bind("127.0.0.1:0").expect("bind failed");
    sock.send_to(&buf, "127.0.0.1:8000")
        .expect("UDP send failed");
}

fn fmt_args(args: &[rosc::OscType]) -> String {
    if args.is_empty() {
        return "()".to_string();
    }
    args.iter()
        .map(|a| match a {
            rosc::OscType::Float(v) => v.to_string(),
            rosc::OscType::Int(v) => v.to_string(),
            rosc::OscType::Bool(v) => v.to_string(),
            rosc::OscType::String(v) => format!("\"{}\"", v),
            _ => "?".to_string(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn send(addr: &str, args: Vec<rosc::OscType>) {
    println!("  > {} {}", addr, fmt_args(&args));
    osc(addr, args);
}

fn delay() {
    thread::sleep(Duration::from_millis(250));
}
fn short_delay() {
    thread::sleep(Duration::from_millis(150));
}

// ── Gled process (RAII – always killed on drop) ───────────────────────────────

struct Gled {
    child: Child,
    errors: Arc<Mutex<Vec<String>>>,
}

impl Gled {
    fn start() -> Self {
        let mut child = Command::new("./target/debug/gled")
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start ./target/debug/gled — run `cargo build` first");

        let pipe = child.stderr.take().unwrap();
        let errors: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        {
            let errors = errors.clone();
            thread::spawn(move || {
                for line in BufReader::new(pipe).lines().map_while(Result::ok) {
                    if line.contains("panicked") || line.contains("ERROR") {
                        errors.lock().unwrap().push(line);
                    }
                }
            });
        }

        // Wait for OSC port to accept connections (up to 30 s).
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            if UdpSocket::bind("127.0.0.1:0")
                .and_then(|s| s.connect("127.0.0.1:8000"))
                .is_ok()
            {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "Timed out waiting for gled OSC port"
            );
            thread::sleep(Duration::from_millis(200));
        }
        println!("[✓] gled started, OSC port 8000 ready");
        Gled { child, errors }
    }

    fn error_count(&self) -> usize {
        self.errors.lock().unwrap().len()
    }
}

impl Drop for Gled {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match self.child.try_wait() {
                Ok(Some(_)) => break,
                _ if Instant::now() >= deadline => break,
                _ => thread::sleep(Duration::from_millis(50)),
            }
        }
        println!("[✓] gled terminated");
    }
}

// ── Suite runner ──────────────────────────────────────────────────────────────

fn suite(name: &str, gled: &Gled, f: impl FnOnce()) {
    println!("\n── {} ──", name);
    let t = Instant::now();
    f();
    println!(
        "   {:.2}s  |  errors so far: {}",
        t.elapsed().as_secs_f64(),
        gled.error_count()
    );
}

// ── Integration test ─────────────────────────────────────────────────────────

#[test]
fn test_osc_integration() {
    let gled = Gled::start();

    suite("1: Basic Connectivity", &gled, || {
        for (addr, args) in [
            ("/project/main_dimmer", vec![f(0.5)]),
            ("/scene/grid/2/1/active", vec![b(true)]),
            ("/scene/grid/0/0/opacity", vec![f(0.8)]),
            ("/project/blackout", vec![b(true)]),
            ("/timing/tap", vec![]),
            ("/scene/quick/3/opacity", vec![f(0.9)]),
            ("/timing/speed/multiply", vec![f(1.5)]),
        ] {
            send(addr, args);
            delay();
        }
    });

    suite("2: Scene Operations", &gled, || {
        for (addr, args) in [
            ("/scene/grid/1/2/toggle", vec![]),
            ("/scene/grid/3/1/color", vec![s("red")]),
            ("/scene/grid/0/3/name", vec![s("Sunset")]),
            ("/scene/grid/4/2/input_dimmer", vec![f(0.75)]),
            ("/scene/grid/5/0/ignore_main_dimmer", vec![b(true)]),
            ("/scene/quick/1/beat_offset", vec![f(0.25)]),
        ] {
            send(addr, args);
            delay();
        }
        for (slot, color) in (0..).zip(["green", "blue", "orange", "yellow", "purple", "pink"]) {
            send(&format!("/scene/quick/{}/color", slot), vec![s(color)]);
            delay();
        }
    });

    suite("3: Timing and Control", &gled, || {
        for (addr, args) in [
            ("/timing/speed/add", vec![f(0.1)]),
            ("/timing/speed/multiply", vec![f(0.5)]),
            ("/timing/speed/multiply", vec![f(1.0)]),
            ("/timing/speed/multiply", vec![f(1.5)]),
            ("/timing/speed/multiply", vec![f(2.0)]),
            ("/timing/speed/multiply", vec![f(0.75)]),
        ] {
            send(addr, args);
            delay();
        }
        for _ in 0..5 {
            osc("/timing/tap", vec![]);
            short_delay();
        }
        send("/project/auto_mode/active", vec![b(true)]);
        delay();
        send("/project/auto_mode/max_scenes", vec![i(10)]);
    });

    suite("4: Edge Cases & Boundary Values", &gled, || {
        for (addr, args) in [
            ("/project/main_dimmer", vec![f(0.0)]),
            ("/project/main_dimmer", vec![f(1.0)]),
            ("/scene/grid/0/0/active", vec![b(true)]),
            ("/scene/grid/7/4/active", vec![b(true)]),
        ] {
            send(addr, args);
            delay();
        }
        for slot in 0..8 {
            osc(&format!("/scene/quick/{}/opacity", slot), vec![f(0.5)]);
            short_delay();
        }
        send("/timing/speed/add", vec![f(-0.2)]);
        delay();
        send("/project/blackout", vec![i(0)]);
        delay();
        send("/project/blackout", vec![i(1)]);
    });

    suite("5: State Subscription", &gled, || {
        send("/osc/state/subscribe", vec![]);
        thread::sleep(Duration::from_millis(500));

        send("/project/main_dimmer", vec![f(0.75)]);
        delay();
        send("/project/blackout", vec![b(true)]);
        delay();

        for row in 0..3 {
            send(&format!("/scene/grid/{}/0/active", row), vec![b(true)]);
            delay();
        }
        for row in 0..3 {
            send(&format!("/scene/grid/{}/0/active", row), vec![b(false)]);
            delay();
        }
        for opacity in [0.25_f32, 0.5, 0.75, 1.0] {
            send("/scene/grid/2/2/opacity", vec![f(opacity)]);
            delay();
        }

        send("/osc/state/unsubscribe", vec![]);
        thread::sleep(Duration::from_millis(500));
    });

    // Final error report (gled is dropped/killed after this scope).
    let errors = gled.errors.lock().unwrap();
    if !errors.is_empty() {
        println!("\n[⚠] {} error(s) detected:", errors.len());
        for (i, e) in errors.iter().take(15).enumerate() {
            println!("  {}: {}", i + 1, e);
        }
        if errors.len() > 15 {
            println!("  ... and {} more", errors.len() - 15);
        }
    }
    let error_count = errors.len();
    drop(errors);

    assert_eq!(
        error_count, 0,
        "gled encountered {} errors/panics",
        error_count
    );
}

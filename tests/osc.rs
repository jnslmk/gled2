// OSC integration test – single gled instance, RAII cleanup, crash monitoring.
// Exercises every OSC parse path at least once against a live gled process.

use std::io::{BufRead, BufReader};
use std::net::UdpSocket;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

// A stable fake UUID used as a placeholder wherever a real asset UUID is not
// available in the fresh integration instance.  All asset-lookup handlers
// treat an unknown UUID as a silent no-op, so this is safe.
const FAKE_UUID: &str = "00000000-0000-0000-0000-000000000001";

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

    suite("6: Remaining scene properties (grid + quick + selected)", &gled, || {
        // set_offset_on_flash, select
        for addr in [
            "/scene/grid/0/0/set_offset_on_flash",
            "/scene/quick/0/set_offset_on_flash",
            "/scene/selected/set_offset_on_flash",
        ] {
            send(addr, vec![b(true)]);
            delay();
            send(addr, vec![b(false)]);
            delay();
        }
        send("/scene/grid/1/1/select", vec![]);
        delay();
        send("/scene/quick/2/select", vec![]);
        delay();
        // clone / delete on grid (result is a no-op on an empty cell – safe)
        send("/scene/grid/6/4/clone", vec![]);
        delay();
        send("/scene/grid/6/4/delete", vec![]);
        delay();
        // add a scene placeholder (unknown UUID → silent no-op)
        send(
            "/scene/grid/7/4/add",
            vec![s(FAKE_UUID)],
        );
        delay();
        // reorder two cells
        send("/scene/reorder/0/0/1/0", vec![]);
        delay();
        send("/scene/reorder/1/0/0/0", vec![]);
        delay();
    });

    suite("7: Scene input bindings (activation / flash / dimmer)", &gled, || {
        for input_kind in ["activation_input", "flash_input", "dimmer_input"] {
            for selector in ["/scene/grid/0/0", "/scene/quick/0", "/scene/selected"] {
                send(&format!("{}/{}/artnet", selector, input_kind), vec![i(42)]);
                delay();
                send(&format!("{}/{}/clear", selector, input_kind), vec![]);
                delay();
            }
        }
    });

    suite("8: Scene palette overwrite", &gled, || {
        // inherit / none / primary / secondary / gradient – via grid, quick, selected
        for selector in ["/scene/grid/0/0", "/scene/quick/0", "/scene/selected"] {
            send(&format!("{}/palette_overwrite", selector), vec![s("inherit")]);
            delay();
            send(&format!("{}/palette_overwrite", selector), vec![s("none")]);
            delay();
            send(
                &format!("{}/palette_overwrite", selector),
                vec![s(FAKE_UUID)],
            );
            delay();
            send(
                &format!("{}/palette_overwrite/primary", selector),
                vec![f(0.1), f(0.2), f(0.3)],
            );
            delay();
            send(
                &format!("{}/palette_overwrite/secondary", selector),
                vec![f(0.4), f(0.5), f(0.6)],
            );
            delay();
            send(
                &format!("{}/palette_overwrite/gradient/2", selector),
                vec![f(0.7), f(0.8), f(0.9)],
            );
            delay();
        }
    });

    suite("9: Scene groups overwrite", &gled, || {
        for selector in ["/scene/grid/0/0", "/scene/quick/0", "/scene/selected"] {
            send(&format!("{}/groups_overwrite/0", selector), vec![s("wash")]);
            delay();
            send(&format!("{}/groups_overwrite/1", selector), vec![s("spot")]);
            delay();
            send(&format!("{}/groups_overwrite/remove/1", selector), vec![]);
            delay();
            send(&format!("{}/groups_overwrite/clear", selector), vec![]);
            delay();
        }
    });

    suite("10: Effect commands (add, properties, config, clone, delete)", &gled, || {
        // effect/add across all selectors
        for selector in ["/scene/grid/0/0", "/scene/quick/0", "/scene/selected"] {
            send(&format!("{}/effect/add", selector), vec![]);
            delay();
        }
        // grid effect property setters on effect index 0
        let base = "/scene/grid/0/0/effect/0";
        for (addr, args) in [
            (format!("{base}/opacity"), vec![f(0.5)]),
            (format!("{base}/color_shift"), vec![f(0.25)]),
            (format!("{base}/beat_progression"), vec![f(0.1)]),
            (format!("{base}/beat_offset"), vec![f(0.5)]),
            (format!("{base}/speed_exponent"), vec![i(1)]),
            (format!("{base}/group_index"), vec![i(0)]),
            (format!("{base}/animation"), vec![s("none")]),
            (format!("{base}/animation"), vec![s(FAKE_UUID)]),
            (format!("{base}/config/u32/0"), vec![i(3)]),
            (format!("{base}/config/f32/0"), vec![f(0.3)]),
        ] {
            send(&addr, args);
            delay();
        }
        // quick effect properties
        let base_q = "/scene/quick/0/effect/0";
        for (addr, args) in [
            (format!("{base_q}/opacity"), vec![f(0.8)]),
            (format!("{base_q}/speed_exponent"), vec![i(-1)]),
            (format!("{base_q}/config/u32/1"), vec![i(7)]),
            (format!("{base_q}/config/f32/1"), vec![f(0.7)]),
            (format!("{base_q}/clone"), vec![]),
            (format!("{base_q}/delete"), vec![]),
        ] {
            send(&addr, args);
            delay();
        }
        // selected effect
        let base_s = "/scene/selected/effect/0";
        for (addr, args) in [
            (format!("{base_s}/opacity"), vec![f(0.6)]),
            (format!("{base_s}/color_shift"), vec![f(0.5)]),
            (format!("{base_s}/beat_progression"), vec![f(0.2)]),
            (format!("{base_s}/beat_offset"), vec![f(0.3)]),
            (format!("{base_s}/speed_exponent"), vec![i(2)]),
            (format!("{base_s}/group_index"), vec![i(1)]),
            (format!("{base_s}/animation"), vec![s("none")]),
            (format!("{base_s}/config/u32/0"), vec![i(5)]),
            (format!("{base_s}/config/f32/0"), vec![f(0.55)]),
            (format!("{base_s}/clone"), vec![]),
            (format!("{base_s}/delete"), vec![]),
        ] {
            send(&addr, args);
            delay();
        }
        // grid effect clone/delete
        send("/scene/grid/0/0/effect/0/clone", vec![]);
        delay();
        send("/scene/grid/0/0/effect/0/delete", vec![]);
        delay();
    });

    suite("11: Project palette", &gled, || {
        send("/project/palette", vec![s("none")]);
        delay();
        send("/project/palette", vec![s("inherit")]);
        delay();
        send("/project/palette", vec![s(FAKE_UUID)]);
        delay();
        send("/project/palette/primary", vec![f(0.9), f(0.1), f(0.1)]);
        delay();
        send("/project/palette/secondary", vec![f(0.1), f(0.9), f(0.1)]);
        delay();
        for idx in 0..4 {
            send(
                &format!("/project/palette/gradient/{idx}"),
                vec![f(0.5), f(0.5), f(0.5)],
            );
            delay();
        }
    });

    suite("12: Project artnet control and groups", &gled, || {
        send("/project/artnet_control/active", vec![b(false)]);
        delay();
        send("/project/artnet_control/active", vec![b(true)]);
        delay();
        send("/project/artnet_control/universe", vec![i(0)]);
        delay();
        // groups
        send("/project/groups/0", vec![s("wash")]);
        delay();
        send("/project/groups/1", vec![s("spot")]);
        delay();
        send("/project/groups/remove/1", vec![]);
        delay();
        send("/project/groups/clear", vec![]);
        delay();
    });

    suite("13: Project input bindings", &gled, || {
        for event_kind in ["tap", "blackout", "blackout_hold", "half", "double"] {
            let ch = 10_i32;
            send(
                &format!("/project/input/{event_kind}/add/artnet"),
                vec![i(ch)],
            );
            delay();
            send(
                &format!("/project/input/{event_kind}/remove/artnet"),
                vec![i(ch)],
            );
            delay();
            send(&format!("/project/input/{event_kind}/clear"), vec![]);
            delay();
        }
    });

    suite("14: Animation asset edits (fake UUID – silent no-op)", &gled, || {
        let base = format!("/asset/animation/{FAKE_UUID}");
        send(&format!("{base}/shader_code"), vec![s("void main() {}")]);
        delay();
        send(&format!("{base}/argument/add"), vec![]);
        delay();
        for kind in ["center", "selection", "slider", "checkbox", "percentage", "degrees"] {
            send(&format!("{base}/argument/0/kind"), vec![s(kind)]);
            delay();
        }
        send(&format!("{base}/argument/0/name"), vec![s("speed")]);
        delay();
        send(&format!("{base}/argument/0/remove"), vec![]);
        delay();
    });

    suite(
        "15: Scene scalar setters — grid, quick, and selected",
        &gled,
        || {
            // Each property is sent to all three target families with distinct values,
            // confirming that every parse path is live.
            for (selector, opacity, color, input_dimmer, beat_offset) in [
                ("/scene/grid/0/0", 0.3_f32, "red", 0.4_f32, 0.1_f32),
                ("/scene/quick/0", 0.6, "green", 0.7, 0.3),
                ("/scene/selected", 0.9, "blue", 0.95, 0.5),
            ] {
                send(&format!("{selector}/opacity"), vec![f(opacity)]);
                delay();
                send(&format!("{selector}/color"), vec![s(color)]);
                delay();
                send(&format!("{selector}/name"), vec![s("Suite15")]);
                delay();
                send(&format!("{selector}/input_dimmer"), vec![f(input_dimmer)]);
                delay();
                send(
                    &format!("{selector}/ignore_main_dimmer"),
                    vec![b(true)],
                );
                delay();
                send(
                    &format!("{selector}/ignore_main_dimmer"),
                    vec![b(false)],
                );
                delay();
                send(&format!("{selector}/beat_offset"), vec![f(beat_offset)]);
                delay();
                send(&format!("{selector}/active"), vec![b(true)]);
                delay();
                send(&format!("{selector}/active"), vec![b(false)]);
                delay();
                send(&format!("{selector}/toggle"), vec![]);
                delay();
            }
        },
    );
    suite(
        "16: Curve/value sub-path setters — scene and effect",
        &gled,
        || {
            // Scene-level: opacity and beat_offset, each with /value and /curve
            for selector in ["/scene/grid/0/0", "/scene/quick/0", "/scene/selected"] {
                send(&format!("{selector}/opacity/value"), vec![f(0.75)]);
                delay();
                send(&format!("{selector}/opacity/static"), vec![f(0.65)]);
                delay();
                send(&format!("{selector}/opacity/curve"), vec![s(FAKE_UUID)]);
                delay();
                send(&format!("{selector}/opacity/curve"), vec![s("none")]);
                delay();
                send(&format!("{selector}/beat_offset/value"), vec![f(0.25)]);
                delay();
                send(&format!("{selector}/beat_offset/static"), vec![f(0.15)]);
                delay();
                send(&format!("{selector}/beat_offset/curve"), vec![s(FAKE_UUID)]);
                delay();
                send(&format!("{selector}/beat_offset/curve"), vec![s("none")]);
                delay();
            }

            // Effect-level: all four curve fields, each with /value and /curve
            for selector in [
                "/scene/grid/0/0/effect/0",
                "/scene/quick/0/effect/0",
                "/scene/selected/effect/0",
            ] {
                for field in ["opacity", "color_shift", "beat_progression", "beat_offset"] {
                    send(&format!("{selector}/{field}/value"), vec![f(0.5)]);
                    delay();
                    send(&format!("{selector}/{field}/static"), vec![f(0.4)]);
                    delay();
                    send(&format!("{selector}/{field}/curve"), vec![s(FAKE_UUID)]);
                    delay();
                    send(&format!("{selector}/{field}/curve"), vec![s("none")]);
                    delay();
                }
            }
        },
    );
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

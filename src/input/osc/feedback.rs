use log::{debug, trace, warn};
use rosc::{OscMessage, OscPacket, OscType};
use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use std::{io, thread};

use crate::storage::asset::project::GridHighlight;
use crate::storage::asset::scene::color::SceneInstanceColor;
use crate::storage::asset::scene::instance::SceneInstance;
use crate::ui::action::UiAction;

use super::parse::parse_message;
use super::{OSCHandler, OscStateSnapshot};

static SUBSCRIBER_TTL: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClientInterestCommand {
    Subscribe,
    Unsubscribe,
    Heartbeat,
}

pub(super) fn start_network_loop(handler: Arc<OSCHandler>) {
    thread::Builder::new()
        .name("gled:artnet:osc".to_string())
        .spawn(move || {
            let mut buf = [0u8; rosc::decoder::MTU];
            let mut subscribers: HashMap<SocketAddr, Instant> = HashMap::new();
            trace!("Starting osc network receiving loop");

            let state_receiver = crate::output_state::new_receiver();

            while handler.running.load(Ordering::Relaxed) {
                subscribers.retain(|_, last_seen| last_seen.elapsed() < SUBSCRIBER_TTL);
                handler
                    .has_subscribers
                    .store(!subscribers.is_empty(), Ordering::Relaxed);

                let mut latest_snapshot = None;
                while let Ok(Some(state)) = state_receiver.try_recv() {
                    latest_snapshot = Some(OscStateSnapshot {
                        project: state.project,
                        selected_scene_instance: state.selected_scene_instance,
                        blackout: state.blackout,
                        beats_per_minute: state.beats_per_minute,
                        beat_progression: state.beat_progression,
                    });
                }
                if let Some(snapshot) = latest_snapshot
                    && !subscribers.is_empty()
                {
                    broadcast_snapshot(&handler.socket, &subscribers, &snapshot);
                }

                trace!("Waiting for osc package...");
                let (size, src) = match handler.socket.recv_from(&mut buf) {
                    Ok(received) => received,
                    Err(err)
                        if matches!(
                            err.kind(),
                            io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                        ) =>
                    {
                        continue;
                    }
                    Err(err) => {
                        debug!("Could not receive osc package ({err:?}). Retrying in 10ms...");
                        thread::sleep(Duration::from_millis(10));
                        continue;
                    }
                };

                crate::network_stats::add_incoming_bytes(size);

                trace!("Received packet with size {} from: {}", size, src);
                let Ok((_, packet)) = rosc::decoder::decode_udp(&buf[..size]) else {
                    warn!("Invalid osc packet from {src}");
                    continue;
                };

                match packet {
                    OscPacket::Message(msg) => {
                        trace!("OSC address: {}", msg.addr);
                        trace!("OSC arguments: {:?}", msg.args);
                        if let Some(command) = parse_client_interest_command(&msg) {
                            match command {
                                ClientInterestCommand::Subscribe => {
                                    subscribers.insert(src, Instant::now());
                                }
                                ClientInterestCommand::Unsubscribe => {
                                    subscribers.remove(&src);
                                }
                                ClientInterestCommand::Heartbeat => {
                                    subscribers.entry(src).and_modify(|t| *t = Instant::now());
                                }
                            }
                            handler
                                .has_subscribers
                                .store(!subscribers.is_empty(), Ordering::Relaxed);
                            continue;
                        }

                        if let Ok(action) = parse_message(&msg) {
                            UiAction::enqueue(action);
                        } else {
                            warn!("Invalid osc message: {} with args {:?}", msg.addr, msg.args);
                        }
                    }
                    OscPacket::Bundle(bundle) => {
                        warn!("OSC Bundle are not supported currently: {:?}", bundle);
                    }
                }
            }
        })
        .expect("Could not spawn osc receiver thread");
}

fn parse_client_interest_command(msg: &OscMessage) -> Option<ClientInterestCommand> {
    match msg.addr.as_str() {
        "/osc/state/subscribe" => Some(ClientInterestCommand::Subscribe),
        "/osc/state/unsubscribe" => Some(ClientInterestCommand::Unsubscribe),
        "/osc/state/heartbeat" => Some(ClientInterestCommand::Heartbeat),
        _ => None,
    }
}

fn broadcast_snapshot(
    socket: &UdpSocket,
    subscribers: &HashMap<SocketAddr, Instant>,
    snapshot: &OscStateSnapshot,
) {
    for subscriber in subscribers.keys().copied() {
        send_feedback(
            socket,
            subscriber,
            "/project/main_dimmer",
            vec![OscType::Float(
                snapshot.project.as_ref().map_or(1.0, |p| p.main_dimmer),
            )],
        );
        send_feedback(
            socket,
            subscriber,
            "/project/blackout",
            vec![OscType::Bool(snapshot.blackout)],
        );
        send_feedback(
            socket,
            subscriber,
            "/timing/bpm",
            vec![OscType::Float(snapshot.beats_per_minute)],
        );
        send_feedback(
            socket,
            subscriber,
            "/timing/beat_progression",
            vec![OscType::Float(snapshot.beat_progression)],
        );

        if let Some(project) = snapshot.project.as_ref() {
            // Project palette
            send_feedback(
                socket,
                subscriber,
                "/project/palette/enabled",
                vec![OscType::Bool(project.palette.is_some())],
            );
            if let Some(palette) = &project.palette {
                let [r, g, b] = palette.primary.rgb();
                send_feedback(
                    socket,
                    subscriber,
                    "/project/palette/primary",
                    vec![OscType::Float(r), OscType::Float(g), OscType::Float(b)],
                );
                let [r, g, b] = palette.secondary.rgb();
                send_feedback(
                    socket,
                    subscriber,
                    "/project/palette/secondary",
                    vec![OscType::Float(r), OscType::Float(g), OscType::Float(b)],
                );
                for (n, color) in palette.gradient.iter().enumerate() {
                    let [r, g, b] = color.rgb();
                    send_feedback(
                        socket,
                        subscriber,
                        &format!("/project/palette/gradient/{n}"),
                        vec![OscType::Float(r), OscType::Float(g), OscType::Float(b)],
                    );
                }
            }

            // Project groups
            for (idx, group) in project.groups.iter() {
                send_feedback(
                    socket,
                    subscriber,
                    &format!("/project/groups/{idx}"),
                    vec![OscType::String(group.0.clone())],
                );
            }

            // All grid scenes
            for (location, scene_instance) in &project.scenes_instances_grid {
                let (col, row) = (location.col, location.row);
                send_scene_instance_feedback(
                    socket,
                    subscriber,
                    &format!("/scene/grid/{col}/{row}"),
                    scene_instance,
                );
                match project.grid_highlight {
                    GridHighlight::Row if row == project.quick_row_index() => {
                        send_scene_instance_feedback(
                            socket,
                            subscriber,
                            &format!("/scene/quick/{col}"),
                            scene_instance,
                        );
                    }
                    GridHighlight::Column if col == project.grid_width().saturating_sub(1) => {
                        send_scene_instance_feedback(
                            socket,
                            subscriber,
                            &format!("/scene/quick/{row}"),
                            scene_instance,
                        );
                    }
                    GridHighlight::None | GridHighlight::Row | GridHighlight::Column => {}
                }
            }

            // Selected scene
            if let Some(scene_instance) = project
                .scenes_instances_grid
                .get(&snapshot.selected_scene_instance)
            {
                send_scene_instance_feedback(socket, subscriber, "/scene/selected", scene_instance);
            }
        }
    }
}

fn send_scene_instance_feedback(
    socket: &UdpSocket,
    addr: SocketAddr,
    prefix: &str,
    scene: &SceneInstance,
) {
    send_feedback(
        socket,
        addr,
        &format!("{prefix}/active"),
        vec![OscType::Bool(scene.active)],
    );
    send_feedback(
        socket,
        addr,
        &format!("{prefix}/opacity"),
        vec![OscType::Float(scene.opacity.multiplier)],
    );
    send_feedback(
        socket,
        addr,
        &format!("{prefix}/name"),
        vec![OscType::String(scene.name.clone())],
    );
    send_feedback(
        socket,
        addr,
        &format!("{prefix}/color"),
        vec![OscType::String(scene_color_name(scene.color).to_string())],
    );
    send_feedback(
        socket,
        addr,
        &format!("{prefix}/input_dimmer"),
        vec![OscType::Float(scene.input_dimmer)],
    );
    send_feedback(
        socket,
        addr,
        &format!("{prefix}/ignore_main_dimmer"),
        vec![OscType::Bool(scene.ignore_main_dimmer)],
    );
    send_feedback(
        socket,
        addr,
        &format!("{prefix}/beat_offset"),
        vec![OscType::Float(scene.beat_progression_offset.multiplier)],
    );
    send_feedback(
        socket,
        addr,
        &format!("{prefix}/set_offset_on_flash"),
        vec![OscType::Bool(scene.set_offset_on_flash)],
    );

    let effects = &scene.scene.effects;
    send_feedback(
        socket,
        addr,
        &format!("{prefix}/effect/count"),
        vec![OscType::Int(i32::try_from(effects.len()).unwrap_or(i32::MAX))],
    );
    for (i, effect) in effects.iter().enumerate() {
        send_feedback(
            socket,
            addr,
            &format!("{prefix}/effect/{i}/opacity"),
            vec![OscType::Float(effect.opacity.multiplier)],
        );
        send_feedback(
            socket,
            addr,
            &format!("{prefix}/effect/{i}/color_shift"),
            vec![OscType::Float(effect.color_shift.multiplier)],
        );
        send_feedback(
            socket,
            addr,
            &format!("{prefix}/effect/{i}/beat_progression"),
            vec![OscType::Float(effect.beat_progression.multiplier)],
        );
        send_feedback(
            socket,
            addr,
            &format!("{prefix}/effect/{i}/beat_offset"),
            vec![OscType::Float(effect.beat_progression_offset.multiplier)],
        );
        send_feedback(
            socket,
            addr,
            &format!("{prefix}/effect/{i}/speed_exponent"),
            vec![OscType::Int(effect.speed_exponent)],
        );
        send_feedback(
            socket,
            addr,
            &format!("{prefix}/effect/{i}/group_index"),
            vec![OscType::Int(
                i32::try_from(effect.group_index).unwrap_or(i32::MAX),
            )],
        );
    }
}

fn send_feedback(socket: &UdpSocket, addr: SocketAddr, path: &str, args: Vec<OscType>) {
    let packet = OscPacket::Message(OscMessage {
        addr: path.to_string(),
        args,
    });
    let Ok(bytes) = rosc::encoder::encode(&packet) else {
        return;
    };
    match socket.send_to(&bytes, addr) {
        Ok(count) => {
            crate::network_stats::add_outgoing_bytes(count);
        }
        Err(err) => {
            debug!("Could not send OSC feedback to {addr}: {err:?}");
        }
    }
}

fn scene_color_name(color: SceneInstanceColor) -> &'static str {
    match color {
        SceneInstanceColor::Red => "red",
        SceneInstanceColor::Green => "green",
        SceneInstanceColor::Blue => "blue",
        SceneInstanceColor::White => "white",
        SceneInstanceColor::Orange => "orange",
        SceneInstanceColor::Yellow => "yellow",
        SceneInstanceColor::Purple => "purple",
        SceneInstanceColor::Pink => "pink",
        SceneInstanceColor::Black => "black",
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::net::UdpSocket;
    use std::time::{Duration, Instant};

    use rosc::{OscPacket, OscType};

    use crate::storage::asset::scene::effect::Effect;
    use crate::storage::asset::scene::grid::GridLocation;
    use crate::storage::asset::{project::Project, scene::instance::SceneInstance};
    use crate::storage::collections::Collections;
    use crate::storage::asset_id::AssetId;

    use super::*;

    fn msg(addr: &str, args: Vec<OscType>) -> OscMessage {
        OscMessage {
            addr: addr.to_string(),
            args,
        }
    }

    #[test]
    fn parses_client_interest_subscribe_command() {
        let message = msg("/osc/state/subscribe", vec![]);
        assert_eq!(
            parse_client_interest_command(&message),
            Some(ClientInterestCommand::Subscribe)
        );
    }

    #[test]
    fn parses_client_interest_unsubscribe_command() {
        let message = msg("/osc/state/unsubscribe", vec![]);
        assert_eq!(
            parse_client_interest_command(&message),
            Some(ClientInterestCommand::Unsubscribe)
        );
    }

    #[test]
    fn parses_client_interest_heartbeat_command() {
        let message = msg("/osc/state/heartbeat", vec![]);
        assert_eq!(
            parse_client_interest_command(&message),
            Some(ClientInterestCommand::Heartbeat)
        );
    }

    #[test]
    fn broadcast_snapshot_sends_core_fields_to_subscriber() {
        let sender = UdpSocket::bind("127.0.0.1:0").unwrap();
        let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
        receiver
            .set_read_timeout(Some(Duration::from_millis(200)))
            .unwrap();
        let receiver_addr = receiver.local_addr().unwrap();

        let mut subscribers = HashMap::new();
        subscribers.insert(receiver_addr, Instant::now());

        let snapshot = OscStateSnapshot {
            project: None,
            selected_scene_instance: GridLocation { col: 0, row: 0 },
            blackout: true,
            beats_per_minute: 120.0,
            beat_progression: 0.25,
        };
        broadcast_snapshot(&sender, &subscribers, &snapshot);

        let mut found_blackout = false;
        let mut found_bpm = false;
        let mut buf = [0u8; rosc::decoder::MTU];
        while let Ok((size, _)) = receiver.recv_from(&mut buf) {
            if let Ok((_, OscPacket::Message(m))) = rosc::decoder::decode_udp(&buf[..size]) {
                match m.addr.as_str() {
                    "/project/blackout" => {
                        assert_eq!(m.args, vec![OscType::Bool(true)]);
                        found_blackout = true;
                    }
                    "/timing/bpm" => {
                        assert_eq!(m.args, vec![OscType::Float(120.0)]);
                        found_bpm = true;
                    }
                    _ => {}
                }
            }
            if found_blackout && found_bpm {
                break;
            }
        }
        assert!(found_blackout, "/project/blackout packet not received");
        assert!(found_bpm, "/timing/bpm packet not received");
    }

    #[test]
    fn broadcast_snapshot_sends_palette_fields() {
        use crate::storage::asset::palette::Palette;

        let sender = UdpSocket::bind("127.0.0.1:0").unwrap();
        let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
        receiver
            .set_read_timeout(Some(Duration::from_millis(200)))
            .unwrap();
        let receiver_addr = receiver.local_addr().unwrap();

        let mut subscribers = HashMap::new();
        subscribers.insert(receiver_addr, Instant::now());

        let mut palette = Palette::default();
        *palette.primary.rgb_mut() = [1.0, 0.0, 0.0];

        let mut project = Project::default();
        project.palette = Some(palette);

        let snapshot = OscStateSnapshot {
            project: Some(project),
            selected_scene_instance: GridLocation { col: 0, row: 0 },
            blackout: false,
            beats_per_minute: 90.0,
            beat_progression: 0.0,
        };
        broadcast_snapshot(&sender, &subscribers, &snapshot);

        let mut found_primary = false;
        let mut buf = [0u8; rosc::decoder::MTU];
        while let Ok((size, _)) = receiver.recv_from(&mut buf) {
            if let Ok((_, OscPacket::Message(m))) = rosc::decoder::decode_udp(&buf[..size])
                && m.addr == "/project/palette/primary"
            {
                assert_eq!(
                    m.args,
                    vec![
                        OscType::Float(1.0),
                        OscType::Float(0.0),
                        OscType::Float(0.0)
                    ]
                );
                found_primary = true;
                break;
            }
        }
        assert!(
            found_primary,
            "/project/palette/primary packet not received"
        );
    }

    #[test]
    fn broadcast_snapshot_sends_extended_scene_fields_for_grid_and_quick() {
        let sender = UdpSocket::bind("127.0.0.1:0").unwrap();
        let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
        receiver
            .set_read_timeout(Some(Duration::from_millis(300)))
            .unwrap();
        let receiver_addr = receiver.local_addr().unwrap();

        let mut subscribers = HashMap::new();
        subscribers.insert(receiver_addr, Instant::now());

        let mut project = Project::default();
        let location = GridLocation {
            col: 2,
            row: project.quick_row_index(),
        };
        let mut scene_instance = SceneInstance::from_scene_id(
            AssetId::default(),
            &Collections::default(),
        );
        scene_instance.input_dimmer = 0.42;
        scene_instance.ignore_main_dimmer = true;
        scene_instance.scene.effects.push(Effect::default());
        project.add_scene_instance(location, scene_instance);

        let snapshot = OscStateSnapshot {
            project: Some(project),
            selected_scene_instance: location,
            blackout: false,
            beats_per_minute: 90.0,
            beat_progression: 0.0,
        };
        broadcast_snapshot(&sender, &subscribers, &snapshot);

        let grid_input_dimmer_path = format!("/scene/grid/{}/{}/input_dimmer", location.col, location.row);
        let grid_effect_count_path = format!("/scene/grid/{}/{}/effect/count", location.col, location.row);
        let quick_ignore_main_dimmer_path = format!("/scene/quick/{}/ignore_main_dimmer", location.col);
        let quick_effect_opacity_path = format!("/scene/quick/{}/effect/0/opacity", location.col);

        let mut found_grid_input_dimmer = false;
        let mut found_grid_effect_count = false;
        let mut found_quick_ignore_main_dimmer = false;
        let mut found_quick_effect_opacity = false;

        let mut buf = [0u8; rosc::decoder::MTU];
        while let Ok((size, _)) = receiver.recv_from(&mut buf) {
            if let Ok((_, OscPacket::Message(m))) = rosc::decoder::decode_udp(&buf[..size]) {
                if m.addr == grid_input_dimmer_path {
                    assert_eq!(m.args, vec![OscType::Float(0.42)]);
                    found_grid_input_dimmer = true;
                }
                if m.addr == grid_effect_count_path {
                    assert_eq!(m.args, vec![OscType::Int(1)]);
                    found_grid_effect_count = true;
                }
                if m.addr == quick_ignore_main_dimmer_path {
                    assert_eq!(m.args, vec![OscType::Bool(true)]);
                    found_quick_ignore_main_dimmer = true;
                }
                if m.addr == quick_effect_opacity_path {
                    assert_eq!(m.args, vec![OscType::Float(1.0)]);
                    found_quick_effect_opacity = true;
                }
            }
            if found_grid_input_dimmer
                && found_grid_effect_count
                && found_quick_ignore_main_dimmer
                && found_quick_effect_opacity
            {
                break;
            }
        }

        assert!(
            found_grid_input_dimmer,
            "{grid_input_dimmer_path} packet not received"
        );
        assert!(
            found_grid_effect_count,
            "{grid_effect_count_path} packet not received"
        );
        assert!(
            found_quick_ignore_main_dimmer,
            "{quick_ignore_main_dimmer_path} packet not received"
        );
        assert!(
            found_quick_effect_opacity,
            "{quick_effect_opacity_path} packet not received"
        );
    }
}

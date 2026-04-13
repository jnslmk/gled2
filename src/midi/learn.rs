use crate::storage::asset::midi_controller::MidiController;
use crate::storage::{asset::midi_controller::MidiTrigger, asset_id::AssetId};
use kanal::{Receiver, Sender, unbounded};
use once_cell::sync::OnceCell;
use std::{
    collections::HashMap,
    sync::atomic::{AtomicBool, Ordering::Relaxed},
    time::{Duration, Instant},
};

static STREAM_ENABLED: AtomicBool = AtomicBool::new(false);
static LEARN_ACTIVE: AtomicBool = AtomicBool::new(false);
static SENDER: OnceCell<Sender<[u8; 3]>> = OnceCell::new();

#[derive(Clone, Copy)]
pub struct MidiLearnRequest {
    pub controller_id: AssetId<MidiController>,
    pub target: MidiLearnTarget,
}

#[derive(Clone)]
struct LearnSession {
    request: MidiLearnRequest,
    started_at: Instant,
    last_seen_at: Option<Instant>,
    stats: HashMap<(u8, u8), LearnStats>,
}

#[derive(Clone, Copy)]
struct LearnStats {
    count: u32,
    min_value: u8,
    max_value: u8,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MidiLearnTarget {
    InputBinding(usize),
    OutputBinding(usize),
}

#[derive(Clone)]
pub struct MidiLearnCapture {
    pub controller_id: AssetId<MidiController>,
    pub target: MidiLearnTarget,
    pub trigger: MidiTrigger,
}

pub fn init() -> Receiver<[u8; 3]> {
    let (sender, receiver) = unbounded();
    let _ = SENDER.set(sender);
    receiver
}

pub struct LearnState {
    receiver: Receiver<[u8; 3]>,
    request: Option<LearnSession>,
    captured: Vec<MidiLearnCapture>,
}

impl LearnState {
    pub fn new(receiver: Receiver<[u8; 3]>) -> Self {
        Self {
            receiver,
            request: None,
            captured: Vec::new(),
        }
    }

    pub fn set_streaming_enabled(&mut self, enabled: bool) {
        set_streaming_enabled(enabled);

        if !enabled {
            self.request = None;
        }
    }

    pub fn arm(&mut self, request: MidiLearnRequest) {
        self.request.replace(LearnSession {
            request,
            started_at: Instant::now(),
            last_seen_at: None,
            stats: HashMap::new(),
        });
        LEARN_ACTIVE.store(true, Relaxed);
    }

    pub fn active_request(&self) -> Option<MidiLearnRequest> {
        self.request.as_ref().map(|session| session.request)
    }

    pub fn flush_timeouts(&mut self) {
        self.poll_captures();

        let now = Instant::now();
        let Some(session) = self.request.as_ref() else {
            return;
        };

        if !should_finalize(session, now) {
            return;
        }

        if let Some(session) = self.request.take()
            && let Some(capture) = finalize_session(session)
        {
            LEARN_ACTIVE.store(false, Relaxed);
            self.captured.push(capture);
        }
    }

    fn poll_captures(&mut self) {
        while let Ok(Some(message)) = self.receiver.try_recv() {
            let now = Instant::now();
            let mut maybe_finalized = None;

            let Some(session) = self.request.as_mut() else {
                continue;
            };

            let key = (message[0], message[1]);
            let value = message[2];
            session.last_seen_at = Some(now);
            session
                .stats
                .entry(key)
                .and_modify(|stats| {
                    stats.count += 1;
                    stats.min_value = stats.min_value.min(value);
                    stats.max_value = stats.max_value.max(value);
                })
                .or_insert(LearnStats {
                    count: 1,
                    min_value: value,
                    max_value: value,
                });

            if should_finalize(session, now) {
                maybe_finalized = self.request.take();
            }

            if let Some(session) = maybe_finalized
                && let Some(capture) = finalize_session(session)
            {
                LEARN_ACTIVE.store(false, Relaxed);
                self.captured.push(capture);
            }
        }
    }

    pub fn pop_capture(&mut self) -> Option<MidiLearnCapture> {
        self.captured.pop()
    }
}

pub fn set_streaming_enabled(enabled: bool) {
    STREAM_ENABLED.store(enabled, Relaxed);

    if !enabled {
        LEARN_ACTIVE.store(false, Relaxed);
    }
}

pub fn capture(message: &[u8]) -> bool {
    if message.len() != 3 {
        return false;
    }

    if !STREAM_ENABLED.load(Relaxed) || !LEARN_ACTIVE.load(Relaxed) {
        return false;
    }

    if let Some(sender) = SENDER.get() {
        let _ = sender.send([message[0], message[1], message[2]]);
        return true;
    }

    false
}

fn should_finalize(session: &LearnSession, now: Instant) -> bool {
    const MIN_MESSAGES: u32 = 8;
    const QUIET_TIME: Duration = Duration::from_millis(180);
    const MAX_LEARN_TIME: Duration = Duration::from_secs(2);

    let total_messages: u32 = session.stats.values().map(|stats| stats.count).sum();
    if total_messages >= MIN_MESSAGES {
        return true;
    }

    if now.duration_since(session.started_at) >= MAX_LEARN_TIME && total_messages > 0 {
        return true;
    }

    if let Some(last_seen_at) = session.last_seen_at
        && now.duration_since(last_seen_at) >= QUIET_TIME
        && total_messages > 0
    {
        return true;
    }

    false
}

fn finalize_session(session: LearnSession) -> Option<MidiLearnCapture> {
    let (status, data1) = best_trigger(&session.stats)?;

    Some(MidiLearnCapture {
        controller_id: session.request.controller_id,
        target: session.request.target,
        trigger: MidiTrigger {
            status,
            data1,
            match_data1: match_data1_for_status(status),
        },
    })
}

fn best_trigger(stats: &HashMap<(u8, u8), LearnStats>) -> Option<(u8, u8)> {
    let mut canonical: HashMap<(u8, u8), (u32, u8)> = HashMap::new();

    for (&(status, data1), message_stats) in stats {
        let canonical_data1 = if status & 0xF0 == 0xB0 && (32..=63).contains(&data1) {
            data1 - 32
        } else {
            data1
        };

        let value_span = message_stats
            .max_value
            .saturating_sub(message_stats.min_value);
        canonical
            .entry((status, canonical_data1))
            .and_modify(|(count, span)| {
                *count += message_stats.count;
                *span = (*span).max(value_span);
            })
            .or_insert((message_stats.count, value_span));
    }

    canonical
        .into_iter()
        .max_by_key(|(_, (count, span))| ((*count as u64) << 8) + (*span as u64))
        .map(|(key, _)| key)
}

fn match_data1_for_status(status: u8) -> bool {
    !matches!(status & 0xF0, 0xC0 | 0xD0 | 0xE0)
}

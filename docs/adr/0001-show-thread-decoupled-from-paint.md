# Show rendering runs on a dedicated thread, decoupled from UI paint

The show — timing tick, scene GPU render, GPU→CPU readback and Art-Net/DMX send — must run at the fps limiter's rate regardless of window visibility or compositor pacing. It is currently scheduled inside eframe's frame callbacks (`App::logic`), which is why hiding the window ever stalled output and why the codebase carries a cadence watchdog, an `eframe::SKIP_PAINTING` fork patch, and a niri IPC watcher to keep `logic` running while hidden. We decided to move the whole show tick onto a dedicated thread; the UI thread keeps egui rendering and editing only.

## Decision details

- **Shared state, not snapshots**: `Project` and `Timing` move behind `Arc<Mutex<…>>` (egui's mutex). The ~18 `&mut project` sites in `src/app`/`src/ui` adapt mechanically. Rejected alternative: pushing `Project` clones to the show thread on edit — it would require proving that nothing the render path mutates (uniform caches, artnet-in dimmer state) is ever read back by the UI, a standing correctness proof instead of a compiler-enforced one.
- **Full tick scope moves**: `timing.tick`, `sound_data.update`, `render_project`, `external_control_state.process_events`, `Input::tick`, and the `ProjectState` broadcast (OSC/MIDI feedback). MIDI/gamepad/Art-Net-driven scene control must keep working while the window is hidden, so input polling lives with the show, not the UI. `App::logic` shrinks to UI-only duties and no longer needs to run while hidden.
- **Hidden-window machinery stays**: the cadence watchdog, the `SKIP_PAINTING` fork patch and the niri watcher keep their current semantics; after decoupling they only stop wasted GPU work on an invisible window — output no longer depends on detection latency.
- **Incremental cutover**, three independently landable steps: (1) show-thread skeleton owning `timing.tick` + `ProjectState` broadcast; (2) GPU render and preview-texture ownership move; (3) input/external-control move, `logic` cleanup.

## Consequences

- `SoundData` is single-owner: the show thread holds the instance (and its channel receiver); UI editors read trigger config from the existing `audio::sound_data` statics.
- `Preview` (and other egui-registered preview textures) become show-thread-owned; `TextureId`s are published for the UI thread. UI preview sampling may lag the show by one frame — acceptable for visuals.
- UI edits and show renders contend on the project mutex for milliseconds; an occasional one-frame slip of the show tick under heavy editing is accepted (lighting frame rates, not hard real time).

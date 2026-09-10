# gled — domain glossary

Terms as used in this codebase. Implementation details live in code and `docs/adr/`, not here.

## Show

The time-driven part of the console: beat clock, scene animation rendering, and the resulting Art-Net/DMX output. The show must run at the configured frame rate whether or not any window is visible, focused, or being painted.

## Paint (UI)

The egui window rendering: menus, editors, previews. Paint cadence follows the compositor and user interaction; it observes the show but never gates it.

## Blackout

A master override that forces rendered output values to zero while the show keeps running. Distinct from pausing or muting: timing and rendering continue, only the emitted levels are forced to zero.

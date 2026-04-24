gled is an application for creating spatial animations on light installations.

# Features
* All animations can be based on beat progression.
* Everything is hackable directly in gled:
    * Animation shaders with a shader editor and preview
    * Color palettes
    * Scenes - a collection of animations
* Support for multiple kind of output devices:
    * Artnet
    * DMX, currently the Enttec DMX USB Pro is supported

# Installation

Download and install a [Release](https://gitlab.com/photonenkollektiv/gled2/-/releases).

Linux RPMs contain a `yum.repos.d` file so that gled is automatically updated with your system.

The Apple developer account for the signed Mac application is sponsored by [FreshX GmbH](https://freshx.de/).

# Documentation

The documentation is available at [photonenkollektiv.gitlab.io/gled2](https://photonenkollektiv.gitlab.io/gled2).

# OSC API (Work In Progress)

OSC input is received on UDP port `8000`.

This implementation introduces hierarchical addresses for project and scene control.

## Project and Timing

* `/osc/state/subscribe`
* `/osc/state/unsubscribe`
* `/project/main_dimmer <float>`
* `/project/blackout <bool|0|1>`
* `/project/palette <palette_uuid_string|none>`
* `/project/palette/primary <r> <g> <b>`
* `/project/palette/secondary <r> <g> <b>`
* `/project/palette/gradient/{gradient_index} <r> <g> <b>`
* `/project/artnet_control/active <bool|0|1>`
* `/project/artnet_control/universe <int>`
* `/project/groups/clear`
* `/project/groups/{group_index} <group_name>`
* `/project/groups/remove/{group_index}`
* `/project/input/tap/clear`
* `/project/input/tap/add/artnet <channel>`
* `/project/input/tap/remove/artnet <channel>`
* `/project/input/blackout/clear`
* `/project/input/blackout/add/artnet <channel>`
* `/project/input/blackout/remove/artnet <channel>`
* `/project/input/blackout_hold/clear`
* `/project/input/blackout_hold/add/artnet <channel>`
* `/project/input/blackout_hold/remove/artnet <channel>`
* `/project/input/half/clear`
* `/project/input/half/add/artnet <channel>`
* `/project/input/half/remove/artnet <channel>`
* `/project/input/double/clear`
* `/project/input/double/add/artnet <channel>`
* `/project/input/double/remove/artnet <channel>`
* `/timing/tap`
* `/timing/speed/add <float>`
* `/timing/speed/multiply <float>`

## Scene Selectors

* Grid selector: `/scene/grid/{col}/{row}/...`
* Quick-row selector: `/scene/quick/{index}/...`
* Selected scene alias: `/scene/selected/...`

Grid uses `0 <= col < 8` and `0 <= row < 5`.
Quick index uses `0 <= index < 8`.
The selected-scene alias resolves against the currently selected scene when the command is handled.

## Scene Commands

Supported for `grid`, `quick`, and `selected` selectors:

* `.../opacity <float>`
* `.../opacity/value <float>` — set the multiplier only (preserves any selected curve)
* `.../opacity/static <float>` — set a static value (replaces curve with multiplier-only)
* `.../opacity/curve <curve_uuid_string|none>` — set or clear the curve (preserves the multiplier)
* `.../active <bool|0|1>`
* `.../toggle`
* `.../select`
* `.../name <string>`
* `.../color <red|green|blue|white|orange|yellow|purple|pink|black>`
* `.../input_dimmer <float>`
* `.../ignore_main_dimmer <bool|0|1>`
* `.../beat_offset <float>`
* `.../beat_offset/value <float>` — set the multiplier only (preserves any selected curve)
* `.../beat_offset/static <float>` — set a static value (replaces curve with multiplier-only)
* `.../beat_offset/curve <curve_uuid_string|none>` — set or clear the curve (preserves the multiplier)
* `.../set_offset_on_flash <bool|0|1>`
* `.../activation_input/clear`
* `.../activation_input/artnet <channel>`
* `.../flash_input/clear`
* `.../flash_input/artnet <channel>`
* `.../dimmer_input/clear`
* `.../dimmer_input/artnet <channel>`
* `.../palette_overwrite <inherit|none|palette_uuid_string>`
* `.../palette_overwrite/primary <r> <g> <b>`
* `.../palette_overwrite/secondary <r> <g> <b>`
* `.../palette_overwrite/gradient/{gradient_index} <r> <g> <b>`
* `.../groups_overwrite/clear`
* `.../groups_overwrite/{group_index} <group_name>`
* `.../groups_overwrite/remove/{group_index}`
* `.../clone`
* `.../delete`

Setting `.../opacity` also activates the targeted scene instance.
Setting `.../opacity/value` and `.../opacity/curve` do **not** activate the scene.
`.../opacity/static` behaves like `.../opacity` and activates the targeted scene instance.

Additional grid-only commands:

* `/scene/grid/{col}/{row}/add <scene_uuid_string>`

Scene reorder command:

* `/scene/reorder/{from_col}/{from_row}/{to_col}/{to_row}`

## Effect Commands

Effect commands are available on grid, quick, and selected scene selectors.

Base effect path:

* `/scene/grid/{col}/{row}/effect/{effect_index}/...`
* `/scene/quick/{index}/effect/{effect_index}/...`
* `/scene/selected/effect/{effect_index}/...`

Supported effect commands:

* `.../add` (for `/effect/add` path)
* `.../opacity <float>`
* `.../opacity/value <float>` — set the multiplier only (preserves any selected curve)
* `.../opacity/static <float>` — set a static value (replaces curve with multiplier-only)
* `.../opacity/curve <curve_uuid_string|none>` — set or clear the curve (preserves the multiplier)
* `.../color_shift <float>`
* `.../color_shift/value <float>`
* `.../color_shift/static <float>`
* `.../color_shift/curve <curve_uuid_string|none>`
* `.../beat_progression <float>`
* `.../beat_progression/value <float>`
* `.../beat_progression/static <float>`
* `.../beat_progression/curve <curve_uuid_string|none>`
* `.../beat_offset <float>`
* `.../beat_offset/value <float>`
* `.../beat_offset/static <float>`
* `.../beat_offset/curve <curve_uuid_string|none>`
* `.../speed_exponent <int>`
* `.../group_index <int>`
* `.../animation <animation_uuid_string|none>`
* `.../clone`
* `.../delete`
* `.../config/u32/{config_index} <int>`
* `.../config/f32/{config_index} <float>`

## Palette Behavior

Project palette selection copies the chosen palette asset into the project.
Subsequent OSC palette color edits modify only the project-local palette state and do not mutate the source palette asset.

Scene palette overwrite selection also copies palette asset data into the scene instance override.
Direct scene palette overwrite color edits create or replace the scene-local override palette and do not touch the source asset.

OSC feedback synchronization is only active for clients that explicitly subscribe.
`/osc/state/subscribe` and `/osc/state/unsubscribe` use the UDP source address/port of the client message as feedback destination.
For scene feedback, subscribed clients receive the same field families on `/scene/grid/{col}/{row}/...` and `/scene/quick/{index}/...` as on `/scene/selected/...` for overlapping scene/effect state (`input_dimmer`, `ignore_main_dimmer`, `beat_offset`, `set_offset_on_flash`, `effect/count`, and effect parameter fields).

## Animation Asset Editing

Animation asset shader and argument edits by UUID:

* `/asset/animation/{animation_uuid}/shader_code <string>`
* `/asset/animation/{animation_uuid}/argument/add`
* `/asset/animation/{animation_uuid}/argument/{argument_index}/remove`
* `/asset/animation/{animation_uuid}/argument/{argument_index}/name <string>`
* `/asset/animation/{animation_uuid}/argument/{argument_index}/kind <center|selection|slider|checkbox|percentage|degrees>`

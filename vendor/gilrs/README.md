GilRs - Game Input Library for Rust
===================================

[![pipeline status](https://gitlab.com/gilrs-project/gilrs/badges/master/pipeline.svg)](https://gitlab.com/gilrs-project/gilrs/commits/master)
[![Crates.io](https://img.shields.io/crates/v/gilrs.svg)](https://crates.io/crates/gilrs)
[![Documentation](https://docs.rs/gilrs/badge.svg)](https://docs.rs/gilrs/)
[![Minimum rustc version](https://img.shields.io/badge/rustc-1.47.0+-yellow.svg)](https://gitlab.com/gilrs-project/gilrs)

[**Documentation (master)**](https://gilrs-project.gitlab.io/gilrs/doc/gilrs/)

GilRs abstract platform specific APIs to provide unified interfaces for working with gamepads.

Main features:

- Unified gamepad layout—buttons and axes are represented by familiar names
- Support for SDL2 mappings including `SDL_GAMECONTROLLERCONFIG` environment
  variable which Steam uses
- Hotplugging—GilRs will try to assign new ID for new gamepads and reuse same
  ID for gamepads which reconnected
- Force feedback (rumble)
- Power information (is gamepad wired, current battery status)

The project's main repository [is on GitLab](https://gitlab.com/gilrs-project/gilrs)
although there is also a [GitHub mirror](https://github.com/Arvamer/gilrs).
Please use GitLab's issue tracker and merge requests.

This repository contains submodule; after you clone it, don't forget to run
`git submodule init; git submodule update` (or clone with `--recursive` flag)
or you will get compile errors.

Example
-------

```toml
[dependencies]
gilrs = "0.8.1"
```

```rust
use gilrs::{Gilrs, Button, Event};

let mut gilrs = Gilrs::new().unwrap();

// Iterate over all connected gamepads
for (_id, gamepad) in gilrs.gamepads() {
    println!("{} is {:?}", gamepad.name(), gamepad.power_info());
}

let mut active_gamepad = None;

loop {
    // Examine new events
    while let Some(Event { id, event, time }) = gilrs.next_event() {
        println!("{:?} New event from {}: {:?}", time, id, event);
        active_gamepad = Some(id);
    }

    // You can also use cached gamepad state
    if let Some(gamepad) = active_gamepad.map(|id| gilrs.gamepad(id)) {
        if gamepad.is_pressed(Button::South) {
            println!("Button South is pressed (XBox - A, PS - X)");
        }
    }
}
```

Supported features
------------------

|                  | Input | Hotplugging | Force feedback |
|------------------|:-----:|:-----------:|:--------------:|
| Linux/BSD (evdev)|   ✓   |      ✓      |        ✓       |
| Windows (XInput) |   ✓   |      ✓      |        ✓       |
| OS X             |   ✓   |      ✓      |        ✕       |
| Wasm             |   ✓   |      ✓      |       n/a      |
| Android          |   ✕   |      ✕      |        ✕       |


Platform specific notes
======================

Linux/BSD (evdev)
-----

With evdev, GilRs read (and write, in case of force feedback) directly from appropriate
`/dev/input/event*` file. This mean that user have to have read and write access to this file.
On most distros it shouldn't be a problem, but if it is, you will have to create udev rule.
On FreeBSD generic HID gamepads use hgame(4) and special use Linux driver via `webcamd`.

To build GilRs, you will need pkg-config and libudev .pc file. On some distributions this file
is packaged in separate archive (e.g., `libudev-dev` in Debian, `libudev-devd` in FreeBSD).

Wasm
----

Wasm implementation uses stdweb, or wasm-bindgen with the wasm-bindgen feature.
For stdweb, you will need [cargo-web](https://github.com/koute/cargo-web) to build gilrs for
wasm32-unknown-unknown. For wasm-bindgen, you will need the wasm-bindgen cli or a tool like
[wasm-pack](https://rustwasm.github.io/wasm-pack/installer/).
Unlike other platforms, events are only generated when you call `Gilrs::next_event()`.

License
=======

This project is licensed under the terms of both the Apache License (Version 2.0) and the MIT
license. See LICENSE-APACHE and LICENSE-MIT for details.

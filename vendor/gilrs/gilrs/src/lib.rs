// Copyright 2016-2018 Mateusz Sieczko and other GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

//! GilRs - Game Input Library for Rust
//! ===================================
//!
//! GilRs abstract platform specific APIs to provide unified interfaces for working with gamepads.
//!
//! Main features:
//!
//! - Unified gamepad layout—buttons and axes are represented by familiar names
//! - Support for SDL2 mappings including `SDL_GAMECONTROLLERCONFIG` environment
//!   variable which Steam uses
//! - Hotplugging—GilRs will try to assign new IDs for new gamepads and reuse same
//!   ID for gamepads which reconnected
//! - Force feedback (rumble)
//! - Power information (is gamepad wired, current battery status)
//!
//! Example
//! -------
//!
//! ```
//! use gilrs::{Gilrs, Button, Event};
//!
//! let mut gilrs = Gilrs::new().unwrap();
//!
//! // Iterate over all connected gamepads
//! for (_id, gamepad) in gilrs.gamepads() {
//!     println!("{} is {:?}", gamepad.name(), gamepad.power_info());
//! }
//!
//! let mut active_gamepad = None;
//!
//! loop {
//!     // Examine new events
//!     while let Some(Event { id, event, time }) = gilrs.next_event() {
//!         println!("{:?} New event from {}: {:?}", time, id, event);
//!         active_gamepad = Some(id);
//!     }
//!
//!     // You can also use cached gamepad state
//!     if let Some(gamepad) = active_gamepad.map(|id| gilrs.gamepad(id)) {
//!         if gamepad.is_pressed(Button::South) {
//!             println!("Button South is pressed (XBox - A, PS - X)");
//!         }
//!     }
//!     # break;
//! }
//! ```
//!
//! Supported features
//! ------------------
//!
//! |                  | Input | Hotplugging | Force feedback |
//! |------------------|:-----:|:-----------:|:--------------:|
//! | Linux/BSD (evdev)|   ✓   |      ✓      |        ✓       |
//! | Windows (XInput) |   ✓   |      ✓      |        ✓       |
//! | OS X             |   ✓   |      ✓      |        ✕       |
//! | Wasm             |   ✓   |      ✓      |       n/a      |
//! | Android          |   ✕   |      ✕      |        ✕       |
//!
//! Controller layout
//! -----------------
//!
//! ![Controller layout](https://gilrs-project.gitlab.io/gilrs/img/controller.svg)
//! [original image by nicefrog](http://opengameart.org/content/generic-gamepad-template)
//!
//! Mappings
//! --------
//!
//! GilRs use SDL-compatible controller mappings to fix on Linux legacy drivers that doesn't follow
//! [Linux Gamepad API](https://www.kernel.org/doc/Documentation/input/gamepad.txt) and to provide
//! unified button layout for platforms that doesn't make any guarantees about it. The main source
//! is [SDL_GameControllerDB](https://github.com/gabomdq/SDL_GameControllerDB), but library also
//! support loading mappings from environment variable `SDL_GAMECONTROLLERCONFIG` (which Steam
//! use).
//!
//! Cargo features
//! --------------
//!
//! - `serde-serialize` - enable deriving of serde's `Serialize` and `Deserialize` for
//!   various types.
//!
//! Platform specific notes
//! ======================
//!
//! Linux/BSD (evdev)
//! -----
//!
//! With evdev, GilRs read (and write, in case of force feedback) directly from appropriate
//! `/dev/input/event*` file. This mean that user have to have read and write access to this file.
//! On most distros it shouldn't be a problem, but if it is, you will have to create udev rule.
//! On FreeBSD generic HID gamepads use hgame(4) and special use Linux driver via `webcamd`.
//!
//! To build GilRs, you will need pkg-config and libudev .pc file. On some distributions this file
//! is packaged in separate archive (e.g., `libudev-dev` in Debian, `libudev-devd` in FreeBSD).
//!
//! Wasm
//! ----
//!
//! Wasm implementation uses stdweb, or wasm-bindgen with the wasm-bindgen feature.
//! For stdweb, you will need [cargo-web](https://github.com/koute/cargo-web) to build gilrs for
//! wasm32-unknown-unknown. For wasm-bindgen, you will need the wasm-bindgen cli or a tool like
//! [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/).
//! Unlike other platforms, events are only generated when you call `Gilrs::next_event()`.

#[macro_use]
extern crate log;

mod constants;
mod gamepad;
mod mapping;
mod utils;

pub mod ev;
pub mod ff;

pub use crate::ev::filter::Filter;
pub use crate::ev::{Axis, Button, Event, EventType};
pub use crate::gamepad::{
    ConnectedGamepadsIterator, Error, Gamepad, GamepadId, Gilrs, GilrsBuilder, MappingSource,
    PowerInfo,
};
pub use crate::mapping::{MappingData as Mapping, MappingError};

// Copyright 2016-2018 Mateusz Sieczko and other GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use super::FfDevice;
use crate::{AxisInfo, Event, EventType, PlatformError, PowerInfo};
use uuid::Uuid;

use std::collections::VecDeque;
#[cfg(not(feature = "wasm-bindgen"))]
use stdweb::web::{Gamepad as WebGamepad, GamepadMappingType};
#[cfg(feature = "wasm-bindgen")]
use web_sys::{Gamepad as WebGamepad, GamepadButton, GamepadMappingType};

use std::fmt::{Display, Formatter, Result as FmtResult};
use std::i32::MAX as I32_MAX;

#[derive(Debug)]
pub struct Gilrs {
    gamepads: Vec<Gamepad>,
    event_cache: VecDeque<Event>,
}

impl Gilrs {
    pub(crate) fn new() -> Result<Self, PlatformError> {
        Ok({
            Gilrs {
                gamepads: Vec::new(),
                event_cache: VecDeque::new(),
            }
        })
    }

    pub(crate) fn next_event(&mut self) -> Option<Event> {
        // Don't duplicate the work of checking the diff between the old and new gamepads if
        // there are still events to return
        if !self.event_cache.is_empty() {
            return self.event_cache.pop_front();
        }

        #[cfg(not(feature = "wasm-bindgen"))]
        let gamepads = WebGamepad::get_all().into_iter();

        #[cfg(feature = "wasm-bindgen")]
        let gamepads = web_sys::window()
            .expect("no window")
            .navigator()
            .get_gamepads()
            .expect("error getting gamepads");
        #[cfg(feature = "wasm-bindgen")]
        let gamepads = gamepads.iter().map(|val| {
            if val.is_null() {
                None
            } else {
                Some(WebGamepad::from(val))
            }
        });

        let new_gamepads: Vec<_> = gamepads.flatten().map(Gamepad::new).collect();
        let mut old_index = 0;
        let mut new_index = 0;

        loop {
            match (self.gamepads.get(old_index), new_gamepads.get(new_index)) {
                (Some(old), Some(new)) if old.gamepad.index() == new.gamepad.index() => {
                    let index = old.index();

                    // Compare the two gamepads and generate events
                    let buttons = old.mapping.buttons().zip(new.mapping.buttons()).enumerate();
                    for (btn_index, (old_button, new_button)) in buttons {
                        let ev_code = crate::EvCode(new.button_code(btn_index));
                        match (old_button, new_button) {
                            (false, true) => self
                                .event_cache
                                .push_back(Event::new(index, EventType::ButtonPressed(ev_code))),
                            (true, false) => self
                                .event_cache
                                .push_back(Event::new(index, EventType::ButtonReleased(ev_code))),
                            _ => (),
                        }
                    }

                    let axes = old.mapping.axes().zip(new.mapping.axes()).enumerate();
                    for (axis_index, (old_axis, new_axis)) in axes {
                        if old_axis != new_axis {
                            let ev_code = crate::EvCode(new.axis_code(axis_index));
                            let value = (new_axis * I32_MAX as f64) as i32;
                            self.event_cache.push_back(Event::new(
                                index,
                                EventType::AxisValueChanged(value, ev_code),
                            ));
                        }
                    }
                    old_index += 1;
                    new_index += 1;
                }
                (Some(old), Some(new)) if old.gamepad.index() > new.gamepad.index() => {
                    // Create a connected event
                    self.event_cache
                        .push_back(Event::new(new.index(), EventType::Connected));
                    new_index += 1;
                }
                (Some(old), Some(_new)) => {
                    // Create a disconnect event
                    self.event_cache
                        .push_back(Event::new(old.index(), EventType::Disconnected));
                    old_index += 1;
                }
                (Some(old), None) => {
                    // Create a disconnect event
                    self.event_cache
                        .push_back(Event::new(old.index(), EventType::Disconnected));
                    old_index += 1;
                }
                (None, Some(new)) => {
                    // Create a connected event
                    let index = new.index();
                    let event = Event::new(index, EventType::Connected);
                    self.event_cache.push_back(event);
                    new_index += 1;
                }
                (None, None) => {
                    break;
                }
            }
        }

        self.gamepads = new_gamepads;
        self.event_cache.pop_front()
    }

    pub fn gamepad(&self, id: usize) -> Option<&Gamepad> {
        self.gamepads.get(id)
    }

    pub fn last_gamepad_hint(&self) -> usize {
        self.gamepads.len()
    }
}

#[derive(Debug)]
enum Mapping {
    Standard { buttons: [bool; 17], axes: [f64; 4] },
    NoMapping { buttons: Vec<bool>, axes: Vec<f64> },
}

impl Mapping {
    fn buttons<'a>(&'a self) -> impl Iterator<Item = bool> + 'a {
        match self {
            Mapping::Standard { buttons, .. } => buttons.iter(),
            Mapping::NoMapping { buttons, .. } => buttons.iter(),
        }
        .cloned()
    }

    fn axes<'a>(&'a self) -> impl Iterator<Item = f64> + 'a {
        match self {
            Mapping::Standard { axes, .. } => axes.iter(),
            Mapping::NoMapping { axes, .. } => axes.iter(),
        }
        .cloned()
    }
}

#[derive(Debug)]
pub struct Gamepad {
    uuid: Uuid,
    gamepad: WebGamepad,
    name: String,
    mapping: Mapping,
}

impl Gamepad {
    fn new(gamepad: WebGamepad) -> Gamepad {
        let name = gamepad.id();

        let buttons = gamepad.buttons();
        let button_iter = {
            #[cfg(feature = "wasm-bindgen")]
            {
                buttons.iter().map(GamepadButton::from)
            }
            #[cfg(not(feature = "wasm-bindgen"))]
            {
                buttons.into_iter()
            }
        };

        let axes = gamepad.axes();
        let axis_iter = {
            #[cfg(feature = "wasm-bindgen")]
            {
                axes.iter()
                    .map(|val| val.as_f64().expect("axes() should be an array of f64"))
            }
            #[cfg(not(feature = "wasm-bindgen"))]
            {
                axes.into_iter()
            }
        };

        let mapping = match gamepad.mapping() {
            GamepadMappingType::Standard => {
                let mut buttons = [false; 17];
                let mut axes = [0.0; 4];

                for (index, button) in button_iter.enumerate().take(buttons.len()) {
                    buttons[index] = button.pressed();
                }

                for (index, axis) in axis_iter.enumerate().take(axes.len()) {
                    axes[index] = axis;
                }

                Mapping::Standard { buttons, axes }
            }
            _ => {
                let buttons = button_iter.map(|button| button.pressed()).collect();
                let axes = axis_iter.collect();
                Mapping::NoMapping { buttons, axes }
            }
        };

        Gamepad {
            uuid: Uuid::nil(),
            gamepad,
            name,
            mapping,
        }
    }

    fn index(&self) -> usize {
        self.gamepad.index() as usize
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn uuid(&self) -> Uuid {
        self.uuid
    }

    pub fn is_connected(&self) -> bool {
        self.gamepad.connected()
    }

    pub fn power_info(&self) -> PowerInfo {
        PowerInfo::Unknown
    }

    pub fn is_ff_supported(&self) -> bool {
        false
    }

    pub fn ff_device(&self) -> Option<FfDevice> {
        None
    }

    pub fn buttons(&self) -> &[EvCode] {
        &native_ev_codes::BUTTONS
    }

    pub fn axes(&self) -> &[EvCode] {
        &native_ev_codes::AXES
    }

    fn button_code(&self, index: usize) -> EvCode {
        self.buttons()
            .get(index)
            .map(|ev| ev.clone())
            .unwrap_or(EvCode(index as u8 + 31))
    }

    fn axis_code(&self, index: usize) -> EvCode {
        self.axes()
            .get(index)
            .map(|ev| ev.clone())
            .unwrap_or(EvCode((index + self.mapping.buttons().count()) as u8 + 31))
    }

    pub(crate) fn axis_info(&self, _nec: EvCode) -> Option<&AxisInfo> {
        Some(&AxisInfo {
            min: i32::min_value() as i32,
            max: i32::max_value() as i32,
            deadzone: None,
        })
    }
}
#[cfg(feature = "serde-serialize")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde-serialize", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct EvCode(u8);

impl EvCode {
    pub fn into_u32(self) -> u32 {
        self.0 as u32
    }
}

impl Display for EvCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        self.0.fmt(f)
    }
}

pub mod native_ev_codes {
    use super::EvCode;

    pub const AXIS_LSTICKX: EvCode = EvCode(0);
    pub const AXIS_LSTICKY: EvCode = EvCode(1);
    pub const AXIS_LEFTZ: EvCode = EvCode(2);
    pub const AXIS_RSTICKX: EvCode = EvCode(3);
    pub const AXIS_RSTICKY: EvCode = EvCode(4);
    pub const AXIS_RIGHTZ: EvCode = EvCode(5);
    pub const AXIS_DPADX: EvCode = EvCode(6);
    pub const AXIS_DPADY: EvCode = EvCode(7);
    pub const AXIS_RT: EvCode = EvCode(8);
    pub const AXIS_LT: EvCode = EvCode(9);
    pub const AXIS_RT2: EvCode = EvCode(10);
    pub const AXIS_LT2: EvCode = EvCode(11);

    pub const BTN_SOUTH: EvCode = EvCode(12);
    pub const BTN_EAST: EvCode = EvCode(13);
    pub const BTN_C: EvCode = EvCode(14);
    pub const BTN_NORTH: EvCode = EvCode(15);
    pub const BTN_WEST: EvCode = EvCode(16);
    pub const BTN_Z: EvCode = EvCode(17);
    pub const BTN_LT: EvCode = EvCode(18);
    pub const BTN_RT: EvCode = EvCode(19);
    pub const BTN_LT2: EvCode = EvCode(20);
    pub const BTN_RT2: EvCode = EvCode(21);
    pub const BTN_SELECT: EvCode = EvCode(22);
    pub const BTN_START: EvCode = EvCode(23);
    pub const BTN_MODE: EvCode = EvCode(24);
    pub const BTN_LTHUMB: EvCode = EvCode(25);
    pub const BTN_RTHUMB: EvCode = EvCode(26);

    pub const BTN_DPAD_UP: EvCode = EvCode(27);
    pub const BTN_DPAD_DOWN: EvCode = EvCode(28);
    pub const BTN_DPAD_LEFT: EvCode = EvCode(29);
    pub const BTN_DPAD_RIGHT: EvCode = EvCode(30);

    pub(super) static BUTTONS: [EvCode; 17] = [
        BTN_SOUTH,
        BTN_EAST,
        BTN_NORTH,
        BTN_WEST,
        BTN_LT,
        BTN_RT,
        BTN_LT2,
        BTN_RT2,
        BTN_SELECT,
        BTN_START,
        BTN_LTHUMB,
        BTN_RTHUMB,
        BTN_DPAD_UP,
        BTN_DPAD_DOWN,
        BTN_DPAD_LEFT,
        BTN_DPAD_RIGHT,
        BTN_MODE,
    ];

    pub(super) static AXES: [EvCode; 4] = [AXIS_LSTICKX, AXIS_LSTICKY, AXIS_RSTICKX, AXIS_RSTICKY];
}

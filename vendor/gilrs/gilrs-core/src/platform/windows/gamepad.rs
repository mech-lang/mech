// Copyright 2016-2018 Mateusz Sieczko and other GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use super::FfDevice;
use crate::{AxisInfo, Event, EventType, PlatformError, PowerInfo};

use std::error::Error as StdError;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;
use std::{mem, thread, u16, u32};

use rusty_xinput::XInputLoadingFailure;
use rusty_xinput::{self, BatteryLevel, BatteryType, XInputState, XInputUsageError};
use uuid::Uuid;
use winapi::um::xinput::{
    XINPUT_GAMEPAD as XGamepad, XINPUT_GAMEPAD_A, XINPUT_GAMEPAD_B, XINPUT_GAMEPAD_BACK,
    XINPUT_GAMEPAD_DPAD_DOWN, XINPUT_GAMEPAD_DPAD_LEFT, XINPUT_GAMEPAD_DPAD_RIGHT,
    XINPUT_GAMEPAD_DPAD_UP, XINPUT_GAMEPAD_LEFT_SHOULDER, XINPUT_GAMEPAD_LEFT_THUMB,
    XINPUT_GAMEPAD_RIGHT_SHOULDER, XINPUT_GAMEPAD_RIGHT_THUMB, XINPUT_GAMEPAD_START,
    XINPUT_GAMEPAD_X, XINPUT_GAMEPAD_Y, XINPUT_STATE as XState,
};

// Chosen by dice roll ;)
const EVENT_THREAD_SLEEP_TIME: u64 = 10;
const ITERATIONS_TO_CHECK_IF_CONNECTED: u64 = 100;

const MAX_XINPUT_CONTROLLERS: usize = 4;

#[derive(Debug)]
pub struct Gilrs {
    gamepads: [Gamepad; MAX_XINPUT_CONTROLLERS],
    rx: Receiver<Event>,
}

impl Gilrs {
    pub(crate) fn new() -> Result<Self, PlatformError> {
        match rusty_xinput::dynamic_load_xinput() {
            Ok(()) => (),
            Err(XInputLoadingFailure::AlreadyLoading)
            | Err(XInputLoadingFailure::AlreadyActive) => (),
            Err(e) => return Err(PlatformError::Other(Box::new(Error::FailedToLoadDll(e)))),
        }

        let mut gamepads: [Gamepad; MAX_XINPUT_CONTROLLERS] = Default::default();
        let mut connected: [bool; MAX_XINPUT_CONTROLLERS] = Default::default();

        // Iterate through each controller ID and set connected state
        for id in 0..MAX_XINPUT_CONTROLLERS {
            gamepads[id] = Gamepad::new(id as u32);
            connected[id] = gamepads[id].is_connected;
        }

        let (tx, rx) = mpsc::channel();
        Self::spawn_thread(tx, connected);

        // Coerce gamepads vector to slice
        Ok(Gilrs { gamepads, rx })
    }

    pub(crate) fn next_event(&mut self) -> Option<Event> {
        let ev = self.rx.recv().ok();

        if let Some(ev) = ev {
            match ev.event {
                EventType::Connected => self.gamepads[ev.id].is_connected = true,
                EventType::Disconnected => self.gamepads[ev.id].is_connected = false,
                _ => (),
            }
        }

        ev
    }

    pub fn gamepad(&self, id: usize) -> Option<&Gamepad> {
        self.gamepads.get(id)
    }

    pub fn last_gamepad_hint(&self) -> usize {
        self.gamepads.len()
    }

    fn spawn_thread(tx: Sender<Event>, connected: [bool; MAX_XINPUT_CONTROLLERS]) {
        thread::spawn(move || unsafe {
            // Issue #70 fix - Maintain a prev_state per controller id. Otherwise the loop will compare the prev_state of a different controller.
            let mut prev_states: [XState; MAX_XINPUT_CONTROLLERS] =
                [mem::zeroed::<XState>(); MAX_XINPUT_CONTROLLERS];
            let mut connected = connected;
            let mut counter = 0;

            loop {
                for id in 0..MAX_XINPUT_CONTROLLERS {
                    if *connected.get_unchecked(id)
                        || counter % ITERATIONS_TO_CHECK_IF_CONNECTED == 0
                    {
                        match rusty_xinput::xinput_get_state(id as u32) {
                            Ok(XInputState { raw: state }) => {
                                if !connected[id] {
                                    connected[id] = true;
                                    let _ = tx.send(Event::new(id, EventType::Connected));
                                }

                                if state.dwPacketNumber != prev_states[id].dwPacketNumber {
                                    Self::compare_state(
                                        id,
                                        &state.Gamepad,
                                        &prev_states[id].Gamepad,
                                        &tx,
                                    );
                                    prev_states[id] = state;
                                }
                            }
                            Err(XInputUsageError::DeviceNotConnected) if connected[id] => {
                                connected[id] = false;
                                let _ = tx.send(Event::new(id, EventType::Disconnected));
                            }
                            Err(XInputUsageError::DeviceNotConnected) => (),
                            Err(e) => error!("Failed to get gamepad state: {:?}", e),
                        }
                    }
                }

                counter = counter.wrapping_add(1);
                thread::sleep(Duration::from_millis(EVENT_THREAD_SLEEP_TIME));
            }
        });
    }

    fn compare_state(id: usize, g: &XGamepad, pg: &XGamepad, tx: &Sender<Event>) {
        if g.bLeftTrigger != pg.bLeftTrigger {
            let _ = tx.send(Event::new(
                id,
                EventType::AxisValueChanged(
                    g.bLeftTrigger as i32,
                    crate::native_ev_codes::AXIS_LT2,
                ),
            ));
        }
        if g.bRightTrigger != pg.bRightTrigger {
            let _ = tx.send(Event::new(
                id,
                EventType::AxisValueChanged(
                    g.bRightTrigger as i32,
                    crate::native_ev_codes::AXIS_RT2,
                ),
            ));
        }
        if g.sThumbLX != pg.sThumbLX {
            let _ = tx.send(Event::new(
                id,
                EventType::AxisValueChanged(
                    g.sThumbLX as i32,
                    crate::native_ev_codes::AXIS_LSTICKX,
                ),
            ));
        }
        if g.sThumbLY != pg.sThumbLY {
            let _ = tx.send(Event::new(
                id,
                EventType::AxisValueChanged(
                    g.sThumbLY as i32,
                    crate::native_ev_codes::AXIS_LSTICKY,
                ),
            ));
        }
        if g.sThumbRX != pg.sThumbRX {
            let _ = tx.send(Event::new(
                id,
                EventType::AxisValueChanged(
                    g.sThumbRX as i32,
                    crate::native_ev_codes::AXIS_RSTICKX,
                ),
            ));
        }
        if g.sThumbRY != pg.sThumbRY {
            let _ = tx.send(Event::new(
                id,
                EventType::AxisValueChanged(
                    g.sThumbRY as i32,
                    crate::native_ev_codes::AXIS_RSTICKY,
                ),
            ));
        }
        if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_DPAD_UP) {
            let _ = match g.wButtons & XINPUT_GAMEPAD_DPAD_UP != 0 {
                true => tx.send(Event::new(
                    id,
                    EventType::ButtonPressed(crate::native_ev_codes::BTN_DPAD_UP),
                )),
                false => tx.send(Event::new(
                    id,
                    EventType::ButtonReleased(crate::native_ev_codes::BTN_DPAD_UP),
                )),
            };
        }
        if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_DPAD_DOWN) {
            let _ = match g.wButtons & XINPUT_GAMEPAD_DPAD_DOWN != 0 {
                true => tx.send(Event::new(
                    id,
                    EventType::ButtonPressed(crate::native_ev_codes::BTN_DPAD_DOWN),
                )),
                false => tx.send(Event::new(
                    id,
                    EventType::ButtonReleased(crate::native_ev_codes::BTN_DPAD_DOWN),
                )),
            };
        }
        if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_DPAD_LEFT) {
            let _ = match g.wButtons & XINPUT_GAMEPAD_DPAD_LEFT != 0 {
                true => tx.send(Event::new(
                    id,
                    EventType::ButtonPressed(crate::native_ev_codes::BTN_DPAD_LEFT),
                )),
                false => tx.send(Event::new(
                    id,
                    EventType::ButtonReleased(crate::native_ev_codes::BTN_DPAD_LEFT),
                )),
            };
        }
        if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_DPAD_RIGHT) {
            let _ = match g.wButtons & XINPUT_GAMEPAD_DPAD_RIGHT != 0 {
                true => tx.send(Event::new(
                    id,
                    EventType::ButtonPressed(crate::native_ev_codes::BTN_DPAD_RIGHT),
                )),
                false => tx.send(Event::new(
                    id,
                    EventType::ButtonReleased(crate::native_ev_codes::BTN_DPAD_RIGHT),
                )),
            };
        }
        if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_START) {
            let _ = match g.wButtons & XINPUT_GAMEPAD_START != 0 {
                true => tx.send(Event::new(
                    id,
                    EventType::ButtonPressed(crate::native_ev_codes::BTN_START),
                )),
                false => tx.send(Event::new(
                    id,
                    EventType::ButtonReleased(crate::native_ev_codes::BTN_START),
                )),
            };
        }
        if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_BACK) {
            let _ = match g.wButtons & XINPUT_GAMEPAD_BACK != 0 {
                true => tx.send(Event::new(
                    id,
                    EventType::ButtonPressed(crate::native_ev_codes::BTN_SELECT),
                )),
                false => tx.send(Event::new(
                    id,
                    EventType::ButtonReleased(crate::native_ev_codes::BTN_SELECT),
                )),
            };
        }
        if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_LEFT_THUMB) {
            let _ = match g.wButtons & XINPUT_GAMEPAD_LEFT_THUMB != 0 {
                true => tx.send(Event::new(
                    id,
                    EventType::ButtonPressed(crate::native_ev_codes::BTN_LTHUMB),
                )),
                false => tx.send(Event::new(
                    id,
                    EventType::ButtonReleased(crate::native_ev_codes::BTN_LTHUMB),
                )),
            };
        }
        if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_RIGHT_THUMB) {
            let _ = match g.wButtons & XINPUT_GAMEPAD_RIGHT_THUMB != 0 {
                true => tx.send(Event::new(
                    id,
                    EventType::ButtonPressed(crate::native_ev_codes::BTN_RTHUMB),
                )),
                false => tx.send(Event::new(
                    id,
                    EventType::ButtonReleased(crate::native_ev_codes::BTN_RTHUMB),
                )),
            };
        }
        if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_LEFT_SHOULDER) {
            let _ = match g.wButtons & XINPUT_GAMEPAD_LEFT_SHOULDER != 0 {
                true => tx.send(Event::new(
                    id,
                    EventType::ButtonPressed(crate::native_ev_codes::BTN_LT),
                )),
                false => tx.send(Event::new(
                    id,
                    EventType::ButtonReleased(crate::native_ev_codes::BTN_LT),
                )),
            };
        }
        if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_RIGHT_SHOULDER) {
            let _ = match g.wButtons & XINPUT_GAMEPAD_RIGHT_SHOULDER != 0 {
                true => tx.send(Event::new(
                    id,
                    EventType::ButtonPressed(crate::native_ev_codes::BTN_RT),
                )),
                false => tx.send(Event::new(
                    id,
                    EventType::ButtonReleased(crate::native_ev_codes::BTN_RT),
                )),
            };
        }
        if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_A) {
            let _ = match g.wButtons & XINPUT_GAMEPAD_A != 0 {
                true => tx.send(Event::new(
                    id,
                    EventType::ButtonPressed(crate::native_ev_codes::BTN_SOUTH),
                )),
                false => tx.send(Event::new(
                    id,
                    EventType::ButtonReleased(crate::native_ev_codes::BTN_SOUTH),
                )),
            };
        }
        if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_B) {
            let _ = match g.wButtons & XINPUT_GAMEPAD_B != 0 {
                true => tx.send(Event::new(
                    id,
                    EventType::ButtonPressed(crate::native_ev_codes::BTN_EAST),
                )),
                false => tx.send(Event::new(
                    id,
                    EventType::ButtonReleased(crate::native_ev_codes::BTN_EAST),
                )),
            };
        }
        if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_X) {
            let _ = match g.wButtons & XINPUT_GAMEPAD_X != 0 {
                true => tx.send(Event::new(
                    id,
                    EventType::ButtonPressed(crate::native_ev_codes::BTN_WEST),
                )),
                false => tx.send(Event::new(
                    id,
                    EventType::ButtonReleased(crate::native_ev_codes::BTN_WEST),
                )),
            };
        }
        if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_Y) {
            let _ = match g.wButtons & XINPUT_GAMEPAD_Y != 0 {
                true => tx.send(Event::new(
                    id,
                    EventType::ButtonPressed(crate::native_ev_codes::BTN_NORTH),
                )),
                false => tx.send(Event::new(
                    id,
                    EventType::ButtonReleased(crate::native_ev_codes::BTN_NORTH),
                )),
            };
        }
    }
}

#[derive(Debug, Default)]
pub struct Gamepad {
    uuid: Uuid,
    id: u32,
    is_connected: bool,
}

impl Gamepad {
    fn new(id: u32) -> Gamepad {
        let is_connected = {
            if rusty_xinput::xinput_get_state(id).is_ok() {
                true
            } else {
                false
            }
        };

        let gamepad = Gamepad {
            uuid: Uuid::nil(),
            id,
            is_connected,
        };

        gamepad
    }

    pub fn name(&self) -> &str {
        "Xbox Controller"
    }

    pub fn uuid(&self) -> Uuid {
        self.uuid
    }

    pub fn is_connected(&self) -> bool {
        self.is_connected
    }

    pub fn power_info(&self) -> PowerInfo {
        match rusty_xinput::xinput_get_gamepad_battery_information(self.id) {
            Ok(binfo) => match binfo.battery_type {
                BatteryType::WIRED => PowerInfo::Wired,
                BatteryType::ALKALINE | BatteryType::NIMH => {
                    let lvl = match binfo.battery_level {
                        BatteryLevel::EMPTY => 0,
                        BatteryLevel::LOW => 33,
                        BatteryLevel::MEDIUM => 67,
                        BatteryLevel::FULL => 100,
                        lvl => {
                            trace!("Unexpected battery level: {}", lvl.0);

                            100
                        }
                    };
                    if lvl == 100 {
                        PowerInfo::Charged
                    } else {
                        PowerInfo::Discharging(lvl)
                    }
                }
                _ => PowerInfo::Unknown,
            },
            Err(e) => {
                debug!("Failed to get battery info: {:?}", e);

                PowerInfo::Unknown
            }
        }
    }

    pub fn is_ff_supported(&self) -> bool {
        true
    }

    pub fn ff_device(&self) -> Option<FfDevice> {
        Some(FfDevice::new(self.id))
    }

    pub fn buttons(&self) -> &[EvCode] {
        &native_ev_codes::BUTTONS
    }

    pub fn axes(&self) -> &[EvCode] {
        &native_ev_codes::AXES
    }

    pub(crate) fn axis_info(&self, nec: EvCode) -> Option<&AxisInfo> {
        native_ev_codes::AXES_INFO
            .get(nec.0 as usize)
            .and_then(|o| o.as_ref())
    }
}

#[inline(always)]
fn is_mask_eq(l: u16, r: u16, mask: u16) -> bool {
    (l & mask != 0) == (r & mask != 0)
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
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        self.0.fmt(f)
    }
}

#[derive(Debug)]
enum Error {
    FailedToLoadDll(XInputLoadingFailure),
}

impl StdError for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            Error::FailedToLoadDll(e) => {
                f.write_fmt(format_args!("Failed to load XInput DLL {:?}", e))
            }
        }
    }
}

pub mod native_ev_codes {
    use std::i16::{MAX as I16_MAX, MIN as I16_MIN};
    use std::u8::{MAX as U8_MAX, MIN as U8_MIN};

    use winapi::um::xinput::{
        XINPUT_GAMEPAD_LEFT_THUMB_DEADZONE, XINPUT_GAMEPAD_RIGHT_THUMB_DEADZONE,
        XINPUT_GAMEPAD_TRIGGER_THRESHOLD,
    };

    use super::EvCode;
    use crate::AxisInfo;

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

    pub(super) static BUTTONS: [EvCode; 15] = [
        BTN_SOUTH,
        BTN_EAST,
        BTN_NORTH,
        BTN_WEST,
        BTN_LT,
        BTN_RT,
        BTN_SELECT,
        BTN_START,
        BTN_MODE,
        BTN_LTHUMB,
        BTN_RTHUMB,
        BTN_DPAD_UP,
        BTN_DPAD_DOWN,
        BTN_DPAD_LEFT,
        BTN_DPAD_RIGHT,
    ];

    pub(super) static AXES: [EvCode; 6] = [
        AXIS_LSTICKX,
        AXIS_LSTICKY,
        AXIS_RSTICKX,
        AXIS_RSTICKY,
        AXIS_RT2,
        AXIS_LT2,
    ];

    pub(super) static AXES_INFO: [Option<AxisInfo>; 12] = [
        // LeftStickX
        Some(AxisInfo {
            min: I16_MIN as i32,
            max: I16_MAX as i32,
            deadzone: Some(XINPUT_GAMEPAD_LEFT_THUMB_DEADZONE as u32),
        }),
        // LeftStickY
        Some(AxisInfo {
            min: I16_MIN as i32,
            max: I16_MAX as i32,
            deadzone: Some(XINPUT_GAMEPAD_LEFT_THUMB_DEADZONE as u32),
        }),
        // LeftZ
        None,
        // RightStickX
        Some(AxisInfo {
            min: I16_MIN as i32,
            max: I16_MAX as i32,
            deadzone: Some(XINPUT_GAMEPAD_RIGHT_THUMB_DEADZONE as u32),
        }),
        // RightStickY
        Some(AxisInfo {
            min: I16_MIN as i32,
            max: I16_MAX as i32,
            deadzone: Some(XINPUT_GAMEPAD_RIGHT_THUMB_DEADZONE as u32),
        }),
        // RightZ
        None,
        // DPadX
        None,
        // DPadY
        None,
        // RightTrigger
        None,
        // LeftTrigger
        None,
        // RightTrigger2
        Some(AxisInfo {
            min: U8_MIN as i32,
            max: U8_MAX as i32,
            deadzone: Some(XINPUT_GAMEPAD_TRIGGER_THRESHOLD as u32),
        }),
        // LeftTrigger2
        Some(AxisInfo {
            min: U8_MIN as i32,
            max: U8_MAX as i32,
            deadzone: Some(XINPUT_GAMEPAD_TRIGGER_THRESHOLD as u32),
        }),
    ];
}

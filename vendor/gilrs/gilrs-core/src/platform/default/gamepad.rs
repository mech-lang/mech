// Copyright 2016-2018 Mateusz Sieczko and other GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
#![allow(unused_variables)]

use super::FfDevice;
use crate::{AxisInfo, Event, PlatformError, PowerInfo};
use uuid::Uuid;

use std::fmt::{Display, Formatter, Result as FmtResult};

#[derive(Debug)]
pub struct Gilrs {}

impl Gilrs {
    pub(crate) fn new() -> Result<Self, PlatformError> {
        Err(PlatformError::NotImplemented(Gilrs {}))
    }

    pub(crate) fn next_event(&mut self) -> Option<Event> {
        None
    }

    pub fn gamepad(&self, id: usize) -> Option<&Gamepad> {
        None
    }

    /// Returns index greater than index of last connected gamepad.
    pub fn last_gamepad_hint(&self) -> usize {
        0
    }
}

#[derive(Debug)]
pub struct Gamepad {
    _priv: u8, // required for `#[repr(transparent)]`
}

impl Gamepad {
    pub fn name(&self) -> &str {
        ""
    }

    pub fn uuid(&self) -> Uuid {
        Uuid::nil()
    }

    pub fn power_info(&self) -> PowerInfo {
        PowerInfo::Unknown
    }

    pub fn is_ff_supported(&self) -> bool {
        false
    }

    /// Creates Ffdevice corresponding to this gamepad.
    pub fn ff_device(&self) -> Option<FfDevice> {
        Some(FfDevice)
    }

    pub fn buttons(&self) -> &[EvCode] {
        &[]
    }

    pub fn axes(&self) -> &[EvCode] {
        &[]
    }

    pub(crate) fn axis_info(&self, nec: EvCode) -> Option<&AxisInfo> {
        None
    }

    pub fn is_connected(&self) -> bool {
        false
    }
}

#[cfg(feature = "serde-serialize")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde-serialize", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct EvCode(u16);

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
}

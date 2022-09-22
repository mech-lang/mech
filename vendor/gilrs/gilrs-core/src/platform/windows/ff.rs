// Copyright 2016-2018 Mateusz Sieczko and other GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use rusty_xinput::{self, XInputUsageError};
use std::time::Duration;

#[derive(Debug)]
pub struct Device {
    id: u32,
}

impl Device {
    pub(crate) fn new(id: u32) -> Self {
        Device { id }
    }

    pub fn set_ff_state(&mut self, strong: u16, weak: u16, _min_duration: Duration) {
        match rusty_xinput::xinput_set_state(self.id, strong, weak) {
            Ok(()) => (),
            Err(XInputUsageError::DeviceNotConnected) => {
                error!(
                    "Failed to change FF state – gamepad with id {} is no longer connected.",
                    self.id
                );
            }
            Err(err) => {
                error!(
                    "Failed to change FF state – unknown error. ID = {}, error = {:?}.",
                    self.id, err
                );
            }
        }
    }
}

// Copyright 2016-2018 Mateusz Sieczko and other GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use std::fs::File;
use std::io::{Error as IoError, ErrorKind, Result as IoResult, Write};
use std::os::unix::io::AsRawFd;
use std::u16::MAX as U16_MAX;
use std::{mem, slice};

use super::ioctl::{self, ff_effect, ff_replay, ff_rumble_effect, input_event};
use std::time::Duration;

#[derive(Debug)]
pub struct Device {
    effect: i16,
    file: File,
}

impl Device {
    pub(crate) fn new(path: &str) -> IoResult<Self> {
        let file = File::create(path)?;
        let mut effect = ff_effect {
            type_: FF_RUMBLE,
            id: -1,
            direction: 0,
            trigger: Default::default(),
            replay: Default::default(),
            u: Default::default(),
        };

        #[allow(clippy::unnecessary_mut_passed)]
        let res = unsafe { ioctl::eviocsff(file.as_raw_fd(), &mut effect) };

        if res.is_err() {
            Err(IoError::new(ErrorKind::Other, "Failed to create effect"))
        } else {
            Ok(Device {
                effect: effect.id,
                file,
            })
        }
    }

    pub fn set_ff_state(&mut self, strong: u16, weak: u16, min_duration: Duration) {
        let duration = min_duration.as_secs() * 1000 + u64::from(min_duration.subsec_millis());
        let duration = if duration > u64::from(U16_MAX) {
            U16_MAX
        } else {
            duration as u16
        };

        let mut effect = ff_effect {
            type_: FF_RUMBLE,
            id: self.effect,
            direction: 0,
            trigger: Default::default(),
            replay: ff_replay {
                delay: 0,
                length: duration,
            },
            u: Default::default(),
        };

        unsafe {
            let rumble = &mut effect.u as *mut _ as *mut ff_rumble_effect;
            (*rumble).strong_magnitude = strong;
            (*rumble).weak_magnitude = weak;

            if let Err(err) = ioctl::eviocsff(self.file.as_raw_fd(), &effect) {
                error!(
                    "Failed to modify effect of gamepad {:?}, error: {}",
                    self.file, err
                );

                return;
            }
        };

        let time = libc::timeval {
            tv_sec: 0,
            tv_usec: 0,
        };
        let ev = input_event {
            type_: EV_FF,
            code: self.effect as u16,
            value: 1,
            time,
        };

        let size = mem::size_of::<input_event>();
        let s = unsafe { slice::from_raw_parts(&ev as *const _ as *const u8, size) };

        match self.file.write(s) {
            Ok(s) if s == size => (),
            Ok(_) => unreachable!(),
            Err(e) => error!("Failed to set ff state: {}", e),
        }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        #[cfg(target_os = "linux")]
        let effect = self.effect as ::libc::c_ulong;
        #[cfg(not(target_os = "linux"))]
        let effect = self.effect as ::libc::c_int;

        if let Err(err) = unsafe { ioctl::eviocrmff(self.file.as_raw_fd(), effect) } {
            error!(
                "Failed to remove effect of gamepad {:?}: {}",
                self.file, err
            )
        };
    }
}

const EV_FF: u16 = 0x15;
const FF_RUMBLE: u16 = 0x50;

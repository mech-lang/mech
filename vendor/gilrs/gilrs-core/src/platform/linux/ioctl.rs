// Copyright 2016-2018 Mateusz Sieczko and other GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

// Some ioctls are exported by ioctl crate only for x86_64, so we have to define them anyway.
// Diffing linux/input.h across different architectures (i686, x86_64 and arm) didn't show any
// difference, so it looks like conditional compilation is not needed.

use nix::{ioctl_read, ioctl_read_buf, ioctl_write_int, ioctl_write_ptr, request_code_read};
use std::mem::MaybeUninit;

#[cfg(target_env = "musl")]
pub type IoctlRequest = libc::c_int;
#[cfg(not(target_env = "musl"))]
pub type IoctlRequest = libc::c_ulong;

ioctl_read!(eviocgid, b'E', 0x02, /*struct*/ input_id);
ioctl_write_int!(eviocrmff, b'E', 0x81);
ioctl_write_ptr!(eviocsff, b'E', 0x80, ff_effect);
ioctl_read_buf!(eviocgname, b'E', 0x06, MaybeUninit<u8>);
ioctl_read_buf!(eviocgkey, b'E', 0x18, u8);

pub unsafe fn eviocgbit(fd: libc::c_int, ev: u32, len: libc::c_int, buf: *mut u8) -> libc::c_int {
    ::nix::libc::ioctl(
        fd,
        request_code_read!(b'E', 0x20 + ev, len) as IoctlRequest,
        buf,
    )
}

pub unsafe fn eviocgabs(fd: ::libc::c_int, abs: u32, buf: *mut input_absinfo) -> libc::c_int {
    ::nix::libc::ioctl(
        fd,
        request_code_read!(b'E', 0x40 + abs, ::std::mem::size_of::<input_absinfo>())
            as IoctlRequest,
        buf,
    )
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct input_event {
    pub time: libc::timeval,
    pub type_: u16,
    pub code: u16,
    pub value: i32,
}

impl ::std::default::Default for input_event {
    fn default() -> Self {
        unsafe { ::std::mem::zeroed() }
    }
}

impl ::std::fmt::Debug for input_event {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        write!(
            f,
            "input_event {{ time: {{ tv_sec: {}, tv_usec: {} }}, type_: {}, code: {}, value: {}",
            self.time.tv_sec, self.time.tv_usec, self.type_, self.code, self.value
        )
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct input_id {
    pub bustype: u16,
    pub vendor: u16,
    pub product: u16,
    pub version: u16,
}

#[derive(Copy, Clone, Default, PartialEq, Debug)]
#[repr(C)]
pub struct input_absinfo {
    pub value: i32,
    pub minimum: i32,
    pub maximum: i32,
    pub fuzz: i32,
    pub flat: i32,
    pub resolution: i32,
}

#[derive(Copy, Clone, Default)]
#[repr(C)]
pub struct ff_replay {
    pub length: u16,
    pub delay: u16,
}

#[derive(Copy, Clone, Default)]
#[repr(C)]
pub struct ff_trigger {
    pub button: u16,
    pub interval: u16,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct ff_envelope {
    pub attack_length: u16,
    pub attack_level: u16,
    pub fade_length: u16,
    pub fade_level: u16,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct ff_constant_effect {
    pub level: i16,
    pub envelope: ff_envelope,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct ff_ramp_effect {
    pub start_level: i16,
    pub end_level: i16,
    pub envelope: ff_envelope,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct ff_condition_effect {
    pub right_saturation: u16,
    pub left_saturation: u16,

    pub right_coeff: i16,
    pub left_coeff: i16,

    pub deadband: u16,
    pub center: i16,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct ff_periodic_effect {
    pub waveform: u16,
    pub period: u16,
    pub magnitude: i16,
    pub offset: i16,
    pub phase: u16,

    pub envelope: ff_envelope,

    pub custom_len: u32,
    pub custom_data: *mut i16,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct ff_rumble_effect {
    pub strong_magnitude: u16,
    pub weak_magnitude: u16,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct ff_effect {
    pub type_: u16,
    pub id: i16,
    pub direction: u16,
    pub trigger: ff_trigger,
    pub replay: ff_replay,
    // FIXME this is actually a union
    #[cfg(target_pointer_width = "64")]
    pub u: [u64; 4],
    #[cfg(target_pointer_width = "32")]
    pub u: [u32; 7],
}

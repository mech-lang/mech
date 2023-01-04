#![allow(warnings)]
extern crate mech_core;
extern crate mech_utilities;
#[cfg(target_os = "windows")]
extern crate gilrs;
extern crate crossbeam_channel;
#[macro_use]
extern crate lazy_static;

pub mod out;
#[cfg(target_os = "windows")]
pub mod gamepad;
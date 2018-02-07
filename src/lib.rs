#![cfg_attr(target_os = "none", no_std)]
#![feature(alloc)]

extern crate rlibc;
extern crate alloc;
#[cfg(not(target_os = "none"))]
extern crate core;

use alloc::Vec;

pub mod database;

fn foo() {
    let v: Vec<u8> = Vec::new();
}
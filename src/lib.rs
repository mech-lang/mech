#![no_std]
#![feature(alloc)]

extern crate rlibc;
extern crate alloc;

use alloc::Vec;

pub mod database;

fn foo() {
    let v: Vec<u8> = Vec::new();
}
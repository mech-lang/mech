// # Mech Program
#![allow(dead_code)]

// ## Prelude
#![feature(extern_prelude)]
#![feature(get_mut_unchecked)]

extern crate core;
extern crate libloading;
extern crate reqwest;

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;
extern crate serde;
#[macro_use]
extern crate crossbeam_channel;

extern crate time;

extern crate mech_core;
extern crate mech_syntax;
extern crate mech_utilities;

// ## Modules

pub mod program;

// ## Exported Modules

pub use self::program::{Program, ProgramRunner, RunLoop, ClientMessage};
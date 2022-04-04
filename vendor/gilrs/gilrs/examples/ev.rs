// Copyright 2016-2018 Mateusz Sieczko and other GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use gilrs::ev::filter::{Filter, Repeat};
use gilrs::GilrsBuilder;

use std::process;
use std::thread;
use std::time::Duration;

fn main() {
    env_logger::init();

    let mut gilrs = match GilrsBuilder::new().set_update_state(false).build() {
        Ok(g) => g,
        Err(gilrs::Error::NotImplemented(g)) => {
            eprintln!("Current platform is not supported");

            g
        }
        Err(e) => {
            eprintln!("Failed to create gilrs context: {}", e);
            process::exit(-1);
        }
    };

    let repeat_filter = Repeat::new();

    loop {
        while let Some(ev) = gilrs.next_event().filter_ev(&repeat_filter, &mut gilrs) {
            gilrs.update(&ev);
            println!("{:?}", ev);
        }

        if gilrs.counter() % 250 == 0 {
            for (id, gamepad) in gilrs.gamepads() {
                println!(
                    "Power info of gamepad {}({}): {:?}",
                    id,
                    gamepad.name(),
                    gamepad.power_info()
                );
            }
        }

        gilrs.inc();
        thread::sleep(Duration::from_millis(33));
    }
}

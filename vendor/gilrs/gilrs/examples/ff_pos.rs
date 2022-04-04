// Copyright 2016-2018 Mateusz Sieczko and other GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use gilrs::ff::{BaseEffect, BaseEffectType, DistanceModel, EffectBuilder};
use gilrs::{Axis, Button, EventType, Gilrs};

use std::io::{self, Write};
use std::thread;
use std::time::Duration;

#[derive(Copy, Clone, PartialEq, Debug)]
enum Modify {
    DistModel,
    RefDistance,
    RolloffFactor,
    MaxDistance,
}

impl Modify {
    fn next(&mut self) {
        use crate::Modify::*;
        *self = match *self {
            DistModel => RefDistance,
            RefDistance => RolloffFactor,
            RolloffFactor => MaxDistance,
            MaxDistance => DistModel,
        };
        print!("\x1b[2K\r{:?}", self);
        io::stdout().flush().unwrap();
    }

    fn prev(&mut self) {
        use crate::Modify::*;
        *self = match *self {
            DistModel => MaxDistance,
            RefDistance => DistModel,
            RolloffFactor => RefDistance,
            MaxDistance => RolloffFactor,
        };
        print!("\x1b[2K\r{:?}", self);
        io::stdout().flush().unwrap();
    }
}

fn main() {
    env_logger::init();
    let mut gilrs = Gilrs::new().unwrap();

    println!("Connected gamepads:");

    let mut support_ff = Vec::new();
    for (idx, gp) in gilrs.gamepads() {
        let ff = gp.is_ff_supported();
        println!(
            "{}) {} ({})",
            idx,
            gp.name(),
            if ff {
                "Force feedback supported"
            } else {
                "Force feedback not supported"
            }
        );
        if ff {
            support_ff.push(idx);
        }
    }

    println!("----------------------------------------");
    println!(
        "Use sticks to move listener. Triggers change properties of distance model. South/west \
         button changes active property. Press east button on action pad to quit."
    );

    let pos1 = [-100.0, 0.0, 0.0];

    let mut effect_builder = EffectBuilder::new()
        .add_effect(BaseEffect {
            kind: BaseEffectType::Strong { magnitude: 45_000 },
            ..Default::default()
        })
        .add_effect(BaseEffect {
            kind: BaseEffectType::Weak { magnitude: 45_000 },
            ..Default::default()
        })
        .distance_model(DistanceModel::None)
        .gamepads(&support_ff)
        .clone();

    let left_effect = effect_builder.position(pos1).finish(&mut gilrs).unwrap();

    left_effect.play().unwrap();

    println!("Playing one effects…");
    println!("Position of effect sources: {:?}", pos1);

    let mut listeners = support_ff
        .iter()
        .map(|&idx| (idx, [0.0, 0.0, 0.0]))
        .collect::<Vec<_>>();

    let mut ref_distance = 10.0;
    let mut rolloff_factor = 0.5;
    let mut max_distance = 100.0;
    let mut modify = Modify::DistModel;
    let mut model = 0usize;

    'main: loop {
        while let Some(event) = gilrs.next_event() {
            match event.event {
                EventType::ButtonReleased(Button::East, ..) => break 'main,
                EventType::ButtonReleased(Button::South, ..) => modify.next(),
                EventType::ButtonReleased(Button::West, ..) => modify.prev(),
                EventType::ButtonReleased(Button::LeftTrigger, ..)
                    if modify == Modify::DistModel =>
                {
                    model = model.wrapping_sub(1);
                }
                EventType::ButtonReleased(Button::RightTrigger, ..)
                    if modify == Modify::DistModel =>
                {
                    model = model.wrapping_add(1);
                }
                _ => (),
            }
        }

        for &mut (idx, ref mut pos) in &mut listeners {
            let velocity = 0.5;

            let gp = gilrs.gamepad(idx);
            let (sx, sy) = (gp.value(Axis::LeftStickX), gp.value(Axis::LeftStickY));

            if sx.abs() > 0.5 || sy.abs() > 0.5 {
                if sx.abs() > 0.5 {
                    pos[0] += velocity * sx.signum();
                }
                if sy.abs() > 0.5 {
                    pos[1] += velocity * sy.signum();
                }

                gilrs.gamepad(idx).set_listener_position(*pos).unwrap();

                let dist = ((pos[0] - pos1[0]).powi(2) + (pos[1] - pos1[1]).powi(2)).sqrt();
                print!(
                    "\x1b[2K\rPosition of listener {:2} has changed: [{:6.1}, {:6.1}].Distance: \
                     {:.1}",
                    idx, pos[0], pos[1], dist
                );
                io::stdout().flush().unwrap();
            }

            let x = if gp.is_pressed(Button::LeftTrigger) {
                -1.0
            } else if gp.is_pressed(Button::RightTrigger) {
                1.0
            } else {
                continue;
            };

            match modify {
                Modify::RolloffFactor => rolloff_factor += x * velocity * 0.1,
                Modify::RefDistance => ref_distance += x * velocity * 0.1,
                Modify::MaxDistance => max_distance += x * velocity * 1.0,
                Modify::DistModel => (), // DistanceModel handled in event loop
            }

            let model = match model % 4 {
                0 => DistanceModel::None,
                1 => DistanceModel::LinearClamped {
                    ref_distance,
                    rolloff_factor,
                    max_distance,
                },
                2 => DistanceModel::InverseClamped {
                    ref_distance,
                    rolloff_factor,
                    max_distance,
                },
                3 => DistanceModel::ExponentialClamped {
                    ref_distance,
                    rolloff_factor,
                    max_distance,
                },
                _ => unreachable!(),
            };

            match left_effect.set_distance_model(model) {
                Ok(()) => print!("\x1b[2K\r{:?}", model),
                Err(e) => print!("\x1b[2K\r{}", e),
            }
            io::stdout().flush().unwrap();
        }

        thread::sleep(Duration::from_millis(16));
    }
}

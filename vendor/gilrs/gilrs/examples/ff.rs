// Copyright 2016-2018 Mateusz Sieczko and other GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use gilrs::ff::{BaseEffect, BaseEffectType, EffectBuilder, Replay, Ticks};
use gilrs::Gilrs;
use std::thread;
use std::time::Duration;

fn main() {
    env_logger::init();
    let mut gilrs = Gilrs::new().unwrap();
    let support_ff = gilrs
        .gamepads()
        .filter_map(|(id, gp)| if gp.is_ff_supported() { Some(id) } else { None })
        .collect::<Vec<_>>();

    let duration = Ticks::from_ms(150);
    let effect = EffectBuilder::new()
        .add_effect(BaseEffect {
            kind: BaseEffectType::Strong { magnitude: 60_000 },
            scheduling: Replay {
                play_for: duration,
                with_delay: duration * 3,
                ..Default::default()
            },
            envelope: Default::default(),
        })
        .add_effect(BaseEffect {
            kind: BaseEffectType::Weak { magnitude: 60_000 },
            scheduling: Replay {
                after: duration * 2,
                play_for: duration,
                with_delay: duration * 3,
            },
            ..Default::default()
        })
        .gamepads(&support_ff)
        .finish(&mut gilrs)
        .unwrap();
    effect.play().unwrap();

    thread::sleep(Duration::from_secs(11));
}

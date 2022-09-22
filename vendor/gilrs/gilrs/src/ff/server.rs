// Copyright 2016-2018 Mateusz Sieczko and other GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use super::effect_source::{DistanceModel, EffectSource, EffectState, Magnitude};
use super::time::{Repeat, Ticks, TICK_DURATION};

use std::ops::{Deref, DerefMut};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use crate::gamepad::GamepadId;
use gilrs_core::FfDevice;

use vec_map::VecMap;

#[derive(Debug)]
pub(crate) enum Message {
    Create {
        id: usize,
        effect: Box<EffectSource>,
    },
    HandleCloned {
        id: usize,
    },
    HandleDropped {
        id: usize,
    },
    Play {
        id: usize,
    },
    Stop {
        id: usize,
    },
    Open {
        id: usize,
        device: FfDevice,
    },
    Close {
        id: usize,
    },
    SetListenerPosition {
        id: usize,
        position: [f32; 3],
    },
    SetGamepads {
        id: usize,
        gamepads: VecMap<()>,
    },
    AddGamepad {
        id: usize,
        gamepad_id: GamepadId,
    },
    SetRepeat {
        id: usize,
        repeat: Repeat,
    },
    SetDistanceModel {
        id: usize,
        model: DistanceModel,
    },
    SetPosition {
        id: usize,
        position: [f32; 3],
    },
    SetGain {
        id: usize,
        gain: f32,
    },
}

impl Message {
    // Whether to use trace level logging or debug
    fn use_trace_level(&self) -> bool {
        use self::Message::*;

        match self {
            &SetListenerPosition { .. } | &HandleCloned { .. } | &HandleDropped { .. } => true,
            _ => false,
        }
    }
}

#[derive(Debug)]
struct Device {
    inner: FfDevice,
    position: [f32; 3],
    gain: f32,
}

struct Effect {
    source: EffectSource,
    /// Number of created effect's handles.
    count: usize,
}

impl Effect {
    fn inc(&mut self) -> usize {
        self.count += 1;
        self.count
    }

    fn dec(&mut self) -> usize {
        self.count -= 1;
        self.count
    }
}

impl From<EffectSource> for Effect {
    fn from(source: EffectSource) -> Self {
        Effect { source, count: 1 }
    }
}

impl Deref for Effect {
    type Target = EffectSource;

    fn deref(&self) -> &Self::Target {
        &self.source
    }
}

impl DerefMut for Effect {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.source
    }
}

impl From<FfDevice> for Device {
    fn from(inner: FfDevice) -> Self {
        Device {
            inner,
            position: [0.0, 0.0, 0.0],
            gain: 1.0,
        }
    }
}

pub(crate) fn run(rx: Receiver<Message>) {
    let mut effects = VecMap::<Effect>::new();
    let mut devices = VecMap::<Device>::new();
    let sleep_dur = Duration::from_millis(TICK_DURATION.into());
    let mut tick = Ticks(0);

    loop {
        let t1 = Instant::now();
        while let Ok(ev) = rx.try_recv() {
            if ev.use_trace_level() {
                trace!("New ff event: {:?}", ev);
            } else {
                debug!("New ff event: {:?}", ev);
            }

            match ev {
                Message::Create { id, effect } => {
                    effects.insert(id, (*effect).into());
                }
                Message::Play { id } => {
                    if let Some(effect) = effects.get_mut(id) {
                        effect.source.state = EffectState::Playing { since: tick }
                    } else {
                        error!("{:?} with wrong ID", ev);
                    }
                }
                Message::Stop { id } => {
                    if let Some(effect) = effects.get_mut(id) {
                        effect.source.state = EffectState::Stopped
                    } else {
                        error!("{:?} with wrong ID", ev);
                    }
                }
                Message::Open { id, device } => {
                    devices.insert(id, device.into());
                }
                Message::Close { id } => {
                    devices.remove(id);
                }
                Message::SetListenerPosition { id, position } => {
                    if let Some(device) = devices.get_mut(id) {
                        device.position = position;
                    } else {
                        error!("{:?} with wrong ID", ev);
                    }
                }
                Message::HandleCloned { id } => {
                    if let Some(effect) = effects.get_mut(id) {
                        effect.inc();
                    } else {
                        error!("{:?} with wrong ID", ev);
                    }
                }
                Message::HandleDropped { id } => {
                    let mut drop = false;
                    if let Some(effect) = effects.get_mut(id) {
                        if effect.dec() == 0 {
                            drop = true;
                        }
                    } else {
                        error!("{:?} with wrong ID", ev);
                    }

                    if drop {
                        effects.remove(id);
                    }
                }
                Message::SetGamepads { id, gamepads } => {
                    if let Some(eff) = effects.get_mut(id) {
                        eff.source.devices = gamepads;
                    } else {
                        error!("Invalid effect id {} when changing gamepads.", id);
                    }
                }
                Message::AddGamepad { id, gamepad_id } => {
                    if let Some(eff) = effects.get_mut(id) {
                        eff.source.devices.insert(gamepad_id.0, ());
                    } else {
                        error!("Invalid effect id {} when changing gamepads.", id);
                    }
                }
                Message::SetRepeat { id, repeat } => {
                    if let Some(eff) = effects.get_mut(id) {
                        eff.source.repeat = repeat;
                    } else {
                        error!("Invalid effect id {} when changing repeat mode.", id);
                    }
                }
                Message::SetDistanceModel { id, model } => {
                    if let Some(eff) = effects.get_mut(id) {
                        eff.source.distance_model = model;
                    } else {
                        error!("Invalid effect id {} when changing distance model.", id);
                    }
                }
                Message::SetPosition { id, position } => {
                    if let Some(eff) = effects.get_mut(id) {
                        eff.source.position = position;
                    } else {
                        error!("Invalid effect id {}.", id);
                    }
                }
                Message::SetGain { id, gain } => {
                    if let Some(eff) = effects.get_mut(id) {
                        eff.source.gain = gain;
                    } else {
                        error!("Invalid effect id {} when changing effect gain.", id);
                    }
                }
            }
        }

        combine_and_play(&mut effects, &mut devices, tick);

        let dur = Instant::now().duration_since(t1);
        if dur > sleep_dur {
            // TODO: Should we add dur - sleep_dur to next iteration's dur?
            warn!(
                "One iteration of a force feedback loop took more than {}ms!",
                TICK_DURATION
            );
        } else {
            thread::sleep(sleep_dur - dur);
        }
        tick.inc();
    }
}

pub(crate) fn init() -> Sender<Message> {
    let (tx, _rx) = mpsc::channel();

    // Wasm doesn't support threads and force feedback
    #[cfg(not(target_arch = "wasm32"))]
    thread::spawn(move || run(_rx));

    tx
}

fn combine_and_play(effects: &mut VecMap<Effect>, devices: &mut VecMap<Device>, tick: Ticks) {
    for (dev_id, dev) in devices {
        let mut magnitude = Magnitude::zero();
        for (_, ref mut effect) in effects.iter_mut() {
            if effect.devices.contains_key(dev_id) {
                magnitude += effect.combine_base_effects(tick, dev.position);
            }
        }
        trace!(
            "({:?}) Setting ff state of {:?} to {:?}",
            tick,
            dev,
            magnitude
        );
        dev.inner.set_ff_state(
            magnitude.strong,
            magnitude.weak,
            Duration::from_millis(u64::from(TICK_DURATION) * 2),
        );
    }
}

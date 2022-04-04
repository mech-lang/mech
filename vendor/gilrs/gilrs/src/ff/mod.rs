// Copyright 2016-2018 Mateusz Sieczko and other GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

// This code is not used on wasm
#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

//! Force feedback module.
//!
//! To use force feedback, you have to create one or more [`Effect`s](struct.Effect.html). Each
//! `Effect` contains one or more [`BasicEffect`s](struct.BasicEffect.html) and parameters that
//! describe effect's source, like it's position, gain or used
//! [`DistanceModel`](enum.DistanceModel.html). Final strength of effect is based on saturating sum
//! (to `u16::MAX`) of all base effects and time from the start of playback, attenuation from
//! distance between effect source and listener (represented by gamepad) and effect's gain.
//!
//! See also [`Gilrs::set_listener_position()`](../struct.Gilrs.html#method.set_listener_position)
//! and [`Gamepad::is_ff_supported()`](../struct.Gamepad.html#method.is_ff_supported).
//!
//! # Example
//!
//! ```rust
//! use gilrs::Gilrs;
//! use gilrs::ff::{EffectBuilder, Replay, BaseEffect, BaseEffectType, Ticks};
//!
//! let mut gilrs = Gilrs::new().unwrap();
//! let support_ff = gilrs
//!     .gamepads()
//!     .filter_map(|(id, gp)| if gp.is_ff_supported() { Some(id) } else { None })
//!     .collect::<Vec<_>>();
//!
//! let duration = Ticks::from_ms(150);
//! let effect = EffectBuilder::new()
//!     .add_effect(BaseEffect {
//!         kind: BaseEffectType::Strong { magnitude: 60_000 },
//!         scheduling: Replay { play_for: duration, with_delay: duration * 3, ..Default::default() },
//!         envelope: Default::default(),
//!     })
//!     .add_effect(BaseEffect {
//!         kind: BaseEffectType::Weak { magnitude: 60_000 },
//!         scheduling: Replay { after: duration * 2, play_for: duration, with_delay: duration * 3 },
//!         ..Default::default()
//!     })
//!     .gamepads(&support_ff)
//!     .finish(&mut gilrs).unwrap();
//!
//! effect.play().unwrap();
//! ```
//!
//! See [`examples/ff_pos.rs`](https://gitlab.com/gilrs-project/gilrs/blob/v0.8.1/examples/ff_pos.rs) for
//! more advanced example.
mod base_effect;
mod effect_source;
pub(crate) mod server;
mod time;

pub use self::base_effect::{BaseEffect, BaseEffectType, Envelope, Replay};
pub use self::effect_source::{DistanceModel, DistanceModelError};
#[allow(unused_imports)]
pub(crate) use self::time::TICK_DURATION;
pub use self::time::{Repeat, Ticks};

use std::error::Error as StdError;
use std::hash::{Hash, Hasher};
use std::sync::mpsc::{SendError, Sender};
use std::{f32, fmt};

use self::effect_source::EffectSource;
use crate::ff::server::Message;
use crate::gamepad::{Gamepad, GamepadId, Gilrs};
use crate::utils;

use vec_map::VecMap;

/// Handle to force feedback effect.
///
/// `Effect` represents force feedback effect that can be played on one or more gamepads. It uses a
/// form of reference counting, so it can be cheaply cloned. To create new `Effect` use
/// [`EffectBuilder`](struct.EffectBuilder.html).
///
/// All methods on can return `Error::SendFailed` although it shouldn't normally happen.
pub struct Effect {
    id: usize,
    tx: Sender<Message>,
}

impl PartialEq for Effect {
    fn eq(&self, other: &Effect) -> bool {
        self.id == other.id
    }
}

impl Eq for Effect {}

impl Hash for Effect {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Clone for Effect {
    fn clone(&self) -> Self {
        let _ = self.tx.send(Message::HandleCloned { id: self.id });
        Effect {
            id: self.id,
            tx: self.tx.clone(),
        }
    }
}

impl Drop for Effect {
    fn drop(&mut self) {
        let _ = self.tx.send(Message::HandleDropped { id: self.id });
    }
}

impl Effect {
    /// Plays effect on all associated gamepads.
    pub fn play(&self) -> Result<(), Error> {
        self.tx.send(Message::Play { id: self.id })?;

        Ok(())
    }

    pub fn stop(&self) -> Result<(), Error> {
        self.tx.send(Message::Stop { id: self.id })?;

        Ok(())
    }

    /// Changes gamepads that are associated with effect. Effect will be only played on gamepads
    /// from last call to this function.
    ///
    /// # Errors
    ///
    /// Returns `Error::Disconnected(id)` or `Error::FfNotSupported(id)` on first gamepad in `ids`
    /// that is disconnected or doesn't support force feedback.
    pub fn set_gamepads(&self, ids: &[GamepadId], gilrs: &Gilrs) -> Result<(), Error> {
        let mut gamepads = VecMap::new();

        for dev in ids.iter().cloned() {
            if !gilrs
                .connected_gamepad(dev)
                .ok_or(Error::Disconnected(dev))?
                .is_ff_supported()
            {
                return Err(Error::FfNotSupported(dev));
            } else {
                gamepads.insert(dev.0, ());
            }
        }

        self.tx.send(Message::SetGamepads {
            id: self.id,
            gamepads,
        })?;

        Ok(())
    }

    /// Adds gamepad to the list of gamepads associated with effect.
    ///
    /// # Errors
    ///
    /// Returns `Error::Disconnected(id)` or `Error::FfNotSupported(id)` if gamepad is not connected
    /// or does not support force feedback.
    pub fn add_gamepad(&self, gamepad: &Gamepad<'_>) -> Result<(), Error> {
        if !gamepad.is_connected() {
            Err(Error::Disconnected(gamepad.id()))
        } else if !gamepad.is_ff_supported() {
            Err(Error::FfNotSupported(gamepad.id()))
        } else {
            self.tx.send(Message::AddGamepad {
                id: self.id,
                gamepad_id: gamepad.id(),
            })?;

            Ok(())
        }
    }

    /// Changes what should happen to effect when it ends.
    pub fn set_repeat(&self, repeat: Repeat) -> Result<(), Error> {
        self.tx.send(Message::SetRepeat {
            id: self.id,
            repeat,
        })?;

        Ok(())
    }

    /// Changes distance model associated with effect.
    ///
    /// # Errors
    ///
    /// Returns `Error::InvalidDistanceModel` if `model` is not valid. See
    /// [`DistanceModel`](enum.DistanceModelError.html) for details.
    pub fn set_distance_model(&self, model: DistanceModel) -> Result<(), Error> {
        model.validate()?;
        self.tx
            .send(Message::SetDistanceModel { id: self.id, model })?;

        Ok(())
    }

    /// Changes position of the source of effect.
    pub fn set_position<Vec3f: Into<[f32; 3]>>(&self, position: Vec3f) -> Result<(), Error> {
        let position = position.into();
        self.tx.send(Message::SetPosition {
            id: self.id,
            position,
        })?;

        Ok(())
    }

    /// Changes gain of the effect. `gain` will be clamped to \[0.0, f32::MAX\].
    pub fn set_gain(&self, gain: f32) -> Result<(), Error> {
        let gain = utils::clamp(gain, 0.0, f32::MAX);
        self.tx.send(Message::SetGain { id: self.id, gain })?;

        Ok(())
    }
}

/// Creates new [`Effect`](struct.Effect.html).
#[derive(Clone, PartialEq, Debug)]
pub struct EffectBuilder {
    base_effects: Vec<BaseEffect>,
    devices: VecMap<()>,
    repeat: Repeat,
    dist_model: DistanceModel,
    position: [f32; 3],
    gain: f32,
}

impl EffectBuilder {
    /// Creates new builder with following defaults: no gamepads, no base effects, repeat set to
    /// infinitely, no distance model, position in (0.0, 0.0, 0.0) and gain 1.0. Use `finish()` to
    /// create new effect.
    pub fn new() -> Self {
        EffectBuilder {
            base_effects: Vec::new(),
            devices: VecMap::new(),
            repeat: Repeat::Infinitely,
            dist_model: DistanceModel::None,
            position: [0.0, 0.0, 0.0],
            gain: 1.0,
        }
    }

    /// Adds new [`BaseEffect`](struct.BaseEffect.html).
    pub fn add_effect(&mut self, effect: BaseEffect) -> &mut Self {
        self.base_effects.push(effect);
        self
    }

    /// Changes gamepads that are associated with effect. Effect will be only played on gamepads
    /// from last call to this function.
    pub fn gamepads(&mut self, ids: &[GamepadId]) -> &mut Self {
        for dev in ids {
            self.devices.insert(dev.0, ());
        }
        self
    }

    /// Adds gamepad to the list of gamepads associated with effect.
    pub fn add_gamepad(&mut self, gamepad: &Gamepad<'_>) -> &mut Self {
        self.devices.insert(gamepad.id().0, ());

        self
    }

    /// Changes what should happen to effect when it ends.
    pub fn repeat(&mut self, repeat: Repeat) -> &mut Self {
        self.repeat = repeat;
        self
    }

    /// Changes distance model associated with effect.
    pub fn distance_model(&mut self, model: DistanceModel) -> &mut Self {
        self.dist_model = model;
        self
    }

    /// Changes position of the source of effect.
    pub fn position<Vec3f: Into<[f32; 3]>>(&mut self, position: Vec3f) -> &mut Self {
        self.position = position.into();
        self
    }

    /// Changes gain of the effect. `gain` will be clamped to \[0.0, f32::MAX\].
    pub fn gain(&mut self, gain: f32) -> &mut Self {
        self.gain = utils::clamp(gain, 0.0, f32::MAX);
        self
    }

    /// Validates all parameters and creates new effect.
    ///
    /// # Errors
    ///
    /// Returns `Error::Disconnected(id)` or `Error::FfNotSupported(id)` on first gamepad in `ids`
    /// that is disconnected or doesn't support force feedback.
    ///
    /// Returns `Error::InvalidDistanceModel` if `model` is not valid. See
    /// [`DistanceModel`](enum.DistanceModelError.html) for details.
    pub fn finish(&mut self, gilrs: &mut Gilrs) -> Result<Effect, Error> {
        for (dev, _) in &self.devices {
            let dev = GamepadId(dev);
            if !gilrs
                .connected_gamepad(dev)
                .ok_or(Error::Disconnected(dev))?
                .is_ff_supported()
            {
                return Err(Error::FfNotSupported(dev));
            }
        }

        self.dist_model.validate()?;

        let effect = EffectSource::new(
            self.base_effects.clone(),
            self.devices.clone(),
            self.repeat,
            self.dist_model,
            self.position,
            self.gain,
        );
        let id = gilrs.next_ff_id();
        let tx = gilrs.ff_sender();
        tx.send(Message::Create {
            id,
            effect: Box::new(effect),
        })?;
        Ok(Effect { id, tx: tx.clone() })
    }
}

/// Basic error type in force feedback module.
#[derive(Copy, Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Error {
    /// Force feedback is not supported by device with this ID
    FfNotSupported(GamepadId),
    /// Device is not connected
    Disconnected(GamepadId),
    /// Distance model is invalid.
    InvalidDistanceModel(DistanceModelError),
    /// The other end of channel was dropped.
    SendFailed,
    /// Unexpected error has occurred
    Other,
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Error::InvalidDistanceModel(m) => Some(m),
            _ => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sbuf;
        let s = match self {
            Error::FfNotSupported(id) => {
                sbuf = format!(
                    "force feedback is not supported by device with id {}.",
                    id.0
                );
                sbuf.as_ref()
            }
            Error::Disconnected(id) => {
                sbuf = format!("device with id {} is not connected.", id.0);
                sbuf.as_ref()
            }
            Error::InvalidDistanceModel(_) => "distance model is invalid",
            Error::SendFailed => "receiving end of a channel is disconnected.",
            Error::Other => "unespected error has occurred.",
        };

        fmt.write_str(s)
    }
}

impl<T> From<SendError<T>> for Error {
    fn from(_: SendError<T>) -> Self {
        Error::SendFailed
    }
}

impl From<DistanceModelError> for Error {
    fn from(f: DistanceModelError) -> Self {
        Error::InvalidDistanceModel(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope() {
        let env = Envelope {
            attack_length: Ticks(10),
            attack_level: 0.2,
            fade_length: Ticks(10),
            fade_level: 0.2,
        };
        let dur = Ticks(40);

        assert_eq!(env.at(Ticks(0), dur), 0.2);
        assert_eq!(env.at(Ticks(5), dur), 0.6);
        assert_eq!(env.at(Ticks(10), dur), 1.0);
        assert_eq!(env.at(Ticks(20), dur), 1.0);
        assert_eq!(env.at(Ticks(30), dur), 1.0);
        assert_eq!(env.at(Ticks(35), dur), 0.6);
        assert_eq!(env.at(Ticks(40), dur), 0.19999999);
    }

    #[test]
    fn envelope_default() {
        let env = Envelope::default();
        let dur = Ticks(40);

        assert_eq!(env.at(Ticks(0), dur), 1.0);
        assert_eq!(env.at(Ticks(20), dur), 1.0);
        assert_eq!(env.at(Ticks(40), dur), 1.0);
    }

    #[test]
    fn replay() {
        let replay = Replay {
            after: Ticks(10),
            play_for: Ticks(50),
            with_delay: Ticks(20),
        };

        assert_eq!(replay.at(Ticks(0)), 1.0);
        assert_eq!(replay.at(Ticks(9)), 1.0);
        assert_eq!(replay.at(Ticks(10)), 1.0);
        assert_eq!(replay.at(Ticks(30)), 1.0);
        assert_eq!(replay.at(Ticks(59)), 0.0);
        assert_eq!(replay.at(Ticks(60)), 0.0);
        assert_eq!(replay.at(Ticks(70)), 0.0);
    }
}

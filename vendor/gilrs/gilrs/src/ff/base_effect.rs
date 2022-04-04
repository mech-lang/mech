// Copyright 2016-2018 Mateusz Sieczko and other GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use std::ops::Mul;

use super::time::Ticks;

/// Kind of [`BaseEffect`](struct.BaseEffect.html).
///
/// Currently base effect support only xinput model of force feedback, which means that  gamepad
/// have weak and strong motor.
#[derive(Copy, Clone, PartialEq, Debug)]
#[non_exhaustive]
pub enum BaseEffectType {
    Weak { magnitude: u16 },
    Strong { magnitude: u16 },
}

impl BaseEffectType {
    fn magnitude(&self) -> u16 {
        match *self {
            BaseEffectType::Weak { magnitude } => magnitude,
            BaseEffectType::Strong { magnitude } => magnitude,
        }
    }
}

impl Mul<f32> for BaseEffectType {
    type Output = BaseEffectType;

    fn mul(self, rhs: f32) -> Self::Output {
        let mg = (self.magnitude() as f32 * rhs) as u16;
        match self {
            BaseEffectType::Weak { .. } => BaseEffectType::Weak { magnitude: mg },
            BaseEffectType::Strong { .. } => BaseEffectType::Strong { magnitude: mg },
        }
    }
}

impl Default for BaseEffectType {
    fn default() -> Self {
        BaseEffectType::Weak { magnitude: 0 }
    }
}

/// Basic building block used to create more complex force feedback effects.
///
/// For each base effect you can specify it's type, for how long should it be played and it's
/// strength during playback.
#[derive(Copy, Clone, PartialEq, Debug, Default)]
pub struct BaseEffect {
    /// Type of base effect.
    pub kind: BaseEffectType,
    /// Defines playback duration and delays between each repetition.
    pub scheduling: Replay,
    // TODO: maybe allow other f(t)?
    /// Basic attenuation function.
    pub envelope: Envelope,
}

impl BaseEffect {
    /// Returns `Weak` or `Strong` after applying envelope.
    pub(super) fn magnitude_at(&self, ticks: Ticks) -> BaseEffectType {
        if let Some(wrapped) = self.scheduling.wrap(ticks) {
            let att =
                self.scheduling.at(wrapped) * self.envelope.at(wrapped, self.scheduling.play_for);
            self.kind * att
        } else {
            self.kind * 0.0
        }
    }
}

// TODO: Image with "envelope"
#[derive(Copy, Clone, PartialEq, Debug, Default)]
/// Envelope shaped attenuation(time) function.
pub struct Envelope {
    pub attack_length: Ticks,
    pub attack_level: f32,
    pub fade_length: Ticks,
    pub fade_level: f32,
}

impl Envelope {
    pub(super) fn at(&self, ticks: Ticks, dur: Ticks) -> f32 {
        debug_assert!(self.fade_length < dur);
        debug_assert!(self.attack_length + self.fade_length < dur);

        if ticks < self.attack_length {
            self.attack_level
                + ticks.0 as f32 * (1.0 - self.attack_level) / self.attack_length.0 as f32
        } else if ticks + self.fade_length > dur {
            1.0 + (ticks + self.fade_length - dur).0 as f32 * (self.fade_level - 1.0)
                / self.fade_length.0 as f32
        } else {
            1.0
        }
    }
}

/// Defines scheduling of the basic force feedback effect.
///
/// ```text
///        ____________            ____________            ____________
///        |          |            |          |            |
/// _______|          |____________|          |____________|
///  after   play_for   with_delay   play_for   with_delay   play_for
/// ```
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Replay {
    /// Start playback `after` ticks after `Effect::play()` is called.
    pub after: Ticks,
    /// Playback duration.
    pub play_for: Ticks,
    /// If playback should be repeated delay it for `with_delay` ticks.
    pub with_delay: Ticks,
}

impl Replay {
    pub(super) fn at(&self, ticks: Ticks) -> f32 {
        if ticks >= self.play_for {
            0.0
        } else {
            1.0
        }
    }

    /// Returns duration of effect calculated as `play_for + with_delay`.
    pub fn dur(&self) -> Ticks {
        self.play_for + self.with_delay
    }

    /// Returns `None` if effect hasn't started; or wrapped value
    fn wrap(&self, ticks: Ticks) -> Option<Ticks> {
        ticks.checked_sub(self.after).map(|t| t % self.dur())
    }
}

impl Default for Replay {
    fn default() -> Self {
        Replay {
            after: Ticks(0),
            play_for: Ticks(1),
            with_delay: Ticks(0),
        }
    }
}

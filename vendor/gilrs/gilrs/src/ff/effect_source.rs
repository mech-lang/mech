// Copyright 2016-2018 Mateusz Sieczko and other GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use std::error::Error;
use std::ops::{AddAssign, Mul};
use std::{fmt, u16};

use super::base_effect::{BaseEffect, BaseEffectType};
use super::time::{Repeat, Ticks};

use vec_map::VecMap;

/// Specifies how distance between effect source and listener attenuates effect.
///
/// They are based on
/// [OpenAL Specification](http://openal.org/documentation/openal-1.1-specification.pdf) (chapter
/// 3.4), but the best way to see how they differ is to run `ff_pos` example.
///
/// Make sure that all parameters are ≥ 0. Additionally `Linear` and `LinearClamped` models don't
/// like if `ref_distance == max_distance` while others would prefer `ref_distance > 0`.
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum DistanceModel {
    /// Effect is not attenuated by distance.
    None,
    /// Linear distance model.
    Linear {
        ref_distance: f32,
        rolloff_factor: f32,
        max_distance: f32,
    },
    /// Linear distance clamped model.
    LinearClamped {
        ref_distance: f32,
        rolloff_factor: f32,
        max_distance: f32,
    },
    /// Inverse distance model.
    Inverse {
        ref_distance: f32,
        rolloff_factor: f32,
    },
    /// Inverse distance clamped model.
    InverseClamped {
        ref_distance: f32,
        rolloff_factor: f32,
        max_distance: f32,
    },
    /// Exponential distance model.
    Exponential {
        ref_distance: f32,
        rolloff_factor: f32,
    },
    /// Exponential distance clamped model.
    ExponentialClamped {
        ref_distance: f32,
        rolloff_factor: f32,
        max_distance: f32,
    },
}

impl DistanceModel {
    fn attenuation(self, mut distance: f32) -> f32 {
        // For now we will follow OpenAL[1] specification for distance models. See chapter 3.4 for
        // more details.
        //
        // [1]: http://openal.org/documentation/openal-1.1-specification.pdf
        match self {
            DistanceModel::Linear {
                ref_distance,
                max_distance,
                rolloff_factor,
            } => {
                distance = distance.min(max_distance);

                1.0 - rolloff_factor * (distance - ref_distance) / (max_distance - ref_distance)
            }
            DistanceModel::LinearClamped {
                ref_distance,
                max_distance,
                rolloff_factor,
            } => {
                distance = distance.max(ref_distance);
                distance = distance.min(max_distance);

                1.0 - rolloff_factor * (distance - ref_distance) / (max_distance - ref_distance)
            }
            DistanceModel::Inverse {
                ref_distance,
                rolloff_factor,
            } => ref_distance / (ref_distance + rolloff_factor * (distance - ref_distance)),
            DistanceModel::InverseClamped {
                ref_distance,
                max_distance,
                rolloff_factor,
            } => {
                distance = distance.max(ref_distance);
                distance = distance.min(max_distance);

                ref_distance / (ref_distance + rolloff_factor * (distance - ref_distance))
            }
            DistanceModel::Exponential {
                ref_distance,
                rolloff_factor,
            } => (distance / ref_distance).powf(-rolloff_factor),
            DistanceModel::ExponentialClamped {
                ref_distance,
                max_distance,
                rolloff_factor,
            } => {
                distance = distance.max(ref_distance);
                distance = distance.min(max_distance);

                (distance / ref_distance).powf(-rolloff_factor)
            }
            DistanceModel::None => 1.0,
        }
    }

    pub(crate) fn validate(self) -> Result<(), DistanceModelError> {
        let (ref_distance, rolloff_factor, max_distance) = match self {
            DistanceModel::Inverse {
                ref_distance,
                rolloff_factor,
            } => {
                if ref_distance <= 0.0 {
                    return Err(DistanceModelError::InvalidModelParameter);
                }

                (ref_distance, rolloff_factor, 0.0)
            }
            DistanceModel::InverseClamped {
                ref_distance,
                max_distance,
                rolloff_factor,
            } => {
                if ref_distance <= 0.0 {
                    return Err(DistanceModelError::InvalidModelParameter);
                }

                (ref_distance, rolloff_factor, max_distance)
            }
            DistanceModel::Linear {
                ref_distance,
                max_distance,
                rolloff_factor,
            } => {
                if ref_distance == max_distance {
                    return Err(DistanceModelError::InvalidModelParameter);
                }

                (ref_distance, rolloff_factor, max_distance)
            }
            DistanceModel::LinearClamped {
                ref_distance,
                max_distance,
                rolloff_factor,
            } => {
                if ref_distance == max_distance {
                    return Err(DistanceModelError::InvalidModelParameter);
                }

                (ref_distance, rolloff_factor, max_distance)
            }
            DistanceModel::Exponential {
                ref_distance,
                rolloff_factor,
            } => {
                if ref_distance <= 0.0 {
                    return Err(DistanceModelError::InvalidModelParameter);
                }

                (ref_distance, rolloff_factor, 0.0)
            }
            DistanceModel::ExponentialClamped {
                ref_distance,
                max_distance,
                rolloff_factor,
            } => {
                if ref_distance <= 0.0 {
                    return Err(DistanceModelError::InvalidModelParameter);
                }

                (ref_distance, rolloff_factor, max_distance)
            }
            DistanceModel::None => (0.0, 0.0, 0.0),
        };

        if ref_distance < 0.0 {
            Err(DistanceModelError::InvalidReferenceDistance)
        } else if rolloff_factor < 0.0 {
            Err(DistanceModelError::InvalidRolloffFactor)
        } else if max_distance < 0.0 {
            Err(DistanceModelError::InvalidMaxDistance)
        } else {
            Ok(())
        }
    }
}

impl Default for DistanceModel {
    fn default() -> Self {
        DistanceModel::None
    }
}

/// Error that can be returned when passing [`DistanceModel`](struct.DistanceModel.html) with
/// invalid value.
#[derive(Copy, Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum DistanceModelError {
    /// Reference distance is < 0.
    InvalidReferenceDistance,
    /// Rolloff factor is < 0.
    InvalidRolloffFactor,
    /// Max distance is < 0.
    InvalidMaxDistance,
    /// Possible divide by zero
    InvalidModelParameter,
}

impl Error for DistanceModelError {}

impl fmt::Display for DistanceModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            DistanceModelError::InvalidReferenceDistance => "reference distance is < 0",
            DistanceModelError::InvalidRolloffFactor => "rolloff factor is < 0",
            DistanceModelError::InvalidMaxDistance => "max distance is < 0",
            DistanceModelError::InvalidModelParameter => "possible divide by zero",
        };

        f.write_str(s)
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub(super) enum EffectState {
    Playing { since: Ticks },
    Stopped,
}

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct EffectSource {
    base_effects: Vec<BaseEffect>,
    // TODO: Use bitset
    pub(super) devices: VecMap<()>,
    pub(super) repeat: Repeat,
    pub(super) distance_model: DistanceModel,
    pub(super) position: [f32; 3],
    pub(super) gain: f32,
    pub(super) state: EffectState,
}

impl EffectSource {
    pub(super) fn new(
        base_effects: Vec<BaseEffect>,
        devices: VecMap<()>,
        repeat: Repeat,
        dist_model: DistanceModel,
        position: [f32; 3],
        gain: f32,
    ) -> Self {
        EffectSource {
            base_effects,
            devices,
            repeat,
            distance_model: dist_model,
            position,
            gain,
            state: EffectState::Stopped,
        }
    }

    pub(super) fn combine_base_effects(&mut self, ticks: Ticks, actor_pos: [f32; 3]) -> Magnitude {
        let ticks = match self.state {
            EffectState::Playing { since } => {
                debug_assert!(ticks >= since);
                ticks - since
            }
            EffectState::Stopped => return Magnitude::zero(),
        };

        match self.repeat {
            Repeat::For(max_dur) if ticks > max_dur => {
                self.state = EffectState::Stopped;
            }
            _ => (),
        }

        let attenuation = self
            .distance_model
            .attenuation(self.position.distance(actor_pos))
            * self.gain;
        if attenuation < 0.05 {
            return Magnitude::zero();
        }

        let mut final_magnitude = Magnitude::zero();
        for effect in &self.base_effects {
            match effect.magnitude_at(ticks) {
                BaseEffectType::Strong { magnitude } => {
                    final_magnitude.strong = final_magnitude.strong.saturating_add(magnitude)
                }
                BaseEffectType::Weak { magnitude } => {
                    final_magnitude.weak = final_magnitude.weak.saturating_add(magnitude)
                }
            };
        }
        final_magnitude * attenuation
    }
}

/// (strong, weak) pair.
#[derive(Copy, Clone, Debug)]
pub(super) struct Magnitude {
    pub strong: u16,
    pub weak: u16,
}

impl Magnitude {
    pub fn zero() -> Self {
        Magnitude { strong: 0, weak: 0 }
    }
}

impl Mul<f32> for Magnitude {
    type Output = Magnitude;

    fn mul(self, rhs: f32) -> Self::Output {
        debug_assert!(rhs >= 0.0);
        let strong = self.strong as f32 * rhs;
        let strong = if strong > u16::MAX as f32 {
            u16::MAX
        } else {
            strong as u16
        };
        let weak = self.weak as f32 * rhs;
        let weak = if weak > u16::MAX as f32 {
            u16::MAX
        } else {
            weak as u16
        };
        Magnitude { strong, weak }
    }
}

impl AddAssign for Magnitude {
    fn add_assign(&mut self, rhs: Magnitude) {
        self.strong = self.strong.saturating_add(rhs.strong);
        self.weak = self.weak.saturating_add(rhs.weak);
    }
}

trait SliceVecExt {
    type Base;

    fn distance(self, from: Self) -> Self::Base;
}

impl SliceVecExt for [f32; 3] {
    type Base = f32;

    fn distance(self, from: Self) -> f32 {
        ((from[0] - self[0]).powi(2) + (from[1] - self[1]).powi(2) + (from[2] - self[2]).powi(2))
            .sqrt()
    }
}

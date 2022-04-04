// Copyright 2016-2018 Mateusz Sieczko and other GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use std::ops::{Add, AddAssign, Mul, MulAssign, Rem, Sub, SubAssign};
use std::time::Duration;

use crate::utils;

pub(crate) const TICK_DURATION: u32 = 50;

/// Represents duration.
///
/// This type is only useful as input parameter for other functions in force feedback module. To
/// create it, use `from_ms()` method. Keep in mind that `Ticks` **is not precise** representation
/// of time.
///
/// # Example
///
/// ```rust
/// use gilrs::ff::Ticks;
/// use std::time::Duration;
///
/// let t1 = Ticks::from_ms(110);
/// let t2 = Ticks::from(Duration::from_millis(130));
///
/// /// `Ticks` is not precise.
/// assert_eq!(t1, t2);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Ticks(pub(super) u32);

impl Ticks {
    pub fn from_ms(dur: u32) -> Self {
        Ticks(utils::ceil_div(dur, TICK_DURATION))
    }

    pub(super) fn inc(&mut self) {
        self.0 += 1
    }

    pub(super) fn checked_sub(self, rhs: Ticks) -> Option<Ticks> {
        self.0.checked_sub(rhs.0).map(Ticks)
    }
}

impl From<Duration> for Ticks {
    fn from(dur: Duration) -> Self {
        Ticks::from_ms(dur.as_secs() as u32 * 1000 + dur.subsec_millis())
    }
}

impl Add for Ticks {
    type Output = Ticks;

    fn add(self, rhs: Ticks) -> Self::Output {
        Ticks(self.0 + rhs.0)
    }
}

impl AddAssign for Ticks {
    fn add_assign(&mut self, rhs: Ticks) {
        self.0 += rhs.0
    }
}

impl Sub for Ticks {
    type Output = Ticks;

    fn sub(self, rhs: Ticks) -> Self::Output {
        Ticks(self.0 - rhs.0)
    }
}

impl SubAssign for Ticks {
    fn sub_assign(&mut self, rhs: Ticks) {
        self.0 -= rhs.0
    }
}

impl Mul<u32> for Ticks {
    type Output = Ticks;

    fn mul(self, rhs: u32) -> Self::Output {
        Ticks(self.0 * rhs)
    }
}

impl MulAssign<u32> for Ticks {
    fn mul_assign(&mut self, rhs: u32) {
        self.0 *= rhs;
    }
}

impl Rem for Ticks {
    type Output = Ticks;

    fn rem(self, rhs: Ticks) -> Self::Output {
        Ticks(self.0 % rhs.0)
    }
}

/// Describes how long effect should be played.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Repeat {
    /// Play effect until stop() is called.
    Infinitely,
    /// Play effect for specified time.
    For(Ticks),
}

impl Default for Repeat {
    fn default() -> Self {
        Repeat::Infinitely
    }
}

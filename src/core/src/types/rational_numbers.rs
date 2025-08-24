use crate::*;
use super::*;

// Rational Numbers
// ----------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, PartialOrd)]
pub struct RationalNumber(pub Rational64);

impl RationalNumber {
  pub fn new(numer: i64, denom: i64) -> RationalNumber {
    RationalNumber(Rational64::new(numer, denom))
  }

  pub fn from_f64(f: f64) -> Option<RationalNumber> {
    match Rational64::from_f64(f) {
      Some(r) => Some(RationalNumber(r)),
      None => None,
    }
  }

  pub fn to_f64(&self) -> Option<f64> {
    match self.0.to_f64() {
      Some(val) => Some(val),
      None => None,
    }
  }

  pub fn numer(&self) -> &i64 {
    self.0.numer()
  }

  pub fn denom(&self) -> &i64 {
    self.0.denom()
  }
}

impl std::fmt::Display for RationalNumber {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.pretty_print())
  }
}


impl Default for RationalNumber {
  fn default() -> Self {
    RationalNumber(Rational64::default())
  }
}


impl PrettyPrint for RationalNumber {
  fn pretty_print(&self) -> String {
    format!("{}/{}", self.numer(), self.denom())
  }
}

impl Mul<RationalNumber> for RationalNumber {
  type Output = RationalNumber;
  fn mul(self, other: RationalNumber) -> RationalNumber {
    RationalNumber(self.0 * other.0)
  }
}

impl One for RationalNumber {
  fn one() -> Self {
    RationalNumber(Rational64::one())
  }
  fn is_one(&self) -> bool {
    self.0.is_one()
  }
}

impl Add<RationalNumber> for RationalNumber {
  type Output = RationalNumber;
  fn add(self, other: RationalNumber) -> RationalNumber {
    RationalNumber(self.0 + other.0)
  }
}

impl AddAssign<RationalNumber> for RationalNumber {
  fn add_assign(&mut self, other: RationalNumber) {
    self.0 += other.0;
  }
}

impl Sub<RationalNumber> for RationalNumber {
  type Output = RationalNumber;
  fn sub(self, other: RationalNumber) -> RationalNumber {
    RationalNumber(self.0 - other.0)
  }
}

impl Div<RationalNumber> for RationalNumber {
  type Output = RationalNumber;
  fn div(self, other: RationalNumber) -> RationalNumber {
    RationalNumber(self.0 / other.0)
  }
}

impl DivAssign<RationalNumber> for RationalNumber {
  fn div_assign(&mut self, other: RationalNumber) {
    self.0 /= other.0;
  }
}


impl SubAssign<RationalNumber> for RationalNumber {
  fn sub_assign(&mut self, other: RationalNumber) {
    self.0 -= other.0;
  }
}

impl MulAssign<RationalNumber> for RationalNumber {
  fn mul_assign(&mut self, other: RationalNumber) {
    self.0 *= other.0;
  }
}

impl Zero for RationalNumber {
  fn zero() -> Self {
    RationalNumber(Rational64::zero())
  }
  fn is_zero(&self) -> bool {
    self.0.is_zero()
  }
}

impl Neg for RationalNumber {
  type Output = Self;
  fn neg(self) -> Self::Output {
    RationalNumber(-self.0)
  }
}

#[cfg(feature = "f64")]
impl From<RationalNumber> for F64 {
  fn from(r: RationalNumber) -> Self {
    F64::new(r.0.to_f64().unwrap())
  }
}

#[cfg(feature = "f64")]
impl From<F64> for RationalNumber {
  fn from(f: F64) -> Self {
    RationalNumber(Rational64::from_f64(f.0).unwrap())
  }
}

impl ToUsize for RationalNumber {
  fn to_usize(&self) -> usize {
    self.0.to_integer() as usize
  }
}

impl ToValue for RationalNumber {
  fn to_value(&self) -> Value {
    Value::RationalNumber(Ref::new(*self))
  }
}
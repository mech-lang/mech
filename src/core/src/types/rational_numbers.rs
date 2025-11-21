use crate::*;
use super::*;

// Rational Numbers
// ----------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, PartialOrd)]
pub struct R64(pub Rational64);

impl R64 {
  pub fn new(numer: i64, denom: i64) -> R64 {
    R64(Rational64::new(numer, denom))
  }

  pub fn abs(&self) -> Self {
    R64(self.0.abs())
  }

  pub fn from_le_bytes(bytes: &[u8; 16]) -> R64 {
    let numer = match bytes[0..8].try_into() {
      Ok(arr) => i64::from_le_bytes(arr),
      Err(_) => panic!("Failed to read numerator from bytes"),
    };
    let denom = match bytes[8..16].try_into() {
      Ok(arr) => i64::from_le_bytes(arr),
      Err(_) => panic!("Failed to read denominator from bytes"),
    };
    if denom == 0 {
      panic!("Denominator cannot be zero");
    }
    R64(Rational64::new(numer, denom))
  }

  pub fn from_f64(f: f64) -> Option<R64> {
    match Rational64::from_f64(f) {
      Some(r) => Some(R64(r)),
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

impl std::fmt::Display for R64 {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.pretty_print())
  }
}


impl Default for R64 {
  fn default() -> Self {
    R64(Rational64::default())
  }
}


impl PrettyPrint for R64 {
  fn pretty_print(&self) -> String {
    format!("{}/{}", self.numer(), self.denom())
  }
}

impl Mul<R64> for R64 {
  type Output = R64;
  fn mul(self, other: R64) -> R64 {
    R64(self.0 * other.0)
  }
}

impl One for R64 {
  fn one() -> Self {
    R64(Rational64::one())
  }
  fn is_one(&self) -> bool {
    self.0.is_one()
  }
}

impl Add<R64> for R64 {
  type Output = R64;
  fn add(self, other: R64) -> R64 {
    R64(self.0 + other.0)
  }
}

impl AddAssign<R64> for R64 {
  fn add_assign(&mut self, other: R64) {
    self.0 += other.0;
  }
}

impl Sub<R64> for R64 {
  type Output = R64;
  fn sub(self, other: R64) -> R64 {
    R64(self.0 - other.0)
  }
}

impl Div<R64> for R64 {
  type Output = R64;
  fn div(self, other: R64) -> R64 {
    R64(self.0 / other.0)
  }
}

impl DivAssign<R64> for R64 {
  fn div_assign(&mut self, other: R64) {
    self.0 /= other.0;
  }
}


impl SubAssign<R64> for R64 {
  fn sub_assign(&mut self, other: R64) {
    self.0 -= other.0;
  }
}

impl MulAssign<R64> for R64 {
  fn mul_assign(&mut self, other: R64) {
    self.0 *= other.0;
  }
}

impl Zero for R64 {
  fn zero() -> Self {
    R64(Rational64::zero())
  }
  fn is_zero(&self) -> bool {
    self.0.is_zero()
  }
}

impl Neg for R64 {
  type Output = Self;
  fn neg(self) -> Self::Output {
    R64(-self.0)
  }
}

#[cfg(feature = "f64")]
impl From<R64> for f64 {
  fn from(r: R64) -> Self {
    r.to_f64().unwrap()
  }
}
  
#[cfg(feature = "f64")]
impl From<f64> for R64 {
  fn from(f: f64) -> Self {
    R64(Rational64::from_f64(f).unwrap())
  }
}

impl ToUsize for R64 {
  fn to_usize(&self) -> usize {
    self.0.to_integer() as usize
  }
}

impl ToValue for R64 {
  fn to_value(&self) -> Value {
    Value::R64(Ref::new(*self))
  }
}
use crate::*;
use super::*;
use nalgebra::Complex;

// Complex Numbers
// ----------------------------------------------------------------------------

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct C64(pub Complex<f64>);

impl fmt::Display for C64 {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.pretty_print())
  }
}

impl PartialOrd for C64 {
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    Some(self.0.norm().partial_cmp(&other.0.norm()).unwrap())
  }
}

impl Default for C64 {
  fn default() -> Self {
    C64(Complex::new(0.0, 0.0))
  }
}

impl Eq for C64 {}

impl Hash for C64 {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.0.re.to_bits().hash(state);
    self.0.im.to_bits().hash(state);
  }
}

impl PrettyPrint for C64 {
  fn pretty_print(&self) -> String {
    if self.0.re == 0.0 {
      return format!("{}i", self.0.im);
    } else if self.0.im >= 0.0 {
      format!("{}+{}i", self.0.re, self.0.im)
    } else {
      format!("{}{}i", self.0.re, self.0.im)
    }
  }
}

impl C64 {
  pub fn new(real: f64, imag: f64) -> C64 {
    C64(Complex::new(real, imag))
  }

  pub fn to_html(&self) -> String {
    let pretty = self.pretty_print();
    format!("<span class='mech-complex-number'>{}</span>", pretty)
  }

  pub fn from_le_bytes(bytes: &[u8; 16]) -> C64 {
    let real = match bytes[0..8].try_into() {
      Ok(arr) => f64::from_le_bytes(arr),
      Err(_) => panic!("Failed to read real part from bytes"),
    };
    let imag = match bytes[8..16].try_into() {
      Ok(arr) => f64::from_le_bytes(arr),
      Err(_) => panic!("Failed to read imaginary part from bytes"),
    };
    C64(Complex::new(real, imag))
  }

}


impl Add for C64 {
  type Output = C64;
  fn add(self, other: C64) -> C64 {
    C64(self.0 + other.0)
  }
}

impl Mul for C64 {
  type Output = C64;
  fn mul(self, other: C64) -> C64 {
    C64(self.0 * other.0)
  }
}

impl Sub for C64 {
  type Output = C64;
  fn sub(self, other: C64) -> C64 {
    C64(self.0 - other.0)
  }
}

impl Div for C64 {
  type Output = C64;
  fn div(self, other: C64) -> C64 {
    C64(self.0 / other.0)
  }
}

impl AddAssign for C64 {
  fn add_assign(&mut self, other: C64) {
    self.0 += other.0;
  }
}

impl SubAssign for C64 {
  fn sub_assign(&mut self, other: C64) {
    self.0 -= other.0;
  }
}

impl MulAssign for C64 {
  fn mul_assign(&mut self, other: C64) {
    self.0 *= other.0;
  }
}


impl DivAssign for C64 {
  fn div_assign(&mut self, other: C64) {
    self.0 /= other.0;
  }
}

impl Zero for C64 {
  fn zero() -> Self {
    C64(Complex::new(0.0, 0.0))
  }
  fn is_zero(&self) -> bool {
    self.0.re == 0.0 && self.0.im == 0.0
  }
}

impl One for C64 {
  fn one() -> Self {
    C64(Complex::new(1.0, 0.0))
  }
  fn is_one(&self) -> bool {
    self.0 == Complex::new(1.0, 0.0)
  }
}

impl Neg for C64 {
  type Output = Self;
  fn neg(self) -> Self::Output {
    C64(-self.0)
  }
}

impl ToUsize for C64 {
  fn to_usize(&self) -> usize {
    self.0.norm() as usize
  }
}

impl ToValue for C64 {
  fn to_value(&self) -> Value {
    Value::C64(Ref::new(*self))
  }
}
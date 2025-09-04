use crate::*;
use super::*;
use nalgebra::Complex;

// Complex Numbers
// ----------------------------------------------------------------------------

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct ComplexNumber(pub Complex<f64>);

impl fmt::Display for ComplexNumber {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.pretty_print())
  }
}

impl PartialOrd for ComplexNumber {
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    Some(self.0.norm().partial_cmp(&other.0.norm()).unwrap())
  }
}

impl Default for ComplexNumber {
  fn default() -> Self {
    ComplexNumber(Complex::new(0.0, 0.0))
  }
}

impl Eq for ComplexNumber {}

impl Hash for ComplexNumber {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.0.re.to_bits().hash(state);
    self.0.im.to_bits().hash(state);
  }
}

impl PrettyPrint for ComplexNumber {
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

impl ComplexNumber {
  pub fn new(real: f64, imag: f64) -> ComplexNumber {
    ComplexNumber(Complex::new(real, imag))
  }

  pub fn to_html(&self) -> String {
    let pretty = self.pretty_print();
    format!("<span class='mech-complex-number'>{}</span>", pretty)
  }

  pub fn from_le_bytes(bytes: &[u8; 16]) -> ComplexNumber {
    let real = match bytes[0..8].try_into() {
      Ok(arr) => f64::from_le_bytes(arr),
      Err(_) => panic!("Failed to read real part from bytes"),
    };
    let imag = match bytes[8..16].try_into() {
      Ok(arr) => f64::from_le_bytes(arr),
      Err(_) => panic!("Failed to read imaginary part from bytes"),
    };
    ComplexNumber(Complex::new(real, imag))
  }

}


impl Add for ComplexNumber {
  type Output = ComplexNumber;
  fn add(self, other: ComplexNumber) -> ComplexNumber {
    ComplexNumber(self.0 + other.0)
  }
}

impl Mul for ComplexNumber {
  type Output = ComplexNumber;
  fn mul(self, other: ComplexNumber) -> ComplexNumber {
    ComplexNumber(self.0 * other.0)
  }
}

impl Sub for ComplexNumber {
  type Output = ComplexNumber;
  fn sub(self, other: ComplexNumber) -> ComplexNumber {
    ComplexNumber(self.0 - other.0)
  }
}

impl Div for ComplexNumber {
  type Output = ComplexNumber;
  fn div(self, other: ComplexNumber) -> ComplexNumber {
    ComplexNumber(self.0 / other.0)
  }
}

impl AddAssign for ComplexNumber {
  fn add_assign(&mut self, other: ComplexNumber) {
    self.0 += other.0;
  }
}

impl SubAssign for ComplexNumber {
  fn sub_assign(&mut self, other: ComplexNumber) {
    self.0 -= other.0;
  }
}

impl MulAssign for ComplexNumber {
  fn mul_assign(&mut self, other: ComplexNumber) {
    self.0 *= other.0;
  }
}


impl DivAssign for ComplexNumber {
  fn div_assign(&mut self, other: ComplexNumber) {
    self.0 /= other.0;
  }
}

impl Zero for ComplexNumber {
  fn zero() -> Self {
    ComplexNumber(Complex::new(0.0, 0.0))
  }
  fn is_zero(&self) -> bool {
    self.0.re == 0.0 && self.0.im == 0.0
  }
}

impl One for ComplexNumber {
  fn one() -> Self {
    ComplexNumber(Complex::new(1.0, 0.0))
  }
  fn is_one(&self) -> bool {
    self.0 == Complex::new(1.0, 0.0)
  }
}

impl Neg for ComplexNumber {
  type Output = Self;
  fn neg(self) -> Self::Output {
    ComplexNumber(-self.0)
  }
}

impl ToUsize for ComplexNumber {
  fn to_usize(&self) -> usize {
    self.0.norm() as usize
  }
}

impl ToValue for ComplexNumber {
  fn to_value(&self) -> Value {
    Value::ComplexNumber(Ref::new(*self))
  }
}
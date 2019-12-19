// |DDDDDDDD|RRRRRRR|SMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMM|
// D: Domain [0, 254]
// R: Range [-64, 63]
// S: mantissa Sign bit
// M: Mantissa [-2^48, 2^48 - 1]
// Credit: Chris Granger, who implemented this for Eve v0.4
// Also credit to Josh Cole for coming up with the spec
// Adapted and extended for Mech by Corey Montella

use core::mem;
use errors::{Error, ErrorType};
#[cfg(feature = "no-std")] use alloc::string::String;
//#[cfg(feature = "no-std")] use num::traits::float::FloatCore;
#[cfg(feature = "no-std")] use libm::F64Ext;

const MANTISSA_MASK:u64 = (((1 as u64) << 49) as u64 - 1); // 49 bits at the end
const META_MASK:u64 = ((1 << 15) as u64 - 1) << 49; // 15 1s at the front
const OVERFLOW_MASK:u64 = ((1 << 16) as u64 - 1) << 48; // 15 1s at the front
const RANGE_MASK:u64 = ((1 << 7) as u64 - 1) << 49;
const SHIFTED_RANGE_DOMAIN_MASK:u64 = ((1 << 7) as u64 - 1);
const SHIFTED_FILL:u64 = ((((1 as u64) << 57) as u64 - 1) << 7);
const SIGN_MASK:u64 = 1 << 48;

pub type Quantity = u64;

pub trait ToQuantity {
    fn to_quantity(&self) -> u64;
}

pub trait FromQuantity<T> {
    fn get_value(self) -> T;
}

impl ToQuantity for u32 {
    #[inline(always)]
    fn to_quantity(&self) -> u64 {
        let result:u64 = (*self).into();
        result | (1 << 63)
    }
}

impl ToQuantity for i32 {
    #[inline(always)]
    fn to_quantity(&self) -> u64 {
        let me = *self;
        if me.is_negative() {
            me as u64 & MANTISSA_MASK
        } else {
            me as u64
        }
    }
}

impl ToQuantity for u64 {
    #[inline(always)]
    fn to_quantity(&self) -> u64 {
        let me = *self;
        if me & META_MASK != 0 {
            let (mantissa, range) = overflow_handler(me);
            (mantissa as u64) & MANTISSA_MASK | shifted_range(range)
        } else {
            me & MANTISSA_MASK
        }
    }
}

impl ToQuantity for i64 {
    #[inline(always)]
    fn to_quantity(&self) -> u64 {
        let me = *self;
        if me.is_negative() {
            if (me as u64) & META_MASK != META_MASK {
                let (mantissa, range) = overflow_handler(me.abs() as u64);
                !(mantissa - 1) & MANTISSA_MASK | shifted_range(range)
            } else {
                (me as u64) & MANTISSA_MASK
            }
        } else if (me as u64) & OVERFLOW_MASK != 0 {
            let (mantissa, range) = overflow_handler(me as u64);
            (mantissa as u64) & MANTISSA_MASK | shifted_range(range)
        } else {
            (me as u64) & MANTISSA_MASK
        }
    }
}

impl ToQuantity for f64 {
  #[inline(always)]
  fn to_quantity(&self) -> u64 {
    let me = *self;
    let (mantissa, exponent, sign) = integer_decode_f64(me);
    if mantissa == 0 {
      let result = make_quantity(0,0,0);
      result
    } else {
      let exp_log = 2f64.powf(exponent as f64).log10();
      let real_exponent = exp_log.floor() as i64 + 1;
      let real_mantissa = (((mantissa as f64) * 10f64.powf(exp_log.fract()))) as i64;
      let mut result = real_mantissa.to_quantity();
      if sign < 0 {
          result = result.negate();
      }
      let cur = result.range();
      result.set_range(cur + real_exponent);
      result
    }
  }
}


#[inline(always)]
pub fn overflow_handler(me:u64) -> (u64, u64) {
    let hi = 64 - me.leading_zeros() - 48;
    let r = (2u64.pow(hi) as f64).log10().ceil() as u32;
    let result = me / 10u64.pow(r) as u64;
    (result, r as u64)
}

pub fn decrease_range(mantissa:i64, range_delta:u64) -> (i64, u64) {
    let remaining_space = mantissa.leading_zeros();
    let thing:u64 = (1 as u64) << remaining_space;
    let remaining_10 = (thing as f64).log10().floor() as u64;
    if range_delta <= remaining_10 {
        let new_mantissa = mantissa * 10u64.pow(range_delta as u32) as i64;
        (new_mantissa, range_delta)
    } else {
        let new_mantissa = mantissa * 10u64.pow(remaining_10 as u32) as i64;
        (new_mantissa, remaining_10)
    }
}

pub fn increase_range(mantissa:i64, range_delta:u64) -> (i64, bool) {
    let range = 10u64.pow(range_delta as u32) as i64;
    (mantissa / range, mantissa % range != 0)
}

#[inline(always)]
pub fn shifted_range(range:u64) -> u64 {
    range << 49
}

pub fn make_quantity(mantissa:i64, range:i64, domain:u64) -> Quantity {
    let value = mantissa.to_quantity();
    let cur_range = (value.range() + range) as u64;
    value & !RANGE_MASK | ((cur_range << 49) & RANGE_MASK) | (domain << 56)
}

pub trait QuantityMath {
    fn domain(self) -> u64;
    fn range(self) -> i64;
    fn set_range(&mut self, range:i64);
    fn mantissa(self) -> i64;
    fn is_negative(self) -> bool;
    fn negate(self) -> Quantity;
    fn add(self, Quantity) -> Result<Quantity, ErrorType>;
    fn sub(self, Quantity) -> Result<Quantity, ErrorType>;
    fn multiply(self, Quantity) -> Result<Quantity, ErrorType>;
    fn divide(self, Quantity) -> Result<Quantity, ErrorType>;
    fn less_than(self, Quantity) -> Result<Quantity, ErrorType>;
    fn greater_than(self, Quantity) -> Result<Quantity, ErrorType>;
    fn less_than_equal(self, Quantity) -> Result<Quantity, ErrorType>;
    fn greater_than_equal(self, Quantity) -> Result<Quantity, ErrorType>;
    fn equal(self, Quantity) -> Result<Quantity, ErrorType>;
    fn not_equal(self, Quantity) -> Result<Quantity, ErrorType>;
    fn to_string(self) -> String;
    fn format(self) -> String;
    fn to_float(self) -> f64;
    fn to_u64(self) -> u64;
}

impl QuantityMath for Quantity {

    #[inline(always)]
    fn domain(self) -> u64 {
        self >> 56
    }

    #[inline(always)]
    fn range(self) -> i64 {
        let range = (self >> 49) & SHIFTED_RANGE_DOMAIN_MASK;
        if range & (1 << 6) == 0 {
            range as i64
        } else {
            (range | SHIFTED_FILL) as i64
        }
    }
    
    fn set_range(&mut self, range:i64) {
        let range_fill = ((range << 49) as u64) & RANGE_MASK;
        *self &= !RANGE_MASK;
        *self |= range_fill;
    }

    #[inline(always)]
    fn mantissa(self) -> i64 {
        if self & SIGN_MASK == SIGN_MASK {
            let a = self & MANTISSA_MASK;
            (a as i64) | (META_MASK as i64)
        } else {
            (self & MANTISSA_MASK) as i64
        }
    }

    fn negate(self) -> Quantity {
        let value = ((self.mantissa() * -1) as u64 & MANTISSA_MASK) as u64;
        self & META_MASK | value
    }

    #[inline(always)]
    fn is_negative(self) -> bool {
        (self & SIGN_MASK) == SIGN_MASK
    }

    fn to_string(self) -> String {
        self.format()
    }

    fn format(self) -> String {
        let mantissa_string = format!("{}", self.mantissa());
        let decimal_ix = (mantissa_string.len() as i64 + self.range()) as isize;
        if decimal_ix < 0 {
            let mut as_string = String::from("0.");
            for i in 0..-1*decimal_ix {
                as_string = format!("{}0", as_string);
            }
            as_string = format!("{}{}", as_string, mantissa_string);
            as_string
        } else if mantissa_string.len() < decimal_ix as usize {
            let mut as_string = mantissa_string;
            while as_string.len() < decimal_ix as usize {
                as_string = format!("{}0", as_string);
            }
            as_string
        } else {
            let mut first = &mantissa_string[..decimal_ix as usize];
            let second = &mantissa_string[decimal_ix as usize ..];
            let mut decimal = "";
            if second.len() != 0 {
                decimal = "."
            }
            if first == "" {
                first = "0";
            }
            let as_string = format!("{}{}{}", first, decimal, second);
            as_string
        }
    }

    fn to_float(self) -> f64 {
        (self.mantissa() as f64) * 10f64.powf(self.range() as f64)
    }

    fn to_u64(self) -> u64 {
        self.to_float() as u64
    }

    #[inline(always)]
    fn add(self, other:Quantity) -> Result<Quantity, ErrorType> {
        // TODO Return self for now... throw an error later
        if self.domain() != other.domain() {
            return Err(ErrorType::DomainMismatch(self.domain(), other.domain()));
        }

        let my_range = self.range();
        let other_range = other.range();
        if self.mantissa() == 0 {
            return Ok(other)
        } else if other.mantissa() == 0 {
            return Ok(self)
        }
        if my_range == other_range {
            let add = self.mantissa() + other.mantissa();
            let mut add_quantity = add.to_quantity();
            add_quantity.set_range(add_quantity.range() + my_range);
            Ok(add_quantity)
        } else {
            let my_mant = self.mantissa();
            let other_mant = other.mantissa();
            let (a_range, b_range, a_mant, b_mant) = if my_range > other_range {
                (my_range, other_range, my_mant, other_mant)
            } else {
                (other_range, my_range, other_mant, my_mant)
            };
            // A is so much bigger than b, we just take a
            if a_range - b_range > 15 {
                return Ok(make_quantity(a_mant,a_range,0))
            }
            let range_delta = (a_range - b_range) as u64;
            let sign = if a_mant < 0 {
                -1
            } else {
                1
            };
            let (new_mantissa, actual_delta) = decrease_range(a_mant * sign, range_delta);
            if actual_delta == range_delta {
                let added = sign * new_mantissa + b_mant;
                let mut added_quantity = added.to_quantity();
                added_quantity.set_range(b_range + added_quantity.range());
                Ok(added_quantity)
            } else {
                let (b_neue, _) = increase_range(b_mant, actual_delta);
                let mut added = (new_mantissa + b_neue).to_quantity();
                added.set_range(a_range - actual_delta as i64);
                Ok(added)
            }
        }
    }

    fn sub(self, other:Quantity) -> Result<Quantity, ErrorType> {
        self.add(other.negate())
    }

    fn multiply(self, other:Quantity) -> Result<Quantity, ErrorType> {
        let result = match self.mantissa().checked_mul(other.mantissa()) {
           Some(result) => { result },
           None => { panic!("QuantityMultiply overflow") }
        };
        let mut quantity = result.to_quantity();
        quantity.set_range(quantity.range() + self.range() + other.range());
        Ok(quantity)
    }

    fn divide(self, other:Quantity) -> Result<Quantity, ErrorType> {
        let result = self.mantissa() * 10000 / other.mantissa();
        Ok(make_quantity(result, -4 + self.range(), 0))
    }

    fn less_than(self, other: Quantity) -> Result<Quantity, ErrorType> {
        if self.is_negative() && !other.is_negative() {
            Ok((1 as u64)<<62)
        } else if !self.is_negative() && other.is_negative() {
            Ok((1 as u64)<<63)
        } else {
            match self.to_float() < other.to_float() {
                true => Ok((1 as u64)<<62),
                false => Ok((1 as u64)<<63),
            }
        }
    }

    fn less_than_equal(self, other: Quantity) -> Result<Quantity, ErrorType> {
        if self.is_negative() && !other.is_negative() {
            Ok((1 as u64)<<63) // false
        } else if !self.is_negative() && other.is_negative() {
            Ok((1 as u64)<<62) // true
        } else {
            match self.to_float() <= other.to_float() {
                true => Ok((1 as u64)<<62),
                false => Ok((1 as u64)<<63),
            }
        }
    }

    fn greater_than_equal(self, other: Quantity) -> Result<Quantity, ErrorType> {
        if self.is_negative() && !other.is_negative() {
            Ok((1 as u64)<<63)
        } else if !self.is_negative() && other.is_negative() {
            Ok((1 as u64)<<62)
        } else {
            match self.to_float() >= other.to_float() {
                true => Ok((1 as u64)<<62),
                false => Ok((1 as u64)<<63),
            }
        }
    }

    fn greater_than(self, other: Quantity) -> Result<Quantity, ErrorType> {
        if self.is_negative() && !other.is_negative() {
            Ok((1 as u64)<<63)
        } else if !self.is_negative() && other.is_negative() {
            Ok((1 as u64)<<62)
        } else {
            match self.to_float() > other.to_float() {
                true => Ok((1 as u64)<<62),
                false => Ok((1 as u64)<<63),
            }
        }
    }

    fn equal(self, other: Quantity) -> Result<Quantity, ErrorType> {
        match self.to_float() == other.to_float() {
            true => Ok((1 as u64)<<62),
            false => Ok((1 as u64)<<63),
        }
    }

    fn not_equal(self, other: Quantity) -> Result<Quantity, ErrorType> {
        match self.to_float() != other.to_float()  {
            true => Ok((1 as u64)<<62),
            false => Ok((1 as u64)<<63),
        }
    }
}

fn integer_decode_f64(f: f64) -> (u64, i16, i8) {
    //println!("BITS {:b}", f as u64);
    let bits: u64 = unsafe { mem::transmute(f) };
    //println!("BITS {:b}", bits);
    let sign: i8 = if bits >> 63 == 0 {
        1
    } else {
        -1
    };
    let mut exponent: i16 = ((bits >> 52) & 0x7ff) as i16;
    let mantissa = if exponent == 0 {
        (bits & 0xfffffffffffff) << 1
    } else {
        (bits & 0xfffffffffffff) | 0x10000000000000
    };
    // Exponent bias + mantissa shift
    exponent -= 1023 + 52;
    (mantissa, exponent, sign)
}
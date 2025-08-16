use crate::*;

// Literals
// ----------------------------------------------------------------------------

pub fn literal(ltrl: &Literal, p: &Interpreter) -> MResult<Value> {
  match &ltrl {
    Literal::Empty(_) => Ok(empty()),
    #[cfg(feature = "bool")]
    Literal::Boolean(bln) => Ok(boolean(bln)),
    Literal::Number(num) => Ok(number(num)),
    #[cfg(feature = "string")]
    Literal::String(strng) => Ok(string(strng)),
    #[cfg(feature = "atom")]
    Literal::Atom(atm) => Ok(atom(atm)),
    Literal::Kind(knd) => kind_value(knd, p),
    Literal::TypedLiteral((ltrl,kind)) => typed_literal(ltrl,kind,p),
    _ => Err(MechError{file: file!().to_string(), tokens: ltrl.tokens(), msg: "".to_string(), id: line!(), kind: MechErrorKind::None}),
  }
}

pub fn kind_value(knd: &NodeKind, p: &Interpreter) -> MResult<Value> {
  let kind = kind_annotation(knd, p)?;
  Ok(Value::Kind(kind.to_value_kind(&p.functions())?))
}

pub fn kind_annotation(knd: &NodeKind, p: &Interpreter) -> MResult<Kind> {
  match knd {
    NodeKind::Any => Ok(Kind::Any),
    NodeKind::Atom(id) => Ok(Kind::Atom(id.hash())),
    NodeKind::Empty => Ok(Kind::Empty),
    NodeKind::Record(elements) => {
      let mut knds = vec![];
      for (id, knd) in elements {
        let knda = kind_annotation(knd, p)?;
        knds.push((id.to_string().clone(), knda));
      }
      Ok(Kind::Record(knds))
    }
    NodeKind::Tuple(elements) => {
      let mut knds = vec![];
      for knd in elements {
        let knda = kind_annotation(knd, p)?;
        knds.push(knda);
      }
      Ok(Kind::Tuple(knds))
    }
    NodeKind::Map(keys, vals) => {
      let key_knd = kind_annotation(keys, p)?;
      let val_knd = kind_annotation(vals, p)?;
      Ok(Kind::Map(Box::new(key_knd), Box::new(val_knd)))
    }
    NodeKind::Scalar(id) => {
      let kind_id = id.hash();
      Ok(Kind::Scalar(kind_id))
    }
    NodeKind::Matrix((knd, size)) => {
      let knda = kind_annotation(knd, p)?;
      let mut dims = vec![];
      for dim in size {
        let dim_val = literal(dim, p)?;
        match dim_val {
          Value::Empty => { dims.push(0); }
          _ => {
            match dim_val.as_usize() {
              Some(size_val) => dims.push(size_val.clone()),
              None => { return Err(MechError{file: file!().to_string(), tokens: knd.tokens(), msg: "".to_string(), id: line!(), kind: MechErrorKind::ExpectedNumericForSize});} 
            }
          }
        }
      }
      Ok(Kind::Matrix(Box::new(knda.clone()),dims))
    }
    NodeKind::Option(knd) => {
      let knda = kind_annotation(knd, p)?;
      Ok(Kind::Option(Box::new(knda)))
    }
    NodeKind::Table((elements, size)) => {
      let mut knds = vec![];
      for (id, knd) in elements {
        let knda = kind_annotation(knd, p)?;
        knds.push((id.to_string().clone(), knda));
      }
      let size_val = literal(size, p)?;
      let size_val = match size_val {
        Value::Empty => 0,
        _ => {
          match size_val.as_usize() {
            Some(size_val) => size_val,
            None => { return Err(MechError{file: file!().to_string(), tokens: knd.tokens(), msg: "".to_string(), id: line!(), kind: MechErrorKind::ExpectedNumericForSize});} 
          }
        }
      };
      Ok(Kind::Table(knds, size_val))
    }
    NodeKind::Set(knd, size) => {
      let knda = kind_annotation(knd, p)?;
      let size_val = match size {
        Some(size) => literal(size, p)?,
        None => Value::Empty,
      };
      match size_val.as_usize() {
        Some(size_val) => Ok(Kind::Set(Box::new(knda.clone()), Some(size_val))),
        None => Ok(Kind::Set(Box::new(knda.clone()), None)),
      }
    }
  }
}

pub fn typed_literal(ltrl: &Literal, knd_attn: &KindAnnotation, p: &Interpreter) -> MResult<Value> {
  let value = literal(ltrl,p)?;
  let kind = kind_annotation(&knd_attn.kind, p)?;
  let args = vec![value, kind.to_value(&p.functions())?];
  let convert_fxn = ConvertKind{}.compile(&args)?;
  convert_fxn.solve();
  let converted_result = convert_fxn.out();
  p.add_plan_step(convert_fxn);
  Ok(converted_result)
}

#[cfg(feature = "atom")]
pub fn atom(atm: &Atom) -> Value {
  let id = atm.name.hash();
  Value::Atom(id)
}

pub fn number(num: &Number) -> Value {
  match num {
    Number::Real(num) => real(num),
    #[cfg(feature = "complex")]
    Number::Complex(num) => complex(num),
    _ => panic!("Number type not supported."),
  }
}

#[cfg(feature = "complex")]
fn complex(num: &ComplexNumberNode) -> Value {
  let im: f64 = match real(&num.imaginary.number).as_f64() {
    Some(val) => val.borrow().0,
    None => 0.0,
  };
  match &num.real {
    Some(real_val) => {
      let re: f64 = match real(&real_val).as_f64() {
        Some(val) => val.borrow().0,
        None => 0.0,
      };      
      Value::ComplexNumber(Ref::new(ComplexNumber::new(re, im)))
    },
    None => Value::ComplexNumber(Ref::new(ComplexNumber::new(0.0, im))),
  }
}

pub fn real(rl: &RealNumber) -> Value {
  match rl {
    #[cfg(feature = "math_neg")]
    RealNumber::Negated(num) => negated(num),
    #[cfg(any(feature = "unsigned_ints", feature = "f64"))]
    RealNumber::Integer(num) => integer(num),
    #[cfg(feature = "floats")]
    RealNumber::Float(num) => float(num),
    #[cfg(feature = "i64")]
    RealNumber::Decimal(num) => dec(num),
    #[cfg(feature = "i64")]
    RealNumber::Hexadecimal(num) => hex(num),
    #[cfg(feature = "i64")]
    RealNumber::Octal(num) => oct(num),
    #[cfg(feature = "i64")]
    RealNumber::Binary(num) => binary(num),
    #[cfg(feature = "floats")]
    RealNumber::Scientific(num) => scientific(num),
    #[cfg(feature = "rational")]
    RealNumber::Rational(num) => rational(num),
    _ => panic!("Number type not supported."),
  }
}

#[cfg(feature = "math_neg")]
pub fn negated(num: &RealNumber) -> Value {
  let num_val = real(&num);
  match num_val {
    #[cfg(feature = "i8")]
    Value::I8(val) => Value::I8(Ref::new(-*val.borrow())),
    #[cfg(feature = "i16")]
    Value::I16(val) => Value::I16(Ref::new(-*val.borrow())),
    #[cfg(feature = "i32")]
    Value::I32(val) => Value::I32(Ref::new(-*val.borrow())),
    #[cfg(feature = "i64")]
    Value::I64(val) => Value::I64(Ref::new(-*val.borrow())),
    #[cfg(feature = "i128")]
    Value::I128(val) => Value::I128(Ref::new(-*val.borrow())),
    #[cfg(feature = "u8")]
    Value::F64(val) => Value::F64(Ref::new(F64::new(-((*val.borrow()).0)))),
    #[cfg(feature = "u16")]
    Value::F32(val) => Value::F32(Ref::new(F32::new(-((*val.borrow()).0)))),
    _ => panic!("Negation is only supported for integer and float types"),
  }
}

#[cfg(feature = "rational")]
pub fn rational(rat: &(Token,Token)) -> Value {
  let (num, denom) = rat;
  let num = num.chars.iter().collect::<String>().parse::<i64>().unwrap();
  let denom = denom.chars.iter().collect::<String>().parse::<i64>().unwrap();
  if denom == 0 {
    panic!("Denominator cannot be zero in a rational number");
  }
  let rat_num = RationalNumber::new(num, denom);
  Value::RationalNumber(Ref::new(rat_num))
}

#[cfg(feature = "i64")]
pub fn dec(bnry: &Token) -> Value {
  let binary_str: String = bnry.chars.iter().collect();
  let num = i64::from_str_radix(&binary_str, 10).unwrap();
  Value::I64(Ref::new(num))
}

#[cfg(feature = "i64")]
pub fn binary(bnry: &Token) -> Value {
  let binary_str: String = bnry.chars.iter().collect();
  let num = i64::from_str_radix(&binary_str, 2).unwrap();
  Value::I64(Ref::new(num))
}

#[cfg(feature = "i64")]
pub fn oct(octl: &Token) -> Value {
  let hex_str: String = octl.chars.iter().collect();
  let num = i64::from_str_radix(&hex_str, 8).unwrap();
  Value::I64(Ref::new(num))
}

#[cfg(feature = "i64")]
pub fn hex(hxdcml: &Token) -> Value {
  let hex_str: String = hxdcml.chars.iter().collect();
  let num = i64::from_str_radix(&hex_str, 16).unwrap();
  Value::I64(Ref::new(num))
}

#[cfg(feature = "f64")]
pub fn scientific(sci: &(Base,Exponent)) -> Value {
  let (base,exp): &(Base,Exponent) = sci;
  let (whole,part): &(Whole,Part) = base;
  let (sign,exp_whole, exp_part): &(Sign, Whole, Part) = exp;

  let a = whole.chars.iter().collect::<String>();
  let b = part.chars.iter().collect::<String>();
  let c = exp_whole.chars.iter().collect::<String>();
  let d = exp_part.chars.iter().collect::<String>();
  let num_f64: f64 = format!("{}.{}",a,b).parse::<f64>().unwrap();
  let mut exp_f64: f64 = format!("{}.{}",c,d).parse::<f64>().unwrap();
  if *sign {
    exp_f64 = -exp_f64;
  }
  let num = num_f64 * 10f64.powf(exp_f64);
  Value::F64(Ref::new(F64(num)))
}

#[cfg(feature = "floats")]
pub fn float(flt: &(Token,Token)) -> Value {
  let a = flt.0.chars.iter().collect::<String>();
  let b = flt.1.chars.iter().collect::<String>();
  let num: f64 = format!("{}.{}",a,b).parse::<f64>().unwrap();
  Value::F64(Ref::new(F64(num)))
}

#[cfg(any(feature = "unsigned_ints", feature = "f64"))]
pub fn integer(int: &Token) -> Value {
  let num: f64 = int.chars.iter().collect::<String>().parse::<f64>().unwrap();
  Value::F64(Ref::new(F64::new(num)))
}

#[cfg(feature = "string")]
pub fn string(tkn: &MechString) -> Value {
  let strng: String = tkn.text.chars.iter().collect::<String>();
  Value::String(Ref::new(strng))
}

pub fn empty() -> Value {
  Value::Empty
}

#[cfg(feature = "bool")]
pub fn boolean(tkn: &Token) -> Value {
  let strng: String = tkn.chars.iter().collect::<String>();
  let val = match strng.as_str() {
    "true" => true,
    "false" => false,
    _ => unreachable!(),
  };
  Value::Bool(Ref::new(val))
}
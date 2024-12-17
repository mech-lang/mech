use crate::*;

// Literals
// ----------------------------------------------------------------------------

pub fn literal(ltrl: &Literal, functions: FunctionsRef) -> MResult<Value> {
  match &ltrl {
    Literal::Empty(_) => Ok(empty()),
    Literal::Boolean(bln) => Ok(boolean(bln)),
    Literal::Number(num) => Ok(number(num)),
    Literal::String(strng) => Ok(string(strng)),
    Literal::Atom(atm) => Ok(atom(atm)),
    Literal::TypedLiteral((ltrl,kind)) => typed_literal(ltrl,kind,functions),
  }
}

pub fn kind_annotation(knd: &NodeKind, functions: FunctionsRef) -> MResult<Kind> {
  match knd {
    NodeKind::Scalar(id) => {
      let kind_id = id.hash();
      Ok(Kind::Scalar(kind_id))
    }
    NodeKind::Bracket((el_knds, size)) => {
      let mut knds = vec![];
      for knd in el_knds {
        let knd = kind_annotation(knd, functions.clone())?;
        knds.push(knd);
      }
      let mut dims = vec![];
      for dim in size {
        let dim_val = literal(dim, functions.clone())?;
        match dim_val.as_usize() {
          Some(size_val) => dims.push(size_val.clone()),
          None => { return Err(MechError{file: file!().to_string(), tokens: knd.tokens(), msg: "".to_string(), id: line!(), kind: MechErrorKind::ExpectedNumericForSize});} 
        }
      }
      if knds.len() != 1 {
        return Err(MechError{file: file!().to_string(), tokens: knd.tokens(), msg: "".to_string(), id: line!(), kind: MechErrorKind::MatrixMustHaveHomogenousKind});
      }
      Ok(Kind::Matrix(Box::new(knds[0].clone()),dims))
    }
    _ => todo!(),
  }
}

pub fn typed_literal(ltrl: &Literal, knd_attn: &KindAnnotation, functions: FunctionsRef) -> MResult<Value> {
  let value = literal(ltrl,functions.clone())?;
  let knd_tkns = knd_attn.tokens();
  let kind = kind_annotation(&knd_attn.kind, functions.clone())?;
  match (&value,kind) {
    (Value::I64(num), Kind::Scalar(to_kind_id)) => {
      match functions.borrow().kinds.get(&to_kind_id) {
        Some(ValueKind::I8)   => Ok(Value::I8(new_ref(*num.borrow() as i8))),
        Some(ValueKind::I16)  => Ok(Value::I16(new_ref(*num.borrow() as i16))),
        Some(ValueKind::I32)  => Ok(Value::I32(new_ref(*num.borrow() as i32))),
        Some(ValueKind::I64)  => Ok(value),
        Some(ValueKind::I128) => Ok(Value::I128(new_ref(*num.borrow() as i128))),
        Some(ValueKind::U8)   => Ok(Value::U8(new_ref(*num.borrow() as u8))),
        Some(ValueKind::U16)  => Ok(Value::U16(new_ref(*num.borrow() as u16))),
        Some(ValueKind::U32)  => Ok(Value::U32(new_ref(*num.borrow() as u32))),
        Some(ValueKind::U64)  => Ok(Value::U64(new_ref(*num.borrow() as u64))),
        Some(ValueKind::U128) => Ok(Value::U128(new_ref(*num.borrow() as u128))),
        Some(ValueKind::F32)  => Ok(Value::F32(new_ref(F32::new(*num.borrow() as f32)))),
        Some(ValueKind::F64)  => Ok(Value::F64(new_ref(F64::new(*num.borrow() as f64)))),
        None => Err(MechError{file: file!().to_string(), tokens: knd_tkns, msg: "".to_string(), id: line!(), kind: MechErrorKind::UndefinedKind(to_kind_id)}),
        _ => Err(MechError{file: file!().to_string(), tokens: knd_tkns, msg: "".to_string(), id: line!(), kind: MechErrorKind::CouldNotAssignKindToValue}),
      }
    }
    (Value::F64(num), Kind::Scalar(to_kind_id)) => {
      match functions.borrow().kinds.get(&to_kind_id) {
        Some(ValueKind::I8)   => Ok(Value::I8(new_ref((*num.borrow()).0 as i8))),
        Some(ValueKind::I16)  => Ok(Value::I16(new_ref((*num.borrow()).0 as i16))),
        Some(ValueKind::I32)  => Ok(Value::I32(new_ref((*num.borrow()).0 as i32))),
        Some(ValueKind::I64)  => Ok(Value::I64(new_ref((*num.borrow()).0 as i64))),
        Some(ValueKind::I128) => Ok(Value::I128(new_ref((*num.borrow()).0 as i128))),
        Some(ValueKind::U8)   => Ok(Value::U8(new_ref((*num.borrow()).0 as u8))),
        Some(ValueKind::U16)  => Ok(Value::U16(new_ref((*num.borrow()).0 as u16))),
        Some(ValueKind::U32)  => Ok(Value::U32(new_ref((*num.borrow()).0 as u32))),
        Some(ValueKind::U64)  => Ok(Value::U64(new_ref((*num.borrow()).0 as u64))),
        Some(ValueKind::U128) => Ok(Value::U128(new_ref((*num.borrow()).0 as u128))),
        Some(ValueKind::F32)  => Ok(Value::F32(new_ref(F32::new((*num.borrow()).0 as f32)))),
        Some(ValueKind::F64)  => Ok(value),
        None => Err(MechError{file: file!().to_string(), tokens: knd_tkns, msg: "".to_string(), id: line!(), kind: MechErrorKind::UndefinedKind(to_kind_id)}),
        _ => Err(MechError{file: file!().to_string(), tokens: knd_tkns, msg: "".to_string(), id: line!(), kind: MechErrorKind::CouldNotAssignKindToValue}),
      }
    }
    _ => todo!(),
  }
}

pub fn atom(atm: &Atom) -> Value {
  let id = atm.name.hash();
  Value::Atom(id)
}

pub fn number(num: &Number) -> Value {
  match num {
    Number::Real(num) => real(num),
    Number::Imaginary(num) => todo!(),
  }
}

pub fn real(rl: &RealNumber) -> Value {
  match rl {
    RealNumber::Negated(num) => todo!(),
    RealNumber::Integer(num) => integer(num),
    RealNumber::Float(num) => float(num),
    RealNumber::Decimal(num) => dec(num),
    RealNumber::Hexadecimal(num) => hex(num),
    RealNumber::Octal(num) => oct(num),
    RealNumber::Binary(num) => binary(num),
    RealNumber::Scientific(num) => scientific(num),
    RealNumber::Rational(num) => todo!(),
  }
}

pub fn dec(bnry: &Token) -> Value {
  let binary_str: String = bnry.chars.iter().collect();
  let num = i64::from_str_radix(&binary_str, 10).unwrap();
  Value::I64(new_ref(num))
}

pub fn binary(bnry: &Token) -> Value {
  let binary_str: String = bnry.chars.iter().collect();
  let num = i64::from_str_radix(&binary_str, 2).unwrap();
  Value::I64(new_ref(num))
}

pub fn oct(octl: &Token) -> Value {
  let hex_str: String = octl.chars.iter().collect();
  let num = i64::from_str_radix(&hex_str, 8).unwrap();
  Value::I64(new_ref(num))
}

pub fn hex(hxdcml: &Token) -> Value {
  let hex_str: String = hxdcml.chars.iter().collect();
  let num = i64::from_str_radix(&hex_str, 16).unwrap();
  Value::I64(new_ref(num))
}

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
  Value::F64(new_ref(F64(num)))
}

pub fn float(flt: &(Token,Token)) -> Value {
  let a = flt.0.chars.iter().collect::<String>();
  let b = flt.1.chars.iter().collect::<String>();
  let num: f64 = format!("{}.{}",a,b).parse::<f64>().unwrap();
  Value::F64(new_ref(F64(num)))
}

pub fn integer(int: &Token) -> Value {
  let num: f64 = int.chars.iter().collect::<String>().parse::<f64>().unwrap();
  Value::F64(new_ref(F64::new(num)))
}

pub fn string(tkn: &MechString) -> Value {
  let strng: String = tkn.text.chars.iter().collect::<String>();
  Value::String(strng)
}

pub fn empty() -> Value {
  Value::Empty
}

pub fn boolean(tkn: &Token) -> Value {
  let strng: String = tkn.chars.iter().collect::<String>();
  let val = match strng.as_str() {
    "true" => true,
    "false" => false,
    _ => unreachable!(),
  };
  Value::Bool(new_ref(val))
}
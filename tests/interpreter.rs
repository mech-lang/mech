#![allow(warnings)]
extern crate mech_syntax;
extern crate mech_core;
extern crate nalgebra as na;
use std::cell::RefCell;
use std::rc::Rc;
use mech_core::matrix::Matrix;
use mech_syntax::*;
use mech_core::*;
use mech_interpreter::*;
use indexmap::set::IndexSet;

/// Compare interpreter output to expected value
macro_rules! test_interpreter {
  ($func:ident, $input:tt, $expected:expr) => (
    #[test]
    fn $func() {
      let s = $input;
      match parser::parse(&s) {
          Ok(tree) => { 
            let mut intrp = Interpreter::new(0);
            let result = intrp.interpret(&tree).unwrap();
            assert_eq!(result, $expected);
          },
          Err(err) => {panic!("{:?}", err);}
      }   
    }
  )
}

/////////////////////////////////////////////////////////////////////////////////

test_interpreter!(interpret_literal_integer, "123", Value::F64(Ref::new(F64::new(123.0))));
test_interpreter!(interpret_literal_sci, "1.23e2", Value::F64(Ref::new(F64::new(123.0))));
#[cfg(feature = "i64")]
test_interpreter!(interpret_literal_bin, "0b10101", Value::I64(Ref::new(0b10101)));
#[cfg(feature = "i64")]
test_interpreter!(interpret_literal_hex, "0x123abc", Value::I64(Ref::new(0x123abc)));
#[cfg(feature = "i64")]
test_interpreter!(interpret_literal_oct, "0o1234", Value::I64(Ref::new(0o1234)));
#[cfg(feature = "i64")]
test_interpreter!(interpret_literal_dec, "0d1234", Value::I64(Ref::new(1234)));
test_interpreter!(interpret_literal_float, "1.23", Value::F64(Ref::new(F64::new(1.23))));
test_interpreter!(interpret_literal_string, r#""Hello""#, Value::String(Ref::new("Hello".to_string())));
test_interpreter!(interpret_literal_string_multiline, r#""Hello 
 World""#, Value::String(Ref::new("Hello \n World".to_string())));
test_interpreter!(interpret_literal_true, "true", Value::Bool(Ref::new(true)));
test_interpreter!(interpret_literal_false, "false", Value::Bool(Ref::new(false)));
test_interpreter!(interpret_literal_atom, "`A", Value::Atom(Ref::new(MechAtom(55450514845822917))));
test_interpreter!(interpret_literal_empty, "_", Value::Empty);
test_interpreter!(interpret_literal_complex, "5+4i", Value::C64(Ref::new(C64::new(5.0, 4.0))));
test_interpreter!(interpret_literal_complex2, "5-4i", Value::C64(Ref::new(C64::new(5.0, -4.0))));
test_interpreter!(interpret_literal_complex3, "5-4j", Value::C64(Ref::new(C64::new(5.0, -4.0))));
test_interpreter!(interpret_literal_rational, "1/2", Value::R64(Ref::new(R64::new(1, 2))));

test_interpreter!(interpret_comment, "123 -- comment", Value::F64(Ref::new(F64::new(123.0))));
test_interpreter!(interpret_comment2, "123 // comment", Value::F64(Ref::new(F64::new(123.0))));

test_interpreter!(interpret_formula_math_add, "2 + 2", Value::F64(Ref::new(F64::new(4.0))));
test_interpreter!(interpret_formula_math_sub, "2 - 2", Value::F64(Ref::new(F64::new(0.0))));
test_interpreter!(interpret_formula_math_mul, "2 * 2", Value::F64(Ref::new(F64::new(4.0))));
test_interpreter!(interpret_formula_math_div, "2 / 2", Value::F64(Ref::new(F64::new(1.0))));
#[cfg(feature = "u8")]
test_interpreter!(interpret_formula_math_exp, "2<u8> ^ 2<u8>", Value::U8(Ref::new(4)));
test_interpreter!(interpret_formula_math_exp_f64, "2.0 ^ 2.0", Value::F64(Ref::new(F64::new(4.0))));
test_interpreter!(interpret_formulat_math_add_rational, "1/10 + 2/10 + 3/10", Value::R64(Ref::new(R64::new(6, 10))));
test_interpreter!(interpret_formulat_math_sub_rational, "1/10 - 2/10 - 3/10", Value::R64(Ref::new(R64::new(-4, 10))));
test_interpreter!(interpret_formula_math_mul_rational, "1/10 * 2/10 * 3/10", Value::R64(Ref::new(R64::new(3, 500))));
test_interpreter!(interpret_formula_math_div_rational, "1/10 / 2/10 / 3/10", Value::R64(Ref::new(R64::new(5, 3))));
test_interpreter!(interpret_formula_math_add_complex, "1+2i + 3+4i", Value::C64(Ref::new(C64::new(4.0, 6.0))));
test_interpreter!(interpret_formula_math_sub_complex, "1+2i - 3+4i", Value::C64(Ref::new(C64::new(-2.0, -2.0))));
test_interpreter!(interpret_formula_math_mul_complex, "1+2i * 3+4i", Value::C64(Ref::new(C64::new(-5.0, 10.0))));
test_interpreter!(interpret_formula_math_div_complex, "1+2i / 3+4i", Value::C64(Ref::new(C64::new(0.44, 0.08))));

test_interpreter!(interpret_matrix_rational, "[1/2 3/4]", Value::MatrixR64(Matrix::from_vec(vec![R64::new(1, 2), R64::new(3, 4)], 1, 2)));
test_interpreter!(interpret_matrix_complex, "[1+2i 3+4i]", Value::MatrixC64(Matrix::from_vec(vec![C64::new(1.0, 2.0), C64::new(3.0, 4.0)], 1, 2)));
test_interpreter!(interpret_matrix_add_rational, "[1/2 3/4] + [1/4 1/2]", Value::MatrixR64(Matrix::from_vec(vec![R64::new(3, 4), R64::new(5, 4)], 1, 2)));
test_interpreter!(interpret_matrix_add_complex, "[1+2i 3+4i] + [5+6i 7+8i]", Value::MatrixC64(Matrix::from_vec(vec![C64::new(6.0, 8.0), C64::new(10.0, 12.0)], 1, 2)));
test_interpreter!(interpret_matrix_sub_rational, "[1/2 3/4] - [1/4 1/2]", Value::MatrixR64(Matrix::from_vec(vec![R64::new(1, 4), R64::new(1, 4)], 1, 2)));
test_interpreter!(interpret_matrix_sub_complex, "[1+2i 3+4i] - [5+6i 7+8i]", Value::MatrixC64(Matrix::from_vec(vec![C64::new(-4.0, -4.0), C64::new(-4.0, -4.0)], 1, 2)));
test_interpreter!(interpret_matrix_mul_rational, "[1/2 3/4] * [1/4 1/2]", Value::MatrixR64(Matrix::from_vec(vec![R64::new(1, 8), R64::new(3, 8)], 1, 2)));
test_interpreter!(interpret_matrix_mul_complex, "[1+2i 3+4i] * [5+6i 7+8i]", Value::MatrixC64(Matrix::from_vec(vec![C64::new(-7.0, 16.0), C64::new(-11.0, 52.0)], 1, 2)));
test_interpreter!(interpret_matrix_div_rational, "[1/2 3/4] / [1/4 1/2]", Value::MatrixR64(Matrix::from_vec(vec![R64::new(2, 1), R64::new(3, 2)], 1, 2)));
test_interpreter!(interpret_matrix_div_complex, "[1+2i 3+4i] / [5+6i 7+8i]", Value::MatrixC64(Matrix::from_vec(vec![C64::new(0.2786885245901639, 0.06557377049180328), C64::new(0.4690265486725664, 0.035398230088495575)], 1, 2)));

test_interpreter!(interpret_matrix_eq_rational, "[1/2 3/4] == [1/2 3/4]", Value::MatrixBool(Matrix::from_vec(vec![true, true], 1, 2)));
test_interpreter!(interpret_matrix_eq_complex, "[1+2i 3+4i] == [1+2i 3+4i]", Value::MatrixBool(Matrix::from_vec(vec![true, true], 1, 2)));
test_interpreter!(interpret_matrix_neq_rational, "[1/2 3/4] != [1/2 3/5]", Value::MatrixBool(Matrix::from_vec(vec![false, true], 1, 2)));
test_interpreter!(interpret_matrix_neq_complex, "[1+2i 3+4i] != [1+2i 3+5i]", Value::MatrixBool(Matrix::from_vec(vec![false, true], 1, 2)));
test_interpreter!(interpret_matrix_gt_rational, "[1/2 3/4] > [1/4 1/2]", Value::MatrixBool(Matrix::from_vec(vec![true, true], 1, 2)));
test_interpreter!(interpret_matrix_gt_complex, "[1+2i 3+4i] > [1+1i 3+3i]", Value::MatrixBool(Matrix::from_vec(vec![true, true], 1, 2)));
test_interpreter!(interpret_matrix_gte_rational, "[1/2 3/4] >= [1/2 3/4]", Value::MatrixBool(Matrix::from_vec(vec![true, true], 1, 2)));
test_interpreter!(interpret_matrix_gte_complex, "[1+2i 3+4i] >= [1+2i 3+4i]", Value::MatrixBool(Matrix::from_vec(vec![true, true], 1, 2)));
test_interpreter!(interpret_matrix_lt_rational, "[1/2 3/4] < [3/4 1/2]", Value::MatrixBool(Matrix::from_vec(vec![true, false], 1, 2)));
test_interpreter!(interpret_matrix_lt_complex, "[1+2i 3+4i] < [2+3i 4+5i]", Value::MatrixBool(Matrix::from_vec(vec![true, true], 1, 2)));
test_interpreter!(interpret_matrix_lte_rational, "[1/2 3/4] <= [1/2 3/4]", Value::MatrixBool(Matrix::from_vec(vec![true, true], 1, 2)));
test_interpreter!(interpret_matrix_lte_complex, "[1+2i 3+4i] <= [1+2i 3+4i]", Value::MatrixBool(Matrix::from_vec(vec![true, true], 1, 2)));
#[cfg(feature = "u64")]
test_interpreter!(interpret_kind_annotation, "1<u64>", Value::U64(Ref::new(1)));
#[cfg(feature = "u64")]
test_interpreter!(interpret_kind_annotation_math, "1<u64> + 1<u64>", Value::U64(Ref::new(2)));


// New tests overflow - unsigned
// test_interpreter!(interpret_kind_math_overflow_u64, "18446744073709551615<u64> + 1<u64>", Value::U64(Ref::new(0)));
// test_interpreter!(interpret_kind_math_overflow_u128, "340282366920938463463374607431768211455<u128> + 1<u128>", Value::U128(Ref::new(0)));

// New test overflow - signed
// test_interpreter!(interpret_kind_math_overflow_i128, "170141183460469231731687303715884105727<i128> + 1<i128>", Value::I128(Ref::new(-170141183460469231731687303715884105728)));

// New test overflow - float
// test_interpreter!(interpret_kind_math_overflow_f32,"1.0<f32> + 1.0<f32>",Value::F32(Ref::new(F32::new(3.402823e+38))));
// test_interpreter!(interpret_kind_math_overflow_f64,"1.0<f64> + 1.0<f64>",Value::F64(Ref::new(F64::new(1.7976931348623157e+308))));

// New tests underflow - unsigned
//test_interpreter!(interpret_kind_math_underflow_u64, "0<u64> - 1<u64>", Value::U64(Ref::new(18446744073709551615)));

// New tests nominal with type def - unsigned
//u8
#[cfg(feature = "u8")]
test_interpreter!(interpret_formula_math_add_u8, "2<u8> + 2<u8>", Value::U8(Ref::new(4)));
#[cfg(feature = "u8")]
test_interpreter!(interpret_formula_math_sub_u8, "2<u8> - 2<u8>", Value::U8(Ref::new(0)));
#[cfg(feature = "u8")]
test_interpreter!(interpret_formula_math_div_u8, "2<u8> / 2<u8>", Value::U8(Ref::new(1)));
#[cfg(feature = "u8")]
test_interpreter!(interpret_formula_math_mul_u8, "2<u8> * 2<u8>", Value::U8(Ref::new(4)));
// u16
#[cfg(feature = "u16")]
test_interpreter!(interpret_formula_math_add_u16, "2<u16> + 2<u16>", Value::U16(Ref::new(4)));
#[cfg(feature = "u16")]
test_interpreter!(interpret_formula_math_sub_u16, "2<u16> - 2<u16>", Value::U16(Ref::new(0)));
#[cfg(feature = "u16")]
test_interpreter!(interpret_formula_math_div_u16, "2<u16> / 2<u16>", Value::U16(Ref::new(1)));
#[cfg(feature = "u16")]
test_interpreter!(interpret_formula_math_mul_u16, "2<u16> * 2<u16>", Value::U16(Ref::new(4)));
// u32
#[cfg(feature = "u32")]
test_interpreter!(interpret_formula_math_add_u32, "2<u32> + 2<u32>", Value::U32(Ref::new(4)));
#[cfg(feature = "u32")]
test_interpreter!(interpret_formula_math_sub_u32, "2<u32> - 2<u32>", Value::U32(Ref::new(0)));
#[cfg(feature = "u32")]
test_interpreter!(interpret_formula_math_div_u32, "2<u32> / 2<u32>", Value::U32(Ref::new(1)));
#[cfg(feature = "u32")]
test_interpreter!(interpret_formula_math_mul_u32, "2<u32> * 2<u32>", Value::U32(Ref::new(4)));
// u64
#[cfg(feature = "u64")]
test_interpreter!(interpret_formula_math_add_u64, "2<u64> + 2<u64>", Value::U64(Ref::new(4)));
#[cfg(feature = "u64")]
test_interpreter!(interpret_formula_math_sub_u64, "2<u64> - 2<u64>", Value::U64(Ref::new(0)));
#[cfg(feature = "u64")]
test_interpreter!(interpret_formula_math_div_u64, "2<u64> / 2<u64>", Value::U64(Ref::new(1)));
#[cfg(feature = "u64")]
test_interpreter!(interpret_formula_math_mul_u64, "2<u64> * 2<u64>", Value::U64(Ref::new(4)));
// u128
#[cfg(feature = "u128")]
test_interpreter!(interpret_formula_math_add_u128, "2<u128> + 2<u128>", Value::U128(Ref::new(4)));
#[cfg(feature = "u128")]
test_interpreter!(interpret_formula_math_sub_u128, "2<u128> - 2<u128>", Value::U128(Ref::new(0)));
#[cfg(feature = "u128")]
test_interpreter!(interpret_formula_math_div_u128, "2<u128> / 2<u128>", Value::U128(Ref::new(1)));
#[cfg(feature = "u128")]
test_interpreter!(interpret_formula_math_mul_u128, "2<u128> * 2<u128>", Value::U128(Ref::new(4)));

// New tests nominal with type def - signed
//i8
#[cfg(feature = "i8")]
test_interpreter!(interpret_formula_math_add_i8, "2<i8> + 2<i8>", Value::I8(Ref::new(4)));
#[cfg(feature = "i8")]
test_interpreter!(interpret_formula_math_sub_i8, "2<i8> - 2<i8>", Value::I8(Ref::new(0)));
#[cfg(feature = "i8")]
test_interpreter!(interpret_formula_math_div_i8, "2<i8> / 2<i8>", Value::I8(Ref::new(1)));
#[cfg(feature = "i8")]
test_interpreter!(interpret_formula_math_mul_i8, "2<i8> * 2<i8>", Value::I8(Ref::new(4)));
// i16
#[cfg(feature = "i16")]
test_interpreter!(interpret_formula_math_add_i16, "2<i16> + 2<i16>", Value::I16(Ref::new(4)));
#[cfg(feature = "i16")]
test_interpreter!(interpret_formula_math_sub_i16, "2<i16> - 2<i16>", Value::I16(Ref::new(0)));
#[cfg(feature = "i16")]
test_interpreter!(interpret_formula_math_div_i16, "2<i16> / 2<i16>", Value::I16(Ref::new(1)));
#[cfg(feature = "i16")]
test_interpreter!(interpret_formula_math_mul_i16, "2<i16> * 2<i16>", Value::I16(Ref::new(4)));
// i32
#[cfg(feature = "i32")]
test_interpreter!(interpret_formula_math_add_i32, "2<i32> + 2<i32>", Value::I32(Ref::new(4)));
#[cfg(feature = "i32")]
test_interpreter!(interpret_formula_math_sub_i32, "2<i32> - 2<i32>", Value::I32(Ref::new(0)));
#[cfg(feature = "i32")]
test_interpreter!(interpret_formula_math_div_i32, "2<i32> / 2<i32>", Value::I32(Ref::new(1)));
#[cfg(feature = "i32")]
test_interpreter!(interpret_formula_math_mul_i32, "2<i32> * 2<i32>", Value::I32(Ref::new(4)));
// i64
#[cfg(feature = "i64")]
test_interpreter!(interpret_formula_math_add_i64, "2<i64> + 2<i64>", Value::I64(Ref::new(4)));
#[cfg(feature = "i64")]
test_interpreter!(interpret_formula_math_sub_i64, "2<i64> - 2<i64>", Value::I64(Ref::new(0)));
#[cfg(feature = "i64")]
test_interpreter!(interpret_formula_math_div_i64, "2<i64> / 2<i64>", Value::I64(Ref::new(1)));
#[cfg(feature = "i64")]
test_interpreter!(interpret_formula_math_mul_i64, "2<i64> * 2<i64>", Value::I64(Ref::new(4)));
// i128
#[cfg(feature = "i128")]
test_interpreter!(interpret_formula_math_add_i128, "2<i128> + 2<i128>", Value::I128(Ref::new(4)));
#[cfg(feature = "i128")]
test_interpreter!(interpret_formula_math_sub_i128, "2<i128> - 2<i128>", Value::I128(Ref::new(0)));
#[cfg(feature = "i128")]
test_interpreter!(interpret_formula_math_div_i128, "2<i128> / 2<i128>", Value::I128(Ref::new(1)));
#[cfg(feature = "i128")]
test_interpreter!(interpret_formula_math_mul_i128, "2<i128> * 2<i128>", Value::I128(Ref::new(4)));

// New tests for nominal with type def - floats
// f32
#[cfg(feature = "f32")]
test_interpreter!(interpret_formula_math_add_f32, "2.0<f32> + 2.0<f32>", Value::F32(Ref::new(F32::new(4.0))));
#[cfg(feature = "f32")]
test_interpreter!(interpret_formula_math_sub_f32, "2.0<f32> - 2.0<f32>", Value::F32(Ref::new(F32::new(0.0))));
#[cfg(feature = "f32")]
test_interpreter!(interpret_formula_math_div_f32, "2.0<f32> / 2.0<f32>", Value::F32(Ref::new(F32::new(1.0))));
#[cfg(feature = "f32")]
test_interpreter!(interpret_formula_math_mul_f32, "2.0<f32> * 2.0<f32>", Value::F32(Ref::new(F32::new(4.0))));
//f64
test_interpreter!(interpret_formula_math_add_f64, "2.0<f64> + 2.0<f64>", Value::F64(Ref::new(F64::new(4.0))));
test_interpreter!(interpret_formula_math_sub_f64, "2.0<f64> - 2.0<f64>", Value::F64(Ref::new(F64::new(0.0))));
test_interpreter!(interpret_formula_math_div_f64, "2.0<f64> / 2.0<f64>", Value::F64(Ref::new(F64::new(1.0))));
test_interpreter!(interpret_formula_math_mul_f64, "2.0<f64> * 2.0<f64>", Value::F64(Ref::new(F64::new(4.0))));

#[cfg(feature = "u16")]
test_interpreter!(interpret_kind_math_no_overflow, "255<u16> + 1<u16>", Value::U16(Ref::new(256)));
#[cfg(feature = "u8")]
test_interpreter!(interpret_kind_matrix_row3, "[1<u8> 2<u8> 3<u8>]", Value::MatrixU8(Matrix::from_vec(vec![1, 2, 3], 1, 3)));
#[cfg(feature = "u64")]
test_interpreter!(interpret_kind_lhs_define, "x<u64> := 1", Value::U64(Ref::new(1)));
#[cfg(all(feature = "u64", feature = "i8"))]
test_interpreter!(interpret_kind_convert_twice, "x<u64> := 1; y<i8> := x", Value::I8(Ref::new(1)));
#[cfg(feature = "f32")]
test_interpreter!(interpret_kind_convert_float, "x<f32> := 123;", Value::F32(Ref::new(F32::new(123.0))));
test_interpreter!(interpret_kind_convert_rational, "x<r64> := 1 / 2; y<f64> := x", Value::F64(Ref::new(F64::new(0.5))));
test_interpreter!(interpret_kind_convert_rational2, "x<f64> := 1/2; y<r64> := x", Value::R64(Ref::new(R64::new(1, 2))));

test_interpreter!(interpret_kind_define, "<foo> := <f64>; x<foo> := 123", Value::F64(Ref::new(F64::new(123.0))));
test_interpreter!(interpret_formula_math_neg, "-1", Value::F64(Ref::new(F64::new(-1.0))));
test_interpreter!(interpret_formula_math_multiple_terms, "1 + 2 + 3", Value::F64(Ref::new(F64::new(6.0))));
test_interpreter!(interpret_formula_comparison_bool, "true == false", Value::Bool(Ref::new(false)));
test_interpreter!(interpret_formula_comparison_bool2, "true == true", Value::Bool(Ref::new(true)));
test_interpreter!(interpret_formula_comparison_eq, "10 == 11", Value::Bool(Ref::new(false)));
test_interpreter!(interpret_formula_comparison_string_eq, r#"["a" "b"] == ["a" "b"]"#, Value::MatrixBool(Matrix::from_vec(vec![true, true], 1, 2)));
test_interpreter!(interpret_formula_comparison_string_neq, r#"["a" "b"] != ["a" "c"]"#, Value::MatrixBool(Matrix::from_vec(vec![false, true], 1, 2)));
test_interpreter!(interpret_formula_comparison_neq, "10 != 11", Value::Bool(Ref::new(true)));
test_interpreter!(interpret_formula_comparison_neq_bool, "false != true", Value::Bool(Ref::new(true)));
test_interpreter!(interpret_formula_comparison_gt, "10 > 11", Value::Bool(Ref::new(false)));
test_interpreter!(interpret_formula_comparison_lt, "10 < 11", Value::Bool(Ref::new(true)));
test_interpreter!(interpret_formula_comparison_gte, "10 >= 10", Value::Bool(Ref::new(true)));
test_interpreter!(interpret_formula_comparison_lte, "10 <= 10", Value::Bool(Ref::new(true)));
test_interpreter!(interpret_formula_comparison_gt_vec, "[1 8; 10 5] > [7 2; 4 11]", Value::MatrixBool(Matrix::from_vec(vec![false, true, true, false], 2, 2)));
test_interpreter!(interpret_formula_comparison_lt_vec, "[1 8 10 5] < [7 2 4 11]", Value::MatrixBool(Matrix::from_vec(vec![true, false, false, true], 1, 4)));
test_interpreter!(interpret_formula_unicode, "😃:=1;🤦🏼‍♂️:=2;y̆és:=🤦🏼‍♂️ + 😃", Value::F64(Ref::new(F64::new(3.0))));
test_interpreter!(interpret_formula_logic_and, "true && true", Value::Bool(Ref::new(true)));
test_interpreter!(interpret_formula_logic_and_vec, "[true false] && [false false]", Value::MatrixBool(Matrix::from_vec(vec![false, false], 1, 2)));
test_interpreter!(interpret_formula_logic_and2, "true && false", Value::Bool(Ref::new(false)));
test_interpreter!(interpret_formula_logic_or_vec, "[true false true] || [false false true]", Value::MatrixBool(Matrix::from_vec(vec![true, false, true], 1, 3)));
test_interpreter!(interpret_formula_logic_or, "true || false", Value::Bool(Ref::new(true)));
test_interpreter!(interpret_formula_logic_or2, "false || false", Value::Bool(Ref::new(false)));
test_interpreter!(interpret_formula_logic_xor_vec, "[true false false true] ⊕ [true true false true]", Value::MatrixBool(Matrix::from_vec(vec![false, true, false, false], 1, 4)));
test_interpreter!(interpret_formula_logic_not, "!false", Value::Bool(Ref::new(true)));
test_interpreter!(interpret_formula_logic_not_vec, "![false true false]", Value::MatrixBool(Matrix::from_vec(vec![true, false, true], 1, 3)));
test_interpreter!(interpret_formula_logic_not_vec1, "![false]", Value::MatrixBool(Matrix::from_vec(vec![true], 1, 1)));

test_interpreter!(interpret_statement_variable_define, "x := 123", Value::F64(Ref::new(F64::new(123.0))));

test_interpreter!(interpret_reference_bool, "x := false; y := true; x && y", Value::Bool(Ref::new(false)));
test_interpreter!(interpret_reference_bool2, "x := false; x && true", Value::Bool(Ref::new(false)));

test_interpreter!(interpret_variable_recall, "a := 1; b := 2; a", Value::MutableReference(Ref::new(Value::F64(Ref::new(F64::new(1.0))))));

test_interpreter!(interpret_matrix_range_exclusive, "1..4", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(2.0), F64::new(3.0)], 1, 3)));
#[cfg(feature = "u8")]
test_interpreter!(interpret_matrix_range_exclusive_u8, "1<u8>..4<u8>", Value::MatrixU8(Matrix::from_vec(vec![1, 2, 3], 1, 3)));
test_interpreter!(interpret_matrix_range_inclusive, "1..=4", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(2.0), F64::new(3.0), F64::new(4.0)], 1, 4)));
#[cfg(feature = "u8")]
test_interpreter!(interpret_matrix_range_inclusive_u8, "1<u8>..=4<u8>", Value::MatrixU8(Matrix::from_vec(vec![1, 2, 3, 4], 1, 4)));
test_interpreter!(interpret_matrix_empty, "[]", Value::MatrixValue(Matrix::from_vec(vec![], 0, 0)));
test_interpreter!(interpret_matrix_row3, "[1 2 3]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(2.0), F64::new(3.0)], 1, 3)));
test_interpreter!(interpret_matrix_mat1, "[123]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(123.0)], 1, 1)));
test_interpreter!(interpret_matrix_row3_float, "[1.2 2.3 3.4]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.2), F64::new(2.3), F64::new(3.4)], 1, 3)));
test_interpreter!(interpret_matrix_mat2, "[1 2; 3 4]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(3.0), F64::new(2.0), F64::new(4.0)], 2, 2)));
test_interpreter!(interpret_matrix_transpose, "[1 2; 3 4]'", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(2.0), F64::new(3.0), F64::new(4.0)], 2, 2)));
#[cfg(feature = "u8")]
test_interpreter!(interpret_matrix_transpose_u8, "[1<u8> 2<u8> 3<u8>]'", Value::MatrixU8(Matrix::from_vec(vec![1u8, 2, 3], 3, 1)));
test_interpreter!(interpret_matrix_transpose_float, "[1.0 2.0 3.0; 4.0 5.0 6.0]'", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(2.0), F64::new(3.0), F64::new(4.0), F64::new(5.0), F64::new(6.0)], 3, 2)));
#[cfg(feature = "i64")]
test_interpreter!(interpret_matrix_transpose_vector, "x := | x<i64> | 1 | 3 | 5 |; x.x'", Value::MatrixI64(Matrix::from_vec(vec![1i64, 3, 5], 1, 3)));
test_interpreter!(interpret_matrix_add_v2s, "[1;2] + 3", Value::MatrixF64(Matrix::from_vec(vec![F64::new(4.0), F64::new(5.0)], 2, 1)));

test_interpreter!(interpret_matrix_mat2_f64, "[1.1 2.2; 3.3 4.4]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.1), F64::new(3.3), F64::new(2.2), F64::new(4.4)], 2, 2)));
test_interpreter!(interpret_matrix_negate, "-[1 2; 3 4]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(-1.0), F64::new(-3.0), F64::new(-2.0), F64::new(-4.0)], 2, 2)));
test_interpreter!(interpret_matrix_negate_float, "-[1.0 2.0; 3.0 4.0]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(-1.0), F64::new(-3.0), F64::new(-2.0), F64::new(-4.0)], 2, 2)));
test_interpreter!(interpret_matrix_negate_mat1, "-[1]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(-1.0)], 1, 1)));

test_interpreter!(interpret_matrix_row3_add, "[1 2 3] + [4 5 6]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(5.0), F64::new(7.0), F64::new(9.0)], 1, 3)));
test_interpreter!(interpret_matrix_row3_mul_scalar, "[1 2 3] * 3", Value::MatrixF64(Matrix::from_vec(vec![F64::new(3.0), F64::new(6.0), F64::new(9.0)], 1, 3)));
test_interpreter!(interpret_matrix_row3_mul_scalar2, "3 * [1 2 3]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(3.0), F64::new(6.0), F64::new(9.0)], 1, 3)));
test_interpreter!(interpret_matrix_row3_add_float, "[1.0 2.0 3.0] + [4.0 5.0 6.0]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(5.0), F64::new(7.0), F64::new(9.0)], 1, 3)));
test_interpreter!(interpret_matrix_row3_sub, "[1 2 3] - [4 5 6]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(-3.0), F64::new(-3.0), F64::new(-3.0)], 1, 3)));
test_interpreter!(interpret_matrix_row3_sub_float, "[1.0 2.0 3.0] - [4.0 5.0 6.0]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(-3.0), F64::new(-3.0), F64::new(-3.0)], 1, 3)));
test_interpreter!(interpret_matrix_row3_add_ref, "a := [1 2 3]; b := [4 5 6]; c := a + b", Value::MatrixF64(Matrix::from_vec(vec![F64::new(5.0), F64::new(7.0), F64::new(9.0)], 1, 3)));
test_interpreter!(interpret_matrix_dynamic_add, "[1 2 3 4; 5 6 7 8] + [1 2 3 4; 5 6 7 8]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(2.0), F64::new(10.0), F64::new(4.0), F64::new(12.0), F64::new(6.0), F64::new(14.0), F64::new(8.0), F64::new(16.0)], 2, 4)));
test_interpreter!(interpret_matrix_dynamic_div, "[2 4 6 8] / [2 2 2 2]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(2.0), F64::new(3.0), F64::new(4.0)], 1, 4)));
test_interpreter!(interpret_matrix_gt, "x := [66.0 2.0 3.0; 66.0 5.0 66.0]; y := [1.0 2.0 3.0; 4.0 5.0 6.0]; x > y", Value::MatrixBool(Matrix::from_vec(vec![true, true, false, false, false, true], 2, 3)));
test_interpreter!(interpret_matrix_lt, "x := [66.0 2.0 3.0; 66.0 4.0 66.0]; y := [1.0 2.0 3.0; 4.0 5.0 6.0]; x < y", Value::MatrixBool(Matrix::from_vec(vec![false, false, false, true, false, false], 2, 3)));
test_interpreter!(interpret_matrix_lt_int, "x := [66 2 3; 66 4 66]; y := [1 2 3; 4 5 6]; x < y", Value::MatrixBool(Matrix::from_vec(vec![false, false, false, true, false, false], 2, 3)));
test_interpreter!(interpret_matrix_add_m2v2, "[1 1; 2 2] + [1;2]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(2.0), F64::new(4.0), F64::new(2.0), F64::new(4.0)], 2, 2)));
test_interpreter!(interpret_matrix_add_v2m2, "[1;2] + [1 1; 2 2]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(2.0), F64::new(4.0), F64::new(2.0), F64::new(4.0)], 2, 2)));
test_interpreter!(interpret_matrix_add_r2m2, "[1 2] + [1 1; 1 1]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(2.0), F64::new(2.0), F64::new(3.0), F64::new(3.0)], 2, 2)));
test_interpreter!(interpret_matrix_add_m2r2, "[1 1; 1 1] + [1 2]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(2.0), F64::new(2.0), F64::new(3.0), F64::new(3.0)], 2, 2)));

test_interpreter!(interpret_matrix_dot, "[1 2 3] · [4 5 6]", Value::F64(Ref::new(F64::new(32.0))));
test_interpreter!(interpret_matrix_matmul_mat1, "[2] ** [10]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(20.0)], 1, 1)));
test_interpreter!(interpret_matrix_matmul_mat2_ref, "a := [1 2; 3 4]; b := [4 5; 6 7]; c := a ** b", Value::MatrixF64(Matrix::from_vec(vec![F64::new(16.0), F64::new(36.0), F64::new(19.0), F64::new(43.0)], 2, 2)));
test_interpreter!(interpret_matrixmatmul_mat2x3_ref, "a := [1.0 2.0 3.0; 4.0 5.0 6.0]; b := [4.0 5.0; 6.0 7.0; 8.0 9.0]; c := a ** b", Value::MatrixF64(Matrix::from_vec(vec![F64::new(40.0), F64::new(94.0), F64::new(46.0), F64::new(109.0)], 2, 2)));
test_interpreter!(interpret_matrixmatmul_r3m3, "a := [1.0 2.0 3.0]; b := [4.0 5.0 6.0; 7.0 8.0 9.0; 10 11 12]; c := a ** b", Value::MatrixF64(Matrix::from_vec(vec![F64::new(48.0), F64::new(54.0), F64::new(60.0)], 1, 3)));
test_interpreter!(interpret_matrixmatmul_m3v3, "b := [4.0 5.0 6.0; 7.0 8.0 9.0; 10 11 12]; a := [1.0 2.0 3.0]'; c := b ** a", Value::MatrixF64(Matrix::from_vec(vec![F64::new(32.0), F64::new(50.0), F64::new(68.0)], 3, 1)));
test_interpreter!(interpret_matrix_string, r#"["Hello" "World"]"#, Value::MatrixString(Matrix::from_vec(vec!["Hello".to_string(), "World".to_string()], 1, 2)));
test_interpreter!(interpret_matrix_string_access, r#"x:=["Hello" "World"];x[2]"#, Value::String(Ref::new("World".to_string())));
test_interpreter!(interpret_matrix_string_assign, r#"~x:=["Hello" "World"];x[1]="Foo";[x[1] x[2]]"#, Value::MatrixString(Matrix::from_vec(vec!["Foo".to_string(), "World".to_string()], 1, 2)));
test_interpreter!(interpret_matrix_string_assign_logical, r#"~x := ["Hello", "World", "!"]; x[[true false true]] = "Foo";"#, Value::MatrixString(Matrix::from_vec(vec!["Foo".to_string(), "World".to_string(), "Foo".to_string()], 1, 3)));
test_interpreter!(interpret_table_string_access, r#"x:=|x<string> y<string> | "a" "b" | "c" "d" |; x.y"#, Value::MatrixString(Matrix::from_vec(vec!["b".to_string(), "d".to_string()], 2, 1)));
test_interpreter!(interpret_matrix_define_ref, r#"x:=123;y<[f64]:1,4>:=x;"#, Value::MatrixF64(Matrix::from_vec(vec![F64::new(123.0); 4], 1, 4)));
#[cfg(all(feature = "f64", feature = "u8"))]
test_interpreter!(interpret_matrix_define_convert, r#"y<[f64]:1,3> := 123<u8>;"#, Value::MatrixF64(Matrix::from_vec(vec![F64::new(123.0); 3], 1, 3)));
#[cfg(all(feature = "u64", feature = "u8"))]
test_interpreter!(interpret_matrix_define_convert_matrix, r#"x := [1 2 3];y<[u64]> := x;z<[u8]> := y;"#, Value::MatrixU8(Matrix::from_vec(vec![1u8, 2, 3], 1, 3)));

// 2x2 Nominal Operations 
test_interpreter!(interpret_matrix_add_2x2, "[1 2; 3 4] + [5 6; 7 8]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(6.0), F64::new(10.0), F64::new(8.0), F64::new(12.0)], 2, 2)));
test_interpreter!(interpret_matrix_sub_2x2, "[1 2; 3 4] - [5 6; 7 8]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(-4.0), F64::new(-4.0), F64::new(-4.0), F64::new(-4.0)], 2, 2)));
test_interpreter!(interpret_matrix_mul_2x2, "[1 2; 3 4] * [5 6; 7 8]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(5.0), F64::new(21.0), F64::new(12.0), F64::new(32.0)], 2, 2)));
test_interpreter!(interpret_matrix_div_2x2, "[20 30; 40 50] / [2 3; 4 5]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(10.0), F64::new(10.0), F64::new(10.0), F64::new(10.0)], 2, 2)));

// 3x3 Nominal Operations
test_interpreter!(interpret_matrix_add_3x3, "[1 2 3; 4 5 6; 7 8 9] + [9 8 7; 6 5 4; 3 2 1]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(10.0); 9], 3, 3)));
test_interpreter!(interpret_matrix_mul_3x3, "[1 2 3; 4 5 6; 7 8 9] * [9 8 7; 6 5 4; 3 2 1]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(9.0), F64::new(24.0), F64::new(21.0), F64::new(16.0), F64::new(25.0), F64::new(16.0), F64::new(21.0), F64::new(24.0), F64::new(9.0)], 3, 3)));
test_interpreter!(interpret_matrix_div_3x3, "[10 20 30; 40 50 60; 70 80 90] / [10 10 10; 10 10 10; 10 10 10]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(4.0), F64::new(7.0), F64::new(2.0), F64::new(5.0), F64::new(8.0), F64::new(3.0), F64::new(6.0), F64::new(9.0)], 3, 3)));


// 4x4 Nominal Operations
test_interpreter!(interpret_matrix_add_4x4, 
          "[1 2 3 4; 5 6 7 8; 9 10 11 12; 13 14 15 16] + [17 18 19 20; 21 22 23 24; 25 26 27 28; 29 30 31 32]", 
          Value::MatrixF64(Matrix::from_vec(vec![F64::new(18.0), F64::new(26.0), F64::new(34.0), F64::new(42.0), 
                              F64::new(20.0), F64::new(28.0), F64::new(36.0), F64::new(44.0), 
                              F64::new(22.0), F64::new(30.0), F64::new(38.0), F64::new(46.0), 
                              F64::new(24.0), F64::new(32.0), F64::new(40.0), F64::new(48.0)], 4, 4)));

test_interpreter!(interpret_matrix_sub_4x4, 
          "[1 2 3 4; 5 6 7 8; 9 10 11 12; 13 14 15 16] - [17 18 19 20; 21 22 23 24; 25 26 27 28; 29 30 31 32]", 
          Value::MatrixF64(Matrix::from_vec(vec![F64::new(-16.0), F64::new(-16.0), F64::new(-16.0), F64::new(-16.0), 
                              F64::new(-16.0), F64::new(-16.0), F64::new(-16.0), F64::new(-16.0), 
                              F64::new(-16.0), F64::new(-16.0), F64::new(-16.0), F64::new(-16.0), 
                              F64::new(-16.0), F64::new(-16.0), F64::new(-16.0), F64::new(-16.0)], 4, 4)));

test_interpreter!(interpret_matrix_mul_4x4, 
          "[1 2 3 4; 5 6 7 8; 9 10 11 12; 13 14 15 16] * [17 18 19 20; 21 22 23 24; 25 26 27 28; 29 30 31 32]", 
          Value::MatrixF64(Matrix::from_vec(vec![F64::new(17.0), F64::new(105.0), F64::new(225.0), F64::new(377.0), 
                              F64::new(36.0), F64::new(132.0), F64::new(260.0), F64::new(420.0), 
                              F64::new(57.0), F64::new(161.0), F64::new(297.0), F64::new(465.0), 
                              F64::new(80.0), F64::new(192.0), F64::new(336.0), F64::new(512.0)], 4, 4)));

test_interpreter!(interpret_matrix_div_4x4, 
          "[2 3 4 5; 6 7 8 9; 10 11 12 13; 14 15 16 17] / [2 2 2 2; 3 3 3 3; 4 4 4 4; 5 5 5 5]", 
          Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(2.0), F64::new(2.5), F64::new(2.8), 
                              F64::new(1.5), F64::new(2.3333333333333335), F64::new(2.75), F64::new(3.0), 
                              F64::new(2.0), F64::new(2.6666666666666665), F64::new(3.0), F64::new(3.2), 
                              F64::new(2.5), F64::new(3.0), F64::new(3.25), F64::new(3.4)], 4, 4)));

// 2x3 Nominal Operations
test_interpreter!(interpret_matrix_sub_2x3, "[1 2 3; 4 5 6] - [7 8 9; 10 11 12]", 
          Value::MatrixF64(Matrix::from_vec(vec![F64::new(-6.0), F64::new(-6.0), F64::new(-6.0), 
                              F64::new(-6.0), F64::new(-6.0), F64::new(-6.0)], 2, 3)));

// 3x2 Nominal Operations
test_interpreter!(interpret_matrix_sub_3x2, "[1 2; 3 4; 5 6] - [7 8; 9 10; 11 12]", 
          Value::MatrixF64(Matrix::from_vec(vec![F64::new(-6.0), F64::new(-6.0), 
                              F64::new(-6.0), F64::new(-6.0), 
                              F64::new(-6.0), F64::new(-6.0)], 3, 2)));

// u8 2x2 Underflow/Overflow
/*test_interpreter!(interpret_matrix_underflow_2x2_u8_sub,
  "[1 2; 3 4] - [5 6; 7 8]",
  Ref::new(Matrix2::from_vec(vec![254u8, 255u8, 253u8, 254u8])).to_value() 
);*/
/*test_interpreter!(interpret_matrix_overflow_2x2_u8_add,
  "[250 251; 252 253] + [10 11; 12 13]",
  Ref::new(Matrix2::from_vec(vec![4u8, 6u8, 8u8, 10u8])).to_value() 
);*/

// u8 3x3 Underflow/Overflow
/*test_interpreter!(interpret_matrix_underflow_3x3_u8_sub,
  "[1 2 3; 4 5 6; 7 8 9] - [10 11 12; 13 14 15; 16 17 18]",
  Ref::new(Matrix3::from_vec(vec![251u8, 252u8, 253u8, 254u8, 255u8, 255u8, 253u8, 254u8, 255u8])).to_value()
);*/
/*test_interpreter!(interpret_matrix_overflow_3x3_u8_add,
  "[250 251 252; 253 254 255; 0 1 2] + [10 11 12; 13 14 15; 16 17 18]",
  Ref::new(Matrix3::from_vec(vec![4u8, 6u8, 8u8, 10u8, 11u8, 12u8, 16u8, 18u8, 20u8])).to_value()
);*/

test_interpreter!(interpret_tuple, "(1,true)", Value::Tuple(Ref::new(MechTuple::from_vec(vec![Value::F64(Ref::new(F64::new(1.0))), Value::Bool(Ref::new(true))]))));
test_interpreter!(interpret_tuple_nested, r#"(1,("Hello",false))"#, Value::Tuple(Ref::new(MechTuple::from_vec(vec![Value::F64(Ref::new(F64::new(1.0))), Value::Tuple(Ref::new(MechTuple::from_vec(vec![Value::String(Ref::new("Hello".to_string())), Value::Bool(Ref::new(false))])))]))));
test_interpreter!(interpret_tuple_access, r#"q := (10, "b", true); r := (q.3, q.2, q.1)"#, Value::Tuple(Ref::new(MechTuple::from_vec(vec![Value::Bool(Ref::new(true)), Value::String(Ref::new("b".to_string())), Value::F64(Ref::new(F64::new(10.0)))]))));
test_interpreter!(interpret_tuple_destructure, r#"a := (10, 11, 12); (x,y,z) := a; x + y + z"#, Value::F64(Ref::new(F64::new(33.0))));

test_interpreter!(interpret_slice, "a := [1,2,3]; a[2]", Value::F64(Ref::new(F64::new(2.0))));
test_interpreter!(interpret_slice_v, "a := [1,2,3]'; a[2]", Value::F64(Ref::new(F64::new(2.0))));
test_interpreter!(interpret_slice_2d, "a := [1,2;3,4]; a[1,2]", Value::F64(Ref::new(F64::new(2.0))));
test_interpreter!(interpret_slice_f64, "a := [1.0,2.0,3.0]; a[2]", Value::F64(Ref::new(F64::new(2.0))));
test_interpreter!(interpret_slice_2d_f64, "a := [1,2;3,4]; a[2,1]", Value::F64(Ref::new(F64::new(3.0))));
test_interpreter!(interpret_slice_range_2d, "x := [1 2 3; 4 5 6; 7 8 9]; x[2..=3, 2..=3]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(5.0), F64::new(8.0), F64::new(6.0), F64::new(9.0)], 2, 2)));
test_interpreter!(interpret_slice_sclar_range, "ix := [true false true]'; x := [1 2 3; 4 5 6; 7 8 9]; x[2,ix]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(4.0), F64::new(6.0)], 1, 2)));
test_interpreter!(interpret_slice_range_scalar, "ix := [true false true]'; x := [1 2 3; 4 5 6; 7 8 9]; x[ix,2]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(2.0), F64::new(8.0)], 2, 1)));
test_interpreter!(interpret_slice_all, "x := [1 2; 4 5]; x[:]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(4.0), F64::new(2.0), F64::new(5.0)], 4, 1)));
test_interpreter!(interpret_slice_all_2d, "x := [1 2; 4 5]; x[:,2]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(2.0), F64::new(5.0)], 2, 1)));
test_interpreter!(interpret_slice_all_2d_row, "x := [1 2; 4 5]; x[2,:]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(4.0), F64::new(5.0)], 1, 2)));
test_interpreter!(interpret_slice_all_2d_row2, "x := [1 2 3 4 5; 6 7 8 9 10]; x[1,:]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(2.0), F64::new(3.0), F64::new(4.0), F64::new(5.0)], 1, 5)));
test_interpreter!(interpret_slice_all_range, "x := [1 2 3 4; 5 6 7 8]; x[:,1..=2]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(5.0), F64::new(2.0), F64::new(6.0)], 2, 2)));
test_interpreter!(interpret_slice_range_all, "x := [1 2 3; 4 5 6; 7 8 9]; x[1..=2,:]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(4.0), F64::new(2.0), F64::new(5.0), F64::new(3.0), F64::new(6.0)], 2, 3)));
test_interpreter!(interpret_slice_range_dupe, "x := [1 2 3; 4 5 6; 7 8 9]; x[[1 1],:]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(1.0), F64::new(2.0), F64::new(2.0), F64::new(3.0), F64::new(3.0)], 2, 3)));
test_interpreter!(interpret_slice_all_reshape, "x := [1 2 3; 4 5 6; 7 8 9]; y := x[:,[1,1]]; y[:]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(4.0), F64::new(7.0), F64::new(1.0), F64::new(4.0), F64::new(7.0)], 6, 1)));
test_interpreter!(interpret_slice_ix_ref, "x := [94 53 13]; y := [3 3]; x[y]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(13.0), F64::new(13.0)], 2, 1)));
test_interpreter!(interpret_slice_ix_ref2, "x := [94 53 13]; y := [3; 3]; x[y]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(13.0), F64::new(13.0)], 2, 1)));
test_interpreter!(interpret_slice_ix_ref3, "x := [94 53 13]; y := 3; x[y]", Value::F64(Ref::new(F64::new(13.0))));
test_interpreter!(interpret_slice_logical_ix, "x := [94 53 13]; ix := [false true true]; x[ix]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(53.0), F64::new(13.0)], 2, 1)));
test_interpreter!(interpret_slice_row, "x := [94 53 13; 4 5 6; 7 8 9]; x[2,1..3]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(4.0), F64::new(5.0)], 1, 2)));
test_interpreter!(interpret_slice_col, "x := [94 53 13; 4 5 6; 7 8 9]; x[1..3,2]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(53.0), F64::new(5.0)], 2, 1)));
test_interpreter!(interpret_slice_dynamic, "x := 1..10; y := x'; ix := 1..5; y[ix]'", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(2.0), F64::new(3.0), F64::new(4.0)], 1, 4)));
test_interpreter!(interpret_slice_all_bool, "ix := [false, false, true]'; x := [1 2 3; 4 5 6; 7 8 9]; x[:,ix]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(3.0), F64::new(6.0), F64::new(9.0)], 3, 1)));
test_interpreter!(interpret_slice_ix_bool, "ix := [false, false, true]; x := [1 2 3; 4 5 6; 7 8 9]; x[[1,2,3,3],ix]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(3.0), F64::new(6.0), F64::new(9.0), F64::new(9.0)], 4, 1)));
test_interpreter!(interpret_slice_bool_bool, "ix := [true, false, true]; x := [1 2 3; 4 5 6;7 8 9]; x[ix,ix]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(7.0), F64::new(3.0), F64::new(9.0)], 2, 2)));
test_interpreter!(interpret_slice_ix_bool_v, "ix1 := [false, false, true]; ix2 := [1,2,3,3]; x := [1 2 3; 4 5 6; 7 8 9]; x[ix1',ix2']", Value::MatrixF64(Matrix::from_vec(vec![F64::new(7.0), F64::new(8.0), F64::new(9.0), F64::new(9.0)], 1, 4)));


test_interpreter!(interpret_swizzle_record, "x := {x: 1, y: 2, z: 3}; x.y,z,z", Value::Tuple(Ref::new(MechTuple::from_vec(vec![Value::F64(Ref::new(F64::new(2.0))),Value::F64(Ref::new(F64::new(3.0))),Value::F64(Ref::new(F64::new(3.0)))]))));
//test_interpreter!(interpret_swizzle_table, "x := | x<i64> y<u8>| 1 2 | 4 5 |; x.x,x,y", Value::Tuple(MechTuple::from_vec(vec![Matrix::Vector2(Ref::new(Vector2::from_vec(vec![Value::I64(Ref::new(1)),Value::I64(Ref::new(4))]))).to_value(),Matrix::Vector2(Ref::new(Vector2::from_vec(vec![Value::I64(Ref::new(1)),Value::I64(Ref::new(4))]))).to_value(),Matrix::Vector2(Ref::new(Vector2::from_vec(vec![Value::U8(Ref::new(2)),Value::U8(Ref::new(5))]))).to_value()])));

test_interpreter!(interpret_dot_record, "x := {x: 1, y: 2, z: 3}; x.x", Value::F64(Ref::new(F64::new(1.0))));

test_interpreter!(interpret_dot_int_matrix, "x := [1,2,3]; x.1", Value::F64(Ref::new(F64::new(1.0))));
#[cfg(all(feature = "i64", feature = "u8"))]
test_interpreter!(interpret_dot_index_table, "x :=  | x<i64> y<u8>| 1 2 | 4 5|; x.x", Value::MatrixI64(Matrix::from_vec(vec![1, 4], 2, 1)));
#[cfg(all(feature = "i64", feature = "u8"))]
test_interpreter!(interpret_dot_index_table2, "x := | x<i64> y<u8>| 1 2 | 4 5|; x.y", Value::MatrixU8(Matrix::from_vec(vec![2, 5], 2, 1)));
#[cfg(feature = "i64")]
test_interpreter!(interpret_dot_index_table3, "x := | x<i64> y<bool>| 1 true | 4 false | 3 true|; x.y", Value::MatrixBool(Matrix::from_vec(vec![true, false, true], 3, 1)));
#[cfg(all(feature = "i64", feature = "u8"))]
test_interpreter!(interpret_dot_index_table4, "x := | x<i64> y<u8>| 1 2| 3 4| 5 6| 7 8 |; x.x", Value::MatrixI64(Matrix::from_vec(vec![1, 3, 5, 7], 4, 1)));
#[cfg(all(feature = "i64", feature = "i8"))]
test_interpreter!(interpret_dot_index_table5, "x := | x<i64> y<i8>| 1 2| 3 4| 5 6| 7 8 |; x.y", Value::MatrixI8(Matrix::from_vec(vec![2, 4, 6, 8], 4, 1)));
#[cfg(all(feature = "u32", feature = "f32", feature = "i8"))]
test_interpreter!(interpret_dot_index_table6, "x := | x<u32> y<f32> z<i8>|1 2 3|4 5 6|; x.y", Value::MatrixF32(Matrix::from_vec(vec![F32::new(2.0), F32::new(5.0)], 2, 1)));

test_interpreter!(interpret_set_empty,"{_}", Value::Set(Ref::new(MechSet::from_vec(vec![]))));
test_interpreter!(interpret_set_empty2,"{}", Value::Set(Ref::new(MechSet::from_vec(vec![]))));
test_interpreter!(interpret_set, "{1,2,3}", Value::Set(Ref::new(MechSet::from_vec(vec![Value::F64(Ref::new(F64::new(1.0))), Value::F64(Ref::new(F64::new(2.0))), Value::F64(Ref::new(F64::new(3.0)))]))));
test_interpreter!(interpret_record,r#"{a: 1, b: "Hello"}"#, Value::Record(Ref::new(MechRecord::from_vec(vec![((55170961230981453,"a".to_string()),Value::F64(Ref::new(F64::new(1.0)))),((44311847522083591,"b".to_string()),Value::String(Ref::new("Hello".to_string())))]))));
test_interpreter!(interpret_record_field_access,r#"a := {x: 1,  y: 2}; a.y"#, Value::F64(Ref::new(F64::new(2.0))));
test_interpreter!(interpret_map, r#"{"a": 1, "b": 2}"#, Value::Map(Ref::new(MechMap::from_vec(vec![(Value::String(Ref::new("a".to_string())),Value::F64(Ref::new(F64::new(1.0)))), (Value::String(Ref::new("b".to_string())),Value::F64(Ref::new(F64::new(2.0))))]))));
test_interpreter!(interpret_set_rational, r#"{1/2, 3/4}"#, Value::Set(Ref::new(MechSet::from_vec(vec![Value::R64(Ref::new(R64::new(1, 2))), Value::R64(Ref::new(R64::new(3, 4)))]))));

/*test_interpreter!(interpret_function_define,r#"foo(x<f64>) = z<f64> :=
z := 10 + x. 
foo(10)"#, Value::F64(Ref::new(F64::new(20.0))));
test_interpreter!(interpret_function_define_2_args,r#"foo(x<f64>, y<f64>) = z<f64> :=
z := x + y.
foo(10,20)"#, Value::F64(Ref::new(F64::new(30.0))));
test_interpreter!(interpret_function_define_statements,r#"foo(x<f64>, y<f64>) = z<f64> :=
    a := 1 + x
    b := y + 1
    z := a + b.
foo(10,20)"#, Value::F64(Ref::new(F64::new(32.0))));*/
test_interpreter!(interpret_function_call_native_vector, "math/sin([1.570796327 1.570796327])", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(1.0)], 1, 2)));
test_interpreter!(interpret_function_call_native, r#"math/sin(1.5707963267948966)"#, Value::F64(Ref::new(F64::new(1.0))));
test_interpreter!(interpret_function_call_native_cos, r#"math/cos(0.0)"#, Value::F64(Ref::new(F64::new(1.0))));
test_interpreter!(interpret_function_call_native_vector2, "math/cos([0.0 0.0])", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(1.0)], 1, 2)));

test_interpreter!(interpret_set_value,"~x := 1.23; x = 4.56;", Value::F64(Ref::new(F64::new(4.56))));
test_interpreter!(interpret_set_value_row_vector,"~x := [6,2]; x[1] = 4;", Value::MatrixF64(Matrix::from_vec(vec![F64::new(4.0), F64::new(2.0)], 1, 2)));
test_interpreter!(interpret_set_value_col_vector,"~x := [false false true true]'; x[1] = true; x[1]", Value::Bool(Ref::new(true)));
test_interpreter!(interpret_set_value_scalar_scalar,"~x := [1 2; 3 4]; x[2,2] = 42; x[2,2];", Value::F64(Ref::new(F64::new(42.0))));
test_interpreter!(interpret_set_value_all,"~x := [1 2; 3 4]; x[:] = 42; x[1] + x[2] + x[3] + x[4]; ", Value::F64(Ref::new(F64::new(168.0))));
test_interpreter!(interpret_set_value_all_scalar,"~x := [1 2; 3 4]; x[:,1] = 42; x[1] + x[2] + x[3] + x[4]", Value::F64(Ref::new(F64::new(90.0))));
test_interpreter!(interpret_set_value_scalar_all,"~x := [1 2; 3 4]; x[1,:] = 42; x[1] + x[3];", Value::F64(Ref::new(F64::new(84.0))));
test_interpreter!(interpret_set_value_slice,"~x := [1 2 3 4]; x[[1 3]] = 42; x[1] + x[2] + x[3] + x[4];", Value::F64(Ref::new(F64::new(90.0))));
test_interpreter!(interpret_set_value_scalar_slice,"~x := [1 2 3; 4 5 6; 7 8 9]; x[1,[1,3]] = 42; x[1] + x[7];", Value::F64(Ref::new(F64::new(84.0))));
test_interpreter!(interpret_set_value_slice_slice,"~x := [1 2 3; 5 6 7; 9 10 11]; x[1..3,1..3] = 42; x[1] + x[2] + x[4] + x[5]", Value::F64(Ref::new(F64::new(168.0))));
test_interpreter!(interpret_set_value_all_slice,"~x := [1 2 3; 5 6 7]; x[:,1..3] = 42; x[1] + x[2] + x[3] + x[4] + x[5] + x[6]", Value::F64(Ref::new(F64::new(178.0))));
test_interpreter!(interpret_set_value_all_slice_vec,"~x := [1;6]; x = [4;5]; x[1] + x[2];", Value::F64(Ref::new(F64::new(9.0))));
test_interpreter!(interpret_set_value_slice_all,"~x := [1 2 3; 5 6 7]'; x[1..3,:] = 42; x[1] + x[2] + x[3] + x[4] + x[5] + x[6]", Value::F64(Ref::new(F64::new(178.0))));
test_interpreter!(interpret_set_value_slice_vec,"~x := [1 2 3 4]; x[1..=3] = [10 20 30]; x[1] + x[2] + x[3] + x[4]", Value::F64(Ref::new(F64::new(64.0))));

test_interpreter!(interpret_set_record_field,"~x := {a: 1, b: true}; x.a = 2; x.a;", Value::F64(Ref::new(F64::new(2.0))));
test_interpreter!(interpret_set_record_field2,"~x := {a: 1, b: true}; x.b = false; x.b;", Value::Bool(Ref::new(false)));
#[cfg(feature = "u64")]
test_interpreter!(interpret_set_record_field3,"~x := {a: 1<u64>, b: true}; x.a = 2<u64>; x.a;", Value::U64(Ref::new(2)));

test_interpreter!(interpret_set_table_col,"~x := | x<f64> y<f64> | 1 2 | 3 4 |; x.x = [42;46]; y := x.x; y[1] + y[2]", Value::F64(Ref::new(F64::new(88.0))));
test_interpreter!(interpret_set_table_col2,"~x := | x<f64> y<f64> | 1 2 | 3 4 | 5 6 | 7 8 |; x.x = [42;46;47;48]; y := x.x; y[1] + y[2] + y[3] + y[4];", Value::F64(Ref::new(F64::new(183.0))));
test_interpreter!(interpret_set_table_col_string,r#"~x := | x<string> | "a" | "b" |; x.x = ["c";"d"]; x.x"#, Value::MatrixString(Matrix::from_vec(vec!["c".to_string(), "d".to_string()], 2, 1)));

test_interpreter!(interpret_set_logical,"~x := [1 2 3]; ix := [true false true]; x[ix] = 4; x[1] + x[2] + x[3];", Value::F64(Ref::new(F64::new(10.0))));
test_interpreter!(interpret_set_logical2,"~x := [1 2 3 4]; ix := [true false true true]; x[ix] = 5; x[1] + x[2] + x[3] + x[4];", Value::F64(Ref::new(F64::new(17.0))));
test_interpreter!(interpret_set_logical_scalar,"~x := [1 2 3]; x[4 > 3] = 5; x[1] + x[2] + x[3]", Value::F64(Ref::new(F64::new(15.0))));

test_interpreter!(interpret_set_logical_vector_scalar_bool,"~x := [1 2; 4 5]; x[[true false], 2] = 42; x[1] + x[2] + x[3] + x[4];", Value::F64(Ref::new(F64::new(52.0))));
test_interpreter!(interpret_set_logical_scalar_vector_bool,"~x := [1 2; 4 5]; x[2,[false true]] = 42; x[1] + x[2] + x[3] + x[4]", Value::F64(Ref::new(F64::new(49.0))));
test_interpreter!(interpret_set_logical_vector_vector_bool,"~x := [1 2; 4 5]; x[[true false],[false true]] = 42;", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(4.0), F64::new(42.0), F64::new(5.0)], 2, 2)));

test_interpreter!(interpret_set_logical_all_vector_bool,"~x := [1 2; 4 5]; x[:,[1 2]] = 42; x[1] + x[2] + x[3] + x[4]", Value::F64(Ref::new(F64::new(168.0))));

test_interpreter!(interpret_horzcat,"x := [1 2]; y := [x 3]; y[1] + y[2] + y[3]", Value::F64(Ref::new(F64::new(6.0))));
test_interpreter!(interpret_horzcat_r2m1,"x := [1 2]; z := [3]; y := [x z]; y[1] + y[2] + y[3]", Value::F64(Ref::new(F64::new(6.0))));
test_interpreter!(interpret_horzcat_m1r2,"x := [1 2]; z := [3]; y := [z x]; y[1] + y[2] + y[3]", Value::F64(Ref::new(F64::new(6.0))));
test_interpreter!(interpret_horzcat_sr2,"x := [1 2]; y := [3 x]; y[1] + y[2] + y[3]", Value::F64(Ref::new(F64::new(6.0))));

test_interpreter!(interpret_horzcat_r2s,"x := [1 2]; y := [x 3];", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(2.0), F64::new(3.0)], 1, 3)));
test_interpreter!(interpret_horzcat_m1,"x := [1]; y := [x]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0)], 1, 1)));
test_interpreter!(interpret_horzcat_r2,"x := [1 2]; y := [x]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(2.0)], 1, 2)));

test_interpreter!(interpret_horzcat_sm1,"x := [2]; y := [1 x]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(2.0)], 1, 2)));
test_interpreter!(interpret_horzcat_m1s,"x := [2]; y := [x 1]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(2.0), F64::new(1.0)], 1, 2)));
test_interpreter!(interpret_horzcat_m1m1,"x := [2]; y := [x x]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(2.0), F64::new(2.0)], 1, 2)));

test_interpreter!(interpret_horzcat_sr3,"x := [1 2 3]; y := [1 x]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(1.0), F64::new(2.0), F64::new(3.0)], 1, 4)));
test_interpreter!(interpret_horzcat_r3s,"x := [1 2 3]; y := [x 1]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(2.0), F64::new(3.0), F64::new(1.0)], 1, 4)));
test_interpreter!(interpret_horzcat_r2r2,"x := [1 2]; y := [x x]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(2.0), F64::new(1.0), F64::new(2.0)], 1, 4)));
test_interpreter!(interpret_horzcat_m1r3,"x := [1 2 3]; z := [1]; y := [z x]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(1.0), F64::new(2.0), F64::new(3.0)], 1, 4)));
test_interpreter!(interpret_horzcat_r3m1,"x := [1 2 3]; z := [1]; y := [x z]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(2.0), F64::new(3.0), F64::new(1.0)], 1, 4)));

test_interpreter!(interpret_horzcat_ssm1,"x := [3]; y := [1 2 x]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(2.0), F64::new(3.0)], 1, 3)));
test_interpreter!(interpret_horzcat_sm1s,"x := [3]; y := [1 x 2]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(3.0), F64::new(2.0)], 1, 3)));
test_interpreter!(interpret_horzcat_m1ss,"x := [3]; y := [x 1 2]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(3.0), F64::new(1.0), F64::new(2.0)], 1, 3)));
test_interpreter!(interpret_horzcat_m1m1m1,"x := [3]; y := [x x x]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(3.0), F64::new(3.0), F64::new(3.0)], 1, 3)));
test_interpreter!(interpret_horzcat_sm1m1,"x := [3]; z:= [2]; y := [1 z x]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(2.0), F64::new(3.0)], 1, 3)));
test_interpreter!(interpret_horzcat_m1sm1,"x := [3]; z:= [2]; y := [z 1 x]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(2.0), F64::new(1.0), F64::new(3.0)], 1, 3)));
test_interpreter!(interpret_horzcat_m1m1s,"x := [3]; z:= [2]; y := [z x 1]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(2.0), F64::new(3.0), F64::new(1.0)], 1, 3)));

test_interpreter!(interpret_horzcat_m1m1r2,"x := [1]; y:= [2 3]; z := [x x y];", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(1.0), F64::new(2.0), F64::new(3.0)], 1, 4)));
test_interpreter!(interpret_horzcat_m1r2m1,"x := [1]; y:= [2 3]; z := [x y x];", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(2.0), F64::new(3.0), F64::new(1.0)], 1, 4)));
test_interpreter!(interpret_horzcat_r2m1m1,"x := [1]; y:= [2 3]; z := [y x x];", Value::MatrixF64(Matrix::from_vec(vec![F64::new(2.0), F64::new(3.0), F64::new(1.0), F64::new(1.0)], 1, 4)));

test_interpreter!(interpret_horzcat_ssr2,"y:= [2 3]; z := [1 1 y];", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(1.0), F64::new(2.0), F64::new(3.0)], 1, 4)));
test_interpreter!(interpret_horzcat_sr2s,"y:= [2 3]; z := [1 y 1];", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(2.0), F64::new(3.0), F64::new(1.0)], 1, 4)));
test_interpreter!(interpret_horzcat_r2ss,"y:= [2 3]; z := [y 1 1];", Value::MatrixF64(Matrix::from_vec(vec![F64::new(2.0), F64::new(3.0), F64::new(1.0), F64::new(1.0)], 1, 4)));

test_interpreter!(interpret_horzcat_sm1r2,"x := [1]; y:= [2 3]; z := [1 x y];", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(1.0), F64::new(2.0), F64::new(3.0)], 1, 4)));
test_interpreter!(interpret_horzcat_m1sr2, "x := [1]; y:= [2 3]; z := [x 1 y];", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(1.0), F64::new(2.0), F64::new(3.0)], 1, 4)));
test_interpreter!(interpret_horzcat_sr2m1, "x := [1]; y:= [2 3]; z := [1 y x];", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(2.0), F64::new(3.0), F64::new(1.0)], 1, 4)));
test_interpreter!(interpret_horzcat_m1r2s, "x := [1]; y:= [2 3]; z := [x y 1];", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(2.0), F64::new(3.0), F64::new(1.0)], 1, 4)));
test_interpreter!(interpret_horzcat_r2sm1, "x := [1]; y:= [2 3]; z := [y 1 x];", Value::MatrixF64(Matrix::from_vec(vec![F64::new(2.0), F64::new(3.0), F64::new(1.0), F64::new(1.0)], 1, 4)));
test_interpreter!(interpret_horzcat_r2m1s, "x := [1]; y:= [2 3]; z := [y x 1];", Value::MatrixF64(Matrix::from_vec(vec![F64::new(2.0), F64::new(3.0), F64::new(1.0), F64::new(1.0)], 1, 4)));

test_interpreter!(interpret_horzcat_sssm1, "x := [4]; y := [1 2 3 x];", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(2.0), F64::new(3.0), F64::new(4.0)], 1, 4)));
test_interpreter!(interpret_horzcat_m1sss, "x := [4]; y := [x 1 2 3];", Value::MatrixF64(Matrix::from_vec(vec![F64::new(4.0), F64::new(1.0), F64::new(2.0), F64::new(3.0)], 1, 4)));
test_interpreter!(interpret_horzcat_sm1ss, "x := [4]; y := [1 x 2 3];", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(4.0), F64::new(2.0), F64::new(3.0)], 1, 4)));
test_interpreter!(interpret_horzcat_ssm1s, "x := [4]; y := [1 2 x 3];", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(2.0), F64::new(4.0), F64::new(3.0)], 1, 4)));
test_interpreter!(interpret_horzcat_sm1sm1, "x := [4]; z := [5]; y := [1 x 2 z];", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(4.0), F64::new(2.0), F64::new(5.0)], 1, 4)));
test_interpreter!(interpret_horzcat_m1ssm1, "x := [4]; z := [5]; y := [x 1 2 z];", Value::MatrixF64(Matrix::from_vec(vec![F64::new(4.0), F64::new(1.0), F64::new(2.0), F64::new(5.0)], 1, 4)));
test_interpreter!(interpret_horzcat_m1sm1s, "x := [4]; z := [5]; y := [x 1 z 2];", Value::MatrixF64(Matrix::from_vec(vec![F64::new(4.0), F64::new(1.0), F64::new(5.0), F64::new(2.0)], 1, 4)));
test_interpreter!(interpret_horzcat_m1m1sm1, "x := [4]; z := [5]; w := [6]; y := [x z 1 w];", Value::MatrixF64(Matrix::from_vec(vec![F64::new(4.0), F64::new(5.0), F64::new(1.0), F64::new(6.0)], 1, 4)));
test_interpreter!(interpret_horzcat_m1m1m1s, "x := [4]; z := [5]; w := [6]; y := [x z w 1];", Value::MatrixF64(Matrix::from_vec(vec![F64::new(4.0), F64::new(5.0), F64::new(6.0), F64::new(1.0)], 1, 4)));
test_interpreter!(interpret_horzcat_m1sm1m1, "x := [4]; z := [5]; w := [6]; y := [x 1 z w];", Value::MatrixF64(Matrix::from_vec(vec![F64::new(4.0), F64::new(1.0), F64::new(5.0), F64::new(6.0)], 1, 4)));
test_interpreter!(interpret_horzcat_sm1m1m1, "x := [4]; z := [5]; w := [6]; y := [1 x z w];", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(4.0), F64::new(5.0), F64::new(6.0)], 1, 4)));
test_interpreter!(interpret_horzcat_m1m1m1m1, "x := [4]; z := [5]; w := [6]; v := [7]; y := [x z w v];", Value::MatrixF64(Matrix::from_vec(vec![F64::new(4.0), F64::new(5.0), F64::new(6.0), F64::new(7.0)], 1, 4)));

test_interpreter!(interpret_horzcat_m2m2m2, "x := [1 2]; y := [x x x]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(2.0), F64::new(1.0), F64::new(2.0), F64::new(1.0), F64::new(2.0)], 1, 6)));

test_interpreter!(interpret_horzcat_rd2, "x := [1 2 3 4 5]; y := [x]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(2.0), F64::new(3.0), F64::new(4.0), F64::new(5.0)], 1, 5)));
test_interpreter!(interpret_horzcat_rd4, "a := [1];b := [2 3];c := [4 5 6];d := [7 8 9 10];z := [a b c d];", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(2.0), F64::new(3.0), F64::new(4.0), F64::new(5.0), F64::new(6.0), F64::new(7.0), F64::new(8.0), F64::new(9.0), F64::new(10.0)], 1, 10)));
test_interpreter!(interpret_horzcat_rd3, "a := [1 1]; z := [a a a];", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(1.0), F64::new(1.0), F64::new(1.0), F64::new(1.0), F64::new(1.0)], 1, 6)));
test_interpreter!(interpret_horzcat_rdn, "a := [1 2 3]; z := [0 0 0 0 0 a];", Value::MatrixF64(Matrix::from_vec(vec![F64::new(0.0), F64::new(0.0), F64::new(0.0), F64::new(0.0), F64::new(0.0), F64::new(1.0), F64::new(2.0), F64::new(3.0)], 1, 8)));

test_interpreter!(interpret_horzcat_m2m2, "a := [1 2; 3 4]; z := [a a];", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(3.0), F64::new(2.0), F64::new(4.0), F64::new(1.0), F64::new(3.0), F64::new(2.0), F64::new(4.0)], 2, 4)));

test_interpreter!(interpret_horzcat_m2x3v2, "x := [1 2 3; 4 5 6]; z := [7 8]'; a := [x z];", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(4.0), F64::new(2.0), F64::new(5.0), F64::new(3.0), F64::new(6.0), F64::new(7.0), F64::new(8.0)], 2, 4)));
test_interpreter!(interpret_horzcat_v2m2x3, "x := [1 2 3; 4 5 6]; z := [7 8]'; a := [z x];", Value::MatrixF64(Matrix::from_vec(vec![F64::new(7.0), F64::new(8.0), F64::new(1.0), F64::new(4.0), F64::new(2.0), F64::new(5.0), F64::new(3.0), F64::new(6.0)], 2, 4)));

test_interpreter!(interpret_horzcat_v2v2, "x := [1;2]; y := [x x];", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(2.0), F64::new(1.0), F64::new(2.0)], 2, 2)));
test_interpreter!(interpret_horzcat_v3v3, "x := [1;2;3]; y := [x x];", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(2.0), F64::new(3.0), F64::new(1.0), F64::new(2.0), F64::new(3.0)], 3, 2)));
test_interpreter!(interpret_horzcat_v4v4, "x := [1;2;3;4]; y := [x x];", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(2.0), F64::new(3.0), F64::new(4.0), F64::new(1.0), F64::new(2.0), F64::new(3.0), F64::new(4.0)], 4, 2)));
test_interpreter!(interpret_horzcat_vdvd, "x := [1;2;3;4;5]; y := [x x];", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(2.0), F64::new(3.0), F64::new(4.0), F64::new(5.0), F64::new(1.0), F64::new(2.0), F64::new(3.0), F64::new(4.0), F64::new(5.0)], 5, 2)));

test_interpreter!(interpret_horzcat_v2m2, "x := [1 2; 3 4]; y := [1; 2]; z := [y x]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(2.0), F64::new(1.0), F64::new(3.0), F64::new(2.0), F64::new(4.0)], 2, 3)));
test_interpreter!(interpret_horzcat_m2v2, "x := [1 2; 3 4]; y := [1; 2]; z := [x y]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(3.0), F64::new(2.0), F64::new(4.0), F64::new(1.0), F64::new(2.0)], 2, 3)));


test_interpreter!(interpret_horzcat_m3x2v3, "x := [1 2; 3 4; 5 6]; y := [1; 2; 3]; z := [x y]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(3.0), F64::new(5.0), F64::new(2.0), F64::new(4.0), F64::new(6.0), F64::new(1.0), F64::new(2.0), F64::new(3.0)], 3, 3)));
test_interpreter!(interpret_horzcat_v3m3x2, "x := [1 2; 3 4; 5 6]; y := [1; 2; 3]; z := [y x]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(2.0), F64::new(3.0), F64::new(1.0), F64::new(3.0), F64::new(5.0), F64::new(2.0), F64::new(4.0), F64::new(6.0)], 3, 3)));

test_interpreter!(interpret_horzcat_mdv4, "x := [1 2; 3 4; 5 6; 7 8]; y := [1; 2; 3; 4]; z := [x y]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(3.0), F64::new(5.0), F64::new(7.0), F64::new(2.0), F64::new(4.0), F64::new(6.0), F64::new(8.0), F64::new(1.0), F64::new(2.0), F64::new(3.0), F64::new(4.0)], 4, 3)));
test_interpreter!(interpret_horzcat_v4md, "x := [1 2; 3 4; 5 6; 7 8]; y := [1; 2; 3; 4]; z := [y x]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(2.0), F64::new(3.0), F64::new(4.0), F64::new(1.0), F64::new(3.0), F64::new(5.0), F64::new(7.0), F64::new(2.0), F64::new(4.0), F64::new(6.0), F64::new(8.0)], 4, 3)));

test_interpreter!(interpret_horzcat_mdvd, "x := [1 2; 3 4; 5 6; 7 8; 9 10]; y := [1; 2; 3; 4; 5]; z := [x y]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(3.0), F64::new(5.0), F64::new(7.0), F64::new(9.0), F64::new(2.0), F64::new(4.0), F64::new(6.0), F64::new(8.0), F64::new(10.0), F64::new(1.0), F64::new(2.0), F64::new(3.0), F64::new(4.0), F64::new(5.0)], 5, 3)));
test_interpreter!(interpret_horzcat_vdmd, "x := [1 2; 3 4; 5 6; 7 8; 9 10]; y := [1; 2; 3; 4; 5]; z := [y x]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(2.0), F64::new(3.0), F64::new(4.0), F64::new(5.0), F64::new(1.0), F64::new(3.0), F64::new(5.0), F64::new(7.0), F64::new(9.0), F64::new(2.0), F64::new(4.0), F64::new(6.0), F64::new(8.0), F64::new(10.0)], 5, 3)));

test_interpreter!(interpret_horzcat_m2, "x := [1 2; 3 4]; z := [x]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(3.0), F64::new(2.0), F64::new(4.0)], 2, 2)));

test_interpreter!(interpret_horzcat_m3v3, "x := [1 2 3; 4 5 6; 7 8 9]; y := [1;2;3]; z := [x y]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(4.0), F64::new(7.0), F64::new(2.0), F64::new(5.0), F64::new(8.0), F64::new(3.0), F64::new(6.0), F64::new(9.0), F64::new(1.0), F64::new(2.0), F64::new(3.0)], 3, 4)));

test_interpreter!(interpret_horzcat_v2v2v2v2, "x := [1; 2;]; z := [x x x x] ", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(2.0), F64::new(1.0), F64::new(2.0), F64::new(1.0), F64::new(2.0), F64::new(1.0), F64::new(2.0)], 2, 4)));
test_interpreter!(interpret_horzcat_v2v2v2v2v2, "x := [1; 2;]; z := [x x x x x] ", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(2.0), F64::new(1.0), F64::new(2.0), F64::new(1.0), F64::new(2.0), F64::new(1.0), F64::new(2.0), F64::new(1.0), F64::new(2.0)], 2, 5)));
test_interpreter!(interpret_vertcat_vd2, "x := [1;2;3;4]; z := [5]; y := [x;z]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(2.0), F64::new(3.0), F64::new(4.0), F64::new(5.0)], 5, 1)));
test_interpreter!(interpret_vertcat_vd3, "x := [1;2;3]; z := [5]; y := [z;z;x]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(5.0), F64::new(5.0), F64::new(1.0), F64::new(2.0), F64::new(3.0)], 5, 1)));
test_interpreter!(interpret_vertcat_m1m1m1m1, "x := [5]; y := [x;x;x;x]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(5.0), F64::new(5.0), F64::new(5.0), F64::new(5.0)], 4, 1)));
test_interpreter!(interpret_vertcat_vd4, "x := [5]; z := [1;2]; y := [x;x;x;z]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(5.0), F64::new(5.0), F64::new(5.0), F64::new(1.0), F64::new(2.0)], 5, 1)));

test_interpreter!(interpret_vertcat_vdn, "x := [5]; y := [x;x;x;x;x]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(5.0), F64::new(5.0), F64::new(5.0), F64::new(5.0), F64::new(5.0)], 5, 1)));
test_interpreter!(interpret_vertcat_r2m2, "x := [5 2;3 4]; y := [8 9];z := [y;x]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(8.0), F64::new(5.0), F64::new(3.0), F64::new(9.0), F64::new(2.0), F64::new(4.0)], 3, 2)));
test_interpreter!(interpret_vertcat_m2r2, "x := [5 2;3 4]; y := [8 9];z := [x;y]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(5.0), F64::new(3.0), F64::new(8.0), F64::new(2.0), F64::new(4.0), F64::new(9.0)], 3, 2)));

test_interpreter!(interpret_vertcat_r2m2x3, "x := [1 2 3; 4 5 6]; y := [7 8 9]; z := [y;x]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(7.0), F64::new(1.0), F64::new(4.0), F64::new(8.0), F64::new(2.0), F64::new(5.0), F64::new(9.0), F64::new(3.0), F64::new(6.0)], 3, 3)));

test_interpreter!(interpret_stats_sum_rowm2, "x := [1 2; 4 5]; y := stats/sum/row(x);", Value::MatrixF64(Matrix::from_vec(vec![F64::new(5.0), F64::new(7.0)], 1, 2)));

test_interpreter!(interpret_add_assign, "~x := 10; x += 20", Value::F64(Ref::new(F64::new(30.0))));
test_interpreter!(interpret_add_assign_formula, "ix := [1 1 2 3]; y := 5; x := [1 2 3 4]; x[ix] += y;", Value::MatrixF64(Matrix::from_vec(vec![F64::new(11.0), F64::new(7.0), F64::new(8.0), F64::new(4.0)], 1, 4)));
test_interpreter!(interpret_add_assign_formula_all_m2m2,"~x := [1 2; 3 4]; y := [1 1 1 1];z := [10 10; 20 20]; x[y,:] += z;", Value::MatrixF64(Matrix::from_vec(vec![F64::new(61.0), F64::new(3.0), F64::new(62.0), F64::new(4.0)], 2, 2)));
test_interpreter!(interpret_sub_assign_formula, "ix := [1 1 2 3]; y := 5; x := [1 2 3 4]; x[ix] -= y;", Value::MatrixF64(Matrix::from_vec(vec![F64::new(-9.0), F64::new(-3.0), F64::new(-2.0), F64::new(4.0)], 1, 4)));
test_interpreter!(interpret_mul_assign_formula, "ix := [1 1 2 3]; y := 5; x := [1 2 3 4]; x[ix] *= y;", Value::MatrixF64(Matrix::from_vec(vec![F64::new(25.0), F64::new(10.0), F64::new(15.0), F64::new(4.0)], 1, 4)));
test_interpreter!(interpret_add_assign_range, "~x := [1 2; 3 4]; x[1..3] += 1", Value::MatrixF64(Matrix::from_vec(vec![F64::new(2.0), F64::new(4.0), F64::new(2.0), F64::new(4.0)], 2, 2)));
test_interpreter!(interpret_div_assign_range_all, "~x := [1 2; 3 4; 5 6]; x[1..3,:] /= [1 2; 3 4];", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(1.0), F64::new(5.0), F64::new(1.0), F64::new(1.0), F64::new(6.0)], 3, 2)));
test_interpreter!(interpret_div_assign_range, "~x := [1 2 3 4]; x[1..3] /= 2;", Value::MatrixF64(Matrix::from_vec(vec![F64::new(0.5), F64::new(1.0), F64::new(3.0), F64::new(4.0)], 1, 4)));
test_interpreter!(interpret_add_assign_range_vec, "~x := [1 2 3 4]; x[1..3] += 2;", Value::MatrixF64(Matrix::from_vec(vec![F64::new(3.0), F64::new(4.0), F64::new(3.0), F64::new(4.0)], 1, 4)));

test_interpreter!(interpret_set_logical_ram2m2_bool,"~x := [1 2; 3 4]; y := [true false]; z := [10 20; 30 40]; x[y,:] = z;", Value::MatrixF64(Matrix::from_vec(vec![F64::new(10.0), F64::new(3.0), F64::new(20.0), F64::new(4.0)], 2, 2)));
test_interpreter!(interpret_set_logical_ram3m3_bool,"~x := [1 2 3; 4 5 6; 7 8 9]; y := [true false true]; z := [10 20 30; 40 50 60; 70 80 90]; x[y,:] = z;", Value::MatrixF64(Matrix::from_vec(vec![F64::new(10.0), F64::new(4.0), F64::new(70.0), F64::new(20.0), F64::new(5.0), F64::new(80.0), F64::new(30.0), F64::new(6.0), F64::new(90.0)], 3, 3)));
test_interpreter!(interpret_set_logical_ram4m4_bool,"~x := [1 2 3 4; 5 6 7 8; 9 10 11 12; 13 14 15 16]; y := [true false true false]; z := [10 20 30 40; 50 60 70 80; 90 100 110 120; 130 140 150 160]; x[y,:] = z;", Value::MatrixF64(Matrix::from_vec(vec![F64::new(10.0), F64::new(5.0), F64::new(90.0), F64::new(13.0), F64::new(20.0), F64::new(6.0), F64::new(100.0), F64::new(14.0), F64::new(30.0), F64::new(7.0), F64::new(110.0), F64::new(15.0), F64::new(40.0), F64::new(8.0), F64::new(120.0), F64::new(16.0)], 4, 4)));

test_interpreter!(interpret_set_logical_ram2m2,"~x := [1 2; 3 4]; y := [2 1]; x[y,:] = x;", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(1.0), F64::new(2.0), F64::new(2.0)], 2, 2)));
test_interpreter!(interpret_set_logical_ram3m3,"~x := [1 2 3; 4 5 6; 7 8 9]; y := [2 1 3]; x[y,:] = x;", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(1.0), F64::new(7.0), F64::new(2.0), F64::new(2.0), F64::new(8.0), F64::new(3.0), F64::new(3.0), F64::new(9.0)], 3, 3)));

test_interpreter!(interpret_modulus,"[1 2 3 4 5] % 5", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(2.0), F64::new(3.0), F64::new(4.0), F64::new(0.0)], 1, 5)));

test_interpreter!(interpret_horzcat_rdn2, "y := [4 5 6]; [y y y * 2]", Value::MatrixF64(Matrix::from_vec(vec![F64::new(4.0), F64::new(5.0), F64::new(6.0), F64::new(4.0), F64::new(5.0), F64::new(6.0), F64::new(8.0), F64::new(10.0), F64::new(12.0)], 1, 9)));

#[cfg(feature = "u8")]
test_interpreter!(interpret_fancy_table, r#"x := |x<f64> y<u8>|
     |1      2    |
     |3      4    |
     |5      6    |
x.x"#, Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(3.0), F64::new(5.0)], 3, 1)));

test_interpreter!(interpret_fancy_matrix, r#"
x := ┏       ┓
     ┃ 1   2 ┃
     ┃ 3   4 ┃
     ┗       ┛"#, Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.0), F64::new(3.0), F64::new(2.0), F64::new(4.0)], 2, 2)));

#[cfg(all(feature = "f32", feature = "u64"))]
test_interpreter!(interpret_fancy_table2, r#"
x:=╭────────┬────────╮
   │ x<u64> │ y<f32> │
   ├────────┼────────┤
   │   1    │   2    │
   ├────────┼────────┤
   │   3    │   4    │
   ╰────────┴────────╯
x.x"#, Value::MatrixU64(Matrix::from_vec(vec![1_u64, 3], 2, 1)));

#[cfg(all(feature = "f32", feature = "u64"))]
test_interpreter!(interpret_fancy_table3, r#"
x:=╭────────┬────────╮
   │ x<u64> │ y<f32> │
   │   1    │   2    │
   │   3    │   4    │
   ╰────────┴────────╯
x.x"#, Value::MatrixU64(Matrix::from_vec(vec![1_u64, 3], 2, 1)));

#[cfg(all(feature = "f32", feature = "u64"))]
test_interpreter!(interpret_fancy_table4, r#"
x:=╭────────┬────────╮
   │ x<u64> │ y<f32> │
   ├────────┼────────┤
   │        │        │
   │   1    │   2    │
   │        │        │
   │        │        │
   │   3    │   4    │
   │        │        │
   ╰────────┴────────╯
x.x"#, Value::MatrixU64(Matrix::from_vec(vec![1_u64, 3], 2, 1)));
#[cfg(all(feature = "f32", feature = "u8"))]
test_interpreter!(interpret_table_access_element, r#"a:=|x<f32>|1.2|1.3|; a.x[1]"#, Ref::new(F32::new(1.2)).to_value());
#[cfg(all(feature = "f32", feature = "u8"))]
test_interpreter!(interpret_table_access_row, r#"x:=|a<f32> b<u8>|1.2 3 |1.3 4|; x{2}"#, Value::Record(Ref::new(MechRecord::new(vec![("a",Value::F32(Ref::new(F32::new(1.3)))),("b",Value::U8(Ref::new(4)))]))));
test_interpreter!(interpret_table_append_row, r#"~x:=|a<f64> b<f64>|1 2 |3 4|; x += {a<f64>: 5, b<f64>: 6}; x{3}"#, Value::Record(Ref::new(MechRecord::new(vec![("a",Value::F64(Ref::new(F64::new(5.0)))),("b",Value::F64(Ref::new(F64::new(6.0))))]))));
test_interpreter!(interpret_table_append_row2, r#"~x:=|a<f64> b<f64>|1 2 |3 4|; x += {b<f64>: 6, a<f64>: 5}; x{3}"#, Value::Record(Ref::new(MechRecord::new(vec![("b",Value::F64(Ref::new(F64::new(6.0)))),("a",Value::F64(Ref::new(F64::new(5.0))))]))));
#[cfg(all(feature = "u8", feature = "u64"))]
test_interpreter!(interpret_table_append_row3, r#"~x := |a<u64> b<u8>| 1 2 | 3 4 |; a:=13; b:=14; y := {c<bool>: false, a<u64>: a, b<u8>: b}; x += y; x{3}"#, Value::Record(Ref::new(MechRecord::new(vec![("a",Value::U64(Ref::new(13))),("b",Value::U8(Ref::new(14)))]))));
#[cfg(all(feature = "u8", feature = "u64"))]
test_interpreter!(interpret_table_append_table, r#"~x := |a<u64> b<u8>| 1 2 | 3 4 |;y := |a<u64> b<u8>| 5 6 | 7 8 |; x += y; x{4}"#, Value::Record(Ref::new(MechRecord::new(vec![("a",Value::U64(Ref::new(7))),("b",Value::U8(Ref::new(8)))]))));

#[cfg(all(feature = "u8", feature = "u64"))]
test_interpreter!(interpret_table_select_rows, r#"x := |a<u64> b<u8>| 1 2 | 3 4 | 5 6 |; x{[1,3]}"#, Value::Table(Ref::new(MechTable::from_records(vec![MechRecord::new(vec![("a",Value::U64(Ref::new(1))),("b",Value::U8(Ref::new(2)))]),MechRecord::new(vec![("a",Value::U64(Ref::new(5))),("b",Value::U8(Ref::new(6)))]),]).expect("Failed to create MechTable"))));
#[cfg(feature = "u64")]
test_interpreter!(interpret_table_select_logical, r#"a := | x<u64>  y<bool> | 2 true  | 3 false | 4 false | 5 true |; a{a.y}"#, Value::Table(Ref::new(MechTable::from_records(vec![MechRecord::new(vec![("x",Value::U64(Ref::new(2))),("y",Value::Bool(Ref::new(true)))]),MechRecord::new(vec![("x",Value::U64(Ref::new(5))),("y",Value::Bool(Ref::new(true)))]),]).expect("Failed to create MechTable"))));
#[cfg(feature = "u64")]
test_interpreter!(interpret_table_select_logical2, r#"a := | x<u64>  y<bool> | 2 true  | 3 false | 4 false | 5 true |; a{a.x > 3<u64>}"#, Value::Table(Ref::new(MechTable::from_records(vec![MechRecord::new(vec![("x",Value::U64(Ref::new(4))),("y",Value::Bool(Ref::new(false)))]),MechRecord::new(vec![("x",Value::U64(Ref::new(5))),("y",Value::Bool(Ref::new(true)))]),]).expect("Failed to create MechTable"))));

test_interpreter!(interpret_table_from_matrix,r#"x := [1 2; 3 4]; a<|foo<f64>,bar<f64>|> := x"#,Value::Table(Ref::new(MechTable::from_records(vec![MechRecord::new(vec![("foo", Value::F64(Ref::new(F64::new(1.0)))),("bar", Value::F64(Ref::new(F64::new(2.0))))]),MechRecord::new(vec![("foo", Value::F64(Ref::new(F64::new(3.0)))),("bar", Value::F64(Ref::new(F64::new(4.0))))]),]).expect("Failed to create MechTable"))));
test_interpreter!(interpret_table_from_matrix2,r#"x := ["true" "false"; "true" "false"]; a<|x<string> y<string>|> := x"#,Value::Table(Ref::new(MechTable::from_records(vec![MechRecord::new(vec![("x", Value::String(Ref::new("true".to_string()))),("y", Value::String(Ref::new("false".to_string())))]),MechRecord::new(vec![("x", Value::String(Ref::new("true".to_string()))),("y", Value::String(Ref::new("false".to_string())))]),]).expect("Failed to create MechTable"))));
test_interpreter!(interpret_table_from_matrix3,r#"x:=[true false; true false]; a<|x<bool> y<bool>|> := x;"#,Value::Table(Ref::new(MechTable::from_records(vec![MechRecord::new(vec![("x", Value::Bool(Ref::new(true))),("y", Value::Bool(Ref::new(false)))]),MechRecord::new(vec![("x", Value::Bool(Ref::new(true))),("y", Value::Bool(Ref::new(false)))]),]).expect("Failed to create MechTable"))));
#[cfg(all(feature = "i8", feature = "u8"))]
test_interpreter!(interpret_table_from_matrix4,r#"x:=[1 2; 3 4]; a<|x<u8> y<i8>|> := x;"#,Value::Table(Ref::new(MechTable::from_records(vec![MechRecord::new(vec![("x", Value::U8(Ref::new(1))),("y", Value::I8(Ref::new(2)))]),MechRecord::new(vec![("x", Value::U8(Ref::new(3))),("y", Value::I8(Ref::new(4)))]),]).expect("Failed to create MechTable"))));

#[cfg(feature = "u64")]
test_interpreter!(interpret_matrix_reshape,r#"x:=[1 3; 2 4]; y<[u64]:4,1> := x"#, Value::MatrixU64(Matrix::from_vec(vec![1, 2, 3, 4], 4, 1)));

test_interpreter!(interpret_matrix_reshape2,r#"x:=[1 2 3 4]; y<[string]:2,2> := x"#, Value::MatrixString(Matrix::from_vec(vec![String::from("1"), String::from("2"), String::from("3"), String::from("4")], 2, 2)));  
test_interpreter!(interpret_matrix_convert_str,r#"x:=1..=4; out<[string]>:=x"#, Value::MatrixString(Matrix::from_vec(vec![String::from("1"), String::from("2"), String::from("3"), String::from("4")], 1, 4)));

test_interpreter!(interpret_matrix_build_rational,r#"x<[r64]:1,2> := 1/2"#, Value::MatrixR64(Matrix::from_vec(vec![R64::new(1,2), R64::new(1,2)], 1, 2)));

test_interpreter!(interpret_convert_rational_to_string,r#"x<string>:=1/2"#, Value::String(Ref::new(String::from("1/2"))));
test_interpreter!(interpret_convert_f64_to_string2,r#"x<string>:=123"#, Value::String(Ref::new(String::from("123"))));

test_interpreter!(interpret_convert_f64_to_rational_to_string,r#"x<string> := 0.5<r64>"#,Value::String(Ref::new(String::from("1/2"))));

test_interpreter!(interpret_matrix_power_and_addition,"~μ := [1 2 3]; K := [0.1 0.2 0.3; 0.4 0.5 0.6; 0.7 0.8 0.9]; Ẑ := [0.01; 0.02; 0.03]; μ = μ + (K ** Ẑ)'", Value::MatrixF64(Matrix::from_vec(vec![F64::new(1.014), F64::new(2.032), F64::new(3.05)], 1, 3)));
test_interpreter!(interpret_assign_scalar_no_space, "~z:=10;z=20", Value::F64(Ref::new(F64::new(20.0))));


test_interpreter!(interpret_paren_term_whitespace, "fahrenheit := ( 25 * 9 / 5 ) + 32", Value::F64(Ref::new(F64::new(77.0))));

test_interpreter!(interpret_formulas_no_whitespace, "x:=10*[1,2,3]**[4,5,6]';", Value::MatrixF64(Matrix::from_vec(vec![F64::new(320.0)], 1, 1)));

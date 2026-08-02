#![allow(warnings)]
#[path = "support/bytecode/catalog.rs"]
mod catalog;
#[path = "support/bytecode/dynamic_matrix_factory.rs"]
mod dynamic_matrix_factory;

extern crate mech_core;
extern crate mech_syntax;
use indexmap::set::IndexSet;
use mech_core::matrix::Matrix;
use mech_core::*;
use mech_engine::{MechProgram as EngineMechProgram, MechProgramConfig, MechProgramEnvironment};
use mech_syntax::*;
use std::cell::RefCell;
use std::rc::Rc;

/// Root bytecode tests compile against the standard distribution. Bare and
/// custom-catalog bytecode behavior lives in `support/bytecode/catalog`.
struct MechProgram;

impl MechProgram {
    fn new(config: MechProgramConfig) -> EngineMechProgram {
        #[cfg(feature = "standard_source")]
        {
            EngineMechProgram::with_function_catalog(config, mech::stdlib::source_catalog())
        }
        #[cfg(not(feature = "standard_source"))]
        {
            EngineMechProgram::new(config)
        }
    }
}

macro_rules! bytecode_test {
    ($name:ident, $code:expr, $expected:expr) => {
        #[test]
        fn $name() {
            let mut prgrm = MechProgram::new(MechProgramConfig {
                name: stringify!($name).to_string(),
                environment: MechProgramEnvironment::default(),
            });

            prgrm
                .run_string($code)
                .unwrap_or_else(|err| panic!("Runtime error: {:?}", err));

            let bytecode = prgrm
                .compile_bytecode()
                .unwrap_or_else(|err| panic!("Compile error: {:?}", err));

            let prog = ParsedProgram::from_bytes(&bytecode)
                .unwrap_or_else(|err| panic!("Deserialize error: {:?}", err));

            let result = prgrm
                .run_bytecode_program(&prog)
                .unwrap_or_else(|err| panic!("Runtime error: {:?}", err));

            assert_eq!(result, $expected);
        }
    };
}

bytecode_test!(
    bytecode_define_string,
    "x := \"Hello World!\"",
    Value::String(Ref::new("Hello World!".to_string()))
);
bytecode_test!(bytecode_var_def, "x := 10", Value::F64(Ref::new(10.0)));
bytecode_test!(bytecode_math, "1 + 2", Value::F64(Ref::new(3.0)));
bytecode_test!(
    bytecode_math_def,
    "x := 1 + 2; y := x + 4",
    Value::F64(Ref::new(7.0))
);
bytecode_test!(
    bytecode_tuple_access,
    "tuple := (1, 2); tuple.2",
    Value::F64(Ref::new(2.0))
);
bytecode_test!(
    bytecode_math_mul,
    "x := 2 * 2; y := x * 4",
    Value::F64(Ref::new(16.0))
);
bytecode_test!(
    bytecode_math_add_assign,
    "~x := 10; x += 20",
    Value::F64(Ref::new(30.0))
);
bytecode_test!(
    bytecode_math_add_assign_vv,
    "~x := [1 2 3]; x += [10 20 30]",
    Value::MatrixF64(Matrix::from_vec(vec![11.0, 22.0, 33.0], 1, 3))
);
bytecode_test!(
    bytecode_math_add_assign_vr,
    "~x := [1 1]; y := [1 2]; z := [10 20]; x[y] += z;",
    Value::MatrixF64(Matrix::from_vec(vec![11.0, 21.0], 1, 2))
);
bytecode_test!(
    bytecode_math_sub_assign,
    "~x := 30; x -= 20",
    Value::F64(Ref::new(10.0))
);
bytecode_test!(
    bytecode_math_sub_assign_vv,
    "~x := [10 20 30]; x -= [1 2 3]",
    Value::MatrixF64(Matrix::from_vec(vec![9.0, 18.0, 27.0], 1, 3))
);
bytecode_test!(
    bytecode_math_sub_assign_vr,
    "~x := [11 21]; y := [1 2]; z := [10 20]; x[y] -= z;",
    Value::MatrixF64(Matrix::from_vec(vec![1.0, 1.0], 1, 2))
);
bytecode_test!(
    bytecode_math_mul_assign,
    "~x := 10; x *= 20",
    Value::F64(Ref::new(200.0))
);
bytecode_test!(
    bytecode_math_mul_assign_vv,
    "~x := [1 2 3]; x *= [10 20 30]",
    Value::MatrixF64(Matrix::from_vec(vec![10.0, 40.0, 90.0], 1, 3))
);
bytecode_test!(
    bytecode_math_mul_assign_vr,
    "~x := [1 2]; y := [1 2]; z := [10 20]; x[y] *= z;",
    Value::MatrixF64(Matrix::from_vec(vec![10.0, 40.0], 1, 2))
);
bytecode_test!(
    bytecode_math_div_assign,
    "~x := 200; x /= 20",
    Value::F64(Ref::new(10.0))
);
bytecode_test!(
    bytecode_math_div_assign_vv,
    "~x := [10 20 30]; x /= [1 2 5]",
    Value::MatrixF64(Matrix::from_vec(vec![10.0, 10.0, 6.0], 1, 3))
);
bytecode_test!(
    bytecode_math_div_assign_vr,
    "~x := [10 20]; y := [1 2]; z := [10 4]; x[y] /= z;",
    Value::MatrixF64(Matrix::from_vec(vec![1.0, 5.0], 1, 2))
);
bytecode_test!(
    bytecode_matrix_rowvector3,
    "[1 2 3]",
    Value::MatrixF64(Matrix::from_vec(vec![1.0, 2.0, 3.0], 1, 3))
);
bytecode_test!(
    bytecode_matrix_vector2,
    "[1; 2]",
    Value::MatrixF64(Matrix::from_vec(vec![1.0, 2.0], 2, 1))
);
bytecode_test!(
    bytecode_matrix_matrix2x2,
    "[1 2; 3 4]",
    Value::MatrixF64(Matrix::from_vec(vec![1.0, 3.0, 2.0, 4.0], 2, 2))
);
#[cfg(feature = "standard_compiler")]
bytecode_test!(
    bytecode_combinatorics_n_choose_k,
    "+> combinatorics\ncombinatorics/n-choose-k(10,2)",
    Value::F64(Ref::new(45.0))
);
bytecode_test!(bytecode_compare_gt, "1 > 2", Value::Bool(Ref::new(false)));
bytecode_test!(
    bytecode_compare_eq,
    r#""foo" == "bar""#,
    Value::Bool(Ref::new(false))
);
bytecode_test!(
    bytecode_logic_and,
    "true && false",
    Value::Bool(Ref::new(false))
);
bytecode_test!(
    bytecode_logic_or,
    "true || false",
    Value::Bool(Ref::new(true))
);
bytecode_test!(bytecode_logic_not, "!true", Value::Bool(Ref::new(false)));
#[cfg(feature = "standard_compiler")]
bytecode_test!(
    bytecode_math_cos,
    "+> math\nmath/cos(0)",
    Value::F64(Ref::new(1.0))
);
#[cfg(feature = "standard_compiler")]
bytecode_test!(
    bytecode_math_sin,
    "+> math\nmath/sin(0)",
    Value::F64(Ref::new(0.0))
);
#[cfg(feature = "standard_compiler")]
bytecode_test!(
    bytecode_math_atan2,
    "+> math\nmath/atan2(1, 1)",
    Value::F64(Ref::new(std::f64::consts::FRAC_PI_4))
);
#[cfg(feature = "standard_compiler")]
bytecode_test!(
    bytecode_math_atan22,
    "+> math\nmath/atan(1, 1)",
    Value::F64(Ref::new(std::f64::consts::FRAC_PI_4))
);
bytecode_test!(
    bytecode_matrix_matmul_transpose,
    "[1 2 3] ** [4 5 6]'",
    Value::MatrixF64(Matrix::from_vec(vec![32.0], 1, 1))
);
bytecode_test!(
    bytecode_matrix_dot,
    "[1 2 3] \u{00b7} [4 5 6]",
    Value::F64(Ref::new(32.0))
);
bytecode_test!(
    bytecode_range_inclusive,
    "1..=4",
    Value::MatrixF64(Matrix::from_vec(vec![1.0, 2.0, 3.0, 4.0], 1, 4))
);
bytecode_test!(
    bytecode_range_inclusive_d,
    "1..=5",
    Value::MatrixF64(Matrix::from_vec(vec![1.0, 2.0, 3.0, 4.0, 5.0], 1, 5))
);
bytecode_test!(
    bytecode_range_inclusive_refs,
    "a := 1; b :=4 ; a..=b",
    Value::MatrixF64(Matrix::from_vec(vec![1.0, 2.0, 3.0, 4.0], 1, 4))
);
bytecode_test!(
    bytecode_range_exclusive,
    "1..5",
    Value::MatrixF64(Matrix::from_vec(vec![1.0, 2.0, 3.0, 4.0], 1, 4))
);
#[cfg(feature = "standard_compiler")]
bytecode_test!(
    bytecode_stats_sum_column,
    "+> stats\nstats/sum/column([1 2 3])",
    Value::MatrixF64(Matrix::from_vec(vec![6.0], 1, 1))
);
bytecode_test!(
    bytecode_matrix_index_assign,
    "~x := [1 2 3]; x[1] = 10",
    Value::MatrixF64(Matrix::from_vec(vec![10.0, 2.0, 3.0], 1, 3))
);
bytecode_test!(
    bytecode_matrix_index_assign_bool,
    "~x := [1 2 3]; x[[true false true]] = [4 5 6]",
    Value::MatrixF64(Matrix::from_vec(vec![4.0, 2.0, 6.0], 1, 3))
);
bytecode_test!(
    bytecode_matrix_index_assign_bool_all,
    "~x := [1 2 3]; x[true] = [4 5 6]",
    Value::MatrixF64(Matrix::from_vec(vec![4.0, 5.0, 6.0], 1, 3))
);
bytecode_test!(
    bytecode_matrix_index_assign_bool_all_scalar,
    "~x := [1 2 3]; x[true] = 10",
    Value::MatrixF64(Matrix::from_vec(vec![10.0, 10.0, 10.0], 1, 3))
);
bytecode_test!(
    bytecode_matrix_index_assign_scalar,
    "~x := [1 2 3]; x[3] = 10",
    Value::MatrixF64(Matrix::from_vec(vec![1.0, 2.0, 10.0], 1, 3))
);
bytecode_test!(
    bytecode_matrix_index_assign_all_scalar,
    "~x := [1 2 3]; x[:] = 10",
    Value::MatrixF64(Matrix::from_vec(vec![10.0, 10.0, 10.0], 1, 3))
);
bytecode_test!(
    bytecode_matrix_index_assign_2d_scalar,
    "~x := [1 2 3; 4 5 6; 7 8 9]; x[1,3] = 10",
    Value::MatrixF64(Matrix::from_vec(
        vec![1.0, 4.0, 7.0, 2.0, 5.0, 8.0, 10.0, 6.0, 9.0],
        3,
        3
    ))
);
bytecode_test!(
    bytecode_matrix_index_assign_2d_scalar_all,
    "~x := [1 2; 4 5]; x[:,1] = 10",
    Value::MatrixF64(Matrix::from_vec(vec![10.0, 10.0, 2.0, 5.0], 2, 2))
);
bytecode_test!(
    bytecode_matrix_index_assign_2d_vector_all,
    "~x := [1 2; 4 5]; x[:,2] = [10 20]",
    Value::MatrixF64(Matrix::from_vec(vec![1.0, 4.0, 10.0, 20.0], 2, 2))
);
bytecode_test!(
    bytecode_matrix_index_assign_2d_vector_all_rows,
    "~x := [1 2; 4 5]; x[1,:] = 10 ",
    Value::MatrixF64(Matrix::from_vec(vec![10.0, 4.0, 10.0, 5.0], 2, 2))
);
bytecode_test!(
    bytecode_matrix_index_assign_2d_vector_rows,
    "~x := [1 2; 4 5; 6 7]; x[[1],2] = 53",
    Value::MatrixF64(Matrix::from_vec(vec![1.0, 4.0, 6.0, 53.0, 5.0, 7.0], 3, 2))
);
bytecode_test!(
    bytecode_matrix_index_assign_2d_vector_rows_multi,
    "~x := [1 2; 4 5; 6 7]; x[[1 3],2] = 53",
    Value::MatrixF64(Matrix::from_vec(vec![1.0, 4.0, 6.0, 53.0, 5.0, 53.0], 3, 2))
);
bytecode_test!(
    bytecode_matrix_index_assign_2d_vector_rows_multi2,
    "~x := [1 2; 4 5; 6 7]; x[[1 3],2] = [10 20]",
    Value::MatrixF64(Matrix::from_vec(vec![1.0, 4.0, 6.0, 10.0, 5.0, 20.0], 3, 2))
);
bytecode_test!(
    bytecode_matrix_index_assign_2d_vector_rows_bool,
    "~x := [1 2; 4 5; 6 7]; x[[true false true],2] = 20",
    Value::MatrixF64(Matrix::from_vec(vec![1.0, 4.0, 6.0, 20.0, 5.0, 20.0], 3, 2))
);
bytecode_test!(
    bytecode_matrix_index_assign_2d_vector_rows_bool2,
    "~x := [1 2; 4 5; 6 7]; x[[false true true],1] = [10 20 30]",
    Value::MatrixF64(Matrix::from_vec(vec![1.0, 20.0, 30.0, 2.0, 5.0, 7.0], 3, 2))
);
bytecode_test!(
    bytecode_matrix_index_assign_2d_scalar_vector,
    "~x := [1 2 3; 4 5 6]; x[1, [2 3]] = 20",
    Value::MatrixF64(Matrix::from_vec(vec![1.0, 4.0, 20.0, 5.0, 20.0, 6.0], 2, 3))
);
bytecode_test!(
    bytecode_matrix_index_assign_2d_scalar_vector2,
    "~x := [1 2 3; 4 5 6]; x[1, [2 3]] = [10 20]",
    Value::MatrixF64(Matrix::from_vec(vec![1.0, 4.0, 10.0, 5.0, 20.0, 6.0], 2, 3))
);
bytecode_test!(
    bytecode_matrix_index_assign_2d_scalar_vector_bool,
    "~x := [1 2 3; 4 5 6]; x[1, [true false true]] = 10",
    Value::MatrixF64(Matrix::from_vec(vec![10.0, 4.0, 2.0, 5.0, 10.0, 6.0], 2, 3))
);
bytecode_test!(
    bytecode_matrix_index_assign_2d_scalar_vector_bool2,
    "~x := [1 2 3; 4 5 6]; x[1, [true false true]] = [10 20 30]",
    Value::MatrixF64(Matrix::from_vec(vec![10.0, 4.0, 2.0, 5.0, 30.0, 6.0], 2, 3))
);
bytecode_test!(
    bytecode_matrix_index_assign_2d_range_range,
    "~x := [1 2 3; 4 5 6; 7 8 9]; x[[1 3], [1 3]] = 10",
    Value::MatrixF64(Matrix::from_vec(
        vec![10.0, 4.0, 10.0, 2.0, 5.0, 8.0, 10.0, 6.0, 10.0],
        3,
        3
    ))
);
bytecode_test!(
    bytecode_matrix_index_assign_2d_range_range_all,
    "~x := [1 2 3; 4 5 6; 7 8 9]; x[[1 3], [1 2 3]] = 10",
    Value::MatrixF64(Matrix::from_vec(
        vec![10.0, 4.0, 10.0, 10.0, 5.0, 10.0, 10.0, 6.0, 10.0],
        3,
        3
    ))
);
bytecode_test!(
    bytecode_matrix_index_assign_2d_range_range_all2,
    "~x := [1 2 3; 4 5 6; 7 8 9]; x[[1 3], [1 2 3]] = [10 20 30 40 50 60]",
    Value::MatrixF64(Matrix::from_vec(
        vec![10.0, 4.0, 40.0, 20.0, 5.0, 50.0, 30.0, 6.0, 60.0],
        3,
        3
    ))
);
bytecode_test!(
    bytecode_matrix_index_assign_2d_range_range_bool,
    "~x := [1 2 3; 4 5 6; 7 8 9]; x[[false true false], [true false true]] = 10",
    Value::MatrixF64(Matrix::from_vec(
        vec![1.0, 10.0, 7.0, 2.0, 5.0, 8.0, 3.0, 10.0, 9.0],
        3,
        3
    ))
);
bytecode_test!(
    bytecode_matrix_index_assign_2d_range_range_bool2,
    "~x := [1 2 3; 4 5 6; 7 8 9]; x[[true false true], [false true false]] = [10 20 30; 40 50 60; 70 80 90]",
    Value::MatrixF64(Matrix::from_vec(
        vec![1.0, 4.0, 7.0, 40.0, 5.0, 60.0, 3.0, 6.0, 9.0],
        3,
        3
    ))
);
bytecode_test!(
    bytecode_matrix_index_assign_2d_range_range_bool3,
    "~x := [1 2 3; 4 5 6; 7 8 9]; x[[true false true], [1 2]] = 10",
    Value::MatrixF64(Matrix::from_vec(
        vec![10.0, 4.0, 10.0, 10.0, 5.0, 10.0, 3.0, 6.0, 9.0],
        3,
        3
    ))
);
bytecode_test!(
    bytecode_matrix_index_assign_2d_range_range_bool4,
    "~x := [1 2 3; 4 5 6; 7 8 9]; x[[true false true], [1 2]] = [10 20; 40 50; 70 80]",
    Value::MatrixF64(Matrix::from_vec(
        vec![10.0, 4.0, 70.0, 20.0, 5.0, 80.0, 3.0, 6.0, 9.0],
        3,
        3
    ))
);
bytecode_test!(
    bytecode_matrix_index_assign_2d_range_range_bool5,
    "~x := [1 2 3; 4 5 6; 7 8 9]; x[[1 3],[true false true]] = 10",
    Value::MatrixF64(Matrix::from_vec(
        vec![10.0, 10.0, 7.0, 2.0, 5.0, 8.0, 10.0, 10.0, 9.0],
        3,
        3
    ))
);
bytecode_test!(
    bytecode_matrix_index_assign_2d_range_range_bool6,
    "~x := [1 2 3; 4 5 6; 7 8 9]; x[[1 2],[true false true]] = [10 20 30; 40 50 60; 70 80 90]",
    Value::MatrixF64(Matrix::from_vec(
        vec![10.0, 40.0, 7.0, 2.0, 5.0, 8.0, 30.0, 60.0, 9.0],
        3,
        3
    ))
);
bytecode_test!(
    bytecode_string_matrix,
    r#"x := ["Hello" "World"]"#,
    Value::MatrixString(Matrix::from_vec(
        vec!["Hello".to_string(), "World".to_string()],
        1,
        2
    ))
);
bytecode_test!(
    bytecode_string_matrix_index,
    r#"x := ["Hello" "World"]; x[2]"#,
    Value::String(Ref::new("World".to_string()))
);
bytecode_test!(
    bytecode_matrix_index_bool_2d,
    r#"ix := [false, false, true]; x := [1 2 3; 4 5 6; 7 8 9]; x[:,ix]"#,
    Value::MatrixF64(Matrix::from_vec(vec![3.0, 6.0, 9.0], 3, 1))
);
bytecode_test!(
    bytecode_matrix_index_scalar_2d,
    r#"ix := [1, 3]; x := [1 2 3 ; 4 5 6 ; 7 8 9]; x[:,ix]"#,
    Value::MatrixF64(Matrix::from_vec(vec![1.0, 4.0, 7.0, 3.0, 6.0, 9.0], 3, 2))
);
bytecode_test!(
    bytecode_matrix_index_bool_2d_all,
    r#"ix := [true, true, false]; x := [1 2 3]; x[:,ix]"#,
    Value::MatrixF64(Matrix::from_vec(vec![1.0, 2.0], 1, 2))
);
bytecode_test!(
    bytecode_matrix_index_2d_vuu,
    r#"x := [1 2 3; 4 5 6;7 8 9]; ix1 := [1, 2]; x[ix1,ix1]"#,
    Value::MatrixF64(Matrix::from_vec(vec![1.0, 4.0, 2.0, 5.0], 2, 2))
);
bytecode_test!(
    bytecode_matrix_index_2d_vbb,
    r#"x := [1 2 3; 4 5 6; 7 8 9]; x[[true false false], [true false true]]"#,
    Value::MatrixF64(Matrix::from_vec(vec![1.0, 3.0], 1, 2))
);
bytecode_test!(
    bytecode_matrix_index_2d_vbb2,
    r#"x := [1 2 3; 4 5 6; 7 8 9]; x[[true false true],[true false false]]"#,
    Value::MatrixF64(Matrix::from_vec(vec![1.0, 7.0], 2, 1))
);
bytecode_test!(
    bytecode_matrix_index_2d_vbb3,
    r#"x := [1 2 3; 4 5 6; 7 8 9]; x[[true false false],[true false false]]"#,
    Value::MatrixF64(Matrix::from_vec(vec![1.0], 1, 1))
);
bytecode_test!(
    bytecode_matrix_index_2d_vbb4,
    r#"x := [1 2 3; 4 5 6; 7 8 9]; x[[true false true],[true false true]]"#,
    Value::MatrixF64(Matrix::from_vec(vec![1.0, 7.0, 3.0, 9.0], 2, 2))
);
bytecode_test!(
    bytecode_matrix_index_2d_vub,
    r#"ix := [false, false, true]; x := [1 2 3; 4 5 6; 7 8 9]; x[[1,2,3,3],ix]"#,
    Value::MatrixF64(Matrix::from_vec(vec![3.0, 6.0, 9.0, 9.0], 4, 1))
);
bytecode_test!(
    bytecode_matrix_index_2d_vbu,
    r#"ix1 := [false, false, true]; ix2 := [1,2,3,3]; x := [1 2 3; 4 5 6; 7 8 9]; x[ix1,ix2]"#,
    Value::MatrixF64(Matrix::from_vec(vec![7.0, 8.0, 9.0, 9.0], 1, 4))
);
#[cfg(feature = "standard_compiler")]
bytecode_test!(
    bytecode_math_sqrt,
    "+> math\nmath/sqrt(9)",
    Value::F64(Ref::new(3.0))
);
bytecode_test!(
    bytecode_define_set,
    "x := {1 2 3 4}",
    Value::Set(Ref::new(MechSet::from_vec(vec![
        Value::F64(Ref::new(1.0)),
        Value::F64(Ref::new(2.0)),
        Value::F64(Ref::new(3.0)),
        Value::F64(Ref::new(4.0))
    ])))
);
bytecode_test!(
    bytecode_set,
    "{1 2 3 3 4}",
    Value::Set(Ref::new(MechSet::from_vec(vec![
        Value::F64(Ref::new(1.0)),
        Value::F64(Ref::new(2.0)),
        Value::F64(Ref::new(3.0)),
        Value::F64(Ref::new(4.0))
    ])))
);
#[cfg(feature = "standard_compiler")]
bytecode_test!(
    bytecode_math_abs,
    "+> math\nmath/abs(-10)",
    Value::F64(Ref::new(10.0))
);
bytecode_test!(
    bytecode_define_table,
    "x := |x<f64> y<u64>| 1 2 | 3 4 |",
    Value::Table(Ref::new(MechTable::new_table(
        vec!["x".to_string(), "y".to_string()],
        vec![ValueKind::F64, ValueKind::U64],
        vec![
            vec![Value::F64(Ref::new(1.0)), Value::F64(Ref::new(3.0))],
            vec![Value::U64(Ref::new(2_u64)), Value::U64(Ref::new(4_u64))],
        ],
    )))
);
bytecode_test!(
    bytecode_define_table_eq,
    "x := |x<f64> y<bool>| 1 true | 3 false |; y := |x<f64> y<bool>| 1 true | 3 false |; x == y",
    Value::Bool(Ref::new(true))
);
//bytecode_test!(bytecode_set_union, "x := {1 2 3}; y := {3 4 5}; x ∪ y", Value::Set(Ref::new(MechSet::from_vec(vec![Value::F64(Ref::new(1.0)),Value::F64(Ref::new(2.0)),Value::F64(Ref::new(3.0)),Value::F64(Ref::new(4.0)),Value::F64(Ref::new(5.0))]))));
fn compile_bytecode_strict_compare_returns_error_without_panic(
    source: &str,
    expected_message: &str,
) {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut prgrm = MechProgram::new(MechProgramConfig {
            name: "strict_compare_no_panic".to_string(),
            environment: MechProgramEnvironment::default(),
        });
        prgrm.run_string(source).unwrap();
        prgrm.compile_bytecode()
    }));
    let compile_result = result.expect("strict compare bytecode compilation should not panic");
    let error =
        compile_result.expect_err("strict compare bytecode compilation should return an error");
    assert!(
        error.full_chain_message().contains(expected_message),
        "unexpected error: {:?}",
        error
    );
}

#[test]
fn bytecode_strict_equality_returns_error_without_panic() {
    compile_bytecode_strict_compare_returns_error_without_panic(
        "x := 1 === 1",
        "dynamic strict equality",
    );
}

#[test]
fn bytecode_strict_inequality_returns_error_without_panic() {
    compile_bytecode_strict_compare_returns_error_without_panic(
        "x := 1 !== 2",
        "dynamic strict inequality",
    );
}

#[test]
fn bytecode_static_bound_string_access_compiles() {
    let mut prgrm = MechProgram::new(MechProgramConfig {
        name: "bytecode_static_bound_string_access_compiles".to_string(),
        environment: MechProgramEnvironment::default(),
    });
    prgrm.run_string("s := \"abc\"\nfirst := s[1]\n").unwrap();
    prgrm.compile_bytecode().unwrap();
}

#[test]
fn bytecode_literal_string_immutable_index_symbol_compiles() {
    let mut prgrm = MechProgram::new(MechProgramConfig {
        name: "bytecode_literal_string_immutable_index_symbol_compiles".to_string(),
        environment: MechProgramEnvironment::default(),
    });
    prgrm
        .run_string("s := \"abc\"\ni := 1\nfirst := s[i]\n")
        .unwrap();
    prgrm.compile_bytecode().unwrap();
}

#[test]
fn bytecode_string_access_constant_string_aliases_compile() {
    let mut prgrm = MechProgram::new(MechProgramConfig {
        name: "bytecode_string_access_constant_string_aliases_compile".to_string(),
        environment: MechProgramEnvironment::default(),
    });
    prgrm
        .run_string("s := \"abc\"\na := s\nb := a\nc := b\nd := c\nfirst := s[1]\n")
        .unwrap();
    prgrm.compile_bytecode().unwrap();
}

#[test]
fn bytecode_string_access_constant_index_aliases_compile() {
    let mut prgrm = MechProgram::new(MechProgramConfig {
        name: "bytecode_string_access_constant_index_aliases_compile".to_string(),
        environment: MechProgramEnvironment::default(),
    });
    prgrm
        .run_string("i := 1\na := i\nb := a\ns := \"abc\"\nfirst := s[i]\n")
        .unwrap();
    prgrm.compile_bytecode().unwrap();
}

#[test]
fn bytecode_live_computed_string_index_rejects_stale_constant_compile() {
    let mut prgrm = MechProgram::new(MechProgramConfig {
        name: "bytecode_live_computed_string_index_rejects_stale_constant_compile".to_string(),
        environment: MechProgramEnvironment::default(),
    });
    prgrm
        .run_string("~p := 1\ni := p + 1\ns := \"abc\"\nfirst := s[i]\n")
        .unwrap();
    let error = format!("{:?}", prgrm.compile_bytecode().unwrap_err());
    assert!(error.contains("dynamic string scalar access is not bytecode-compilable yet") || error.contains("string scalar access cannot be bytecode-compiled because its source or index may be live"), "got {error}");
}

#[test]
fn bytecode_constant_string_plan_output_with_unrelated_mutable_symbol_compiles() {
    let mut prgrm = MechProgram::new(MechProgramConfig {
        name: "bytecode_constant_string_plan_output_with_unrelated_mutable_symbol_compiles"
            .to_string(),
        environment: MechProgramEnvironment::default(),
    });
    prgrm
        .run_string(
            r#"~unused := 0
s := "a" + "bc"
first := s[1]
"#,
        )
        .unwrap();
    prgrm.compile_bytecode().unwrap();
}

#[test]
fn bytecode_string_plan_output_depending_on_mutable_symbol_rejects() {
    let mut prgrm = MechProgram::new(MechProgramConfig {
        name: "bytecode_string_plan_output_depending_on_mutable_symbol_rejects".to_string(),
        environment: MechProgramEnvironment::default(),
    });
    prgrm
        .run_string(
            r#"~p := "a"
s := p + "bc"
first := s[1]
"#,
        )
        .unwrap();
    let error = format!("{:?}", prgrm.compile_bytecode().unwrap_err());
    assert!(error.contains("dynamic string scalar access is not bytecode-compilable yet") || error.contains("string scalar access cannot be bytecode-compiled because its source or index may be live"), "got {error}");
}

#[test]
fn bytecode_constant_index_plan_output_with_unrelated_mutable_symbol_compiles() {
    let mut prgrm = MechProgram::new(MechProgramConfig {
        name: "bytecode_constant_index_plan_output_with_unrelated_mutable_symbol_compiles"
            .to_string(),
        environment: MechProgramEnvironment::default(),
    });
    prgrm
        .run_string(
            r#"~unused := 0
i := 1 + 0
s := "abc"
first := s[i]
"#,
        )
        .unwrap();
    prgrm.compile_bytecode().unwrap();
}

#[test]
fn bytecode_index_plan_output_depending_on_mutable_symbol_rejects() {
    let mut prgrm = MechProgram::new(MechProgramConfig {
        name: "bytecode_index_plan_output_depending_on_mutable_symbol_rejects".to_string(),
        environment: MechProgramEnvironment::default(),
    });
    prgrm
        .run_string(
            r#"~i0 := 1
i := i0 + 0
s := "abc"
first := s[i]
"#,
        )
        .unwrap();
    let error = format!("{:?}", prgrm.compile_bytecode().unwrap_err());
    assert!(error.contains("dynamic string scalar access is not bytecode-compilable yet") || error.contains("string scalar access cannot be bytecode-compiled because its source or index may be live"), "got {error}");
}

#[test]
fn bytecode_live_markers_do_not_leak_between_programs_for_string_source() {
    let mut live = MechProgram::new(MechProgramConfig {
        name: "bytecode_live_markers_do_not_leak_between_programs_live".to_string(),
        environment: MechProgramEnvironment::default(),
    });
    live.run_string(
        r#"~p := "a"
s := p + "bc"
first := s[1]
"#,
    )
    .unwrap();
    let error = format!("{:?}", live.compile_bytecode().unwrap_err());
    assert!(error.contains("dynamic string scalar access is not bytecode-compilable yet") || error.contains("string scalar access cannot be bytecode-compiled because its source or index may be live"), "got {error}");

    let mut constant = MechProgram::new(MechProgramConfig {
        name: "bytecode_live_markers_do_not_leak_between_programs_constant".to_string(),
        environment: MechProgramEnvironment::default(),
    });
    constant
        .run_string(
            r#"s := "abc"
first := s[1]
"#,
        )
        .unwrap();
    constant.compile_bytecode().unwrap();
}

#[test]
fn bytecode_live_markers_do_not_leak_between_programs_for_index() {
    let mut live = MechProgram::new(MechProgramConfig {
        name: "bytecode_live_markers_do_not_leak_between_programs_index_live".to_string(),
        environment: MechProgramEnvironment::default(),
    });
    live.run_string(
        r#"~i0 := 1
i := i0 + 0
s := "abc"
first := s[i]
"#,
    )
    .unwrap();
    let error = format!("{:?}", live.compile_bytecode().unwrap_err());
    assert!(error.contains("dynamic string scalar access is not bytecode-compilable yet") || error.contains("string scalar access cannot be bytecode-compiled because its source or index may be live"), "got {error}");

    let mut constant = MechProgram::new(MechProgramConfig {
        name: "bytecode_live_markers_do_not_leak_between_programs_index_constant".to_string(),
        environment: MechProgramEnvironment::default(),
    });
    constant
        .run_string(
            r#"i := 1 + 0
s := "abc"
first := s[i]
"#,
        )
        .unwrap();
    constant.compile_bytecode().unwrap();
}

#[test]
fn bytecode_inline_mutable_string_source_access_rejects() {
    let mut prgrm = MechProgram::new(MechProgramConfig {
        name: "bytecode_inline_mutable_string_source_access_rejects".to_string(),
        environment: MechProgramEnvironment::default(),
    });
    prgrm
        .run_string(
            r#"~p := "a"
s := p + "bc"
first := s[1]
"#,
        )
        .unwrap();
    let error = format!("{:?}", prgrm.compile_bytecode().unwrap_err());
    assert!(error.contains("dynamic string scalar access is not bytecode-compilable yet") || error.contains("string scalar access cannot be bytecode-compiled because its source or index may be live"), "got {error}");
}

#[test]
fn bytecode_inline_mutable_index_access_rejects() {
    let mut prgrm = MechProgram::new(MechProgramConfig {
        name: "bytecode_inline_mutable_index_access_rejects".to_string(),
        environment: MechProgramEnvironment::default(),
    });
    prgrm
        .run_string(
            r#"~i0 := 1
s := "abc"
first := s[i0 + 1]
"#,
        )
        .unwrap();
    let error = format!("{:?}", prgrm.compile_bytecode().unwrap_err());
    assert!(error.contains("dynamic string scalar access is not bytecode-compilable yet") || error.contains("string scalar access cannot be bytecode-compiled because its source or index may be live"), "got {error}");
}

#[test]
fn bytecode_inline_constant_string_source_access_compiles() {
    let mut prgrm = MechProgram::new(MechProgramConfig {
        name: "bytecode_inline_constant_string_source_access_compiles".to_string(),
        environment: MechProgramEnvironment::default(),
    });
    prgrm
        .run_string(
            r#"s := "a" + "bc"
first := s[1]
"#,
        )
        .unwrap();
    prgrm.compile_bytecode().unwrap();
}

#[test]
fn bytecode_inline_constant_index_access_compiles() {
    let mut prgrm = MechProgram::new(MechProgramConfig {
        name: "bytecode_inline_constant_index_access_compiles".to_string(),
        environment: MechProgramEnvironment::default(),
    });
    prgrm
        .run_string(
            r#"s := "abc"
first := s[1 + 0]
"#,
        )
        .unwrap();
    prgrm.compile_bytecode().unwrap();
}

#[test]
fn bytecode_live_direct_string_access_rejects_stale_constant_compile() {
    let mut prgrm = MechProgram::new(MechProgramConfig {
        name: "bytecode_live_direct_string_access_rejects_stale_constant_compile".to_string(),
        environment: MechProgramEnvironment::default(),
    });
    prgrm
        .run_string("~p := \"a\"\ns := p + \"bc\"\nch := s[1]\n")
        .unwrap();
    let error = format!("{:?}", prgrm.compile_bytecode().unwrap_err());
    assert!(error.contains("string scalar access cannot be bytecode-compiled because its source or index may be live"), "got {error}");
}

#[test]
fn bytecode_dynamic_string_index_rejects_stale_constant_compile() {
    let mut prgrm = MechProgram::new(MechProgramConfig {
        name: "bytecode_dynamic_string_index_rejects_stale_constant_compile".to_string(),
        environment: MechProgramEnvironment::default(),
    });
    prgrm
        .run_string(
            r#"s := "abc"
~i := 1
first := s[i]
"#,
        )
        .unwrap();
    let error = format!("{:?}", prgrm.compile_bytecode().unwrap_err());
    assert!(
        error.contains("dynamic string scalar access is not bytecode-compilable yet"),
        "got {error}"
    );
}

#[test]
fn bytecode_dynamic_string_access_rejects_stale_constant_compile() {
    let mut prgrm = MechProgram::new(MechProgramConfig {
        name: "bytecode_dynamic_string_access_rejects_stale_constant_compile".to_string(),
        environment: MechProgramEnvironment::default(),
    });
    prgrm.run_string("~s := \"abc\"\nfirst := s[1]\n").unwrap();
    let error = format!("{:?}", prgrm.compile_bytecode().unwrap_err());
    assert!(
        error.contains("dynamic string scalar access is not bytecode-compilable yet"),
        "got {error}"
    );
}

#[test]
fn bytecode_constant_string_access_after_live_statement_still_compiles() {
    let code = r#"
~i := 1
i == 1
s := "abc"
ch := s[1]
"#;

    let mut prgrm = MechProgram::new(MechProgramConfig {
        name: "bytecode_constant_string_access_after_live_statement_still_compiles".to_string(),
        environment: MechProgramEnvironment::default(),
    });

    prgrm.run_string(code).unwrap();
    prgrm.compile_bytecode().unwrap_or_else(|err| {
        panic!(
            "constant string access should compile after unrelated live statement: {:?}",
            err
        )
    });
}

#[cfg(feature = "u8")]
#[test]
fn bytecode_rejects_live_u8_string_index_dependency() {
    let code = r#"
~i0 := 1<u8>
i := i0 + 0<u8>
s := "abc"
ch := s[i]
"#;

    let mut prgrm = MechProgram::new(MechProgramConfig {
        name: "bytecode_rejects_live_u8_string_index_dependency".to_string(),
        environment: MechProgramEnvironment::default(),
    });

    prgrm.run_string(code).unwrap();
    let err = prgrm.compile_bytecode();
    assert!(
        err.is_err(),
        "live u8-derived string index must not compile as a frozen constant"
    );
}

#[test]
fn bytecode_rejects_live_function_input_index_result_dependency() {
    let code = r#"
~i0 := 1
id(ix<f64>) = out<f64> :=
out := ix + 0.

i := id(i0 + 0)
s := "abc"
ch := s[i]
"#;

    let mut prgrm = MechProgram::new(MechProgramConfig {
        name: "bytecode_rejects_live_function_input_index_result_dependency".to_string(),
        environment: MechProgramEnvironment::default(),
    });

    prgrm.run_string(code).unwrap();
    let err = prgrm.compile_bytecode();
    assert!(
        err.is_err(),
        "live function-input-derived index result must not compile as a frozen string access"
    );
}

fn add_assign_topology_source_program() -> EngineMechProgram {
    EngineMechProgram::with_function_catalog(
        MechProgramConfig::default(),
        mech::stdlib::source_catalog(),
    )
}

fn add_assign_topology_runtime_program() -> EngineMechProgram {
    EngineMechProgram::with_function_catalog(
        MechProgramConfig::default(),
        mech::stdlib::runtime_catalog(),
    )
}

fn add_assign_topology_symbol(interpreter: &mech_engine::Interpreter, name: &str) -> Value {
    interpreter
        .symbols()
        .borrow()
        .get(hash_str(name))
        .unwrap_or_else(|| panic!("missing symbol {name}"))
        .borrow()
        .clone()
}

fn add_assign_topology_root_cell(value: &Value) -> ReactiveCellId {
    let cells = value.reactive_root_cell_ids();
    assert_eq!(cells.len(), 1);
    cells[0]
}

fn add_assign_topology_register_node_id_for_output(
    interpreter: &mech_engine::Interpreter,
    output_cell: ReactiveCellId,
) -> ReactiveNodeId {
    let plan = interpreter.plan();
    let plan = plan.borrow();
    let node_ids = plan
        .nodes
        .iter()
        .filter(|node| node.kind == ReactiveNodeKind::Register && node.outputs == vec![output_cell])
        .map(|node| node.id)
        .collect::<Vec<_>>();
    assert_eq!(node_ids.len(), 1);
    node_ids[0]
}

#[derive(Debug, PartialEq, Eq)]
struct AddAssignTopologyRegisterGraphShape {
    output_count: usize,
    input_kinds: Vec<ReactiveDependencyKind>,
    output_is_first_input: bool,
    source_is_second_input: bool,
    output_is_sampled_consumer: bool,
    output_is_reactive_consumer: bool,
    source_is_reactive_consumer: bool,
    source_is_sampled_consumer: bool,
}

fn add_assign_topology_distinct_graph_shape(
    interpreter: &mech_engine::Interpreter,
    target_name: &str,
    source_name: &str,
) -> AddAssignTopologyRegisterGraphShape {
    let target_cell =
        add_assign_topology_root_cell(&add_assign_topology_symbol(interpreter, target_name));
    let source_cell =
        add_assign_topology_root_cell(&add_assign_topology_symbol(interpreter, source_name));
    assert_ne!(target_cell, source_cell);
    let node_id = add_assign_topology_register_node_id_for_output(interpreter, target_cell);
    let plan = interpreter.plan();
    let plan = plan.borrow();
    let node = plan.node(node_id).unwrap();
    assert_eq!(node.kind, ReactiveNodeKind::Register);
    assert_eq!(node.outputs, vec![target_cell]);
    assert_eq!(node.inputs.len(), 2);
    assert_eq!(node.inputs[0].cell, target_cell);
    assert_eq!(node.inputs[0].kind, ReactiveDependencyKind::Sampled);
    assert_eq!(node.inputs[1].cell, source_cell);
    assert_eq!(node.inputs[1].kind, ReactiveDependencyKind::Reactive);
    AddAssignTopologyRegisterGraphShape {
        output_count: node.outputs.len(),
        input_kinds: node.inputs.iter().map(|input| input.kind).collect(),
        output_is_first_input: node.inputs[0].cell == target_cell,
        source_is_second_input: node.inputs[1].cell == source_cell,
        output_is_sampled_consumer: plan.sampled_consumers_for(target_cell).contains(&node_id),
        output_is_reactive_consumer: plan.reactive_consumers_for(target_cell).contains(&node_id),
        source_is_reactive_consumer: plan.reactive_consumers_for(source_cell).contains(&node_id),
        source_is_sampled_consumer: plan.sampled_consumers_for(source_cell).contains(&node_id),
    }
}

fn add_assign_topology_decoded_graph_shape(
    interpreter: &mech_engine::Interpreter,
    output: &Value,
) -> AddAssignTopologyRegisterGraphShape {
    let resolved_output = match output {
        Value::MutableReference(reference) => reference.borrow().clone(),
        other => other.clone(),
    };
    let output_cell = add_assign_topology_root_cell(&resolved_output);
    let node_id = add_assign_topology_register_node_id_for_output(interpreter, output_cell);
    let plan = interpreter.plan();
    let plan = plan.borrow();
    let node = plan.node(node_id).unwrap();
    assert_eq!(node.kind, ReactiveNodeKind::Register);
    assert_eq!(node.outputs, vec![output_cell]);
    assert_eq!(node.outputs.len(), 1);
    assert_eq!(node.inputs.len(), 2);
    assert_eq!(node.inputs[0].cell, output_cell);
    assert_eq!(node.inputs[0].kind, ReactiveDependencyKind::Sampled);
    assert_ne!(node.inputs[1].cell, output_cell);
    assert_eq!(node.inputs[1].kind, ReactiveDependencyKind::Reactive);
    let source_cell = node.inputs[1].cell;
    AddAssignTopologyRegisterGraphShape {
        output_count: node.outputs.len(),
        input_kinds: node.inputs.iter().map(|input| input.kind).collect(),
        output_is_first_input: node.inputs[0].cell == output_cell,
        source_is_second_input: node.inputs[1].cell == source_cell,
        output_is_sampled_consumer: plan.sampled_consumers_for(output_cell).contains(&node_id),
        output_is_reactive_consumer: plan.reactive_consumers_for(output_cell).contains(&node_id),
        source_is_reactive_consumer: plan.reactive_consumers_for(source_cell).contains(&node_id),
        source_is_sampled_consumer: plan.sampled_consumers_for(source_cell).contains(&node_id),
    }
}

fn add_assign_topology_expected_graph_shape() -> AddAssignTopologyRegisterGraphShape {
    AddAssignTopologyRegisterGraphShape {
        output_count: 1,
        input_kinds: vec![
            ReactiveDependencyKind::Sampled,
            ReactiveDependencyKind::Reactive,
        ],
        output_is_first_input: true,
        source_is_second_input: true,
        output_is_sampled_consumer: true,
        output_is_reactive_consumer: false,
        source_is_reactive_consumer: true,
        source_is_sampled_consumer: false,
    }
}

fn add_assign_topology_register(
    interpreter: &mech_engine::Interpreter,
    output_cell: ReactiveCellId,
) -> ReactiveNodeId {
    let plan = interpreter.plan();
    let plan = plan.borrow();
    let node_ids = plan
        .nodes
        .iter()
        .filter(|node| {
            node.kind == ReactiveNodeKind::Register && node.outputs.contains(&output_cell)
        })
        .map(|node| node.id)
        .collect::<Vec<_>>();
    assert_eq!(node_ids.len(), 1);
    node_ids[0]
}

#[test]
fn decoded_whole_add_assignment_matches_source_graph() -> MResult<()> {
    let code = "~x := 1.0; y := 2.0; x += y; x";
    let mut source = add_assign_topology_source_program();
    let source_output = source.run_string(code)?;
    let bytecode = source.compile_bytecode()?;
    let mut decoded = add_assign_topology_runtime_program();
    let decoded_output = decoded.run_bytecode(&bytecode)?;

    assert_eq!(*source_output.as_f64().unwrap().borrow(), 3.0);
    assert_eq!(*decoded_output.as_f64().unwrap().borrow(), 3.0);
    let source_shape = add_assign_topology_distinct_graph_shape(source.interpreter(), "x", "y");
    let decoded_shape =
        add_assign_topology_decoded_graph_shape(decoded.interpreter(), &decoded_output);
    assert_eq!(source_shape, add_assign_topology_expected_graph_shape());
    assert_eq!(decoded_shape, add_assign_topology_expected_graph_shape());
    assert_eq!(source_shape, decoded_shape);
    Ok(())
}

#[test]
fn decoded_register_commit_add_assignment_uses_staging() -> MResult<()> {
    let code = "~x := 1.0\ny := 2.0\nx += y\nx";
    let mut source = add_assign_topology_source_program();
    let source_output = source.run_string(code)?;
    let bytecode = source.compile_bytecode()?;
    let mut decoded = add_assign_topology_runtime_program();
    let decoded_output = decoded.run_bytecode(&bytecode)?;

    assert_eq!(*source_output.as_f64().unwrap().borrow(), 3.0);
    assert_eq!(*decoded_output.as_f64().unwrap().borrow(), 3.0);
    let output_cell = add_assign_topology_root_cell(&decoded_output);
    let register_node = add_assign_topology_register(decoded.interpreter(), output_cell);
    let source_cell = {
        let plan = decoded.interpreter().plan();
        let plan = plan.borrow();
        let node = plan.node(register_node).unwrap();
        let dependencies = node
            .inputs
            .iter()
            .filter(|dependency| {
                dependency.kind == ReactiveDependencyKind::Reactive
                    && dependency.cell != output_cell
            })
            .collect::<Vec<_>>();
        assert_eq!(
            dependencies.len(),
            1,
            "decoded register must have exactly one distinct reactive source",
        );
        dependencies[0].cell
    };
    let scheduling = decoded
        .interpreter()
        .plan()
        .solve_dirty_cells(&[source_cell])?;
    assert_eq!(scheduling.pending_register_nodes, vec![register_node]);
    let commit = decoded
        .interpreter()
        .plan()
        .commit_pending_registers(&scheduling.pending_register_nodes)?;
    assert_eq!(commit.staged_nodes, vec![register_node]);
    assert_eq!(commit.committed_nodes, vec![register_node]);
    assert_eq!(commit.dirty_cells, vec![output_cell]);
    assert_eq!(*decoded_output.as_f64().unwrap().borrow(), 5.0);
    Ok(())
}

#[test]
fn decoded_reactive_turn_reuses_compiled_plan() -> MResult<()> {
    let code = "~x := 1.0\ny := 2.0\nx += y\nz := x + 1.0\nz";
    let mut source = add_assign_topology_source_program();
    let source_output = source.run_string(code)?;
    let bytecode = source.compile_bytecode()?;
    let mut decoded = add_assign_topology_runtime_program();
    let decoded_output = decoded.run_bytecode(&bytecode)?;

    assert_eq!(*source_output.as_f64().unwrap().borrow(), 4.0);
    assert_eq!(*decoded_output.as_f64().unwrap().borrow(), 4.0);
    let z_cell = add_assign_topology_root_cell(&decoded_output);
    let (x_register, x_ref, x_cell, source_cell, x_consumers, plan_length, node_ids, output_cells) = {
        let plan = decoded.interpreter().plan();
        let plan = plan.borrow();
        let registers = plan
            .nodes
            .iter()
            .filter(|node| node.kind == ReactiveNodeKind::Register)
            .collect::<Vec<_>>();
        assert_eq!(registers.len(), 1);
        let x_register = registers[0].id;
        let x_output = plan.node(x_register).unwrap().function.out();
        let x_ref = x_output.as_f64().unwrap().clone();
        let x_cell = add_assign_topology_root_cell(&x_output);
        let source_dependencies = plan
            .node(x_register)
            .unwrap()
            .inputs
            .iter()
            .filter(|dependency| {
                dependency.kind == ReactiveDependencyKind::Reactive && dependency.cell != x_cell
            })
            .collect::<Vec<_>>();
        assert_eq!(source_dependencies.len(), 1);
        let x_consumers = plan.reactive_consumers_for(x_cell).to_vec();
        assert!(!x_consumers.is_empty());
        (
            x_register,
            x_ref,
            x_cell,
            source_dependencies[0].cell,
            x_consumers,
            plan.len(),
            plan.nodes.iter().map(|node| node.id).collect::<Vec<_>>(),
            plan.nodes
                .iter()
                .map(|node| node.outputs.clone())
                .collect::<Vec<_>>(),
        )
    };
    assert_eq!(*x_ref.borrow(), 3.0);
    let mut turn_state = ReactiveTurnState::default();
    for (expected_x, expected_z) in [(5.0, 6.0), (7.0, 8.0)] {
        let outcome = decoded
            .interpreter()
            .plan()
            .advance_reactive_turn(&mut turn_state, &[source_cell])?;
        assert_eq!(
            outcome.before_commit.pending_register_nodes,
            vec![x_register]
        );
        assert_eq!(outcome.register_commit.staged_nodes, vec![x_register]);
        assert_eq!(outcome.register_commit.committed_nodes, vec![x_register]);
        assert_eq!(outcome.register_commit.dirty_cells, vec![x_cell]);
        for node_id in &x_consumers {
            assert!(outcome.after_commit.executed_nodes.contains(node_id));
        }
        let executed_z_nodes = {
            let plan = decoded.interpreter().plan();
            let plan = plan.borrow();
            outcome
                .after_commit
                .executed_nodes
                .iter()
                .copied()
                .filter(|node_id| plan.node(*node_id).unwrap().outputs.contains(&z_cell))
                .collect::<Vec<_>>()
        };
        assert!(!executed_z_nodes.is_empty());
        assert_eq!(*x_ref.borrow(), expected_x);
        assert_eq!(*decoded_output.as_f64().unwrap().borrow(), expected_z);
        assert!(turn_state.pending_register_nodes.is_empty());
        let plan = decoded.interpreter().plan();
        let plan = plan.borrow();
        assert_eq!(plan.len(), plan_length);
        assert_eq!(
            plan.nodes.iter().map(|node| node.id).collect::<Vec<_>>(),
            node_ids,
        );
        assert_eq!(
            plan.nodes
                .iter()
                .map(|node| node.outputs.clone())
                .collect::<Vec<_>>(),
            output_cells,
        );
    }
    Ok(())
}

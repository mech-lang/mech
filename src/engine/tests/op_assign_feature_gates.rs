#[cfg(any(feature = "math_mul_assign", feature = "math_div_assign",))]
use mech_engine::Interpreter;

#[cfg(any(feature = "math_mul_assign", feature = "math_div_assign",))]
fn evaluate_f64(source: &str) -> f64 {
    let tree = mech_syntax::parser::parse(source).unwrap();
    let mut interpreter = Interpreter::new_with_full_stdlib(0);
    let output = interpreter.interpret(&tree).unwrap();
    *output.as_f64().unwrap().borrow()
}

#[cfg(feature = "math_mul_assign")]
#[test]
fn whole_mul_assignment_uses_mul_feature_only() {
    assert_eq!(
        evaluate_f64(
            "~x := 6.0\n\
       y := 3.0\n\
       x *= y\n\
       x",
        ),
        18.0,
    );
}

#[cfg(feature = "math_div_assign")]
#[test]
fn whole_div_assignment_uses_div_feature_only() {
    assert_eq!(
        evaluate_f64(
            "~x := 6.0\n\
       y := 3.0\n\
       x /= y\n\
       x",
        ),
        2.0,
    );
}

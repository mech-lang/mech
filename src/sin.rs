trigonometry_rad_vv!(
  MathSinRadVV,    // MechFunction
  MathSin,         // MechFunctionCompiler
  sinf,            // libm function call
  math_sin_reg,    // registry function name
  math_sin,        // export name
  "math/sin",      // function name in Mech
);

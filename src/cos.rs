trigonometry_rad_vv!(
  MathCosRadVV,    // MechFunction
  MathCos,         // MechFunctionCompiler
  cosf,            // libm function call
  math_cos_reg,    // registry function name
  math_cos,        // export name
  "math/cos",      // function name in Mech
);

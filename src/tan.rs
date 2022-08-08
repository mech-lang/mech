trigonometry_rad_vv!(
  MathTanRadVV,    // MechFunction
  MathTan,         // MechFunctionCompiler
  tanf,            // libm function call
  math_tan_reg,    // registry function name
  math_tan,        // export name
  "math/tan",      // function name in Mech
);

pub use crate::*;

// The Standard Libray
// ----------------------------------------------------------------------------

// These macros are used by various libraries to generate function impl and
// match arms. They're gated on feature flags so although there's a lot of
// code here to account for all the different combinations, but only
// the relevant code will be compiled in any given build.

#[macro_export]
macro_rules! compile_nullop {
  ($out:expr, $ctx:ident, $feature_flag:expr) => {
    // allocate one register as an array
    let mut registers = [0];

    // Compile out
    let out_addr = $out.addr();
    let out_reg = $ctx.alloc_register_for_ptr(out_addr);
    let out_borrow = $out.borrow();
    let out_const_id = out_borrow.compile_const($ctx).unwrap();
    $ctx.emit_const_load(out_reg, out_const_id);
    registers[0] = out_reg;

    $ctx.features.insert($feature_flag);

    // Emit the operation
    $ctx.emit_nullop(
      hash_str(stringify!($struct_name)),
      registers[0],
    );

    return Ok(registers[0]);
  };
}


#[macro_export]
macro_rules! compile_unop {
  ($out:expr, $arg:expr, $ctx:ident, $feature_flag:expr) => {
    // allocate three registers as an array
    let mut registers = [0,0];

    // Compile out
    let out_addr = $out.addr();
    let out_reg = $ctx.alloc_register_for_ptr(out_addr);
    let out_borrow = $out.borrow();
    let out_const_id = out_borrow.compile_const($ctx).unwrap();
    $ctx.emit_const_load(out_reg, out_const_id);
    registers[0] = out_reg;
    
    // Compile arg
    let arg_addr = $arg.addr();
    let arg_reg = $ctx.alloc_register_for_ptr(arg_addr);
    let arg_borrow = $arg.borrow();
    let arg_const_id = arg_borrow.compile_const($ctx).unwrap();
    $ctx.emit_const_load(arg_reg, arg_const_id);
    registers[1] = arg_reg;
  
    $ctx.features.insert($feature_flag);

    // Emit the operation
    $ctx.emit_unop(
      hash_str(stringify!($struct_name)),
      registers[0],
      registers[1],
    );

    return Ok(registers[0]);
  };
}

#[macro_export]
macro_rules! compile_binop {
  ($out:expr, $arg1:expr, $arg2:expr, $ctx:ident, $feature_flag:expr) => {
    let mut registers = [0,0,0];

    let out_addr = $out.addr();
    let out_reg = $ctx.alloc_register_for_ptr(out_addr);
    let out_borrow = $out.borrow();
    let out_const_id = out_borrow.compile_const($ctx).unwrap();
    $ctx.emit_const_load(out_reg, out_const_id);
    registers[0] = out_reg;

    let lhs_addr = $arg1.addr();
    let lhs_reg = $ctx.alloc_register_for_ptr(lhs_addr);
    let lhs_borrow = $arg1.borrow();
    let lhs_const_id = lhs_borrow.compile_const($ctx).unwrap();
    $ctx.emit_const_load(lhs_reg, lhs_const_id);
    registers[1] = lhs_reg;

    let rhs_addr = $arg2.addr();
    let rhs_reg = $ctx.alloc_register_for_ptr(rhs_addr);
    let rhs_borrow = $arg2.borrow();
    let rhs_const_id = rhs_borrow.compile_const($ctx).unwrap();
    $ctx.emit_const_load(rhs_reg, rhs_const_id);
    registers[2] = rhs_reg;

    $ctx.features.insert($feature_flag);

    $ctx.emit_binop(
      hash_str(stringify!($struct_name)),
      registers[0],
      registers[1],
      registers[2],
    );

    return Ok(registers[0])
  };
}

#[macro_export]
macro_rules! compile_ternop {
  ($out:expr, $arg1:expr, $arg2:expr, $arg3:expr, $ctx:ident, $feature_flag:expr) => {
    let mut registers = [0,0,0,0];

    let out_addr = $out.addr();
    let out_reg = $ctx.alloc_register_for_ptr(out_addr);
    let out_borrow = $out.borrow();
    let out_const_id = out_borrow.compile_const($ctx).unwrap();
    $ctx.emit_const_load(out_reg, out_const_id);
    registers[0] = out_reg;

    let lhs_addr = $arg1.addr();
    let lhs_reg = $ctx.alloc_register_for_ptr(lhs_addr);
    let lhs_borrow = $arg1.borrow();
    let lhs_const_id = lhs_borrow.compile_const($ctx).unwrap();
    $ctx.emit_const_load(lhs_reg, lhs_const_id);
    registers[1] = lhs_reg;

    let mid_addr = $arg2.addr();
    let mid_reg = $ctx.alloc_register_for_ptr(mid_addr);
    let mid_borrow = $arg2.borrow();
    let mid_const_id = mid_borrow.compile_const($ctx).unwrap();
    $ctx.emit_const_load(mid_reg, mid_const_id);
    registers[2] = mid_reg;

    let rhs_addr = $arg3.addr();
    let rhs_reg = $ctx.alloc_register_for_ptr(rhs_addr);
    let rhs_borrow = $arg3.borrow();
    let rhs_const_id = rhs_borrow.compile_const($ctx).unwrap();
    $ctx.emit_const_load(rhs_reg, rhs_const_id);
    registers[3] = rhs_reg;

    $ctx.features.insert($feature_flag);

    $ctx.emit_ternop(
      hash_str(stringify!($struct_name)),
      registers[0],
      registers[1],
      registers[2],
      registers[3],
    );

    return Ok(registers[0])
  };
}

#[macro_export]
macro_rules! compile_quadop {
  ($out:expr, $arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr, $ctx:ident, $feature_flag:expr) => {
    let mut registers = [0,0,0,0,0];

    let out_addr = $out.addr();
    let out_reg = $ctx.alloc_register_for_ptr(out_addr);
    let out_borrow = $out.borrow();
    let out_const_id = out_borrow.compile_const($ctx).unwrap();
    $ctx.emit_const_load(out_reg, out_const_id);
    registers[0] = out_reg;

    let lhs_addr = $arg1.addr();
    let lhs_reg = $ctx.alloc_register_for_ptr(lhs_addr);
    let lhs_borrow = $arg1.borrow();
    let lhs_const_id = lhs_borrow.compile_const($ctx).unwrap();
    $ctx.emit_const_load(lhs_reg, lhs_const_id);
    registers[1] = lhs_reg;

    let mid1_addr = $arg2.addr();
    let mid1_reg = $ctx.alloc_register_for_ptr(mid1_addr);
    let mid1_borrow = $arg2.borrow();
    let mid1_const_id = mid1_borrow.compile_const($ctx).unwrap();
    $ctx.emit_const_load(mid1_reg, mid1_const_id);
    registers[2] = mid1_reg;

    let mid2_addr = $arg3.addr();
    let mid2_reg = $ctx.alloc_register_for_ptr(mid2_addr);
    let mid2_borrow = $arg3.borrow();
    let mid2_const_id = mid2_borrow.compile_const($ctx).unwrap();
    $ctx.emit_const_load(mid2_reg, mid2_const_id);
    registers[3] = mid2_reg;

    let rhs_addr = $arg4.addr();
    let rhs_reg = $ctx.alloc_register_for_ptr(rhs_addr);
    let rhs_borrow = $arg4.borrow();
    let rhs_const_id = rhs_borrow.compile_const($ctx).unwrap();
    $ctx.emit_const_load(rhs_reg, rhs_const_id);
    registers[4] = rhs_reg;

    $ctx.features.insert($feature_flag);

    $ctx.emit_quadop(
      hash_str(stringify!($struct_name)),
      registers[0],
      registers[1],
      registers[2],
      registers[3],
      registers[4],
    );
    return Ok(registers[0])
  };
}

#[macro_export]
macro_rules! impl_binop {
  ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $op:ident, $feature_flag:expr) => {
    #[derive(Debug)]
    pub struct $struct_name<T> {
      pub lhs: Ref<$arg1_type>,
      pub rhs: Ref<$arg2_type>,
      pub out: Ref<$out_type>,
    }
    impl<T> MechFunctionImpl for $struct_name<T>
    where
      T: Copy + Debug + Display + Clone + Sync + Send + 'static + 
      PartialEq + PartialOrd +
      Add<Output = T> + AddAssign +
      Sub<Output = T> + SubAssign +
      Mul<Output = T> + MulAssign +
      Div<Output = T> + DivAssign +
      Zero + One,
      Ref<$out_type>: ToValue
    {
      fn solve(&self) {
          let lhs_ptr = self.lhs.as_ptr();
          let rhs_ptr = self.rhs.as_ptr();
          let out_ptr = self.out.as_mut_ptr();
          $op!(lhs_ptr,rhs_ptr,out_ptr);
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
    }   
    #[cfg(feature = "compiler")]
    impl<T> MechFunctionCompiler for $struct_name<T> 
    where
      T: ConstElem + CompileConst
    {
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        compile_binop!(self.out, self.lhs, self.rhs, ctx, $feature_flag);
      }
    }};}

#[macro_export]  
macro_rules! impl_unop {
  ($struct_name:ident, $arg_type:ty, $out_type:ty, $op:ident, $feature_flag:expr) => {
    #[derive(Debug)]
    struct $struct_name {
      arg: Ref<$arg_type>,
      out: Ref<$out_type>,
    }
    impl MechFunctionImpl for $struct_name {
      fn solve(&self) {
        let arg_ptr = self.arg.as_ptr();
        let out_ptr = self.out.as_mut_ptr();
        $op!(arg_ptr,out_ptr);
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
    }
    #[cfg(feature = "compiler")]
    impl MechFunctionCompiler for $struct_name {
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        compile_unop!(self.out, self.arg, ctx, $feature_flag);
      }
    }};} 

#[macro_export]
macro_rules! impl_fxns {
  ($lib:ident, $in:ident, $out:ident, $op:ident) => {
    paste!{
      // Scalar
      $op!([<$lib SS>], $in, $in, $out, [<$lib:lower _op>], FeatureFlag::Builtin(FeatureKind::$lib));
      // Scalar Matrix
      #[cfg(feature = "matrix1")]
      $op!([<$lib SM1>], $in, Matrix1<$in>, Matrix1<$out>,[<$lib:lower _scalar_rhs_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(feature = "matrix2")]
      $op!([<$lib SM2>], $in, Matrix2<$in>, Matrix2<$out>,[<$lib:lower _scalar_rhs_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(feature = "matrix3")]
      $op!([<$lib SM3>], $in, Matrix3<$in>, Matrix3<$out>,[<$lib:lower _scalar_rhs_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      
      #[cfg(feature = "matrix4")]
      $op!([<$lib SM4>], $in, Matrix4<$in>, Matrix4<$out>,[<$lib:lower _scalar_rhs_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(feature = "matrix2x3")]
      $op!([<$lib SM2x3>], $in, Matrix2x3<$in>, Matrix2x3<$out>,[<$lib:lower _scalar_rhs_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(feature = "matrix3x2")]
      $op!([<$lib SM3x2>], $in, Matrix3x2<$in>, Matrix3x2<$out>,[<$lib:lower _scalar_rhs_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(feature = "matrixd")]
      $op!([<$lib SMD>], $in, DMatrix<$in>, DMatrix<$out>,[<$lib:lower _scalar_rhs_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      // Scalar Row
      #[cfg(feature = "row_vector2")]
      $op!([<$lib SR2>], $in, RowVector2<$in>, RowVector2<$out>,[<$lib:lower _scalar_rhs_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(feature = "row_vector3")]
      $op!([<$lib SR3>], $in, RowVector3<$in>, RowVector3<$out>,[<$lib:lower _scalar_rhs_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(feature = "row_vector4")]
      $op!([<$lib SR4>], $in, RowVector4<$in>, RowVector4<$out>,[<$lib:lower _scalar_rhs_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(feature = "row_vectord")]
      $op!([<$lib SRD>], $in, RowDVector<$in>, RowDVector<$out>,[<$lib:lower _scalar_rhs_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      // Scalar Vector
      #[cfg(feature = "vector2")]
      $op!([<$lib SV2>], $in, Vector2<$in>, Vector2<$out>,[<$lib:lower _scalar_rhs_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(feature = "vector3")]
      $op!([<$lib SV3>], $in, Vector3<$in>, Vector3<$out>,[<$lib:lower _scalar_rhs_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(feature = "vector4")]
      $op!([<$lib SV4>], $in, Vector4<$in>, Vector4<$out>,[<$lib:lower _scalar_rhs_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(feature = "vectord")]
      $op!([<$lib SVD>], $in, DVector<$in>, DVector<$out>,[<$lib:lower _scalar_rhs_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      // Matrix Scalar
      #[cfg(feature = "matrix1")]
      $op!([<$lib M1S>], Matrix1<$in>, $in, Matrix1<$out>,[<$lib:lower _scalar_lhs_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(feature = "matrix2")]
      $op!([<$lib M2S>], Matrix2<$in>, $in, Matrix2<$out>,[<$lib:lower _scalar_lhs_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(feature = "matrix3")]
      $op!([<$lib M3S>], Matrix3<$in>, $in, Matrix3<$out>,[<$lib:lower _scalar_lhs_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(feature = "matrix4")]
      $op!([<$lib M4S>], Matrix4<$in>, $in, Matrix4<$out>,[<$lib:lower _scalar_lhs_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(feature = "matrix2x3")]
      $op!([<$lib M2x3S>], Matrix2x3<$in>, $in, Matrix2x3<$out>,[<$lib:lower _scalar_lhs_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(feature = "matrix3x2")]
      $op!([<$lib M3x2S>], Matrix3x2<$in>, $in, Matrix3x2<$out>,[<$lib:lower _scalar_lhs_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(feature = "matrixd")]
      $op!([<$lib MDS>], DMatrix<$in>, $in, DMatrix<$out>,[<$lib:lower _scalar_lhs_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      // Row Scalar
      #[cfg(feature = "row_vector2")]
      $op!([<$lib R2S>], RowVector2<$in>, $in, RowVector2<$out>,[<$lib:lower _scalar_lhs_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(feature = "row_vector3")]
      $op!([<$lib R3S>], RowVector3<$in>, $in, RowVector3<$out>,[<$lib:lower _scalar_lhs_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(feature = "row_vector4")]
      $op!([<$lib R4S>], RowVector4<$in>, $in, RowVector4<$out>,[<$lib:lower _scalar_lhs_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(feature = "row_vectord")]
      $op!([<$lib RDS>], RowDVector<$in>, $in, RowDVector<$out>,[<$lib:lower _scalar_lhs_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      // Vector Scalar
      #[cfg(feature = "vector2")]
      $op!([<$lib V2S>], Vector2<$in>, $in, Vector2<$out>,[<$lib:lower _scalar_lhs_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(feature = "vector3")]
      $op!([<$lib V3S>], Vector3<$in>, $in, Vector3<$out>,[<$lib:lower _scalar_lhs_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(feature = "vector4")]
      $op!([<$lib V4S>], Vector4<$in>, $in, Vector4<$out>,[<$lib:lower _scalar_lhs_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(feature = "vectord")]
      $op!([<$lib VDS>], DVector<$in>, $in, DVector<$out>,[<$lib:lower _scalar_lhs_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      // Matrix Matrix
      #[cfg(feature = "matrix1")]
      $op!([<$lib M1M1>], Matrix1<$in>, Matrix1<$in>, Matrix1<$out>, [<$lib:lower _vec_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(feature = "matrix2")]
      $op!([<$lib M2M2>], Matrix2<$in>, Matrix2<$in>, Matrix2<$out>, [<$lib:lower _vec_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(feature = "matrix3")]
      $op!([<$lib M3M3>], Matrix3<$in>, Matrix3<$in>, Matrix3<$out>, [<$lib:lower _vec_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(feature = "matrix4")]
      $op!([<$lib M4M4>], Matrix4<$in>, Matrix4<$in>, Matrix4<$out>, [<$lib:lower _vec_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(feature = "matrix2x3")]
      $op!([<$lib M2x3M2x3>], Matrix2x3<$in>, Matrix2x3<$in>, Matrix2x3<$out>, [<$lib:lower _vec_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(feature = "matrix3x2")]
      $op!([<$lib M3x2M3x2>], Matrix3x2<$in>, Matrix3x2<$in>, Matrix3x2<$out>, [<$lib:lower _vec_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(feature = "matrixd")]
      $op!([<$lib MDMD>], DMatrix<$in>, DMatrix<$in>, DMatrix<$out>, [<$lib:lower _vec_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      // Matrix Vector
      #[cfg(all(feature = "matrix2", feature = "vector2"))]
      $op!([<$lib M2V2>], Matrix2<$in>, Vector2<$in>, Matrix2<$out>, [<$lib:lower _mat_vec_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(all(feature = "matrix3", feature = "vector3"))]
      $op!([<$lib M3V3>], Matrix3<$in>, Vector3<$in>, Matrix3<$out>, [<$lib:lower _mat_vec_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(all(feature = "matrix4", feature = "vector4"))]
      $op!([<$lib M4V4>], Matrix4<$in>, Vector4<$in>, Matrix4<$out>, [<$lib:lower _mat_vec_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(all(feature = "matrix2x3", feature = "vector2"))]
      $op!([<$lib M2x3V2>], Matrix2x3<$in>, Vector2<$in>, Matrix2x3<$out>, [<$lib:lower _mat_vec_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(all(feature = "matrix3x2", feature = "vector3"))]
      $op!([<$lib M3x2V3>], Matrix3x2<$in>, Vector3<$in>, Matrix3x2<$out>, [<$lib:lower _mat_vec_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(all(feature = "matrixd", feature = "vectord"))]
      $op!([<$lib MDVD>], DMatrix<$in>, DVector<$in>, DMatrix<$out>, [<$lib:lower _mat_vec_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(all(feature = "matrixd", feature = "vector2"))]
      $op!([<$lib MDV2>], DMatrix<$in>, Vector2<$in>, DMatrix<$out>, [<$lib:lower _mat_vec_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(all(feature = "matrixd", feature = "vector3"))]
      $op!([<$lib MDV3>], DMatrix<$in>, Vector3<$in>, DMatrix<$out>, [<$lib:lower _mat_vec_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(all(feature = "matrixd", feature = "vector4"))]
      $op!([<$lib MDV4>], DMatrix<$in>, Vector4<$in>, DMatrix<$out>, [<$lib:lower _mat_vec_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      // Vector Matrix
      #[cfg(all(feature = "vector2", feature = "matrix2"))]
      $op!([<$lib V2M2>], Vector2<$in>, Matrix2<$in>, Matrix2<$out>, [<$lib:lower _vec_mat_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(all(feature = "vector3", feature = "matrix3"))]
      $op!([<$lib V3M3>], Vector3<$in>, Matrix3<$in>, Matrix3<$out>, [<$lib:lower _vec_mat_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(all(feature = "vector4", feature = "matrix4"))]
      $op!([<$lib V4M4>], Vector4<$in>, Matrix4<$in>, Matrix4<$out>, [<$lib:lower _vec_mat_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(all(feature = "vector2", feature = "matrix2x3"))]
      $op!([<$lib V2M2x3>], Vector2<$in>, Matrix2x3<$in>, Matrix2x3<$out>, [<$lib:lower _vec_mat_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(all(feature = "vector3", feature = "matrix3x2"))]
      $op!([<$lib V3M3x2>], Vector3<$in>, Matrix3x2<$in>, Matrix3x2<$out>, [<$lib:lower _vec_mat_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(all(feature = "vectord", feature = "matrixd"))]
      $op!([<$lib VDMD>], DVector<$in>, DMatrix<$in>, DMatrix<$out>, [<$lib:lower _vec_mat_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(all(feature = "vector2", feature = "matrixd"))]
      $op!([<$lib V2MD>], Vector2<$in>, DMatrix<$in>, DMatrix<$out>, [<$lib:lower _vec_mat_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(all(feature = "vector3", feature = "matrixd"))]
      $op!([<$lib V3MD>], Vector3<$in>, DMatrix<$in>, DMatrix<$out>, [<$lib:lower _vec_mat_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(all(feature = "vector4", feature = "matrixd"))]
      $op!([<$lib V4MD>], Vector4<$in>, DMatrix<$in>, DMatrix<$out>, [<$lib:lower _vec_mat_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      // Matrix Row
      #[cfg(all(feature = "matrix2", feature = "row_vector2"))]
      $op!([<$lib M2R2>], Matrix2<$in>, RowVector2<$in>, Matrix2<$out>, [<$lib:lower _mat_row_op>], FeatureFlag::Builtin(FeatureKind::$lib)); 
      #[cfg(all(feature = "matrix3", feature = "row_vector3"))]
      $op!([<$lib M3R3>], Matrix3<$in>, RowVector3<$in>, Matrix3<$out>, [<$lib:lower _mat_row_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(all(feature = "matrix4", feature = "row_vector4"))]
      $op!([<$lib M4R4>], Matrix4<$in>, RowVector4<$in>, Matrix4<$out>, [<$lib:lower _mat_row_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(all(feature = "matrix2x3", feature = "row_vector3"))]
      $op!([<$lib M2x3R3>], Matrix2x3<$in>, RowVector3<$in>, Matrix2x3<$out>, [<$lib:lower _mat_row_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(all(feature = "matrix3x2", feature = "row_vector2"))]
      $op!([<$lib M3x2R2>], Matrix3x2<$in>, RowVector2<$in>, Matrix3x2<$out>, [<$lib:lower _mat_row_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(all(feature = "matrixd", feature = "row_vectord"))]
      $op!([<$lib MDRD>], DMatrix<$in>, RowDVector<$in>, DMatrix<$out>, [<$lib:lower _mat_row_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(all(feature = "matrixd", feature = "row_vector2"))]
      $op!([<$lib MDR2>], DMatrix<$in>, RowVector2<$in>, DMatrix<$out>, [<$lib:lower _mat_row_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(all(feature = "matrixd", feature = "row_vector3"))]
      $op!([<$lib MDR3>], DMatrix<$in>, RowVector3<$in>, DMatrix<$out>, [<$lib:lower _mat_row_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(all(feature = "matrixd", feature = "row_vector4"))]
      $op!([<$lib MDR4>], DMatrix<$in>, RowVector4<$in>, DMatrix<$out>, [<$lib:lower _mat_row_op>], FeatureFlag::Builtin(FeatureKind::$lib)); 
      // Row Matrix
      #[cfg(all(feature = "row_vector2", feature = "matrix2"))]
      $op!([<$lib R2M2>], RowVector2<$in>, Matrix2<$in>, Matrix2<$out>, [<$lib:lower _row_mat_op>], FeatureFlag::Builtin(FeatureKind::$lib)); 
      #[cfg(all(feature = "row_vector3", feature = "matrix3"))]
      $op!([<$lib R3M3>], RowVector3<$in>, Matrix3<$in>, Matrix3<$out>, [<$lib:lower _row_mat_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(all(feature = "row_vector4", feature = "matrix4"))]
      $op!([<$lib R4M4>], RowVector4<$in>, Matrix4<$in>, Matrix4<$out>, [<$lib:lower _row_mat_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(all(feature = "row_vector3", feature = "matrix2x3"))]
      $op!([<$lib R3M2x3>], RowVector3<$in>, Matrix2x3<$in>, Matrix2x3<$out>, [<$lib:lower _row_mat_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(all(feature = "row_vector2", feature = "matrix3x2"))]
      $op!([<$lib R2M3x2>], RowVector2<$in>, Matrix3x2<$in>, Matrix3x2<$out>, [<$lib:lower _row_mat_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(all(feature = "row_vectord", feature = "matrixd"))]
      $op!([<$lib RDMD>], RowDVector<$in>, DMatrix<$in>, DMatrix<$out>, [<$lib:lower _row_mat_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(all(feature = "row_vector2", feature = "matrixd"))]
      $op!([<$lib R2MD>], RowVector2<$in>, DMatrix<$in>, DMatrix<$out>, [<$lib:lower _row_mat_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(all(feature = "row_vector3", feature = "matrixd"))]
      $op!([<$lib R3MD>], RowVector3<$in>, DMatrix<$in>, DMatrix<$out>, [<$lib:lower _row_mat_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(all(feature = "row_vector4", feature = "matrixd"))]
      $op!([<$lib R4MD>], RowVector4<$in>, DMatrix<$in>, DMatrix<$out>, [<$lib:lower _row_mat_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      // Row Row
      #[cfg(feature = "row_vector2")]
      $op!([<$lib R2R2>], RowVector2<$in>, RowVector2<$in>, RowVector2<$out>, [<$lib:lower _vec_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(feature = "row_vector3")]
      $op!([<$lib R3R3>], RowVector3<$in>, RowVector3<$in>, RowVector3<$out>, [<$lib:lower _vec_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(feature = "row_vector4")]
      $op!([<$lib R4R4>], RowVector4<$in>, RowVector4<$in>, RowVector4<$out>, [<$lib:lower _vec_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(feature = "row_vectord")]
      $op!([<$lib RDRD>], RowDVector<$in>, RowDVector<$in>, RowDVector<$out>, [<$lib:lower _vec_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      // Vector Vector
      #[cfg(feature = "vector2")]
      $op!([<$lib V2V2>], Vector2<$in>, Vector2<$in>, Vector2<$out>, [<$lib:lower _vec_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(feature = "vector3")]
      $op!([<$lib V3V3>], Vector3<$in>, Vector3<$in>, Vector3<$out>, [<$lib:lower _vec_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(feature = "vector4")]
      $op!([<$lib V4V4>], Vector4<$in>, Vector4<$in>, Vector4<$out>, [<$lib:lower _vec_op>], FeatureFlag::Builtin(FeatureKind::$lib));
      #[cfg(feature = "vectord")]
      $op!([<$lib VDVD>], DVector<$in>, DVector<$in>, DVector<$out>, [<$lib:lower _vec_op>], FeatureFlag::Builtin(FeatureKind::$lib));
    }
  }}

#[macro_export]
macro_rules! impl_binop_match_arms {
  ($lib:ident, $arg:expr, $($lhs_type:ident, $($target_type:ident, $value_string:tt),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            // Scalar Scalar
            #[cfg(all(feature = $value_string))]
            (Value::$lhs_type(lhs), Value::$lhs_type(rhs)) => Ok(Box::new([<$lib SS>]{lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new($target_type::default()) })),
            // Scalar Matrix
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            (Value::$lhs_type(lhs), Value::[<Matrix $lhs_type>](Matrix::Matrix1(rhs))) => Ok(Box::new([<$lib SM1>]{lhs, rhs, out: Ref::new(Matrix1::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrix2"))]
            (Value::$lhs_type(lhs), Value::[<Matrix $lhs_type>](Matrix::Matrix2(rhs))) => Ok(Box::new([<$lib SM2>]{lhs, rhs, out: Ref::new(Matrix2::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            (Value::$lhs_type(lhs), Value::[<Matrix $lhs_type>](Matrix::Matrix3(rhs))) => Ok(Box::new([<$lib SM3>]{lhs, rhs, out: Ref::new(Matrix3::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            (Value::$lhs_type(lhs), Value::[<Matrix $lhs_type>](Matrix::Matrix4(rhs))) => Ok(Box::new([<$lib SM4>]{lhs, rhs, out: Ref::new(Matrix4::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            (Value::$lhs_type(lhs), Value::[<Matrix $lhs_type>](Matrix::Matrix2x3(rhs))) => Ok(Box::new([<$lib SM2x3>]{lhs, rhs, out: Ref::new(Matrix2x3::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            (Value::$lhs_type(lhs), Value::[<Matrix $lhs_type>](Matrix::Matrix3x2(rhs))) => Ok(Box::new([<$lib SM3x2>]{lhs, rhs, out: Ref::new(Matrix3x2::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            (Value::$lhs_type(lhs), Value::[<Matrix $lhs_type>](Matrix::DMatrix(rhs))) => {
              let (rows,cols) = {rhs.borrow().shape()};
              Ok(Box::new([<$lib SMD>]{lhs, rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
            },   
            // Scalar Row
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            (Value::$lhs_type(lhs), Value::[<Matrix $lhs_type>](Matrix::RowVector2(rhs))) => Ok(Box::new([<$lib SR2>]{lhs, rhs, out: Ref::new(RowVector2::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            (Value::$lhs_type(lhs), Value::[<Matrix $lhs_type>](Matrix::RowVector3(rhs))) => Ok(Box::new([<$lib SR3>]{lhs, rhs, out: Ref::new(RowVector3::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            (Value::$lhs_type(lhs), Value::[<Matrix $lhs_type>](Matrix::RowVector4(rhs))) => Ok(Box::new([<$lib SR4>]{lhs, rhs, out: Ref::new(RowVector4::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            (Value::$lhs_type(lhs), Value::[<Matrix $lhs_type>](Matrix::RowDVector(rhs))) => Ok(Box::new([<$lib SRD>]{lhs, rhs: rhs.clone(), out: Ref::new(RowDVector::from_element(rhs.borrow().len(),$target_type::default()))})),
            // Scalar Vector
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            (Value::$lhs_type(lhs), Value::[<Matrix $lhs_type>](Matrix::Vector2(rhs))) => Ok(Box::new([<$lib SV2>]{lhs, rhs, out: Ref::new(Vector2::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            (Value::$lhs_type(lhs), Value::[<Matrix $lhs_type>](Matrix::Vector3(rhs))) => Ok(Box::new([<$lib SV3>]{lhs, rhs, out: Ref::new(Vector3::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            (Value::$lhs_type(lhs), Value::[<Matrix $lhs_type>](Matrix::Vector4(rhs))) => Ok(Box::new([<$lib SV4>]{lhs, rhs, out: Ref::new(Vector4::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            (Value::$lhs_type(lhs), Value::[<Matrix $lhs_type>](Matrix::DVector(rhs))) => Ok(Box::new([<$lib SVD>]{lhs, rhs: rhs.clone(), out: Ref::new(DVector::from_element(rhs.borrow().len(),$target_type::default()))})),
            // Matrix Scalar
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix1(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib M1S>]{lhs, rhs, out: Ref::new(Matrix1::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrix2"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix2(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib M2S>]{lhs, rhs, out: Ref::new(Matrix2::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix3(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib M3S>]{lhs, rhs, out: Ref::new(Matrix3::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix4(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib M4S>]{lhs, rhs, out: Ref::new(Matrix4::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix2x3(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib M2x3S>]{lhs, rhs, out: Ref::new(Matrix2x3::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix3x2(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib M3x2S>]{lhs, rhs, out: Ref::new(Matrix3x2::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            (Value::[<Matrix $lhs_type>](Matrix::DMatrix(lhs)),Value::$lhs_type(rhs)) => {
              let (rows,cols) = {lhs.borrow().shape()};
              Ok(Box::new([<$lib MDS>]{lhs: lhs.clone(), rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
            },
            // Row Scalar
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowVector2(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib R2S>]{lhs, rhs, out: Ref::new(RowVector2::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowVector3(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib R3S>]{lhs, rhs, out: Ref::new(RowVector3::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowVector4(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib R4S>]{lhs, rhs, out: Ref::new(RowVector4::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowDVector(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib RDS>]{lhs: lhs.clone(), rhs, out: Ref::new(RowDVector::from_element(lhs.borrow().len(),$target_type::default()))})),
            // Vector Scalar
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            (Value::[<Matrix $lhs_type>](Matrix::Vector2(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib V2S>]{lhs, rhs, out: Ref::new(Vector2::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            (Value::[<Matrix $lhs_type>](Matrix::Vector3(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib V3S>]{lhs, rhs, out: Ref::new(Vector3::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            (Value::[<Matrix $lhs_type>](Matrix::Vector4(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib V4S>]{lhs, rhs, out: Ref::new(Vector4::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            (Value::[<Matrix $lhs_type>](Matrix::DVector(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib VDS>]{lhs: lhs.clone(), rhs, out: Ref::new(DVector::from_element(lhs.borrow().len(),$target_type::default()))})),
            // Matrix Matrix
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix1(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix1(rhs))) => Ok(Box::new([<$lib M1M1>]{lhs, rhs, out: Ref::new(Matrix1::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrix2"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix2(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix2(rhs))) => Ok(Box::new([<$lib M2M2>]{lhs, rhs, out: Ref::new(Matrix2::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix3(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix3(rhs))) => Ok(Box::new([<$lib M3M3>]{lhs, rhs, out: Ref::new(Matrix3::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix4(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix4(rhs))) => Ok(Box::new([<$lib M4M4>]{lhs, rhs, out: Ref::new(Matrix4::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix2x3(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix2x3(rhs))) => Ok(Box::new([<$lib M2x3M2x3>]{lhs, rhs, out: Ref::new(Matrix2x3::from_element($target_type::default()))})),  
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix3x2(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix3x2(rhs))) => Ok(Box::new([<$lib M3x2M3x2>]{lhs, rhs, out: Ref::new(Matrix3x2::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            (Value::[<Matrix $lhs_type>](Matrix::DMatrix(lhs)), Value::[<Matrix $lhs_type>](Matrix::DMatrix(rhs))) => {
              let (rows,cols) = {lhs.borrow().shape()};
              Ok(Box::new([<$lib MDMD>]{lhs, rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
            },
            // Row Row
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowVector2(lhs)), Value::[<Matrix $lhs_type>](Matrix::RowVector2(rhs))) => Ok(Box::new([<$lib R2R2>]{lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(RowVector2::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowVector3(lhs)), Value::[<Matrix $lhs_type>](Matrix::RowVector3(rhs))) => Ok(Box::new([<$lib R3R3>]{lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(RowVector3::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowVector4(lhs)), Value::[<Matrix $lhs_type>](Matrix::RowVector4(rhs))) => Ok(Box::new([<$lib R4R4>]{lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(RowVector4::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowDVector(lhs)), Value::[<Matrix $lhs_type>](Matrix::RowDVector(rhs))) => Ok(Box::new([<$lib RDRD>]{lhs: lhs.clone(), rhs, out: Ref::new(RowDVector::from_element(lhs.borrow().len(),$target_type::default()))})),
            // Vector Vector
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            (Value::[<Matrix $lhs_type>](Matrix::Vector2(lhs)), Value::[<Matrix $lhs_type>](Matrix::Vector2(rhs))) => Ok(Box::new([<$lib V2V2>]{lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Vector2::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            (Value::[<Matrix $lhs_type>](Matrix::Vector3(lhs)), Value::[<Matrix $lhs_type>](Matrix::Vector3(rhs))) => Ok(Box::new([<$lib V3V3>]{lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Vector3::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            (Value::[<Matrix $lhs_type>](Matrix::Vector4(lhs)), Value::[<Matrix $lhs_type>](Matrix::Vector4(rhs))) => Ok(Box::new([<$lib V4V4>]{lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Vector4::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            (Value::[<Matrix $lhs_type>](Matrix::DVector(lhs)), Value::[<Matrix $lhs_type>](Matrix::DVector(rhs))) => Ok(Box::new([<$lib VDVD>]{lhs: lhs.clone(), rhs, out: Ref::new(DVector::from_element(lhs.borrow().len(),$target_type::default()))})),
            // Matrix Vector     
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector2"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix2(lhs)),Value::[<Matrix $lhs_type>](Matrix::Vector2(rhs))) => Ok(Box::new([<$lib M2V2>]{lhs, rhs, out: Ref::new(Matrix2::from_element($target_type::default()))})),         
            #[cfg(all(feature = $value_string, feature = "matrix3", feature = "vector3"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix3(lhs)),Value::[<Matrix $lhs_type>](Matrix::Vector3(rhs))) => Ok(Box::new([<$lib M3V3>]{lhs, rhs, out: Ref::new(Matrix3::from_element($target_type::default()))})),         
            #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "vector2"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix2x3(lhs)),Value::[<Matrix $lhs_type>](Matrix::Vector2(rhs))) => Ok(Box::new([<$lib M2x3V2>]{lhs, rhs, out: Ref::new(Matrix2x3::from_element($target_type::default()))})),         
            #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "vector3"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix3x2(lhs)),Value::[<Matrix $lhs_type>](Matrix::Vector3(rhs))) => Ok(Box::new([<$lib M3x2V3>]{lhs, rhs, out: Ref::new(Matrix3x2::from_element($target_type::default()))})),         
            #[cfg(all(feature = $value_string, feature = "matrix4", feature = "vector4"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix4(lhs)),Value::[<Matrix $lhs_type>](Matrix::Vector4(rhs))) => Ok(Box::new([<$lib M4V4>]{lhs, rhs, out: Ref::new(Matrix4::from_element($target_type::default()))})),         
            // Vector Matrix
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "row_vector2"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix2(lhs)),Value::[<Matrix $lhs_type>](Matrix::RowVector2(rhs))) => Ok(Box::new([<$lib M2R2>]{lhs, rhs, out: Ref::new(Matrix2::from_element($target_type::default()))})),         
            #[cfg(all(feature = $value_string, feature = "matrix3", feature = "row_vector3"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix3(lhs)),Value::[<Matrix $lhs_type>](Matrix::RowVector3(rhs))) => Ok(Box::new([<$lib M3R3>]{lhs, rhs, out: Ref::new(Matrix3::from_element($target_type::default()))})),         
            #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "row_vector3"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix2x3(lhs)),Value::[<Matrix $lhs_type>](Matrix::RowVector3(rhs))) => Ok(Box::new([<$lib M2x3R3>]{lhs, rhs, out: Ref::new(Matrix2x3::from_element($target_type::default()))})),         
            #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "row_vector2"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix3x2(lhs)),Value::[<Matrix $lhs_type>](Matrix::RowVector2(rhs))) => Ok(Box::new([<$lib M3x2R2>]{lhs, rhs, out: Ref::new(Matrix3x2::from_element($target_type::default()))})),         
            #[cfg(all(feature = $value_string, feature = "matrix4", feature = "row_vector4"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix4(lhs)),Value::[<Matrix $lhs_type>](Matrix::RowVector4(rhs))) => Ok(Box::new([<$lib M4R4>]{lhs, rhs, out: Ref::new(Matrix4::from_element($target_type::default()))})),         
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            (Value::[<Matrix $lhs_type>](Matrix::DMatrix(lhs)),Value::[<Matrix $lhs_type>](rhs)) => {
              let (rows,cols) = {lhs.borrow().shape()};
              let rhs_shape = rhs.shape();
              match (rows,cols,rhs_shape[0],rhs_shape[1]) {
                // matching rows
                (n,_,m,1) if n == m => (),
                // matching cols
                (_,n,1,m) if n == m => (),
                // mismatching dimensions
                _ => {return Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::DimensionMismatch(vec![]) });},
              }
              match rhs {
                #[cfg(feature = "vector2")]
                Matrix::Vector2(rhs) => Ok(Box::new([<$lib MDV2>]{lhs: lhs.clone(), rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))})),
                #[cfg(feature = "vector3")]
                Matrix::Vector3(rhs) => Ok(Box::new([<$lib MDV3>]{lhs: lhs.clone(), rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))})),
                #[cfg(feature = "vector4")]
                Matrix::Vector4(rhs) => Ok(Box::new([<$lib MDV4>]{lhs: lhs.clone(), rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))})),
                #[cfg(feature = "vectord")]
                Matrix::DVector(rhs) => Ok(Box::new([<$lib MDVD>]{lhs: lhs.clone(), rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))})),
                #[cfg(feature = "row_vector2")]
                Matrix::RowVector2(rhs) => Ok(Box::new([<$lib MDR2>]{lhs: lhs.clone(), rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))})),
                #[cfg(reature = "row_vector3")]
                Matrix::RowVector3(rhs) => Ok(Box::new([<$lib MDR3>]{lhs: lhs.clone(), rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))})),
                #[cfg(feature = "row_vector4")]
                Matrix::RowVector4(rhs) => Ok(Box::new([<$lib MDR4>]{lhs: lhs.clone(), rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))})),
                #[cfg(feature = "row_vectord")]
                Matrix::RowDVector(rhs) => Ok(Box::new([<$lib MDRD>]{lhs: lhs.clone(), rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))})),
                _ => {return Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::DimensionMismatch(vec![]) });},
              }
            },
            // Vector Matrix
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector2"))]
            (Value::[<Matrix $lhs_type>](Matrix::Vector2(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix2(rhs))) => Ok(Box::new([<$lib V2M2>]{lhs, rhs, out: Ref::new(Matrix2::from_element($target_type::default()))})),         
            #[cfg(all(feature = $value_string, feature = "matrix3", feature = "vector3"))]
            (Value::[<Matrix $lhs_type>](Matrix::Vector3(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix3(rhs))) => Ok(Box::new([<$lib V3M3>]{lhs, rhs, out: Ref::new(Matrix3::from_element($target_type::default()))})),         
            #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "vector2"))]
            (Value::[<Matrix $lhs_type>](Matrix::Vector2(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix2x3(rhs))) => Ok(Box::new([<$lib V2M2x3>]{lhs, rhs, out: Ref::new(Matrix2x3::from_element($target_type::default()))})),         
            #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "vector3"))]
            (Value::[<Matrix $lhs_type>](Matrix::Vector3(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix3x2(rhs))) => Ok(Box::new([<$lib V3M3x2>]{lhs, rhs, out: Ref::new(Matrix3x2::from_element($target_type::default()))})),                     
            #[cfg(all(feature = $value_string, feature = "matrix4", feature = "vector4"))]
            (Value::[<Matrix $lhs_type>](Matrix::Vector4(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix4(rhs))) => Ok(Box::new([<$lib V4M4>]{lhs, rhs, out: Ref::new(Matrix4::from_element($target_type::default()))})),                     
            // Row Matrix     
            #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "matrix2"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowVector2(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix2(rhs))) => Ok(Box::new([<$lib R2M2>]{lhs, rhs, out: Ref::new(Matrix2::from_element($target_type::default()))})),         
            #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "matrix3"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowVector3(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix3(rhs))) => Ok(Box::new([<$lib R3M3>]{lhs, rhs, out: Ref::new(Matrix3::from_element($target_type::default()))})),         
            #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "matrix2x3"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowVector3(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix2x3(rhs))) => Ok(Box::new([<$lib R3M2x3>]{lhs, rhs, out: Ref::new(Matrix2x3::from_element($target_type::default()))})),         
            #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "matrix3x2"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowVector2(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix3x2(rhs))) => Ok(Box::new([<$lib R2M3x2>]{lhs, rhs, out: Ref::new(Matrix3x2::from_element($target_type::default()))})),         
            #[cfg(all(feature = $value_string, feature = "row_vector4", feature = "matrix4"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowVector4(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix4(rhs))) => Ok(Box::new([<$lib R4M4>]{lhs, rhs, out: Ref::new(Matrix4::from_element($target_type::default()))})),         
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            (Value::[<Matrix $lhs_type>](lhs),Value::[<Matrix $lhs_type>](Matrix::DMatrix(rhs))) => {
              let (rows,cols) = {rhs.borrow().shape()};
              let lhs_shape = lhs.shape();
              match (lhs_shape[0],lhs_shape[1],rows,cols) {
                // matching rows
                (m,1,n,_) if n == m => (),
                // matching cols
                (1,m,_,n) if n == m => (),
                // mismatching dimensions
                _ => {return Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::DimensionMismatch(vec![]) });},
              }
              match lhs {
                #[cfg(feature = "vector2")]
                Matrix::Vector2(lhs) => Ok(Box::new([<$lib V2MD>]{lhs, rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))})),
                #[cfg(feature = "vector3")]
                Matrix::Vector3(lhs) => Ok(Box::new([<$lib V3MD>]{lhs, rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))})),
                #[cfg(feature = "vector4")]
                Matrix::Vector4(lhs) => Ok(Box::new([<$lib V4MD>]{lhs, rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))})),
                #[cfg(feature = "vectord")]
                Matrix::DVector(lhs) => Ok(Box::new([<$lib VDMD>]{lhs, rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))})),
                #[cfg(feature = "row_vector2")]
                Matrix::RowVector2(lhs) => Ok(Box::new([<$lib R2MD>]{lhs, rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))})),
                #[cfg(feature = "row_vector3")]
                Matrix::RowVector3(lhs) => Ok(Box::new([<$lib R3MD>]{lhs, rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))})),
                #[cfg(feature = "row_vector4")]
                Matrix::RowVector4(lhs) => Ok(Box::new([<$lib R4MD>]{lhs, rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))})),
                #[cfg(feature = "row_vectord")]
                Matrix::RowDVector(lhs) => Ok(Box::new([<$lib RDMD>]{lhs, rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))})),
                _ => {return Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::DimensionMismatch(vec![]) });},
              }
            }
          )+
        )+
        x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}  

#[macro_export]
macro_rules! impl_urnop_match_arms {
  ($lib:tt, $arg:tt, $($lhs_type:tt, $($target_type:tt, $value_string:tt),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            #[cfg(feature = $value_string)]
            (Value::$lhs_type(arg)) => Ok(Box::new([<$lib S>]{arg: arg.clone(), out: Ref::new($target_type::default()), _marker: PhantomData::default() })),
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix1(arg))) => Ok(Box::new([<$lib V>]{arg, out: Ref::new(Matrix1::from_element($target_type::default())), _marker: PhantomData::default() })),
            #[cfg(all(feature = $value_string, feature = "matrix2"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix2(arg))) => Ok(Box::new([<$lib V>]{arg, out: Ref::new(Matrix2::from_element($target_type::default())), _marker: PhantomData::default() })),
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix3(arg))) => Ok(Box::new([<$lib V>]{arg, out: Ref::new(Matrix3::from_element($target_type::default())), _marker: PhantomData::default() })),
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix4(arg))) => Ok(Box::new([<$lib V>]{arg, out: Ref::new(Matrix4::from_element($target_type::default())), _marker: PhantomData::default() })),
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix2x3(arg))) => Ok(Box::new([<$lib V>]{arg, out: Ref::new(Matrix2x3::from_element($target_type::default())), _marker: PhantomData::default() })),         
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix3x2(arg))) => Ok(Box::new([<$lib V>]{arg, out: Ref::new(Matrix3x2::from_element($target_type::default())), _marker: PhantomData::default() })),         
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowVector2(arg))) => Ok(Box::new([<$lib V>]{arg: arg.clone(), out: Ref::new(RowVector2::from_element($target_type::default())), _marker: PhantomData::default() })),
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowVector3(arg))) => Ok(Box::new([<$lib V>]{arg: arg.clone(), out: Ref::new(RowVector3::from_element($target_type::default())), _marker: PhantomData::default() })),
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowVector4(arg))) => Ok(Box::new([<$lib V>]{arg: arg.clone(), out: Ref::new(RowVector4::from_element($target_type::default())), _marker: PhantomData::default() })),
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowDVector(arg))) => Ok(Box::new([<$lib V>]{arg: arg.clone(), out: Ref::new(RowDVector::from_element(arg.borrow().len(),$target_type::default())), _marker: PhantomData::default() })),
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            (Value::[<Matrix $lhs_type>](Matrix::Vector2(arg))) => Ok(Box::new([<$lib V>]{arg: arg.clone(), out: Ref::new(Vector2::from_element($target_type::default())), _marker: PhantomData::default() })),
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            (Value::[<Matrix $lhs_type>](Matrix::Vector3(arg))) => Ok(Box::new([<$lib V>]{arg: arg.clone(), out: Ref::new(Vector3::from_element($target_type::default())), _marker: PhantomData::default() })),
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            (Value::[<Matrix $lhs_type>](Matrix::Vector4(arg))) => Ok(Box::new([<$lib V>]{arg: arg.clone(), out: Ref::new(Vector4::from_element($target_type::default())), _marker: PhantomData::default() })),
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            (Value::[<Matrix $lhs_type>](Matrix::DVector(arg))) => Ok(Box::new([<$lib V>]{arg: arg.clone(), out: Ref::new(DVector::from_element(arg.borrow().len(),$target_type::default())), _marker: PhantomData::default() })),
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            (Value::[<Matrix $lhs_type>](Matrix::DMatrix(arg))) => {
              let (rows,cols) = {arg.borrow().shape()};
              Ok(Box::new([<$lib V>]{arg, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default())), _marker: PhantomData::default() }))},
          )+
        )+
        x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

#[macro_export]
macro_rules! impl_mech_binop_fxn {
  ($fxn_name:ident, $gen_fxn:tt) => {
    pub struct $fxn_name {}
    impl NativeFunctionCompiler for $fxn_name {
      fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
        if arguments.len() != 2 {
          return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
        }
        let lhs_value = arguments[0].clone();
        let rhs_value = arguments[1].clone();
        match $gen_fxn(lhs_value.clone(), rhs_value.clone()) {
          Ok(fxn) => Ok(fxn),
          Err(_) => {
            match (lhs_value,rhs_value) {
              (Value::MutableReference(lhs),Value::MutableReference(rhs)) => {$gen_fxn(lhs.borrow().clone(), rhs.borrow().clone())}
              (lhs_value,Value::MutableReference(rhs)) => { $gen_fxn(lhs_value.clone(), rhs.borrow().clone())}
              (Value::MutableReference(lhs),rhs_value) => { $gen_fxn(lhs.borrow().clone(), rhs_value.clone()) }
              x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
            }
          }
        }
      }
    }
  }
}

#[macro_export]
macro_rules! impl_mech_urnop_fxn {
  ($fxn_name:ident, $gen_fxn:tt) => {
    pub struct $fxn_name {}
    impl NativeFunctionCompiler for $fxn_name {
      fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
        if arguments.len() != 1 {
          return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
        }
        let input = arguments[0].clone();
        match $gen_fxn(input.clone()) {
          Ok(fxn) => Ok(fxn),
          Err(_) => {
            match (input) {
              (Value::MutableReference(input)) => {$gen_fxn(input.borrow().clone())}
              x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:#?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
            }
          }
        }
      }
    }
  }
}

#[macro_export]
macro_rules! impl_set_range_all_match_arms {
  ($fxn_name:ident, $arg:expr, $($value_kind:ident, $value_string:tt);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          // Vector Scalar
          #[cfg(all(feature = $value_string, feature = "matrix4"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix1"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrixd"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          
          // Vector Vector
          #[cfg(all(feature = $value_string, feature = "matrixd"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::DMatrix(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name V>]{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrixd", feature = "matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix2(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name V>]{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          
          #[cfg(all(feature = $value_string, feature = "matrixd", feature = "matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix2x3(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name V>]{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrixd", feature = "matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix3(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name V>]{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrixd", feature = "matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix3x2(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name V>]{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrixd", feature = "matrix4"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix4(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name V>]{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          
          #[cfg(all(feature = $value_string, feature = "matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix2(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name V>]{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2", feature = "matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix3x2(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name V>]{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2", feature = "matrixd"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::DMatrix(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name V>]{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),

          #[cfg(all(feature = $value_string, feature = "matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix3(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name V>]{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3", feature = "matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix2x3(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name V>]{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3", feature = "matrixd"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::DMatrix(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name V>]{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),

          #[cfg(all(feature = $value_string, feature = "matrix4"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix4(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name V>]{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix4", feature = "matrixd"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::DMatrix(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name V>]{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),

          #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix2x3(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name V>]{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix3(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name V>]{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "matrixd"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::DMatrix(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name V>]{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),

          #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix3x2(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name V>]{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix2(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name V>]{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "matrixd"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::DMatrix(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name V>]{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),

          // Matrix Scalar Bool
          #[cfg(all(feature = $value_string, feature = "matrix4", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)),   [Value::MatrixBool(Matrix::DVector(ix)),Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name SB>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)),   [Value::MatrixBool(Matrix::DVector(ix)),Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name SB>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)),   [Value::MatrixBool(Matrix::DVector(ix)),Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name SB>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix1", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)),   [Value::MatrixBool(Matrix::DVector(ix)),Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name SB>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::MatrixBool(Matrix::DVector(ix)),Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name SB>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::MatrixBool(Matrix::DVector(ix)),Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name SB>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrixd", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)),   [Value::MatrixBool(Matrix::DVector(ix)),Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name SB>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
        
          // Matrix Vector Bool
          #[cfg(all(feature = $value_string, feature = "matrixd", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)), [Value::MatrixBool(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::DMatrix(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)), [Value::MatrixBool(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix2(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)), [Value::MatrixBool(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix3(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix4", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)), [Value::MatrixBool(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix4(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::MatrixBool(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix2x3(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::MatrixBool(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix3x2(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
        )+
        x => {
          println!("{:#?}", x);
          Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind })
        }
      }
    }
  }
}

#[macro_export]
macro_rules! impl_set_range_match_arms {
  ($fxn_name:ident, $arg:expr, $($value_kind:ident,$value_string:tt);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          // Set vector
          #[cfg(all(feature = $value_string, feature = "row_vector4"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector4(sink)),[Value::MatrixIndex(Matrix::DVector(ix))], Value::[<Matrix $value_kind>](Matrix::RowVector4(source))) => Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "row_vector4", feature = "row_vector3"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector4(sink)),[Value::MatrixIndex(Matrix::DVector(ix))], Value::[<Matrix $value_kind>](Matrix::RowVector3(source))) => Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "row_vector4", feature = "row_vector2"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector4(sink)),[Value::MatrixIndex(Matrix::DVector(ix))], Value::[<Matrix $value_kind>](Matrix::RowVector2(source))) => Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "vector4"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector4(sink)),[Value::MatrixIndex(Matrix::DVector(ix))], Value::[<Matrix $value_kind>](Matrix::Vector4(source))) => Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector3"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector4(sink)),[Value::MatrixIndex(Matrix::DVector(ix))], Value::[<Matrix $value_kind>](Matrix::Vector3(source))) => Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector2"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector4(sink)),[Value::MatrixIndex(Matrix::DVector(ix))], Value::[<Matrix $value_kind>](Matrix::Vector2(source))) => Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),

          // Set Matrix
          #[cfg(all(feature = $value_string, feature = "matrixd"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)),[Value::MatrixIndex(Matrix::DVector(ix))], Value::[<Matrix $value_kind>](Matrix::DMatrix(source))) => Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          
          // Set scalar
          #[cfg(all(feature = $value_string, feature = "row_vector4"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector4(sink)),[Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),            
          #[cfg(all(feature = $value_string, feature = "row_vector3"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector3(sink)),[Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "row_vector2"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector2(sink)),[Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "vector4"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector4(sink)),   [Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "vector3"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector3(sink)),   [Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "vector2"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector2(sink)),   [Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix4"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)),   [Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)),   [Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)),   [Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix1"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)),   [Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrixd"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)),   [Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "row_vectord"))]
          (Value::[<Matrix $value_kind>](Matrix::RowDVector(sink)),[Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "vectord"))]
          (Value::[<Matrix $value_kind>](Matrix::DVector(sink)),   [Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),

          // Bool
          #[cfg(all(feature = $value_string, feature = "row_vector4", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector4(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector3(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector2(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "vector4", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector4(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "vector3", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector3(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "vector2", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector2(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix4", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix1", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),            
          #[cfg(all(feature = $value_string, feature = "matrixd", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "vectord", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::DVector(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::RowDVector(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
        )+
        x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}
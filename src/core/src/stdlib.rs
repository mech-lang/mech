pub use crate::*;

// The Standard Library
// ----------------------------------------------------------------------------

// These macros are used by various libraries to generate function impl and
// match arms. They're gated on feature flags so although there's a lot of
// code here to account for all the different combinations, but only
// the relevant code will be compiled in any given build.

#[macro_export]
macro_rules! register_descriptor {
    ($descriptor:expr) => {
        #[cfg(not(target_arch = "wasm32"))]
        inventory::submit!{ $descriptor }

        #[cfg(target_arch = "wasm32")]
        const _: () = {
            // no-op pprevents unused warnings
            let _ = &$descriptor;
        };
    };
}


#[macro_export]
macro_rules! compile_register_brrw {
  ($reg:expr, $ctx:ident) => {
    {
      let addr = $reg.addr();
      let reg = $ctx.alloc_register_for_ptr(addr);
      let borrow = $reg.borrow();
      let const_id = borrow.compile_const($ctx).unwrap();
      $ctx.emit_const_load(reg, const_id);
      reg
    }
  };
}

#[macro_export]
macro_rules! compile_register {
  ($reg:expr, $ctx:ident) => {
    {
      let addr = $reg.addr();
      let reg = $ctx.alloc_register_for_ptr(addr);
      let const_id = $reg.compile_const($ctx).unwrap();
      $ctx.emit_const_load(reg, const_id);
      reg
    }
  };
}

#[macro_export]
macro_rules! compile_register_mat {
  ($reg:expr, $ctx:ident) => {
    {
      let addr = $reg.addr();
      let reg = $ctx.alloc_register_for_ptr(addr);
      let const_id = $reg.compile_const_mat($ctx).unwrap();
      $ctx.emit_const_load(reg, const_id);
      reg
    }
  };
}

#[macro_export]
macro_rules! compile_nullop {
  ($name:tt, $out:expr, $ctx:ident, $feature_flag:expr) => {
    // allocate one register as an array
    let mut registers = [0];

    // Compile out
    registers[0] = compile_register_brrw!($out, $ctx);

    $ctx.features.insert($feature_flag);

    // Emit the operation
    $ctx.emit_nullop(
      hash_str(&$name),
      registers[0],
    );

    return Ok(registers[0]);
  };
}

#[macro_export]
macro_rules! compile_unop {
  ($name:tt, $out:expr, $arg:expr, $ctx:ident, $feature_flag:expr) => {
    // Allocate three registers as an array
    let mut registers = [0,0];

    // Allocate registers
    registers[0] = compile_register_brrw!($out, $ctx);
    registers[1] = compile_register_brrw!($arg, $ctx);
  
    $ctx.features.insert($feature_flag);

    // Emit the operation
    $ctx.emit_unop(
      hash_str(&$name),
      registers[0],
      registers[1],
    );

    return Ok(registers[0]);
  };
}

#[macro_export]
macro_rules! compile_binop {
  ($name:tt, $out:expr, $arg1:expr, $arg2:expr, $ctx:ident, $feature_flag:expr) => {
    let mut registers = [0,0,0];
    
    registers[0] = compile_register_brrw!($out, $ctx);
    registers[1] = compile_register_brrw!($arg1, $ctx);
    registers[2] = compile_register_brrw!($arg2, $ctx);

    $ctx.features.insert($feature_flag);

    $ctx.emit_binop(
      hash_str(&$name),
      registers[0],
      registers[1],
      registers[2],
    );

    return Ok(registers[0])
  };
}

#[macro_export]
macro_rules! compile_ternop {
  ($name:tt, $out:expr, $arg1:expr, $arg2:expr, $arg3:expr, $ctx:ident, $feature_flag:expr) => {
    let mut registers = [0,0,0,0];

    registers[0] = compile_register_brrw!($out, $ctx);
    registers[1] = compile_register_brrw!($arg1, $ctx);
    registers[2] = compile_register_brrw!($arg2, $ctx);
    registers[3] = compile_register_brrw!($arg3, $ctx);

    $ctx.features.insert($feature_flag);

    $ctx.emit_ternop(
      hash_str(&$name),
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
  ($name:tt, $out:expr, $arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr, $ctx:ident, $feature_flag:expr) => {
    let mut registers = [0,0,0,0,0];

    registers[0] = compile_register_brrw!($out, $ctx);
    registers[1] = compile_register_brrw!($arg1, $ctx);
    registers[2] = compile_register_brrw!($arg2, $ctx);
    registers[3] = compile_register_brrw!($arg3, $ctx);
    registers[4] = compile_register_brrw!($arg4, $ctx);

    $ctx.features.insert($feature_flag);

    $ctx.emit_quadop(
      hash_str(&$name),
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
macro_rules! compile_varop {
  ($name:tt, $out:expr, $args:expr, $ctx:ident, $feature_flag:expr) => {
    let arg_count = $args.len();
    let mut registers = vec![0; arg_count + 1];
    registers[0] = compile_register_brrw!($out, $ctx);
    for i in 0..arg_count {
      registers[i + 1] = compile_register_brrw!($args[i], $ctx);
    }
    $ctx.features.insert($feature_flag);
    $ctx.emit_varop(
      hash_str(&$name),
      registers[0],
      (&registers[1..]).to_vec(),
    );
    return Ok(registers[0])
  };
}

#[macro_export]
macro_rules! register_fxn_descriptor_inner_logic {
  // single type
  ($struct_name:ident, $type:ty, $type_string:tt) => {
    paste!{
      #[cfg(not(target_arch = "wasm32"))]
      #[cfg(feature = $type_string)]
      inventory::submit! {
        FunctionDescriptor {
          name: concat!(stringify!($struct_name), "<", stringify!([<$type:lower>]), ">"),
          ptr: $struct_name::new,
        }
      }
    }
  };
}

#[macro_export]
macro_rules! register_fxn_descriptor_inner {
  // single type
  ($struct_name:ident, $type:ty, $type_string:tt) => {
    paste!{
      #[cfg(not(target_arch = "wasm32"))]
      #[cfg(feature = $type_string)]
      inventory::submit! {
        FunctionDescriptor {
          name: concat!(stringify!($struct_name), "<", stringify!([<$type:lower>]), ">"),
          ptr: $struct_name::<$type>::new,
        }
      }
    }
  };
}

#[macro_export]
macro_rules! register_fxn_descriptor {
  ($struct_name:ident, $($type:ty, $type_string:tt),+ $(,)?) => {
    $( register_fxn_descriptor_inner!($struct_name, $type, $type_string); )+
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
    impl<T> MechFunctionFactory for $struct_name<T> 
    where
      #[cfg(feature = "compiler")]T: Copy + Debug + Display + Clone + Sync + Send + 'static + PartialEq + PartialOrd + ConstElem + CompileConst + AsValueKind +Add<Output = T> + AddAssign +Sub<Output = T> + SubAssign +Mul<Output = T> + MulAssign +Div<Output = T> + DivAssign +Zero + One,
      #[cfg(not(feature = "compiler"))] T: Copy + Debug + Display + Clone + Sync + Send + 'static + PartialEq + PartialOrd + AsValueKind +Add<Output = T> + AddAssign +Sub<Output = T> + SubAssign +Mul<Output = T> + MulAssign +Div<Output = T> + DivAssign +Zero + One,
      Ref<$out_type>: ToValue,
    {
      fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
          FunctionArgs::Binary(out, arg1, arg2) => {
            let lhs: Ref<$arg1_type> = unsafe { arg1.as_unchecked() }.clone();
            let rhs: Ref<$arg2_type> = unsafe { arg2.as_unchecked() }.clone();
            let out: Ref<$out_type> = unsafe { out.as_unchecked() }.clone();
            Ok(Box::new(Self {lhs, rhs, out }))
          },
          _ => Err(MechError2::new(
              IncorrectNumberOfArguments { expected: 2, found: args.len() }, 
              None
            ).with_compiler_loc()
          ),
        }
      }
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
      T: ConstElem + CompileConst + AsValueKind
    {
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        let name = format!("{}<{}>", stringify!($struct_name), T::as_value_kind());
        compile_binop!(name, self.out, self.lhs, self.rhs, ctx, $feature_flag);
      }
    }
  };
}

#[macro_export]  
macro_rules! impl_unop {
  ($struct_name:ident, $arg_type:ty, $out_type:ty, $op:ident, $feature_flag:expr) => {
    #[derive(Debug)]
    struct $struct_name {
      arg: Ref<$arg_type>,
      out: Ref<$out_type>,
    }
    impl MechFunctionFactory for $struct_name {
      fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
          FunctionArgs::Unary(out, arg) => {
            let arg: Ref<$arg_type> = unsafe { arg.as_unchecked() }.clone();
            let out: Ref<$out_type> = unsafe { out.as_unchecked() }.clone();
            Ok(Box::new(Self {arg, out }))
          },
          _ => Err(MechError2::new(
              IncorrectNumberOfArguments { expected: 1, found: args.len() }, 
              None
            ).with_compiler_loc()
          ),
        }
      }
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
        let name = format!("{}", stringify!($struct_name));
        compile_unop!(name, self.out, self.arg, ctx, $feature_flag);
      }
    }
    #[cfg(not(target_arch = "wasm32"))]
    inventory::submit! {
      FunctionDescriptor {
        name: stringify!($struct_name),
        ptr: $struct_name::new,
      }
    }
  };} 

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
  ($lib:ident, $registrar:tt, $arg:expr, $($lhs_type:ident, $($target_type:ident, $value_string:tt),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            // Scalar Scalar
            #[cfg(all(feature = $value_string))]
            (Value::$lhs_type(lhs), Value::$lhs_type(rhs)) => {
              $registrar!([<$lib SS>], $target_type, $value_string);
              Ok(Box::new([<$lib SS>]{lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new($target_type::default()) }))
            },
            // Scalar Matrix
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            (Value::$lhs_type(lhs), Value::[<Matrix $lhs_type>](Matrix::Matrix1(rhs))) => {
              $registrar!([<$lib SM1>], $target_type, $value_string);
              Ok(Box::new([<$lib SM1>]{lhs, rhs, out: Ref::new(Matrix1::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2"))]
            (Value::$lhs_type(lhs), Value::[<Matrix $lhs_type>](Matrix::Matrix2(rhs))) => {
              $registrar!([<$lib SM2>], $target_type, $value_string);
              Ok(Box::new([<$lib SM2>]{lhs, rhs, out: Ref::new(Matrix2::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            (Value::$lhs_type(lhs), Value::[<Matrix $lhs_type>](Matrix::Matrix3(rhs))) => {
              $registrar!([<$lib SM3>], $target_type, $value_string);
              Ok(Box::new([<$lib SM3>]{lhs, rhs, out: Ref::new(Matrix3::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            (Value::$lhs_type(lhs), Value::[<Matrix $lhs_type>](Matrix::Matrix4(rhs))) => {
              $registrar!([<$lib SM4>], $target_type, $value_string);
              Ok(Box::new([<$lib SM4>]{lhs, rhs, out: Ref::new(Matrix4::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            (Value::$lhs_type(lhs), Value::[<Matrix $lhs_type>](Matrix::Matrix2x3(rhs))) => {
              $registrar!([<$lib SM2x3>], $target_type, $value_string);
              Ok(Box::new([<$lib SM2x3>]{lhs, rhs, out: Ref::new(Matrix2x3::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            (Value::$lhs_type(lhs), Value::[<Matrix $lhs_type>](Matrix::Matrix3x2(rhs))) => {
              $registrar!([<$lib SM3x2>], $target_type, $value_string);
              Ok(Box::new([<$lib SM3x2>]{lhs, rhs, out: Ref::new(Matrix3x2::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            (Value::$lhs_type(lhs), Value::[<Matrix $lhs_type>](Matrix::DMatrix(rhs))) => {
              let (rows,cols) = {rhs.borrow().shape()};
              $registrar!([<$lib SMD>], $target_type, $value_string);
              Ok(Box::new([<$lib SMD>]{lhs, rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
            },   
            // Scalar Row
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            (Value::$lhs_type(lhs), Value::[<Matrix $lhs_type>](Matrix::RowVector2(rhs))) => {
              $registrar!([<$lib SR2>], $target_type, $value_string);
              Ok(Box::new([<$lib SR2>]{lhs, rhs, out: Ref::new(RowVector2::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            (Value::$lhs_type(lhs), Value::[<Matrix $lhs_type>](Matrix::RowVector3(rhs))) => {
              $registrar!([<$lib SR3>], $target_type, $value_string);
              Ok(Box::new([<$lib SR3>]{lhs, rhs, out: Ref::new(RowVector3::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            (Value::$lhs_type(lhs), Value::[<Matrix $lhs_type>](Matrix::RowVector4(rhs))) => {
              $registrar!([<$lib SR4>], $target_type, $value_string);
              Ok(Box::new([<$lib SR4>]{lhs, rhs, out: Ref::new(RowVector4::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            (Value::$lhs_type(lhs), Value::[<Matrix $lhs_type>](Matrix::RowDVector(rhs))) => {
              $registrar!([<$lib SRD>], $target_type, $value_string);
              Ok(Box::new([<$lib SRD>]{lhs, rhs: rhs.clone(), out: Ref::new(RowDVector::from_element(rhs.borrow().len(),$target_type::default()))}))
            },
            // Scalar Vector
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            (Value::$lhs_type(lhs), Value::[<Matrix $lhs_type>](Matrix::Vector2(rhs))) => {
              $registrar!([<$lib SV2>], $target_type, $value_string);
              Ok(Box::new([<$lib SV2>]{lhs, rhs, out: Ref::new(Vector2::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            (Value::$lhs_type(lhs), Value::[<Matrix $lhs_type>](Matrix::Vector3(rhs))) => {
              $registrar!([<$lib SV3>], $target_type, $value_string);
              Ok(Box::new([<$lib SV3>]{lhs, rhs, out: Ref::new(Vector3::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            (Value::$lhs_type(lhs), Value::[<Matrix $lhs_type>](Matrix::Vector4(rhs))) => {
              $registrar!([<$lib SV4>], $target_type, $value_string);
              Ok(Box::new([<$lib SV4>]{lhs, rhs, out: Ref::new(Vector4::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            (Value::$lhs_type(lhs), Value::[<Matrix $lhs_type>](Matrix::DVector(rhs))) => {
              $registrar!([<$lib SVD>], $target_type, $value_string);
              Ok(Box::new([<$lib SVD>]{lhs, rhs: rhs.clone(), out: Ref::new(DVector::from_element(rhs.borrow().len(),$target_type::default()))}))
            },
            // Matrix Scalar
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix1(lhs)),Value::$lhs_type(rhs)) => {
              $registrar!([<$lib M1S>], $target_type, $value_string);
              Ok(Box::new([<$lib M1S>]{lhs, rhs, out: Ref::new(Matrix1::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix2(lhs)),Value::$lhs_type(rhs)) => {
              $registrar!([<$lib M2S>], $target_type, $value_string);
              Ok(Box::new([<$lib M2S>]{lhs, rhs, out: Ref::new(Matrix2::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix3(lhs)),Value::$lhs_type(rhs)) => {
              $registrar!([<$lib M3S>], $target_type, $value_string);
              Ok(Box::new([<$lib M3S>]{lhs, rhs, out: Ref::new(Matrix3::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix4(lhs)),Value::$lhs_type(rhs)) => {
              $registrar!([<$lib M4S>], $target_type, $value_string);
              Ok(Box::new([<$lib M4S>]{lhs, rhs, out: Ref::new(Matrix4::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix2x3(lhs)),Value::$lhs_type(rhs)) => {
              $registrar!([<$lib M2x3S>], $target_type, $value_string);
              Ok(Box::new([<$lib M2x3S>]{lhs, rhs, out: Ref::new(Matrix2x3::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix3x2(lhs)),Value::$lhs_type(rhs)) => {
              $registrar!([<$lib M3x2S>], $target_type, $value_string);
              Ok(Box::new([<$lib M3x2S>]{lhs, rhs, out: Ref::new(Matrix3x2::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            (Value::[<Matrix $lhs_type>](Matrix::DMatrix(lhs)),Value::$lhs_type(rhs)) => {
              let (rows,cols) = {lhs.borrow().shape()};
              $registrar!([<$lib MDS>], $target_type, $value_string);
              Ok(Box::new([<$lib MDS>]{lhs: lhs.clone(), rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
            },
            // Row Scalar
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowVector2(lhs)),Value::$lhs_type(rhs)) => {
              $registrar!([<$lib R2S>], $target_type, $value_string);
              Ok(Box::new([<$lib R2S>]{lhs, rhs, out: Ref::new(RowVector2::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowVector3(lhs)),Value::$lhs_type(rhs)) => {
              $registrar!([<$lib R3S>], $target_type, $value_string);
              Ok(Box::new([<$lib R3S>]{lhs, rhs, out: Ref::new(RowVector3::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowVector4(lhs)),Value::$lhs_type(rhs)) => {
              $registrar!([<$lib R4S>], $target_type, $value_string);
              Ok(Box::new([<$lib R4S>]{lhs, rhs, out: Ref::new(RowVector4::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowDVector(lhs)),Value::$lhs_type(rhs)) => {
              $registrar!([<$lib RDS>], $target_type, $value_string);
              Ok(Box::new([<$lib RDS>]{lhs: lhs.clone(), rhs, out: Ref::new(RowDVector::from_element(lhs.borrow().len(),$target_type::default()))}))
            },
            // Vector Scalar
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            (Value::[<Matrix $lhs_type>](Matrix::Vector2(lhs)),Value::$lhs_type(rhs)) => {
              $registrar!([<$lib V2S>], $target_type, $value_string);
              Ok(Box::new([<$lib V2S>]{lhs, rhs, out: Ref::new(Vector2::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            (Value::[<Matrix $lhs_type>](Matrix::Vector3(lhs)),Value::$lhs_type(rhs)) => {
              $registrar!([<$lib V3S>], $target_type, $value_string);
              Ok(Box::new([<$lib V3S>]{lhs, rhs, out: Ref::new(Vector3::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            (Value::[<Matrix $lhs_type>](Matrix::Vector4(lhs)),Value::$lhs_type(rhs)) => {
              $registrar!([<$lib V4S>], $target_type, $value_string);
              Ok(Box::new([<$lib V4S>]{lhs, rhs, out: Ref::new(Vector4::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            (Value::[<Matrix $lhs_type>](Matrix::DVector(lhs)),Value::$lhs_type(rhs)) => {
              $registrar!([<$lib VDS>], $target_type, $value_string);
              Ok(Box::new([<$lib VDS>]{lhs: lhs.clone(), rhs, out: Ref::new(DVector::from_element(lhs.borrow().len(),$target_type::default()))}))
            },
            // Matrix Matrix
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix1(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix1(rhs))) => {
              $registrar!([<$lib M1M1>], $target_type, $value_string);
              Ok(Box::new([<$lib M1M1>]{lhs, rhs, out: Ref::new(Matrix1::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix2(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix2(rhs))) => {
              $registrar!([<$lib M2M2>], $target_type, $value_string);
              Ok(Box::new([<$lib M2M2>]{lhs, rhs, out: Ref::new(Matrix2::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix3(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix3(rhs))) => {
              $registrar!([<$lib M3M3>], $target_type, $value_string);
              Ok(Box::new([<$lib M3M3>]{lhs, rhs, out: Ref::new(Matrix3::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix4(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix4(rhs))) => {
              $registrar!([<$lib M4M4>], $target_type, $value_string);
              Ok(Box::new([<$lib M4M4>]{lhs, rhs, out: Ref::new(Matrix4::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix2x3(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix2x3(rhs))) => {
              $registrar!([<$lib M2x3M2x3>], $target_type, $value_string);
              Ok(Box::new([<$lib M2x3M2x3>]{lhs, rhs, out: Ref::new(Matrix2x3::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix3x2(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix3x2(rhs))) => {
              $registrar!([<$lib M3x2M3x2>], $target_type, $value_string);
              Ok(Box::new([<$lib M3x2M3x2>]{lhs, rhs, out: Ref::new(Matrix3x2::from_element($target_type::default()))}))
            },
              #[cfg(all(feature = $value_string, feature = "matrixd"))]
            (Value::[<Matrix $lhs_type>](Matrix::DMatrix(lhs)), Value::[<Matrix $lhs_type>](Matrix::DMatrix(rhs))) => {
              let (rows,cols) = {lhs.borrow().shape()};
              Ok(Box::new([<$lib MDMD>]{lhs, rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
            },
            // Row Row
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowVector2(lhs)), Value::[<Matrix $lhs_type>](Matrix::RowVector2(rhs))) => {
              $registrar!([<$lib R2R2>], $target_type, $value_string);
              Ok(Box::new([<$lib R2R2>]{lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(RowVector2::from_element($target_type::default())) }))
            },
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowVector3(lhs)), Value::[<Matrix $lhs_type>](Matrix::RowVector3(rhs))) => {
              $registrar!([<$lib R3R3>], $target_type, $value_string);
              Ok(Box::new([<$lib R3R3>]{lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(RowVector3::from_element($target_type::default())) }))
            },
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowVector4(lhs)), Value::[<Matrix $lhs_type>](Matrix::RowVector4(rhs))) => {
              $registrar!([<$lib R4R4>], $target_type, $value_string);
              Ok(Box::new([<$lib R4R4>]{lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(RowVector4::from_element($target_type::default())) }))
            },
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowDVector(lhs)), Value::[<Matrix $lhs_type>](Matrix::RowDVector(rhs))) => {
              $registrar!([<$lib RDRD>], $target_type, $value_string);
              Ok(Box::new([<$lib RDRD>]{lhs: lhs.clone(), rhs, out: Ref::new(RowDVector::from_element(lhs.borrow().len(),$target_type::default())) }))
            },
            // Vector Vector
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            (Value::[<Matrix $lhs_type>](Matrix::Vector2(lhs)), Value::[<Matrix $lhs_type>](Matrix::Vector2(rhs))) => {
              $registrar!([<$lib V2V2>], $target_type, $value_string);
              Ok(Box::new([<$lib V2V2>]{lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Vector2::from_element($target_type::default())) }))
            },
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            (Value::[<Matrix $lhs_type>](Matrix::Vector3(lhs)), Value::[<Matrix $lhs_type>](Matrix::Vector3(rhs))) => {
              $registrar!([<$lib V3V3>], $target_type, $value_string);
              Ok(Box::new([<$lib V3V3>]{lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Vector3::from_element($target_type::default())) }))
            },
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            (Value::[<Matrix $lhs_type>](Matrix::Vector4(lhs)), Value::[<Matrix $lhs_type>](Matrix::Vector4(rhs))) => {
              $registrar!([<$lib V4V4>], $target_type, $value_string);
              Ok(Box::new([<$lib V4V4>]{lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Vector4::from_element($target_type::default())) }))
            },
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            (Value::[<Matrix $lhs_type>](Matrix::DVector(lhs)), Value::[<Matrix $lhs_type>](Matrix::DVector(rhs))) => {
              $registrar!([<$lib VDVD>], $target_type, $value_string);
              Ok(Box::new([<$lib VDVD>]{lhs: lhs.clone(), rhs, out: Ref::new(DVector::from_element(lhs.borrow().len(),$target_type::default())) }))
            },
            // Matrix Vector
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector2"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix2(lhs)),Value::[<Matrix $lhs_type>](Matrix::Vector2(rhs))) => {
              $registrar!([<$lib M2V2>], $target_type, $value_string);
              Ok(Box::new([<$lib M2V2>]{lhs, rhs, out: Ref::new(Matrix2::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3", feature = "vector3"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix3(lhs)),Value::[<Matrix $lhs_type>](Matrix::Vector3(rhs))) => {
              $registrar!([<$lib M3V3>], $target_type, $value_string);
              Ok(Box::new([<$lib M3V3>]{lhs, rhs, out: Ref::new(Matrix3::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "vector2"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix2x3(lhs)),Value::[<Matrix $lhs_type>](Matrix::Vector2(rhs))) => {
              $registrar!([<$lib M2x3V2>], $target_type, $value_string);
              Ok(Box::new([<$lib M2x3V2>]{lhs, rhs, out: Ref::new(Matrix2x3::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "vector3"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix3x2(lhs)),Value::[<Matrix $lhs_type>](Matrix::Vector3(rhs))) => {
              $registrar!([<$lib M3x2V3>], $target_type, $value_string);
              Ok(Box::new([<$lib M3x2V3>]{lhs, rhs, out: Ref::new(Matrix3x2::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix4", feature = "vector4"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix4(lhs)),Value::[<Matrix $lhs_type>](Matrix::Vector4(rhs))) => {
              $registrar!([<$lib M4V4>], $target_type, $value_string);
              Ok(Box::new([<$lib M4V4>]{lhs, rhs, out: Ref::new(Matrix4::from_element($target_type::default()))}))
            },
            // Vector Matrix
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "row_vector2"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix2(lhs)),Value::[<Matrix $lhs_type>](Matrix::RowVector2(rhs))) => {
              $registrar!([<$lib M2R2>], $target_type, $value_string);
              Ok(Box::new([<$lib M2R2>]{lhs, rhs, out: Ref::new(Matrix2::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3", feature = "row_vector3"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix3(lhs)),Value::[<Matrix $lhs_type>](Matrix::RowVector3(rhs))) => {
              $registrar!([<$lib M3R3>], $target_type, $value_string);
              Ok(Box::new([<$lib M3R3>]{lhs, rhs, out: Ref::new(Matrix3::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "row_vector3"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix2x3(lhs)),Value::[<Matrix $lhs_type>](Matrix::RowVector3(rhs))) => {
              $registrar!([<$lib M2x3R3>], $target_type, $value_string);
              Ok(Box::new([<$lib M2x3R3>]{lhs, rhs, out: Ref::new(Matrix2x3::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "row_vector2"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix3x2(lhs)),Value::[<Matrix $lhs_type>](Matrix::RowVector2(rhs))) => {
              $registrar!([<$lib M3x2R2>], $target_type, $value_string);
              Ok(Box::new([<$lib M3x2R2>]{lhs, rhs, out: Ref::new(Matrix3x2::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix4", feature = "row_vector4"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix4(lhs)),Value::[<Matrix $lhs_type>](Matrix::RowVector4(rhs))) => {
              $registrar!([<$lib M4R4>], $target_type, $value_string);
              Ok(Box::new([<$lib M4R4>]{lhs, rhs, out: Ref::new(Matrix4::from_element($target_type::default()))}))
            },
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
                _ => {
                  return Err(
                    MechError2::new(
                      DimensionMismatch { dims: vec![rows, cols, rhs_shape[0], rhs_shape[1]] },
                      None
                    ).with_compiler_loc()
                  );
                }
              }
              match rhs {
                #[cfg(feature = "vector2")]
                Matrix::Vector2(rhs) => {
                  $registrar!([<$lib MDV2>], $target_type, $value_string);
                  Ok(Box::new([<$lib MDV2>]{lhs: lhs.clone(), rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
                },
                #[cfg(feature = "vector3")]
                Matrix::Vector3(rhs) => {
                  $registrar!([<$lib MDV3>], $target_type, $value_string);
                  Ok(Box::new([<$lib MDV3>]{lhs: lhs.clone(), rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
                },
                #[cfg(feature = "vector4")]
                Matrix::Vector4(rhs) => {
                  $registrar!([<$lib MDV4>], $target_type, $value_string);
                  Ok(Box::new([<$lib MDV4>]{lhs: lhs.clone(), rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
                },
                #[cfg(feature = "vectord")]
                Matrix::DVector(rhs) => {
                  $registrar!([<$lib MDVD>], $target_type, $value_string);
                  Ok(Box::new([<$lib MDVD>]{lhs: lhs.clone(), rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
                },
                #[cfg(feature = "row_vector2")]
                Matrix::RowVector2(rhs) => {
                  $registrar!([<$lib MDR2>], $target_type, $value_string);
                  Ok(Box::new([<$lib MDR2>]{lhs: lhs.clone(), rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
                },
                #[cfg(feature = "row_vector3")]
                Matrix::RowVector3(rhs) => {
                  $registrar!([<$lib MDR3>], $target_type, $value_string);
                  Ok(Box::new([<$lib MDR3>]{lhs: lhs.clone(), rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
                },
                #[cfg(feature = "row_vector4")]
                Matrix::RowVector4(rhs) => {
                  $registrar!([<$lib MDR4>], $target_type, $value_string);
                  Ok(Box::new([<$lib MDR4>]{lhs: lhs.clone(), rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
                },
                #[cfg(feature = "row_vectord")]
                Matrix::RowDVector(rhs) => {
                  $registrar!([<$lib MDRD>], $target_type, $value_string);
                  Ok(Box::new([<$lib MDRD>]{lhs: lhs.clone(), rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
                },
                x => {
                  return Err(
                    MechError2::new(
                      DimensionMismatch { dims: vec![rows, cols, rhs_shape[0], rhs_shape[1]] },
                      None
                    ).with_compiler_loc()
                  );
                }
              }
            },
            // Vector Matrix
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector2"))]
            (Value::[<Matrix $lhs_type>](Matrix::Vector2(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix2(rhs))) => {
              $registrar!([<$lib V2M2>], $target_type, $value_string);
              Ok(Box::new([<$lib V2M2>]{lhs, rhs, out: Ref::new(Matrix2::from_element($target_type::default()))}))
            },         
            #[cfg(all(feature = $value_string, feature = "matrix3", feature = "vector3"))]
            (Value::[<Matrix $lhs_type>](Matrix::Vector3(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix3(rhs))) => {
              $registrar!([<$lib V3M3>], $target_type, $value_string);
              Ok(Box::new([<$lib V3M3>]{lhs, rhs, out: Ref::new(Matrix3::from_element($target_type::default()))}))
            },         
            #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "vector2"))]
            (Value::[<Matrix $lhs_type>](Matrix::Vector2(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix2x3(rhs))) => {
              $registrar!([<$lib V2M2x3>], $target_type, $value_string);
              Ok(Box::new([<$lib V2M2x3>]{lhs, rhs, out: Ref::new(Matrix2x3::from_element($target_type::default()))}))
            },         
            #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "vector3"))]
            (Value::[<Matrix $lhs_type>](Matrix::Vector3(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix3x2(rhs))) => {
              $registrar!([<$lib V3M3x2>], $target_type, $value_string);
              Ok(Box::new([<$lib V3M3x2>]{lhs, rhs, out: Ref::new(Matrix3x2::from_element($target_type::default()))}))
            },                     
            #[cfg(all(feature = $value_string, feature = "matrix4", feature = "vector4"))]
            (Value::[<Matrix $lhs_type>](Matrix::Vector4(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix4(rhs))) => {
              $registrar!([<$lib V4M4>], $target_type, $value_string);
              Ok(Box::new([<$lib V4M4>]{lhs, rhs, out: Ref::new(Matrix4::from_element($target_type::default()))}))
            },                     
            // Row Matrix     
            #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "matrix2"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowVector2(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix2(rhs))) => {
              $registrar!([<$lib R2M2>], $target_type, $value_string);
              Ok(Box::new([<$lib R2M2>]{lhs, rhs, out: Ref::new(Matrix2::from_element($target_type::default()))}))
            },         
            #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "matrix3"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowVector3(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix3(rhs))) => {
              $registrar!([<$lib R3M3>], $target_type, $value_string);
              Ok(Box::new([<$lib R3M3>]{lhs, rhs, out: Ref::new(Matrix3::from_element($target_type::default()))}))
            },         
            #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "matrix2x3"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowVector3(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix2x3(rhs))) => {
              $registrar!([<$lib R3M2x3>], $target_type, $value_string);
              Ok(Box::new([<$lib R3M2x3>]{lhs, rhs, out: Ref::new(Matrix2x3::from_element($target_type::default()))}))
            },         
            #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "matrix3x2"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowVector2(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix3x2(rhs))) => {
              $registrar!([<$lib R2M3x2>], $target_type, $value_string);
              Ok(Box::new([<$lib R2M3x2>]{lhs, rhs, out: Ref::new(Matrix3x2::from_element($target_type::default()))}))
            },         
            #[cfg(all(feature = $value_string, feature = "row_vector4", feature = "matrix4"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowVector4(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix4(rhs))) => {
              $registrar!([<$lib R4M4>], $target_type, $value_string);
              Ok(Box::new([<$lib R4M4>]{lhs, rhs, out: Ref::new(Matrix4::from_element($target_type::default()))}))
            },
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
                _ => {
                  return Err(
                    MechError2::new(
                      DimensionMismatch { dims: vec![lhs_shape[0], lhs_shape[1], rows, cols] },
                      None
                    ).with_compiler_loc()
                  );
                }
              }
              match lhs {
                #[cfg(feature = "vector2")]
                Matrix::Vector2(lhs) => {
                  $registrar!([<$lib V2MD>], $target_type, $value_string);
                  Ok(Box::new([<$lib V2MD>]{lhs, rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
                },
                #[cfg(feature = "vector3")]
                Matrix::Vector3(lhs) => {
                  $registrar!([<$lib V3MD>], $target_type, $value_string);
                  Ok(Box::new([<$lib V3MD>]{lhs, rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
                },
                #[cfg(feature = "vector4")]
                Matrix::Vector4(lhs) => {
                  $registrar!([<$lib V4MD>], $target_type, $value_string);
                  Ok(Box::new([<$lib V4MD>]{lhs, rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
                },
                #[cfg(feature = "vectord")]
                Matrix::DVector(lhs) => {
                  $registrar!([<$lib VDMD>], $target_type, $value_string);
                  Ok(Box::new([<$lib VDMD>]{lhs, rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
                },
                #[cfg(feature = "row_vector2")]
                Matrix::RowVector2(lhs) => {
                  $registrar!([<$lib R2MD>], $target_type, $value_string);
                  Ok(Box::new([<$lib R2MD>]{lhs, rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
                },
                #[cfg(feature = "row_vector3")]
                Matrix::RowVector3(lhs) => {
                  $registrar!([<$lib R3MD>], $target_type, $value_string);
                  Ok(Box::new([<$lib R3MD>]{lhs, rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
                },
                #[cfg(feature = "row_vector4")]
                Matrix::RowVector4(lhs) => {
                  $registrar!([<$lib R4MD>], $target_type, $value_string);
                  Ok(Box::new([<$lib R4MD>]{lhs, rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
                },
                #[cfg(feature = "row_vectord")]
                Matrix::RowDVector(lhs) => {
                  $registrar!([<$lib RDMD>], $target_type, $value_string);
                  Ok(Box::new([<$lib RDMD>]{lhs, rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
                },
                _ => {
                  return Err(
                    MechError2::new(
                      DimensionMismatch { dims: vec![lhs_shape[0], lhs_shape[1], rows, cols] },
                      None
                    ).with_compiler_loc()
                  );
                }
              }
            }
          )+
        )+
        (lhs,rhs) => Err(MechError2::new(
          UnhandledFunctionArgumentKind2{arg: (lhs, rhs), fxn_name: stringify!($lib).to_string()},
          None
        ).with_compiler_loc()),
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
        x => Err(MechError2::new(
          UnhandledFunctionArgumentKind1{arg: x.clone(), fxn_name: stringify!($lib).to_string()},
          None
        ).with_compiler_loc()),
      }
    }
  }
}

#[macro_export]
macro_rules! impl_mech_binop_fxn {
  ($fxn_name:ident, $gen_fxn:tt, $fxn_string:tt) => {
    pub struct $fxn_name {}
    impl NativeFunctionCompiler for $fxn_name {
      fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
        if arguments.len() != 2 {
          return Err(MechError2::new(IncorrectNumberOfArguments { expected: 2, found: arguments.len() }, None).with_compiler_loc());
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
              (lhs, rhs) => Err(MechError2::new(
                  UnhandledFunctionArgumentKind2 { arg: (lhs, rhs), fxn_name: "combinatorics/n-choose-k".to_string() },
                  None
                ).with_compiler_loc()
              ),            
            }
          }
        }
      }
    }
    #[cfg(not(target_arch = "wasm32"))]
    inventory::submit! {
      FunctionCompilerDescriptor {
        name: $fxn_string,
        ptr: &$fxn_name{},
      }
    }
  };
}

#[macro_export]
macro_rules! impl_mech_urnop_fxn {
  ($fxn_name:ident, $gen_fxn:tt, $fxn_string:tt) => {
    pub struct $fxn_name {}
    impl NativeFunctionCompiler for $fxn_name {
      fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
        if arguments.len() != 1 {
          return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() }, None).with_compiler_loc());
        }
        let input = arguments[0].clone();
        match $gen_fxn(input.clone()) {
          Ok(fxn) => Ok(fxn),
          Err(_) => {
            match (input) {
              (Value::MutableReference(input)) => {$gen_fxn(input.borrow().clone())}
              x => Err(MechError2::new(
                  UnhandledFunctionArgumentKind1 { arg: x.clone(), fxn_name: "combinatorics/n-choose-k".to_string() },
                  None
                ).with_compiler_loc()
              ),
            }
          }
        }
      }
    }
    #[cfg(not(target_arch = "wasm32"))]
    inventory::submit! {
      FunctionCompilerDescriptor {
        name: $fxn_string,
        ptr: & $fxn_name{},
      }
    }
  }
}

#[cfg(feature = "functions")]
pub fn box_mech_fxn<T>(r: MResult<Box<T>>) -> MResult<Box<dyn MechFunction>>
where
  T: MechFunction + 'static,
{
  r.map(|x| x as Box<dyn MechFunction>)
}

#[macro_export]
macro_rules! register_assign {
  ($fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt, $row3:tt) => {
    paste! {
      register_descriptor! {
        FunctionDescriptor {
          name: concat!(stringify!($fxn_name), "<", $scalar_string , stringify!($row1), stringify!($row2), stringify!($row3), ">") ,
          ptr: $fxn_name::<$scalar,$row1<$scalar>,$row2<$scalar>,$row3<usize>>::new,
        }
      }
    }
  };
}

#[macro_export]
macro_rules! register_assign_s {
  ($fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt) => {
    paste! {
      register_descriptor! {
        FunctionDescriptor {
          name: concat!(stringify!($fxn_name), "<", $scalar_string , stringify!($row1), stringify!($row2), ">") ,
          ptr: $fxn_name::<$scalar,$row1<$scalar>,$row2<usize>>::new,
        }
      }
    }
  };
}

#[macro_export]
macro_rules! register_assign_srr {
  ($fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt, $row3:tt) => {
    paste! {
      register_descriptor! {
        FunctionDescriptor {
          name: concat!(stringify!($fxn_name), "<", $scalar_string , stringify!($row1), stringify!($row2), stringify!($row3), ">") ,
          ptr: $fxn_name::<$scalar,$row1<$scalar>,$row2<usize>,$row3<usize>>::new,
        }
      }
    }
  };
}

#[macro_export]
macro_rules! register_assign_srr_b {
  ($fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt, $row3:tt) => {
    paste! {
      register_descriptor! {
        FunctionDescriptor {
          name: concat!(stringify!($fxn_name), "<", $scalar_string , stringify!($row1), stringify!($row2), stringify!($row3), ">") ,
          ptr: $fxn_name::<$scalar,$row1<$scalar>,$row2<bool>,$row3<bool>>::new,
        }
      }
    }
  };
}

#[macro_export]
macro_rules! register_assign_srr_bu {
  ($fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt, $row3:tt) => {
    paste! {
      register_descriptor! {
        FunctionDescriptor {
          name: concat!(stringify!($fxn_name), "<", $scalar_string , stringify!($row1), stringify!($row2), stringify!($row3), ">") ,
          ptr: $fxn_name::<$scalar,$row1<$scalar>,$row2<bool>,$row3<usize>>::new,
        }
      }
    }
  };
}

#[macro_export]
macro_rules! register_assign_srr_ub {
  ($fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt, $row3:tt) => {
    paste! {
      register_descriptor! {
        FunctionDescriptor {
          name: concat!(stringify!($fxn_name), "<", $scalar_string , stringify!($row1), stringify!($row2), stringify!($row3), ">") ,
          ptr: $fxn_name::<$scalar,$row1<$scalar>,$row2<usize>,$row3<bool>>::new,
        }
      }
    }
  };
}

#[macro_export]
macro_rules! register_assign_srr_b2 {
  ($fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt, $row3:tt, $row4:tt) => {
    paste! {
      register_descriptor! {
        FunctionDescriptor {
          name: concat!(stringify!($fxn_name), "<", $scalar_string , stringify!($row1), stringify!($row2), stringify!($row3), stringify!($row4), ">") ,
          ptr: $fxn_name::<$scalar,$row1<$scalar>,$row2<$scalar>,$row3<bool>,$row4<bool>>::new,
        }
      }
    }
  };
}

#[macro_export]
macro_rules! register_assign_srr_bu2 {
  ($fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt, $row3:tt, $row4:tt) => {
    paste! {
      register_descriptor! {
        FunctionDescriptor {
          name: concat!(stringify!($fxn_name), "<", $scalar_string , stringify!($row1), stringify!($row2), stringify!($row3), stringify!($row4), ">") ,
          ptr: $fxn_name::<$scalar,$row1<$scalar>,$row2<$scalar>,$row3<bool>,$row4<usize>>::new,
        }
      }
    }
  };
}

#[macro_export]
macro_rules! register_assign_srr_ub2 {
  ($fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt, $row3:tt, $row4:tt) => {
    paste! {
      register_descriptor! {
        FunctionDescriptor {
          name: concat!(stringify!($fxn_name), "<", $scalar_string , stringify!($row1), stringify!($row2), stringify!($row3), stringify!($row4), ">") ,
          ptr: $fxn_name::<$scalar,$row1<$scalar>,$row2<$scalar>,$row3<usize>,$row4<bool>>::new,
        }
      }
    }
  };
}

#[macro_export]
macro_rules! register_assign_srr2 {
  ($fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt, $row3:tt, $row4:tt) => {
    paste! {
      register_descriptor! {
        FunctionDescriptor {
          name: concat!(stringify!($fxn_name), "<", $scalar_string , stringify!($row1), stringify!($row2), stringify!($row3), stringify!($row4), ">") ,
          ptr: $fxn_name::<$scalar,$row1<$scalar>,$row2<$scalar>,$row3<usize>,$row4<usize>>::new,
        }
      }
    }
  };
}

#[macro_export]
macro_rules! register_assign_s1 {
  ($fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt) => {
    paste! {
      register_descriptor! {
        FunctionDescriptor {
          name: concat!(stringify!($fxn_name), "<", $scalar_string , stringify!($row1), ">") ,
          ptr: $fxn_name::<$scalar,$row1<$scalar>>::new,
        }
      }
    }
  };
}

#[macro_export]
macro_rules! register_assign_s2 {
  ($fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt) => {
    paste! {
      register_descriptor! {
        FunctionDescriptor {
          name: concat!(stringify!($fxn_name), "<", $scalar_string , stringify!($row1), stringify!($row2), ">") ,
          ptr: $fxn_name::<$scalar,$row1<$scalar>,$row2<$scalar>>::new,
        }
      }
    }
  };
}

#[macro_export]
macro_rules! register_assign_b {
  ($fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt, $row3:tt) => {
    paste! {
      register_descriptor! {
        FunctionDescriptor {
          name: concat!(stringify!($fxn_name), "<", $scalar_string , stringify!($row1), stringify!($row2), stringify!($row3), ">") ,
          ptr: $fxn_name::<$scalar,$row1<$scalar>,$row2<$scalar>,$row3<bool>>::new,
        }
      }
    }
  };
}

#[macro_export]
macro_rules! register_assign_s_b {
  ($fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt, $row2:tt) => {
    paste! {
      register_descriptor! {
        FunctionDescriptor {
          name: concat!(stringify!($fxn_name), "<", $scalar_string , stringify!($row1), stringify!($row2), ">") ,
          ptr: $fxn_name::<$scalar,$row1<$scalar>,$row2<bool>>::new,
        }
      }
    }
  };
}

#[macro_export]
macro_rules! impl_assign_fxn {
  ($op:tt, $fxn_name:ident, $arg:expr, $value_kind:ident, $value_string:tt) => {{
    let mut res: MResult<_> = Err(MechError2::new(
      GenericError {
        msg: "No matching types found".to_string(),
      },
      None,
    ).with_compiler_loc());
    
    #[cfg(feature = "row_vector2")]
    {
      res = res.or_else(|_| $op!($fxn_name, RowVector2, &$arg, $value_kind, $value_string));
    }

    #[cfg(feature = "row_vector3")]
    {
      res = res.or_else(|_| $op!($fxn_name, RowVector3, &$arg, $value_kind, $value_string));
    }

    #[cfg(feature = "row_vector4")]
    {
      res = res.or_else(|_| $op!($fxn_name, RowVector4, &$arg, $value_kind, $value_string));
    }

    #[cfg(feature = "vector2")]
    {
      res = res.or_else(|_| $op!($fxn_name, Vector2, &$arg, $value_kind, $value_string));
    }

    #[cfg(feature = "vector3")]
    {
      res = res.or_else(|_| $op!($fxn_name, Vector3, &$arg, $value_kind, $value_string));
    }

    #[cfg(feature = "vector4")]
    {
      res = res.or_else(|_| $op!($fxn_name, Vector4, &$arg, $value_kind, $value_string));
    }

    #[cfg(feature = "matrix1")]
    {
      res = res.or_else(|_| $op!($fxn_name, Matrix1, &$arg, $value_kind, $value_string));
    }

    #[cfg(feature = "matrix2")]
    {
      res = res.or_else(|_| $op!($fxn_name, Matrix2, &$arg, $value_kind, $value_string));
    }

    #[cfg(feature = "matrix3")]
    {
      res = res.or_else(|_| $op!($fxn_name, Matrix3, &$arg, $value_kind, $value_string));
    }

    #[cfg(feature = "matrix4")]
    {
      res = res.or_else(|_| $op!($fxn_name, Matrix4, &$arg, $value_kind, $value_string));
    }

    #[cfg(feature = "matrix2x3")]
    {
      res = res.or_else(|_| $op!($fxn_name, Matrix2x3, &$arg, $value_kind, $value_string));
    }

    #[cfg(feature = "matrix3x2")]
    {
      res = res.or_else(|_| $op!($fxn_name, Matrix3x2, &$arg, $value_kind, $value_string));
    }

    #[cfg(feature = "matrixd")]
    {
      res = res.or_else(|_| $op!($fxn_name, DMatrix, &$arg, $value_kind, $value_string));
    }

    #[cfg(feature = "row_vectord")]
    {
      res = res.or_else(|_| $op!($fxn_name, RowDVector, &$arg, $value_kind, $value_string));
    }

    #[cfg(feature = "vectord")]
    {
      res = res.or_else(|_| $op!($fxn_name, DVector, &$arg, $value_kind, $value_string));
    }
    let (ref source, ref ixes, ref sink) = &$arg;
    res.map_err(|_| MechError2::new(
      UnhandledFunctionArgumentIxes {
        arg: (sink.clone(), ixes.to_vec(), source.clone()),
        fxn_name: stringify!($fxn_name).to_string(),
      },
      None,
    ).with_compiler_loc())
  }}
}

#[macro_export]
macro_rules! impl_assign_scalar_arms {
  ($fxn_name:ident, $shape:tt, $arg:expr, $value_kind:ident, $value_string:tt) => {
    paste! {
      match $arg {
        // Scalar source
        #[cfg(all(feature = $value_string))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[Value::Index(ix)], Value::[<$value_kind:camel>](source)) => {
          register_assign_s1!([<$fxn_name S>], $value_kind, $value_string, $shape);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        _ => Err(MechError2::new(
          UnhandledFunctionArgumentKind2 { arg: $arg.clone(), fxn_name: stringify!($fxn_name).to_string() },
          None
        ).with_compiler_loc()),
      }
    };
  };
}

#[macro_export]
macro_rules! impl_assign_all_arms {
  ($fxn_name:ident, $shape:tt, $arg:expr, $value_kind:ident, $value_string:tt) => {
    paste! {
      match $arg {
        #[cfg(feature = $value_string)]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[Value::IndexAll], Value::[<$value_kind:camel>](source)) => {
          register_assign_s1!([<$fxn_name S>], $value_kind, $value_string, $shape);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        _ => Err(MechError2::new(
          UnhandledFunctionArgumentKind2 { arg: $arg.clone(), fxn_name: stringify!($fxn_name).to_string() },
          None
        ).with_compiler_loc()),
      }
    };
  };
}

#[macro_export]
macro_rules! impl_assign_scalar_scalar_arms {
  ($fxn_name:ident, $shape:tt, $arg:expr, $value_kind:ident, $value_string:tt) => {
    paste! {
      match $arg {
        #[cfg(feature = $value_string)]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[Value::Index(ix1), Value::Index(ix2)], Value::[<$value_kind:camel>](source)) => {
          register_assign_s1!([<$fxn_name S>], $value_kind, $value_string, $shape);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "matrixd", not(feature = "matrix1")))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[Value::MatrixIndex(Matrix::DMatrix(ix1)), Value::Index(ix2)], Value::[<$value_kind:camel>](source)) => {
          register_assign_s!([<$fxn_name MD>], $value_kind, $value_string, $shape, DMatrix);
          box_mech_fxn(Ok(Box::new([<$fxn_name MD>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))           
        },
        _ => Err(MechError2::new(
          UnhandledFunctionArgumentKind2 { arg: $arg.clone(), fxn_name: stringify!($fxn_name).to_string() },
          None
        ).with_compiler_loc()),
      }
    };
  };
}

#[macro_export]
macro_rules! impl_set_range_arms {
  ($fxn_name:ident, $shape:tt, $arg:expr, $value_kind:ident, $value_string:tt) => {
    paste! {
      match $arg {
        // Scalar source
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[Value::MatrixIndex(Matrix::Matrix1(ix))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s!([<$fxn_name S>], $value_kind, $value_string, $shape, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[Value::MatrixIndex(Matrix::Vector2(ix))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s!([<$fxn_name S>], $value_kind, $value_string, $shape, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[Value::MatrixIndex(Matrix::Vector3(ix))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s!([<$fxn_name S>], $value_kind, $value_string, $shape, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[Value::MatrixIndex(Matrix::Vector4(ix))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s!([<$fxn_name S>], $value_kind, $value_string, $shape, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[Value::MatrixIndex(Matrix::DVector(ix))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s!([<$fxn_name S>], $value_kind, $value_string, $shape, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        // Vector source
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Matrix1(ix))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix1(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, Matrix1, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector4(ix))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix2(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::DVector(ix))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix3(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::DVector(ix))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix4(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, Matrix4, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::DVector(ix))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2x3, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::DVector(ix))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3x2, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::DVector(ix))], Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::DVector(ix))], Value::[<Matrix $value_kind:camel>](Matrix::DVector(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, DVector, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::DVector(ix))], Value::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector2(ix))], Value::[<Matrix $value_kind:camel>](Matrix::Vector2(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, Vector2, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector3(ix))], Value::[<Matrix $value_kind:camel>](Matrix::Vector3(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, Vector3, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector4(ix))], Value::[<Matrix $value_kind:camel>](Matrix::Vector4(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, Vector4, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector2(ix))], Value::[<Matrix $value_kind:camel>](Matrix::RowVector2(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, RowVector2, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector3(ix))], Value::[<Matrix $value_kind:camel>](Matrix::RowVector3(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, RowVector3, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector4(ix))], Value::[<Matrix $value_kind:camel>](Matrix::RowVector4(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, RowVector4, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        (source, ixes, sink) => Err(MechError2::new(
          UnhandledFunctionArgumentIxes { arg: (source.clone(), ixes.to_vec(), sink.clone()), fxn_name: stringify!($fxn_name).to_string() },
          None
        ).with_compiler_loc()),
      }
    };
  };
}

#[macro_export]
macro_rules! impl_assign_all_arms_b {
  ($fxn_name:ident, $shape:tt, $arg:expr, $value_kind:ident, $value_string:tt) => {
    paste! {
      match $arg {
        #[cfg(feature = $value_string)]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[Value::Bool(ix)], Value::[<$value_kind:camel>](source)) => {
          register_assign_s1!([<$fxn_name B>], $value_kind, $value_string, $shape);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        // Vector source, must have equal size to output
        #[cfg(feature = $value_string)]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::Bool(ix)], Value::[<Matrix $value_kind:camel>](Matrix::$shape(source))) => {
          register_assign_s2!([<$fxn_name VB>], $value_kind, $value_string, $shape, $shape);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        (sink, ixes, source) => Err(MechError2::new(
          UnhandledFunctionArgumentIxes { arg: (sink.clone(), ixes.to_vec(), source.clone()), fxn_name: stringify!($fxn_name).to_string() },
          None
        ).with_compiler_loc()),
      }
    };
  };
}

#[macro_export]
macro_rules! impl_set_range_all_arms {
  ($fxn_name:ident, $shape:tt, $arg:expr, $value_kind:ident, $value_string:tt) => {
    paste! {
      match $arg {
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[Value::MatrixIndex(Matrix::Matrix1(ix)),Value::IndexAll], Value::[<$value_kind:camel>](source)) => {
          register_assign_s!([<$fxn_name S>], $value_kind, $value_string, $shape, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[Value::MatrixIndex(Matrix::Vector2(ix)),Value::IndexAll], Value::[<$value_kind:camel>](source)) => {
          register_assign_s!([<$fxn_name S>], $value_kind, $value_string, $shape, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[Value::MatrixIndex(Matrix::Vector3(ix)),Value::IndexAll], Value::[<$value_kind:camel>](source)) => {
          register_assign_s!([<$fxn_name S>], $value_kind, $value_string, $shape, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[Value::MatrixIndex(Matrix::Vector4(ix)),Value::IndexAll], Value::[<$value_kind:camel>](source)) => {
          register_assign_s!([<$fxn_name S>], $value_kind, $value_string, $shape, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<$value_kind:camel>](source)) => {
          register_assign_s!([<$fxn_name S>], $value_kind, $value_string, $shape, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Matrix1(ix)),Value::IndexAll], Value::[<Matrix $value_kind:camel>](Matrix::Matrix1(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, Matrix1, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector4(ix)),Value::IndexAll], Value::[<Matrix $value_kind:camel>](Matrix::Matrix2(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind:camel>](Matrix::Matrix3(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind:camel>](Matrix::Matrix4(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, Matrix4, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2x3, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3x2, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind:camel>](Matrix::DVector(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, DVector, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector2(ix)),Value::IndexAll], Value::[<Matrix $value_kind:camel>](Matrix::Vector2(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, Vector2, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector3(ix)),Value::IndexAll], Value::[<Matrix $value_kind:camel>](Matrix::Vector3(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, Vector3, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector4(ix)),Value::IndexAll], Value::[<Matrix $value_kind:camel>](Matrix::Vector4(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, Vector4, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector2(ix)),Value::IndexAll], Value::[<Matrix $value_kind:camel>](Matrix::RowVector2(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, RowVector2, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector3(ix)),Value::IndexAll], Value::[<Matrix $value_kind:camel>](Matrix::RowVector3(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, RowVector3, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector4(ix)),Value::IndexAll], Value::[<Matrix $value_kind:camel>](Matrix::RowVector4(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, RowVector4, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        (source, ixes, sink) => Err(MechError2::new(
          UnhandledFunctionArgumentIxes { arg: (source.clone(), ixes.to_vec(), sink.clone()), fxn_name: stringify!($fxn_name).to_string() },
          None
        ).with_compiler_loc()),
      }
    };
  };
}

#[macro_export]
macro_rules! impl_assign_range_scalar_arms {
  ($fxn_name:ident, $shape:tt, $arg:expr, $value_kind:ident, $value_string:tt) => {
    paste! {
      match $arg {
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[Value::MatrixIndex(Matrix::Matrix1(ix1)), Value::Index(ix2)], Value::[<$value_kind:camel>](source)) => {
          register_assign_s!([<$fxn_name S>], $value_kind, $value_string, $shape, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[Value::MatrixIndex(Matrix::Vector2(ix1)), Value::Index(ix2)], Value::[<$value_kind:camel>](source)) => {
          register_assign_s!([<$fxn_name S>], $value_kind, $value_string, $shape, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[Value::MatrixIndex(Matrix::Vector3(ix1)), Value::Index(ix2)], Value::[<$value_kind:camel>](source)) => {
          register_assign_s!([<$fxn_name S>], $value_kind, $value_string, $shape, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[Value::MatrixIndex(Matrix::Vector4(ix1)), Value::Index(ix2)], Value::[<$value_kind:camel>](source)) => {
          register_assign_s!([<$fxn_name S>], $value_kind, $value_string, $shape, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "matrixd"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[Value::MatrixIndex(Matrix::DMatrix(ix1)), Value::Index(ix2)], Value::[<$value_kind:camel>](source)) => {
          register_assign_s!([<$fxn_name S>], $value_kind, $value_string, $shape, DMatrix);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[Value::MatrixIndex(Matrix::DVector(ix1)), Value::Index(ix2)], Value::[<$value_kind:camel>](source)) => {
          register_assign_s!([<$fxn_name S>], $value_kind, $value_string, $shape, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Matrix1(ix1)), Value::Index(ix2)], Value::[<Matrix $value_kind:camel>](Matrix::Matrix1(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, Matrix1, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector4(ix1)), Value::Index(ix2)], Value::[<Matrix $value_kind:camel>](Matrix::Matrix2(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::Index(ix2)], Value::[<Matrix $value_kind:camel>](Matrix::Matrix3(source))) if ix1.borrow().len() == source.borrow().len() => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::Index(ix2)], Value::[<Matrix $value_kind:camel>](Matrix::Matrix4(source))) if ix1.borrow().len() == source.borrow().len() => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, Matrix4, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::Index(ix2)], Value::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(source))) if ix1.borrow().len() == source.borrow().len() => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2x3, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::Index(ix2)], Value::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(source))) if ix1.borrow().len() == source.borrow().len() => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3x2, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector2(ix1)), Value::Index(ix2)], Value::[<Matrix $value_kind:camel>](Matrix::Vector2(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, Vector2, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector3(ix1)), Value::Index(ix2)], Value::[<Matrix $value_kind:camel>](Matrix::Vector3(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, Vector3, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector4(ix1)), Value::Index(ix2)], Value::[<Matrix $value_kind:camel>](Matrix::Vector4(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, Vector4, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector2(ix1)), Value::Index(ix2)], Value::[<Matrix $value_kind:camel>](Matrix::RowVector2(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, RowVector2, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector3(ix1)), Value::Index(ix2)], Value::[<Matrix $value_kind:camel>](Matrix::RowVector3(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, RowVector3, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector4(ix1)), Value::Index(ix2)], Value::[<Matrix $value_kind:camel>](Matrix::RowVector4(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, RowVector4, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::Index(ix2)], Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) if ix1.borrow().len() == source.borrow().len() => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::Index(ix2)], Value::[<Matrix $value_kind:camel>](Matrix::DVector(source))) if ix1.borrow().len() == source.borrow().len() => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, DVector, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::Index(ix2)], Value::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) if ix1.borrow().len() == source.borrow().len() => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        (source, ixes, sink) => Err(MechError2::new(
          UnhandledFunctionArgumentKindIxes { arg: (source.clone(), ixes.to_vec(), sink.clone()), fxn_name: stringify!($fxn_name).to_string() },
          None
        ).with_compiler_loc()),
      }
    };
  };
}

#[macro_export]
macro_rules! impl_assign_scalar_range_arms {
  ($fxn_name:ident, $shape:tt, $arg:expr, $value_kind:ident, $value_string:tt) => {
    paste! {
      match $arg {
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[Value::Index(ix1), Value::MatrixIndex(Matrix::Matrix1(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s!([<$fxn_name S>], $value_kind, $value_string, $shape, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[Value::Index(ix1), Value::MatrixIndex(Matrix::Vector2(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s!([<$fxn_name S>], $value_kind, $value_string, $shape, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[Value::Index(ix1), Value::MatrixIndex(Matrix::Vector3(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s!([<$fxn_name S>], $value_kind, $value_string, $shape, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[Value::Index(ix1), Value::MatrixIndex(Matrix::Vector4(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s!([<$fxn_name S>], $value_kind, $value_string, $shape, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "matrixd", not(feature = "matrix1")))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[Value::Index(ix1), Value::MatrixIndex(Matrix::DMatrix(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s!([<$fxn_name S>], $value_kind, $value_string, $shape, DMatrix);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[Value::Index(ix1), Value::MatrixIndex(Matrix::DVector(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s!([<$fxn_name S>], $value_kind, $value_string, $shape, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::Index(ix1), Value::MatrixIndex(Matrix::Matrix1(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix1(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, Matrix1, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::Index(ix1), Value::MatrixIndex(Matrix::Vector4(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix2(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::Index(ix1), Value::MatrixIndex(Matrix::DVector(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix3(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::Index(ix1), Value::MatrixIndex(Matrix::DVector(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix4(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, Matrix4, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::Index(ix1), Value::MatrixIndex(Matrix::DVector(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(source))) if ix2.borrow().len() == source.borrow().len() => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2x3, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::Index(ix1), Value::MatrixIndex(Matrix::DVector(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(source))) if ix2.borrow().len() == source.borrow().len() => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3x2, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::Index(ix1), Value::MatrixIndex(Matrix::Vector2(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::Vector2(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, Vector2, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::Index(ix1), Value::MatrixIndex(Matrix::Vector3(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::Vector3(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, Vector3, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::Index(ix1), Value::MatrixIndex(Matrix::Vector4(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::Vector4(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, Vector4, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::Index(ix1), Value::MatrixIndex(Matrix::Vector2(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::RowVector2(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, RowVector2, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::Index(ix1), Value::MatrixIndex(Matrix::Vector3(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::RowVector3(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, RowVector3, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::Index(ix1), Value::MatrixIndex(Matrix::Vector4(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::RowVector4(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, RowVector4, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::Index(ix1), Value::MatrixIndex(Matrix::DVector(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) if ix2.borrow().len() == source.borrow().len() => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::Index(ix1), Value::MatrixIndex(Matrix::DVector(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::DVector(source))) if ix2.borrow().len() == source.borrow().len() => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, DVector, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::Index(ix1), Value::MatrixIndex(Matrix::DVector(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) if ix2.borrow().len() == source.borrow().len() => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        _ => Err(MechError2::new(
          UnhandledFunctionArgumentKind2 { arg: $arg.clone(), fxn_name: stringify!($fxn_name).to_string() },
          None
        ).with_compiler_loc()),
      }
    };
  };
}

pub fn check_index_lengths<Ix1, Ix2, Source, A, B, C>(
  ix1: &Ref<Ix1>,
  ix2: &Ref<Ix2>,
  source: &Ref<Source>
) -> MResult<()>
where
  Ix1: AsRef<[A]>,
  Ix2: AsRef<[B]>,
  Source: AsRef<[C]>,
{
  let ix1_len = ix1.borrow().as_ref().len();
  let ix2_len = ix2.borrow().as_ref().len();
  let source_len = source.borrow().as_ref().len();

  if ix1_len * ix2_len != source_len {
    return Err(
      MechError2::new(
        MismatchedIndexLengthsError {
          ix1_len,
          ix2_len,
          source_len,
        },
        None
      )
      .with_compiler_loc()
    );
  }
  Ok(())
}

#[derive(Debug)]
pub struct MismatchedIndexLengthsError {
  pub ix1_len: usize,
  pub ix2_len: usize,
  pub source_len: usize,
}
impl MechErrorKind2 for MismatchedIndexLengthsError {
  fn name(&self) -> &str { "MismatchedIndexLengths" }

  fn message(&self) -> String {
    format!(
      "Mismatched lengths for indexed assignment: ix1 length ({}) * ix2 length ({}) \
       must equal source length ({})",
      self.ix1_len, self.ix2_len, self.source_len
    )
  }
}

#[macro_export]
macro_rules! impl_assign_range_range_arms {
  ($fxn_name:ident, $shape:tt, $arg:expr, $value_kind:ident, $value_string:tt) => {
    paste! {
      match $arg {
        // Scalar source
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Matrix1(ix1)), Value::MatrixIndex(Matrix::Matrix1(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_srr!([<$fxn_name S>], $value_kind, $value_string, $shape, Matrix1, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix1", feature = "vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Matrix1(ix1)), Value::MatrixIndex(Matrix::Vector2(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_srr!([<$fxn_name S>], $value_kind, $value_string, $shape, Matrix1, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix1", feature = "vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Matrix1(ix1)), Value::MatrixIndex(Matrix::Vector3(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_srr!([<$fxn_name S>], $value_kind, $value_string, $shape, Matrix1, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix1", feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Matrix1(ix1)), Value::MatrixIndex(Matrix::Vector4(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_srr!([<$fxn_name S>], $value_kind, $value_string, $shape, Matrix1, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix1", feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Matrix1(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_srr!([<$fxn_name S>], $value_kind, $value_string, $shape, Matrix1, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "matrix1"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector2(ix1)), Value::MatrixIndex(Matrix::Matrix1(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_srr!([<$fxn_name S>], $value_kind, $value_string, $shape, Vector2, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector2(ix1)), Value::MatrixIndex(Matrix::Vector2(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_srr!([<$fxn_name S>], $value_kind, $value_string, $shape, Vector2, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector2(ix1)), Value::MatrixIndex(Matrix::Vector3(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_srr!([<$fxn_name S>], $value_kind, $value_string, $shape, Vector2, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector2(ix1)), Value::MatrixIndex(Matrix::Vector4(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_srr!([<$fxn_name S>], $value_kind, $value_string, $shape, Vector2, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector2(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_srr!([<$fxn_name S>], $value_kind, $value_string, $shape, Vector2, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "matrix1"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector3(ix1)), Value::MatrixIndex(Matrix::Matrix1(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_srr!([<$fxn_name S>], $value_kind, $value_string, $shape, Vector3, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector3(ix1)), Value::MatrixIndex(Matrix::Vector2(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_srr!([<$fxn_name S>], $value_kind, $value_string, $shape, Vector3, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector3(ix1)), Value::MatrixIndex(Matrix::Vector3(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_srr!([<$fxn_name S>], $value_kind, $value_string, $shape, Vector3, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector3(ix1)), Value::MatrixIndex(Matrix::Vector4(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_srr!([<$fxn_name S>], $value_kind, $value_string, $shape, Vector3, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector3(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_srr!([<$fxn_name S>], $value_kind, $value_string, $shape, Vector3, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "matrix1"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector4(ix1)), Value::MatrixIndex(Matrix::Matrix1(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_srr!([<$fxn_name S>], $value_kind, $value_string, $shape, Vector4, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector4(ix1)), Value::MatrixIndex(Matrix::Vector2(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_srr!([<$fxn_name S>], $value_kind, $value_string, $shape, Vector4, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector4(ix1)), Value::MatrixIndex(Matrix::Vector3(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_srr!([<$fxn_name S>], $value_kind, $value_string, $shape, Vector4, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector4(ix1)), Value::MatrixIndex(Matrix::Vector4(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_srr!([<$fxn_name S>], $value_kind, $value_string, $shape, Vector4, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector4(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_srr!([<$fxn_name S>], $value_kind, $value_string, $shape, Vector4, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord", feature = "matrix1"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::Matrix1(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_srr!([<$fxn_name S>], $value_kind, $value_string, $shape, DVector, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::Vector2(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_srr!([<$fxn_name S>], $value_kind, $value_string, $shape, DVector, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::Vector3(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_srr!([<$fxn_name S>], $value_kind, $value_string, $shape, DVector, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::Vector4(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_srr!([<$fxn_name S>], $value_kind, $value_string, $shape, DVector, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_srr!([<$fxn_name S>], $value_kind, $value_string, $shape, DVector, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        // Vector source
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Matrix1(ix1)), Value::MatrixIndex(Matrix::Matrix1(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix1(source))) => {
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, Matrix1, Matrix1, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix1", feature = "vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Matrix1(ix1)), Value::MatrixIndex(Matrix::Vector2(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::Vector2(source))) => {
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, Vector2, Matrix1, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix1", feature = "row_vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Matrix1(ix1)), Value::MatrixIndex(Matrix::Vector2(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::RowVector2(source))) => {
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, RowVector2, Matrix1, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix1", feature = "vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Matrix1(ix1)), Value::MatrixIndex(Matrix::Vector3(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::Vector3(source))) => {
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, Vector3, Matrix1, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix1", feature = "row_vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Matrix1(ix1)), Value::MatrixIndex(Matrix::Vector3(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::RowVector3(source))) => {
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, RowVector3, Matrix1, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix1", feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Matrix1(ix1)), Value::MatrixIndex(Matrix::Vector4(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::Vector4(source))) => {
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, Vector4, Matrix1, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix1", feature = "vector2", feature = "matrix2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Matrix1(ix1)), Value::MatrixIndex(Matrix::Vector2(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix2(source))) => {
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2, Matrix1, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix1", feature = "row_vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Matrix1(ix1)), Value::MatrixIndex(Matrix::Vector4(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::RowVector4(source))) => {
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, RowVector4, Matrix1, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix1", feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Matrix1(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::DVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Matrix1, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix1", feature = "row_vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Matrix1(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, Matrix1, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix1", feature = "matrixd"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Matrix1(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, Matrix1, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "matrix1", feature = "vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector2(ix1)), Value::MatrixIndex(Matrix::Matrix1(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::Vector2(source))) => {
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, Vector2, Vector2, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "matrix1", feature = "row_vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector2(ix1)), Value::MatrixIndex(Matrix::Matrix1(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::RowVector2(source))) => {
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, RowVector2, Vector2, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector2(ix1)), Value::MatrixIndex(Matrix::Vector2(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::Vector4(source))) => {
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, Vector4, Vector2, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "row_vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector2(ix1)), Value::MatrixIndex(Matrix::Vector2(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::RowVector4(source))) => {
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, RowVector4, Vector2, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector3", feature = "matrix2x3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector2(ix1)), Value::MatrixIndex(Matrix::Vector3(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(source))) => {
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2x3, Vector2, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector3", feature = "matrix3x2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector2(ix1)), Value::MatrixIndex(Matrix::Vector3(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(source))) => {
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3x2, Vector2, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector3", feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector2(ix1)), Value::MatrixIndex(Matrix::Vector3(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::DVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Vector2, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector3", feature = "row_vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector2(ix1)), Value::MatrixIndex(Matrix::Vector3(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, Vector2, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector4", feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector2(ix1)), Value::MatrixIndex(Matrix::Vector4(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::DVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Vector2, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector4", feature = "row_vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector2(ix1)), Value::MatrixIndex(Matrix::Vector4(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, Vector2, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector4", feature = "matrixd"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector2(ix1)), Value::MatrixIndex(Matrix::Vector4(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, Vector2, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "vectord", feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector2(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::DVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Vector2, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "vectord", feature = "row_vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector2(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, Vector2, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "vectord", feature = "matrixd"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector2(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, Vector2, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "matrix1", feature = "vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector3(ix1)), Value::MatrixIndex(Matrix::Matrix1(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::Vector3(source))) => {
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, Vector3, Vector3, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "matrix1", feature = "row_vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector3(ix1)), Value::MatrixIndex(Matrix::Matrix1(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::RowVector3(source))) => {
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, RowVector3, Vector3, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector2", feature = "matrix2x3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector3(ix1)), Value::MatrixIndex(Matrix::Vector2(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(source))) => {
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2x3, Vector3, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector2", feature = "matrix3x2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector3(ix1)), Value::MatrixIndex(Matrix::Vector2(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(source))) => {
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3x2, Vector3, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector2", feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector3(ix1)), Value::MatrixIndex(Matrix::Vector2(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::DVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Vector3, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector2", feature = "row_vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector3(ix1)), Value::MatrixIndex(Matrix::Vector2(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, Vector3, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "matrix3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector3(ix1)), Value::MatrixIndex(Matrix::Vector3(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix3(source))) => {
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3, Vector3, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector3(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::DVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Vector3, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "vectord", feature = "row_vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector3(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, Vector3, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector4", feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector3(ix1)), Value::MatrixIndex(Matrix::Vector4(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::DVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Vector3, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector4", feature = "row_vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector3(ix1)), Value::MatrixIndex(Matrix::Vector4(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, Vector3, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector4", feature = "matrixd"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector3(ix1)), Value::MatrixIndex(Matrix::Vector4(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, Vector3, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "vectord", feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector3(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::DVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Vector3, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "vectord", feature = "row_vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector3(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, Vector3, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "vectord", feature = "matrixd"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector3(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, Vector3, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "matrix1", feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector4(ix1)), Value::MatrixIndex(Matrix::Matrix1(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::Vector4(source))) => {
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, Vector4, Vector4, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "matrix1", feature = "row_vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector4(ix1)), Value::MatrixIndex(Matrix::Matrix1(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::RowVector4(source))) => {
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, RowVector4, Vector4, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "matrix1", feature = "matrix2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector4(ix1)), Value::MatrixIndex(Matrix::Matrix1(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix2(source))) => {
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2, Vector4, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector2", feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector4(ix1)), Value::MatrixIndex(Matrix::Vector2(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::DVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Vector4, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector2", feature = "row_vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector4(ix1)), Value::MatrixIndex(Matrix::Vector2(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, Vector4, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector2", feature = "matrixd"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector4(ix1)), Value::MatrixIndex(Matrix::Vector2(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, Vector4, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector3", feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector4(ix1)), Value::MatrixIndex(Matrix::Vector3(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::DVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Vector4, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector3", feature = "row_vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector4(ix1)), Value::MatrixIndex(Matrix::Vector3(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, Vector4, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector3", feature = "matrixd"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector4(ix1)), Value::MatrixIndex(Matrix::Vector3(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, Vector4, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector4(ix1)), Value::MatrixIndex(Matrix::Vector4(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::DVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Vector4, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "row_vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector4(ix1)), Value::MatrixIndex(Matrix::Vector4(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, Vector4, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "matrixd"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector4(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, Vector4, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector4(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::DVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, DVector, Vector4, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "row_vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector4(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, Vector4, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "matrix4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::Vector4(ix1)), Value::MatrixIndex(Matrix::Vector4(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix4(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, Matrix4, Vector4, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord", feature = "matrix1", feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::Matrix1(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::DVector(source))) => {
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, DVector, DVector, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord", feature = "matrix1", feature = "row_vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::Matrix1(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) => {
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, DVector, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord", feature = "matrix1", feature = "matrixd"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::Matrix1(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) => {
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, DVector, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector2", feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::Vector2(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::DVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, DVector, DVector, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector2", feature = "row_vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::Vector2(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, DVector, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector2", feature = "matrixd"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::Vector2(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, DVector, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector3", feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::Vector3(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::DVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, DVector, DVector, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector3", feature = "row_vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::Vector3(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, DVector, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector3", feature = "matrixd"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::Vector3(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, DVector, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector4", feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::Vector4(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::DVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, DVector, DVector, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector4", feature = "row_vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::Vector4(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, DVector, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector4", feature = "matrixd"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::Vector4(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, DVector, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::DVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, DVector, DVector, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord", feature = "matrixd"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, DVector, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord", feature = "row_vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          register_assign_srr2!([<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, DVector, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        (source, ixes, sink) => Err(MechError2::new(
          UnhandledFunctionArgumentIxes { arg: (source.clone(), ixes.to_vec(), sink.clone()), fxn_name: stringify!($fxn_name).to_string() },
          None
        ).with_compiler_loc()),
      }
    };
  };
}

#[macro_export]
macro_rules! impl_assign_all_range_arms {
  ($fxn_name:ident, $shape:tt, $arg:expr, $value_kind:ident, $value_string:tt) => {
    paste! {
      match $arg {
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[Value::IndexAll, Value::MatrixIndex(Matrix::Matrix1(ix))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s!([<$fxn_name S>], $value_kind, $value_string, $shape, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[Value::IndexAll, Value::MatrixIndex(Matrix::Vector2(ix))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s!([<$fxn_name S>], $value_kind, $value_string, $shape, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[Value::IndexAll, Value::MatrixIndex(Matrix::Vector3(ix))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s!([<$fxn_name S>], $value_kind, $value_string, $shape, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[Value::IndexAll, Value::MatrixIndex(Matrix::Vector4(ix))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s!([<$fxn_name S>], $value_kind, $value_string, $shape, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[Value::IndexAll, Value::MatrixIndex(Matrix::DVector(ix))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s!([<$fxn_name S>], $value_kind, $value_string, $shape, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::IndexAll, Value::MatrixIndex(Matrix::Matrix1(ix))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix1(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, Matrix1, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::IndexAll, Value::MatrixIndex(Matrix::Vector4(ix))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix2(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::IndexAll, Value::MatrixIndex(Matrix::DVector(ix))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix3(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::IndexAll, Value::MatrixIndex(Matrix::DVector(ix))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix4(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, Matrix4, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::IndexAll, Value::MatrixIndex(Matrix::DVector(ix))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, Matrix2x3, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::IndexAll, Value::MatrixIndex(Matrix::DVector(ix))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, Matrix3x2, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::IndexAll, Value::MatrixIndex(Matrix::DVector(ix))], Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, DMatrix, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::IndexAll, Value::MatrixIndex(Matrix::DVector(ix))], Value::[<Matrix $value_kind:camel>](Matrix::DVector(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, DVector, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::IndexAll, Value::MatrixIndex(Matrix::DVector(ix))], Value::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, RowDVector, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::IndexAll, Value::MatrixIndex(Matrix::Vector2(ix))], Value::[<Matrix $value_kind:camel>](Matrix::Vector2(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, Vector2, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::IndexAll, Value::MatrixIndex(Matrix::Vector3(ix))], Value::[<Matrix $value_kind:camel>](Matrix::Vector3(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, Vector3, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::IndexAll, Value::MatrixIndex(Matrix::Vector4(ix))], Value::[<Matrix $value_kind:camel>](Matrix::Vector4(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, Vector4, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::IndexAll, Value::MatrixIndex(Matrix::Vector2(ix))], Value::[<Matrix $value_kind:camel>](Matrix::RowVector2(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, RowVector2, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::IndexAll, Value::MatrixIndex(Matrix::Vector3(ix))], Value::[<Matrix $value_kind:camel>](Matrix::RowVector3(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, RowVector3, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::IndexAll, Value::MatrixIndex(Matrix::Vector4(ix))], Value::[<Matrix $value_kind:camel>](Matrix::RowVector4(source))) => {
          register_assign!([<$fxn_name V>], $value_kind, $value_string, $shape, RowVector4, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        (source, ixes, sink) => Err(MechError2::new(
          UnhandledFunctionArgumentIxes { arg: (source.clone(), ixes.to_vec(), sink.clone()), fxn_name: stringify!($fxn_name).to_string() },
          None
        ).with_compiler_loc()),
      }
    };
  };
}

#[macro_export]
macro_rules! impl_assign_all_scalar_arms {
  ($fxn_name:ident, $arg:expr, $value_kind:ident, $value_string:tt) => {
    paste! {
      match $arg {
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix1(sink)),[Value::IndexAll, Value::Index(ix)], Value::[<$value_kind:camel>](source)) => {
          register_assign_s1!([<$fxn_name S>], $value_kind, $value_string, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "matrix2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix2(sink)),[Value::IndexAll, Value::Index(ix)], Value::[<$value_kind:camel>](source)) => {
          register_assign_s1!([<$fxn_name S>], $value_kind, $value_string, Matrix2);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "matrix3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix3(sink)),[Value::IndexAll, Value::Index(ix)], Value::[<$value_kind:camel>](source)) => {
          register_assign_s1!([<$fxn_name S>], $value_kind, $value_string, Matrix3);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "matrix4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix4(sink)),[Value::IndexAll, Value::Index(ix)], Value::[<$value_kind:camel>](source)) => {
          register_assign_s1!([<$fxn_name S>], $value_kind, $value_string, Matrix4);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(sink)),[Value::IndexAll, Value::Index(ix)], Value::[<$value_kind:camel>](source)) => {
          register_assign_s1!([<$fxn_name S>], $value_kind, $value_string, Matrix2x3);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(sink)),[Value::IndexAll, Value::Index(ix)], Value::[<$value_kind:camel>](source)) => {
          register_assign_s1!([<$fxn_name S>], $value_kind, $value_string, Matrix3x2);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "matrixd"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)),[Value::IndexAll, Value::Index(ix)], Value::[<$value_kind:camel>](source)) => {
          register_assign_s1!([<$fxn_name S>], $value_kind, $value_string, DMatrix);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::DVector(sink)),[Value::IndexAll, Value::Index(ix)], Value::[<$value_kind:camel>](source)) => {
          register_assign_s1!([<$fxn_name S>], $value_kind, $value_string, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "row_vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::RowDVector(sink)),[Value::IndexAll, Value::Index(ix)], Value::[<$value_kind:camel>](source)) => {
          register_assign_s1!([<$fxn_name S>], $value_kind, $value_string, RowDVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Vector2(sink)),[Value::IndexAll, Value::Index(ix)], Value::[<$value_kind:camel>](source)) => {
          register_assign_s1!([<$fxn_name S>], $value_kind, $value_string, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Vector3(sink)),[Value::IndexAll, Value::Index(ix)], Value::[<$value_kind:camel>](source)) => {
          register_assign_s1!([<$fxn_name S>], $value_kind, $value_string, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Vector4(sink)),[Value::IndexAll, Value::Index(ix)], Value::[<$value_kind:camel>](source)) => {
          register_assign_s1!([<$fxn_name S>], $value_kind, $value_string, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "row_vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::RowVector2(sink)),[Value::IndexAll, Value::Index(ix)], Value::[<$value_kind:camel>](source)) => {
          register_assign_s1!([<$fxn_name S>], $value_kind, $value_string, RowVector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "row_vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::RowVector3(sink)),[Value::IndexAll, Value::Index(ix)], Value::[<$value_kind:camel>](source)) => {
          register_assign_s1!([<$fxn_name S>], $value_kind, $value_string, RowVector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "row_vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::RowVector4(sink)),[Value::IndexAll, Value::Index(ix)], Value::[<$value_kind:camel>](source)) => {
          register_assign_s1!([<$fxn_name S>], $value_kind, $value_string, RowVector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        // Vector source
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix1(sink)), [Value::IndexAll, Value::Index(ix)], Value::[<Matrix $value_kind:camel>](Matrix::Matrix1(source))) => {
          register_assign_s2!([<$fxn_name V>], $value_kind, $value_string, Matrix1, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "matrix2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix2(sink)), [Value::IndexAll, Value::Index(ix)], Value::[<Matrix $value_kind:camel>](Matrix::RowVector2(source))) => {
          register_assign_s2!([<$fxn_name V>], $value_kind, $value_string, Matrix2, RowVector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "matrix2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix2(sink)), [Value::IndexAll, Value::Index(ix)], Value::[<Matrix $value_kind:camel>](Matrix::Vector2(source))) => {
          register_assign_s2!([<$fxn_name V>], $value_kind, $value_string, Matrix2, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "matrix3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix3(sink)), [Value::IndexAll, Value::Index(ix)], Value::[<Matrix $value_kind:camel>](Matrix::RowVector3(source))) => {
          register_assign_s2!([<$fxn_name V>], $value_kind, $value_string, Matrix3, RowVector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "matrix3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix3(sink)), [Value::IndexAll, Value::Index(ix)], Value::[<Matrix $value_kind:camel>](Matrix::Vector3(source))) => {
          register_assign_s2!([<$fxn_name V>], $value_kind, $value_string, Matrix3, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector4", feature = "matrix4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix4(sink)), [Value::IndexAll, Value::Index(ix)], Value::[<Matrix $value_kind:camel>](Matrix::RowVector4(source))) => {
          register_assign_s2!([<$fxn_name V>], $value_kind, $value_string, Matrix4, RowVector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "matrix4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix4(sink)), [Value::IndexAll, Value::Index(ix)], Value::[<Matrix $value_kind:camel>](Matrix::Vector4(source))) => {
          register_assign_s2!([<$fxn_name V>], $value_kind, $value_string, Matrix4, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "matrix2x3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(sink)), [Value::IndexAll, Value::Index(ix)], Value::[<Matrix $value_kind:camel>](Matrix::RowVector2(source))) => {
          register_assign_s2!([<$fxn_name V>], $value_kind, $value_string, Matrix2x3, RowVector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "matrix2x3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(sink)), [Value::IndexAll, Value::Index(ix)], Value::[<Matrix $value_kind:camel>](Matrix::Vector2(source))) => {
          register_assign_s2!([<$fxn_name V>], $value_kind, $value_string, Matrix2x3, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "matrix3x2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(sink)), [Value::IndexAll, Value::Index(ix)], Value::[<Matrix $value_kind:camel>](Matrix::RowVector3(source))) => {
          register_assign_s2!([<$fxn_name V>], $value_kind, $value_string, Matrix3x2, RowVector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "matrix3x2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(sink)), [Value::IndexAll, Value::Index(ix)], Value::[<Matrix $value_kind:camel>](Matrix::Vector3(source))) => {
          register_assign_s2!([<$fxn_name V>], $value_kind, $value_string, Matrix3x2, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord", feature = "matrixd"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [Value::IndexAll, Value::Index(ix)], Value::[<Matrix $value_kind:camel>](Matrix::DVector(source))) if sink.borrow().nrows() == source.borrow().len() => {
          register_assign_s2!([<$fxn_name V>], $value_kind, $value_string, DMatrix, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "matrixd"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [Value::IndexAll, Value::Index(ix)], Value::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) if sink.borrow().nrows() == source.borrow().len() => {
          register_assign_s2!([<$fxn_name V>], $value_kind, $value_string, DMatrix, RowDVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        (source, ixes, sink) => Err(MechError2::new(
          UnhandledFunctionArgumentIxes { arg: (source.clone(), ixes.to_vec(), sink.clone()), fxn_name: stringify!($fxn_name).to_string() },
          None
        ).with_compiler_loc()),
      }
    };
  };
}

#[macro_export]
macro_rules! impl_assign_scalar_all_arms {
  ($fxn_name:ident, $arg:expr, $value_kind:ident, $value_string:tt) => {
    paste! {
      match $arg {
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix1(sink)),[Value::Index(ix), Value::IndexAll], Value::[<$value_kind:camel>](source)) => {
          register_assign_s1!([<$fxn_name S>], $value_kind, $value_string, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "matrix2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix2(sink)),[Value::Index(ix), Value::IndexAll], Value::[<$value_kind:camel>](source)) => {
          register_assign_s1!([<$fxn_name S>], $value_kind, $value_string, Matrix2);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "matrix3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix3(sink)),[Value::Index(ix), Value::IndexAll], Value::[<$value_kind:camel>](source)) => {
          register_assign_s1!([<$fxn_name S>], $value_kind, $value_string, Matrix3);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "matrix4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix4(sink)),[Value::Index(ix), Value::IndexAll], Value::[<$value_kind:camel>](source)) => {
          register_assign_s1!([<$fxn_name S>], $value_kind, $value_string, Matrix4);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(sink)),[Value::Index(ix), Value::IndexAll], Value::[<$value_kind:camel>](source)) => {
          register_assign_s1!([<$fxn_name S>], $value_kind, $value_string, Matrix2x3);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(sink)),[Value::Index(ix), Value::IndexAll], Value::[<$value_kind:camel>](source)) => {
          register_assign_s1!([<$fxn_name S>], $value_kind, $value_string, Matrix3x2);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "matrixd"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)),[Value::Index(ix), Value::IndexAll], Value::[<$value_kind:camel>](source)) => {
          register_assign_s1!([<$fxn_name S>], $value_kind, $value_string, DMatrix);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::DVector(sink)),[Value::Index(ix), Value::IndexAll], Value::[<$value_kind:camel>](source)) => {
          register_assign_s1!([<$fxn_name S>], $value_kind, $value_string, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "row_vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::RowDVector(sink)),[Value::Index(ix), Value::IndexAll], Value::[<$value_kind:camel>](source)) => {
          register_assign_s1!([<$fxn_name S>], $value_kind, $value_string, RowDVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Vector2(sink)),[Value::Index(ix), Value::IndexAll], Value::[<$value_kind:camel>](source)) => {
          register_assign_s1!([<$fxn_name S>], $value_kind, $value_string, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Vector3(sink)),[Value::Index(ix), Value::IndexAll], Value::[<$value_kind:camel>](source)) => {
          register_assign_s1!([<$fxn_name S>], $value_kind, $value_string, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Vector4(sink)),[Value::Index(ix), Value::IndexAll], Value::[<$value_kind:camel>](source)) => {
          register_assign_s1!([<$fxn_name S>], $value_kind, $value_string, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "row_vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::RowVector2(sink)),[Value::Index(ix), Value::IndexAll], Value::[<$value_kind:camel>](source)) => {
          register_assign_s1!([<$fxn_name S>], $value_kind, $value_string, RowVector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "row_vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::RowVector3(sink)),[Value::Index(ix), Value::IndexAll], Value::[<$value_kind:camel>](source)) => {
          register_assign_s1!([<$fxn_name S>], $value_kind, $value_string, RowVector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "row_vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::RowVector4(sink)),[Value::Index(ix), Value::IndexAll], Value::[<$value_kind:camel>](source)) => {
          register_assign_s1!([<$fxn_name S>], $value_kind, $value_string, RowVector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        // Vector source
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix1(sink)), [Value::Index(ix), Value::IndexAll], Value::[<Matrix $value_kind:camel>](Matrix::Matrix1(source))) => {
          register_assign_s2!([<$fxn_name V>], $value_kind, $value_string, Matrix1, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "matrix2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix2(sink)), [Value::Index(ix), Value::IndexAll], Value::[<Matrix $value_kind:camel>](Matrix::RowVector2(source))) => {
          register_assign_s2!([<$fxn_name V>], $value_kind, $value_string, Matrix2, RowVector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "matrix2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix2(sink)), [Value::Index(ix), Value::IndexAll], Value::[<Matrix $value_kind:camel>](Matrix::Vector2(source))) => {
          register_assign_s2!([<$fxn_name V>], $value_kind, $value_string, Matrix2, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "matrix3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix3(sink)), [Value::Index(ix), Value::IndexAll], Value::[<Matrix $value_kind:camel>](Matrix::RowVector3(source))) => {
          register_assign_s2!([<$fxn_name V>], $value_kind, $value_string, Matrix3, RowVector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "matrix3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix3(sink)), [Value::Index(ix), Value::IndexAll], Value::[<Matrix $value_kind:camel>](Matrix::Vector3(source))) => {
          register_assign_s2!([<$fxn_name V>], $value_kind, $value_string, Matrix3, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector4", feature = "matrix4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix4(sink)), [Value::Index(ix), Value::IndexAll], Value::[<Matrix $value_kind:camel>](Matrix::RowVector4(source))) => {
          register_assign_s2!([<$fxn_name V>], $value_kind, $value_string, Matrix4, RowVector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "matrix4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix4(sink)), [Value::Index(ix), Value::IndexAll], Value::[<Matrix $value_kind:camel>](Matrix::Vector4(source))) => {
          register_assign_s2!([<$fxn_name V>], $value_kind, $value_string, Matrix4, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "matrix2x3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(sink)), [Value::Index(ix), Value::IndexAll], Value::[<Matrix $value_kind:camel>](Matrix::RowVector3(source))) => {
          register_assign_s2!([<$fxn_name V>], $value_kind, $value_string, Matrix2x3, RowVector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "matrix2x3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(sink)), [Value::Index(ix), Value::IndexAll], Value::[<Matrix $value_kind:camel>](Matrix::Vector3(source))) => {
          register_assign_s2!([<$fxn_name V>], $value_kind, $value_string, Matrix2x3, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "matrix3x2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(sink)), [Value::Index(ix), Value::IndexAll], Value::[<Matrix $value_kind:camel>](Matrix::RowVector2(source))) => {
          register_assign_s2!([<$fxn_name V>], $value_kind, $value_string, Matrix3x2, RowVector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "matrix3x2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(sink)), [Value::Index(ix), Value::IndexAll], Value::[<Matrix $value_kind:camel>](Matrix::Vector2(source))) => {
          register_assign_s2!([<$fxn_name V>], $value_kind, $value_string, Matrix3x2, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord", feature = "matrixd"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [Value::Index(ix), Value::IndexAll], Value::[<Matrix $value_kind:camel>](Matrix::DVector(source))) if sink.borrow().ncols() == source.borrow().len() => {
          register_assign_s2!([<$fxn_name V>], $value_kind, $value_string, DMatrix, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "matrixd"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [Value::Index(ix), Value::IndexAll], Value::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) if sink.borrow().ncols() == source.borrow().len() => {
          register_assign_s2!([<$fxn_name V>], $value_kind, $value_string, DMatrix, RowDVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        (source, ixes, sink) => Err(MechError2::new(
          UnhandledFunctionArgumentIxes { arg: (source.clone(), ixes.to_vec(), sink.clone()), fxn_name: stringify!($fxn_name).to_string() },
          None
        ).with_compiler_loc()),
      }
    };
  };
}

#[macro_export]
macro_rules! impl_set_all_range_arms_b {
  ($fxn_name:ident, $arg:expr, $value_kind:ident, $value_string:tt) => {
    paste! {
      match $arg {
        // Scalar source
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix1(sink)),[Value::IndexAll, Value::MatrixBool(Matrix::Matrix1(ix))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, Matrix1, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix2(sink)),[Value::IndexAll, Value::MatrixBool(Matrix::Vector4(ix))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, Matrix2, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "matrix3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix3(sink)),[Value::IndexAll, Value::MatrixBool(Matrix::DVector(ix))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, Matrix3, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "matrix4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix4(sink)),[Value::IndexAll, Value::MatrixBool(Matrix::DVector(ix))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, Matrix4, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(sink)),[Value::IndexAll, Value::MatrixBool(Matrix::DVector(ix))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, Matrix2x3, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(sink)),[Value::IndexAll, Value::MatrixBool(Matrix::DVector(ix))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, Matrix3x2, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "matrixd"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)),[Value::IndexAll, Value::MatrixBool(Matrix::DVector(ix))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, DMatrix, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Vector2(sink)),[Value::IndexAll, Value::MatrixBool(Matrix::Vector2(ix))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, Vector2, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Vector3(sink)),[Value::IndexAll, Value::MatrixBool(Matrix::Vector3(ix))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, Vector3, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Vector4(sink)),[Value::IndexAll, Value::MatrixBool(Matrix::Vector4(ix))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, Vector4, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::DVector(sink)),[Value::IndexAll, Value::MatrixBool(Matrix::DVector(ix))], Value::[<$value_kind:camel>](source)) if sink.borrow().len() == ix.borrow().len() => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, DVector, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "row_vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::RowVector2(sink)),[Value::IndexAll, Value::MatrixBool(Matrix::Vector2(ix))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, RowVector2, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "row_vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::RowVector3(sink)),[Value::IndexAll, Value::MatrixBool(Matrix::Vector3(ix))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, RowVector3, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "row_vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::RowVector4(sink)),[Value::IndexAll, Value::MatrixBool(Matrix::Vector4(ix))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, RowVector4, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "row_vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::RowDVector(sink)),[Value::IndexAll, Value::MatrixBool(Matrix::DVector(ix))], Value::[<$value_kind:camel>](source)) if sink.borrow().len() == ix.borrow().len() => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, RowDVector, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        // Vector source
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix1(sink)), [Value::IndexAll, Value::MatrixBool(Matrix::Matrix1(ix))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix1(source))) => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, Matrix1, Matrix1, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix2(sink)), [Value::IndexAll, Value::MatrixBool(Matrix::Vector2(ix))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix2(source))) => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, Matrix2, Matrix2, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix3(sink)), [Value::IndexAll, Value::MatrixBool(Matrix::Vector3(ix))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix3(source))) => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, Matrix3, Matrix3, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix4(sink)), [Value::IndexAll, Value::MatrixBool(Matrix::Vector4(ix))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix4(source))) => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, Matrix4, Matrix4, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(sink)), [Value::IndexAll, Value::MatrixBool(Matrix::Vector3(ix))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(source))) => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, Matrix2x3, Matrix2x3, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(sink)), [Value::IndexAll, Value::MatrixBool(Matrix::Vector2(ix))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(source))) => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, Matrix3x2, Matrix3x2, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [Value::IndexAll, Value::MatrixBool(Matrix::DVector(ix))], Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) if sink.borrow().ncols() == source.borrow().ncols() && sink.borrow().nrows() == source.borrow().nrows() && ix.borrow().len() == source.borrow().ncols() => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, DMatrix, DMatrix, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::DVector(sink)), [Value::IndexAll, Value::MatrixBool(Matrix::DVector(ix))], Value::[<Matrix $value_kind:camel>](Matrix::DVector(source))) if sink.borrow().len() == source.borrow().len() && sink.borrow().len() == ix.borrow().len() => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, DVector, DVector, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::RowDVector(sink)), [Value::IndexAll, Value::MatrixBool(Matrix::DVector(ix))], Value::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) if sink.borrow().len() == source.borrow().len() && sink.borrow().len() == ix.borrow().len() => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, RowDVector, RowDVector, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Vector2(sink)), [Value::IndexAll, Value::MatrixBool(Matrix::Vector2(ix))], Value::[<Matrix $value_kind:camel>](Matrix::Vector2(source))) => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, Vector2, Vector2, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Vector3(sink)), [Value::IndexAll, Value::MatrixBool(Matrix::Vector3(ix))], Value::[<Matrix $value_kind:camel>](Matrix::Vector3(source))) => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, Vector3, Vector3, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Vector4(sink)), [Value::IndexAll, Value::MatrixBool(Matrix::Vector4(ix))], Value::[<Matrix $value_kind:camel>](Matrix::Vector4(source))) => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, Vector4, Vector4, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::RowVector2(sink)), [Value::IndexAll, Value::MatrixBool(Matrix::Vector2(ix))], Value::[<Matrix $value_kind:camel>](Matrix::RowVector2(source))) => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, RowVector2, RowVector2, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::RowVector3(sink)), [Value::IndexAll, Value::MatrixBool(Matrix::Vector3(ix))], Value::[<Matrix $value_kind:camel>](Matrix::RowVector3(source))) => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, RowVector3, RowVector3, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::RowVector4(sink)), [Value::IndexAll, Value::MatrixBool(Matrix::Vector4(ix))], Value::[<Matrix $value_kind:camel>](Matrix::RowVector4(source))) => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, RowVector4, RowVector4, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        (source, ixes, sink) => Err(MechError2::new(
          UnhandledFunctionArgumentIxes { arg: (source.clone(), ixes.to_vec(), sink.clone()), fxn_name: stringify!($fxn_name).to_string() },
          None
        ).with_compiler_loc()),
      }
    }
  }
}

#[macro_export]
macro_rules! impl_set_range_all_arms_b {
  ($fxn_name:ident, $arg:expr, $value_kind:ident, $value_string:tt) => {
    paste! {
      match $arg {
        // Scalar source
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix1(sink)),[Value::MatrixBool(Matrix::Matrix1(ix)), Value::IndexAll], Value::[<$value_kind:camel>](source)) if ix.borrow().len() == sink.borrow().nrows() => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, Matrix1, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix2(sink)),[Value::MatrixBool(Matrix::Vector2(ix)), Value::IndexAll], Value::[<$value_kind:camel>](source)) if ix.borrow().len() == sink.borrow().nrows() => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, Matrix2, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "matrix3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix3(sink)),[Value::MatrixBool(Matrix::Vector3(ix)), Value::IndexAll], Value::[<$value_kind:camel>](source)) if ix.borrow().len() == sink.borrow().nrows() => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, Matrix3, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "matrix4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix4(sink)),[Value::MatrixBool(Matrix::Vector4(ix)), Value::IndexAll], Value::[<$value_kind:camel>](source)) if ix.borrow().len() == sink.borrow().nrows() => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, Matrix4, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(sink)),[Value::MatrixBool(Matrix::Vector2(ix)), Value::IndexAll], Value::[<$value_kind:camel>](source)) if ix.borrow().len() == sink.borrow().nrows() => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, Matrix2x3, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(sink)),[Value::MatrixBool(Matrix::Vector3(ix)), Value::IndexAll], Value::[<$value_kind:camel>](source)) if ix.borrow().len() == sink.borrow().nrows() => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, Matrix3x2, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "matrixd"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)),[Value::MatrixBool(Matrix::DVector(ix)), Value::IndexAll], Value::[<$value_kind:camel>](source)) if ix.borrow().len() == sink.borrow().nrows() => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, DMatrix, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Vector2(sink)),[Value::MatrixBool(Matrix::Vector2(ix)), Value::IndexAll], Value::[<$value_kind:camel>](source)) if ix.borrow().len() == sink.borrow().nrows() => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, Vector2, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Vector3(sink)),[Value::MatrixBool(Matrix::Vector3(ix)), Value::IndexAll], Value::[<$value_kind:camel>](source)) if ix.borrow().len() == sink.borrow().nrows() => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, Vector3, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Vector4(sink)),[Value::MatrixBool(Matrix::Vector4(ix)), Value::IndexAll], Value::[<$value_kind:camel>](source)) if ix.borrow().len() == sink.borrow().nrows() => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, Vector4, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::DVector(sink)),[Value::MatrixBool(Matrix::DVector(ix)), Value::IndexAll], Value::[<$value_kind:camel>](source)) if sink.borrow().len() == ix.borrow().len() => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, DVector, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "row_vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::RowVector2(sink)),[Value::MatrixBool(Matrix::Vector2(ix)), Value::IndexAll], Value::[<$value_kind:camel>](source)) if ix.borrow().len() == sink.borrow().nrows() => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, RowVector2, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "row_vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::RowVector3(sink)),[Value::MatrixBool(Matrix::Vector3(ix)), Value::IndexAll], Value::[<$value_kind:camel>](source)) if ix.borrow().len() == sink.borrow().nrows() => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, RowVector3, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "row_vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::RowVector4(sink)),[Value::MatrixBool(Matrix::Vector4(ix)), Value::IndexAll], Value::[<$value_kind:camel>](source)) if ix.borrow().len() == sink.borrow().nrows() => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, RowVector4, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "row_vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::RowDVector(sink)),[Value::MatrixBool(Matrix::DVector(ix)), Value::IndexAll], Value::[<$value_kind:camel>](source)) if sink.borrow().len() == ix.borrow().len() => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, RowDVector, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        // Vector source
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix1(sink)), [Value::MatrixBool(Matrix::Matrix1(ix)), Value::IndexAll], Value::[<Matrix $value_kind:camel>](Matrix::Matrix1(source))) if ix.borrow().len() == sink.borrow().nrows() => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, Matrix1, Matrix1, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix2(sink)), [Value::MatrixBool(Matrix::Vector2(ix)), Value::IndexAll], Value::[<Matrix $value_kind:camel>](Matrix::Matrix2(source))) if ix.borrow().len() == sink.borrow().nrows() => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, Matrix2, Matrix2, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix3(sink)), [Value::MatrixBool(Matrix::Vector3(ix)), Value::IndexAll], Value::[<Matrix $value_kind:camel>](Matrix::Matrix3(source))) if ix.borrow().len() == sink.borrow().nrows() => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, Matrix3, Matrix3, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix4(sink)), [Value::MatrixBool(Matrix::Vector4(ix)), Value::IndexAll], Value::[<Matrix $value_kind:camel>](Matrix::Matrix4(source))) if ix.borrow().len() == sink.borrow().nrows() => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, Matrix4, Matrix4, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(sink)), [Value::MatrixBool(Matrix::Vector2(ix)), Value::IndexAll], Value::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(source))) if ix.borrow().len() == sink.borrow().nrows() => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, Matrix2x3, Matrix2x3, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(sink)), [Value::MatrixBool(Matrix::Vector3(ix)), Value::IndexAll], Value::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(source))) if ix.borrow().len() == sink.borrow().nrows() => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, Matrix3x2, Matrix3x2, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [Value::MatrixBool(Matrix::DVector(ix)), Value::IndexAll], Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) if sink.borrow().ncols() == source.borrow().ncols() && sink.borrow().nrows() == source.borrow().nrows() && ix.borrow().len() == sink.borrow().nrows() => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, DMatrix, DMatrix, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::DVector(sink)), [Value::MatrixBool(Matrix::DVector(ix)), Value::IndexAll], Value::[<Matrix $value_kind:camel>](Matrix::DVector(source))) if sink.borrow().len() == source.borrow().len() && sink.borrow().len() == ix.borrow().len() => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, DVector, DVector, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::RowDVector(sink)), [Value::MatrixBool(Matrix::DVector(ix)), Value::IndexAll], Value::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) if sink.borrow().len() == source.borrow().len() && sink.borrow().len() == ix.borrow().len() => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, RowDVector, RowDVector, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        (source, ixes, sink) => Err(MechError2::new(
          UnhandledFunctionArgumentIxes { arg: (source.clone(), ixes.to_vec(), sink.clone()), fxn_name: stringify!($fxn_name).to_string() },
          None
        ).with_compiler_loc()),
      }
    }
  }
}

#[macro_export]
macro_rules! impl_set_range_arms_b {
  ($fxn_name:ident, $arg:expr, $value_kind:ident, $value_string:tt) => {
    paste! {
      match $arg {
        // Scalar source
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix1(sink)),[Value::MatrixBool(Matrix::Matrix1(ix))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, Matrix1, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix2(sink)),[Value::MatrixBool(Matrix::Vector4(ix))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, Matrix2, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "matrix3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix3(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, Matrix3, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "matrix4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix4(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, Matrix4, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, Matrix2x3, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, Matrix3x2, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "matrixd"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, DMatrix, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Vector2(sink)),[Value::MatrixBool(Matrix::Vector2(ix))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, Vector2, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Vector3(sink)),[Value::MatrixBool(Matrix::Vector3(ix))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, Vector3, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Vector4(sink)),[Value::MatrixBool(Matrix::Vector4(ix))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, Vector4, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::DVector(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, DVector, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "row_vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::RowVector2(sink)),[Value::MatrixBool(Matrix::Vector2(ix))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, RowVector2, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "row_vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::RowVector3(sink)),[Value::MatrixBool(Matrix::Vector3(ix))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, RowVector3, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "row_vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::RowVector4(sink)),[Value::MatrixBool(Matrix::Vector4(ix))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, RowVector4, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "row_vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::RowDVector(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, RowDVector, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        // Vector source, must have equal size to output
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix1(sink)), [Value::MatrixBool(Matrix::Matrix1(ix))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix1(source))) => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, Matrix1, Matrix1, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix2(sink)), [Value::MatrixBool(Matrix::Vector4(ix))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix2(source))) => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, Matrix2, Matrix2, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix3(sink)), [Value::MatrixBool(Matrix::DVector(ix))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix3(source))) => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, Matrix3, Matrix3, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix4(sink)), [Value::MatrixBool(Matrix::DVector(ix))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix4(source))) => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, Matrix4, Matrix4, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(sink)), [Value::MatrixBool(Matrix::DVector(ix))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(source))) => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, Matrix2x3, Matrix2x3, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(sink)), [Value::MatrixBool(Matrix::DVector(ix))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(source))) => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, Matrix3x2, Matrix3x2, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [Value::MatrixBool(Matrix::DVector(ix))], Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, DMatrix, DMatrix, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::DVector(sink)), [Value::MatrixBool(Matrix::DVector(ix))], Value::[<Matrix $value_kind:camel>](Matrix::DVector(source))) => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, DVector, DVector, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::RowDVector(sink)), [Value::MatrixBool(Matrix::DVector(ix))], Value::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, RowDVector, RowDVector, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Vector2(sink)), [Value::MatrixBool(Matrix::Vector2(ix))], Value::[<Matrix $value_kind:camel>](Matrix::Vector2(source))) => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, Vector2, Vector2, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Vector3(sink)), [Value::MatrixBool(Matrix::Vector3(ix))], Value::[<Matrix $value_kind:camel>](Matrix::Vector3(source))) => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, Vector3, Vector3, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Vector4(sink)), [Value::MatrixBool(Matrix::Vector4(ix))], Value::[<Matrix $value_kind:camel>](Matrix::Vector4(source))) => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, Vector4, Vector4, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::RowVector2(sink)), [Value::MatrixBool(Matrix::Vector2(ix))], Value::[<Matrix $value_kind:camel>](Matrix::RowVector2(source))) => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, RowVector2, RowVector2, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::RowVector3(sink)), [Value::MatrixBool(Matrix::Vector3(ix))], Value::[<Matrix $value_kind:camel>](Matrix::RowVector3(source))) => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, RowVector3, RowVector3, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::RowVector4(sink)), [Value::MatrixBool(Matrix::Vector4(ix))], Value::[<Matrix $value_kind:camel>](Matrix::RowVector4(source))) => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, RowVector4, RowVector4, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        (source, ixes, sink) => Err(MechError2::new(
          UnhandledFunctionArgumentIxes { arg: (source.clone(), ixes.to_vec(), sink.clone()), fxn_name: stringify!($fxn_name).to_string() },
          None
        ).with_compiler_loc()),
      }
    };
  };
}

#[macro_export]
macro_rules! impl_assign_scalar_arms_b {
  ($fxn_name:ident, $shape:tt, $arg:expr, $value_kind:ident, $value_string:tt) => {
    paste! {
      match $arg {
        #[cfg(feature = $value_string)]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[Value::Bool(ix)], Value::[<$value_kind:camel>](source)) => {
          register_assign_s1!([<$fxn_name B>], $value_kind, $value_string, $shape);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))           
        },
        // Vector source, must have equal size to output
        #[cfg(feature = $value_string)]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::Bool(ix)], Value::[<Matrix $value_kind:camel>](Matrix::$shape(source))) => {
          register_assign_s2!([<$fxn_name VB>], $value_kind, $value_string, $shape, $shape);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        (source, ixes, sink) => Err(MechError2::new(
          UnhandledFunctionArgumentIxes { arg: (source.clone(), ixes.to_vec(), sink.clone()), fxn_name: stringify!($fxn_name).to_string() },
          None
        ).with_compiler_loc()),
      }
    };
  };
}

#[macro_export]
macro_rules! impl_assign_range_scalar_arms_b {
  ($fxn_name:ident, $shape:tt, $arg:expr, $value_kind:ident, $value_string:tt) => {
    paste! {
      match $arg {
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[Value::MatrixBool(Matrix::Matrix1(ix1)), Value::Index(ix2)], Value::[<$value_kind:camel>](source)) => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, $shape, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[Value::MatrixBool(Matrix::Vector2(ix1)), Value::Index(ix2)], Value::[<$value_kind:camel>](source)) => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, $shape, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[Value::MatrixBool(Matrix::Vector3(ix1)), Value::Index(ix2)], Value::[<$value_kind:camel>](source)) => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, $shape, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[Value::MatrixBool(Matrix::Vector4(ix1)), Value::Index(ix2)], Value::[<$value_kind:camel>](source)) => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, $shape, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[Value::MatrixBool(Matrix::DVector(ix1)), Value::Index(ix2)], Value::[<$value_kind:camel>](source)) => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, $shape, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixBool(Matrix::Matrix1(ix1)), Value::Index(ix2)], Value::[<Matrix $value_kind:camel>](Matrix::Matrix1(source))) => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix1, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixBool(Matrix::Vector4(ix1)), Value::Index(ix2)], Value::[<Matrix $value_kind:camel>](Matrix::Matrix2(source))) => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix2, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixBool(Matrix::DVector(ix1)), Value::Index(ix2)], Value::[<Matrix $value_kind:camel>](Matrix::Matrix3(source))) if ix1.borrow().len() == source.borrow().len() => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix3, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixBool(Matrix::DVector(ix1)), Value::Index(ix2)], Value::[<Matrix $value_kind:camel>](Matrix::Matrix4(source))) if ix1.borrow().len() == source.borrow().len() => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix4, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixBool(Matrix::DVector(ix1)), Value::Index(ix2)], Value::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(source))) if ix1.borrow().len() == source.borrow().len() => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix2x3, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixBool(Matrix::DVector(ix1)), Value::Index(ix2)], Value::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(source))) if ix1.borrow().len() == source.borrow().len() => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix3x2, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixBool(Matrix::Vector2(ix1)), Value::Index(ix2)], Value::[<Matrix $value_kind:camel>](Matrix::Vector2(source))) => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, $shape, Vector2, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixBool(Matrix::Vector3(ix1)), Value::Index(ix2)], Value::[<Matrix $value_kind:camel>](Matrix::Vector3(source))) => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, $shape, Vector3, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixBool(Matrix::Vector4(ix1)), Value::Index(ix2)], Value::[<Matrix $value_kind:camel>](Matrix::Vector4(source))) => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, $shape, Vector4, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixBool(Matrix::Vector2(ix1)), Value::Index(ix2)], Value::[<Matrix $value_kind:camel>](Matrix::RowVector2(source))) => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, $shape, RowVector2, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixBool(Matrix::Vector3(ix1)), Value::Index(ix2)], Value::[<Matrix $value_kind:camel>](Matrix::RowVector3(source))) => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, $shape, RowVector3, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixBool(Matrix::Vector4(ix1)), Value::Index(ix2)], Value::[<Matrix $value_kind:camel>](Matrix::RowVector4(source))) => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, $shape, RowVector4, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixBool(Matrix::DVector(ix1)), Value::Index(ix2)], Value::[<Matrix $value_kind:camel>](Matrix::DVector(source))) if ix1.borrow().len() == source.borrow().len() => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, $shape, DVector, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixBool(Matrix::DVector(ix1)), Value::Index(ix2)], Value::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) if ix1.borrow().len() == source.borrow().len() => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, $shape, RowDVector, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::MatrixBool(Matrix::DVector(ix1)), Value::Index(ix2)], Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) if ix1.borrow().len() == source.borrow().len() => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, $shape, DMatrix, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        (source, ixes, sink) => Err(MechError2::new(
          UnhandledFunctionArgumentIxes { arg: (source.clone(), ixes.to_vec(), sink.clone()), fxn_name: stringify!($fxn_name).to_string() },
          None
        ).with_compiler_loc()),
      }
    };
  };
}

#[macro_export]
macro_rules! impl_assign_scalar_range_arms_b {
  ($fxn_name:ident, $shape:tt, $arg:expr, $value_kind:ident, $value_string:tt) => {
    paste! {
      match $arg {
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[Value::Index(ix1), Value::MatrixBool(Matrix::Matrix1(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, $shape, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[Value::Index(ix1), Value::MatrixBool(Matrix::Vector2(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, $shape, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[Value::Index(ix1), Value::MatrixBool(Matrix::Vector3(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, $shape, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[Value::Index(ix1), Value::MatrixBool(Matrix::Vector4(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, $shape, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))           
        },    
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[Value::Index(ix1), Value::MatrixBool(Matrix::DVector(ix2))], Value::[<$value_kind:camel>](source)) if ix2.borrow().len() == sink.borrow().ncols() => {
          register_assign_s_b!([<$fxn_name B>], $value_kind, $value_string, $shape, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))           
        },
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::Index(ix1), Value::MatrixBool(Matrix::Matrix1(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix1(source))) => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix1, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::Index(ix1), Value::MatrixBool(Matrix::Vector4(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix2(source))) => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix2, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::Index(ix1), Value::MatrixBool(Matrix::DVector(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix3(source))) if ix2.borrow().len() == source.borrow().len() && ix2.borrow().len() == sink.borrow().ncols() => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix3, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::Index(ix1), Value::MatrixBool(Matrix::DVector(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix4(source))) if ix2.borrow().len() == source.borrow().len() && ix2.borrow().len() == sink.borrow().ncols() => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix4, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::Index(ix1), Value::MatrixBool(Matrix::DVector(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(source))) if ix2.borrow().len() == source.borrow().len() && ix2.borrow().len() == sink.borrow().ncols() => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix2x3, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::Index(ix1), Value::MatrixBool(Matrix::DVector(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(source))) if ix2.borrow().len() == source.borrow().len() && ix2.borrow().len() == sink.borrow().ncols() => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, $shape, Matrix3x2, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::Index(ix1), Value::MatrixBool(Matrix::Vector2(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::Vector2(source))) => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, $shape, Vector2, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::Index(ix1), Value::MatrixBool(Matrix::Vector3(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::Vector3(source))) => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, $shape, Vector3, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::Index(ix1), Value::MatrixBool(Matrix::Vector4(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::Vector4(source))) => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, $shape, Vector4, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::Index(ix1), Value::MatrixBool(Matrix::Vector2(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::RowVector2(source))) => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, $shape, RowVector2, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::Index(ix1), Value::MatrixBool(Matrix::Vector3(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::RowVector3(source))) => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, $shape, RowVector3, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::Index(ix1), Value::MatrixBool(Matrix::Vector4(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::RowVector4(source))) => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, $shape, RowVector4, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::Index(ix1), Value::MatrixBool(Matrix::DVector(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::DVector(source))) if ix2.borrow().len() == sink.borrow().ncols() && ix2.borrow().len() == source.borrow().len() => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, $shape, DVector, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::Index(ix1), Value::MatrixBool(Matrix::DVector(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) if ix2.borrow().len() == sink.borrow().ncols() && ix2.borrow().len() == source.borrow().len() => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, $shape, RowDVector, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [Value::Index(ix1), Value::MatrixBool(Matrix::DVector(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) if ix2.borrow().len() == sink.borrow().ncols() && ix2.borrow().len() == source.borrow().len() => {
          register_assign_b!([<$fxn_name VB>], $value_kind, $value_string, $shape, DMatrix, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        (source, ixes, sink) => Err(MechError2::new(
          UnhandledFunctionArgumentIxes { arg: (source.clone(), ixes.to_vec(), sink.clone()), fxn_name: stringify!($fxn_name).to_string() },
          None
        ).with_compiler_loc()),
      }
    };
  };
}

#[macro_export]
macro_rules! impl_assign_range_range_arms_b {
  ($fxn_name:ident, $shape:tt, $arg:expr, $value_kind:ident, $value_string:tt) => {
    paste! {
      match $arg {
        // Scalar source
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix1(sink)), [Value::MatrixBool(Matrix::Matrix1(ix1)), Value::MatrixBool(Matrix::Matrix1(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_srr_b!([<$fxn_name BB>], $value_kind, $value_string, Matrix1, Matrix1, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name BB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "matrix1", feature = "row_vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::RowVector2(sink)), [Value::MatrixBool(Matrix::Matrix1(ix1)), Value::MatrixBool(Matrix::Vector2(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_srr_b!([<$fxn_name BB>], $value_kind, $value_string, RowVector2, Matrix1, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name BB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "matrix1", feature = "row_vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::RowVector3(sink)), [Value::MatrixBool(Matrix::Matrix1(ix1)), Value::MatrixBool(Matrix::Vector3(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_srr_b!([<$fxn_name BB>], $value_kind, $value_string, RowVector3, Matrix1, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name BB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector4", feature = "matrix1", feature = "row_vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::RowVector4(sink)), [Value::MatrixBool(Matrix::Matrix1(ix1)), Value::MatrixBool(Matrix::Vector4(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_srr_b!([<$fxn_name BB>], $value_kind, $value_string, RowVector4, Matrix1, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name BB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "matrix1", feature = "row_vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::RowDVector(sink)), [Value::MatrixBool(Matrix::Matrix1(ix1)), Value::MatrixBool(Matrix::DVector(ix2))], Value::[<$value_kind:camel>](source)) if ix2.borrow().len() == sink.borrow().len() => {
          register_assign_srr_b!([<$fxn_name BB>], $value_kind, $value_string, RowDVector,Matrix1,DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name BB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector2", feature = "matrix1"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Vector2(sink)), [Value::MatrixBool(Matrix::Vector2(ix1)), Value::MatrixBool(Matrix::Matrix1(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_srr_b!([<$fxn_name BB>], $value_kind, $value_string, Vector2,Vector2,Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name BB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector3", feature = "matrix1"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Vector3(sink)), [Value::MatrixBool(Matrix::Vector3(ix1)), Value::MatrixBool(Matrix::Matrix1(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_srr_b!([<$fxn_name BB>], $value_kind, $value_string, Vector3,Vector3,Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name BB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector4", feature = "matrix1"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Vector4(sink)), [Value::MatrixBool(Matrix::Vector4(ix1)), Value::MatrixBool(Matrix::Matrix1(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_srr_b!([<$fxn_name BB>], $value_kind, $value_string, Vector4,Vector4,Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name BB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord", feature = "vectord", feature = "matrix1"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::DVector(sink)), [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::Matrix1(ix2))], Value::[<$value_kind:camel>](source)) if ix1.borrow().len() == sink.borrow().len() => {
          register_assign_srr_b!([<$fxn_name BB>], $value_kind, $value_string, DVector,DVector,Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name BB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix2(sink)), [Value::MatrixBool(Matrix::Vector2(ix1)), Value::MatrixBool(Matrix::Vector2(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_srr_b!([<$fxn_name BB>], $value_kind, $value_string, Matrix2,Vector2,Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name BB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3", feature = "vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix3(sink)), [Value::MatrixBool(Matrix::Vector3(ix1)), Value::MatrixBool(Matrix::Vector3(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_srr_b!([<$fxn_name BB>], $value_kind, $value_string, Matrix2,Matrix3,Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name BB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix4", feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix4(sink)), [Value::MatrixBool(Matrix::Vector4(ix1)), Value::MatrixBool(Matrix::Vector4(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_srr_b!([<$fxn_name BB>], $value_kind, $value_string, Matrix4,Vector4,Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name BB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "vector3", feature = "vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(sink)), [Value::MatrixBool(Matrix::Vector2(ix1)), Value::MatrixBool(Matrix::Vector3(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_srr_b!([<$fxn_name BB>], $value_kind, $value_string, Matrix2x3,Vector2,Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name BB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "vector2", feature = "vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(sink)), [Value::MatrixBool(Matrix::Vector3(ix1)), Value::MatrixBool(Matrix::Vector2(ix2))], Value::[<$value_kind:camel>](source)) => {
          register_assign_srr_b!([<$fxn_name BB>], $value_kind, $value_string, Matrix3x2,Vector3,Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name BB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vector4", feature = "vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [Value::MatrixBool(Matrix::Vector4(ix1)), Value::MatrixBool(Matrix::Vector2(ix2))], Value::[<$value_kind:camel>](source)) if sink.borrow().nrows() == 4 && sink.borrow().ncols() == 2 => {
          register_assign_srr_b!([<$fxn_name BB>], $value_kind, $value_string, DMatrix, Vector4, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name BB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vector2", feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [Value::MatrixBool(Matrix::Vector2(ix1)), Value::MatrixBool(Matrix::Vector4(ix2))], Value::[<$value_kind:camel>](source)) if sink.borrow().nrows() == 2 && sink.borrow().ncols() == 4 => {
          register_assign_srr_b!([<$fxn_name BB>], $value_kind, $value_string, DMatrix, Vector2, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name BB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::Vector2(ix2))], Value::[<$value_kind:camel>](source)) if sink.borrow().ncols() == 2 && ix1.borrow().len() == sink.borrow().nrows() => {
          register_assign_srr_b!([<$fxn_name BB>], $value_kind, $value_string, DMatrix, DVector, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name BB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::Vector3(ix2))], Value::[<$value_kind:camel>](source)) if sink.borrow().ncols() == 3 && ix1.borrow().len() == sink.borrow().nrows() => {
          register_assign_srr_b!([<$fxn_name BB>], $value_kind, $value_string, DMatrix, DVector, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name BB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::Vector4(ix2))], Value::[<$value_kind:camel>](source)) if sink.borrow().ncols() == 4 && ix1.borrow().len() == sink.borrow().nrows() => {
          register_assign_srr_b!([<$fxn_name BB>], $value_kind, $value_string, DMatrix, DVector, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name BB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::DVector(ix2))], Value::[<$value_kind:camel>](source)) if ix1.borrow().len() == sink.borrow().nrows() && ix2.borrow().len() == sink.borrow().ncols() => {
          register_assign_srr_b!([<$fxn_name BB>], $value_kind, $value_string, DMatrix, DVector, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name BB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        // Vector source
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix1(sink)), [Value::MatrixBool(Matrix::Matrix1(ix1)), Value::MatrixBool(Matrix::Matrix1(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix1(source))) => {
          register_assign_srr_b2!([<$fxn_name VBB>], $value_kind, $value_string, Matrix1, Matrix1, Matrix1, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "matrix1", feature = "row_vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::RowVector2(sink)), [Value::MatrixBool(Matrix::Matrix1(ix1)), Value::MatrixBool(Matrix::Vector2(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::RowVector2(source))) => {
          register_assign_srr_b2!([<$fxn_name VBB>], $value_kind, $value_string, RowVector2, RowVector2, Matrix1, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "matrix1", feature = "row_vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::RowVector3(sink)), [Value::MatrixBool(Matrix::Matrix1(ix1)), Value::MatrixBool(Matrix::Vector3(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::RowVector3(source))) => {
          register_assign_srr_b2!([<$fxn_name VBB>], $value_kind, $value_string, RowVector3, RowVector3, Matrix1, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector4", feature = "matrix1", feature = "row_vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::RowVector4(sink)), [Value::MatrixBool(Matrix::Matrix1(ix1)), Value::MatrixBool(Matrix::Vector4(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::RowVector4(source))) => {
          register_assign_srr_b2!([<$fxn_name VBB>], $value_kind, $value_string, RowVector4, RowVector4, Matrix1, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "matrix1", feature = "row_vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::RowDVector(sink)), [Value::MatrixBool(Matrix::Matrix1(ix1)), Value::MatrixBool(Matrix::DVector(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) if ix2.borrow().len() == sink.borrow().len() && ix2.borrow().len() == source.borrow().len() => {
          register_assign_srr_b2!([<$fxn_name VBB>], $value_kind, $value_string, RowDVector, RowDVector, Matrix1, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector2", feature = "matrix1"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Vector2(sink)), [Value::MatrixBool(Matrix::Vector2(ix1)), Value::MatrixBool(Matrix::Matrix1(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::Vector2(source))) => {
          register_assign_srr_b2!([<$fxn_name VBB>], $value_kind, $value_string, Vector2, Vector2, Vector2, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector3", feature = "matrix1"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Vector3(sink)), [Value::MatrixBool(Matrix::Vector3(ix1)), Value::MatrixBool(Matrix::Matrix1(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::Vector3(source))) => {
          register_assign_srr_b2!([<$fxn_name VBB>], $value_kind, $value_string, Vector3, Vector3, Vector3, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector4", feature = "matrix1"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Vector4(sink)), [Value::MatrixBool(Matrix::Vector4(ix1)), Value::MatrixBool(Matrix::Matrix1(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::Vector4(source))) => {
          register_assign_srr_b2!([<$fxn_name VBB>], $value_kind, $value_string, Vector4, Vector4, Vector4, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord", feature = "vectord", feature = "matrix1"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::DVector(sink)), [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::Matrix1(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::DVector(source))) if ix1.borrow().len() == sink.borrow().len() && ix2.borrow().len() == source.borrow().len() => {
          register_assign_srr_b2!([<$fxn_name VBB>], $value_kind, $value_string, DVector, DVector, DVector, Matrix1);
          box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix2(sink)), [Value::MatrixBool(Matrix::Vector2(ix1)), Value::MatrixBool(Matrix::Vector2(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix2(source))) => {
          register_assign_srr_b2!([<$fxn_name VBB>], $value_kind, $value_string, Matrix2, Matrix2, Vector2, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3", feature = "vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix3(sink)), [Value::MatrixBool(Matrix::Vector3(ix1)), Value::MatrixBool(Matrix::Vector3(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix3(source))) => {
          register_assign_srr_b2!([<$fxn_name VBB>], $value_kind, $value_string, Matrix3, Matrix3, Vector3, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix4", feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix4(sink)), [Value::MatrixBool(Matrix::Vector4(ix1)), Value::MatrixBool(Matrix::Vector4(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix4(source))) => {
          register_assign_srr_b2!([<$fxn_name VBB>], $value_kind, $value_string, Matrix4, Matrix4, Vector4, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "vector3", feature = "vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(sink)), [Value::MatrixBool(Matrix::Vector2(ix1)), Value::MatrixBool(Matrix::Vector3(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(source))) => {
          register_assign_srr_b2!([<$fxn_name VBB>], $value_kind, $value_string, Matrix2x3, Matrix2x3, Vector2, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "vector2", feature = "vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(sink)), [Value::MatrixBool(Matrix::Vector3(ix1)), Value::MatrixBool(Matrix::Vector2(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(source))) => {
          register_assign_srr_b2!([<$fxn_name VBB>], $value_kind, $value_string, Matrix3x2, Matrix3x2, Vector3, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vector4", feature = "vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [Value::MatrixBool(Matrix::Vector4(ix1)), Value::MatrixBool(Matrix::Vector2(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) if sink.borrow().nrows() == 4 && sink.borrow().ncols() == 2 && source.borrow().nrows() == ix1.borrow().len() && source.borrow().ncols() == ix2.borrow().len() => {
          register_assign_srr_b2!([<$fxn_name VBB>], $value_kind, $value_string, DMatrix, DMatrix, Vector4, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vector2", feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [Value::MatrixBool(Matrix::Vector2(ix1)), Value::MatrixBool(Matrix::Vector4(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) if sink.borrow().nrows() == 2 && sink.borrow().ncols() == 4 && source.borrow().nrows() == ix1.borrow().len() && source.borrow().ncols() == ix2.borrow().len() => {
          register_assign_srr_b2!([<$fxn_name VBB>], $value_kind, $value_string, DMatrix, DMatrix, Vector2, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vector2"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::Vector2(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) if sink.borrow().ncols() == 2 && ix1.borrow().len() == sink.borrow().nrows() && source.borrow().nrows() == ix1.borrow().len() && source.borrow().ncols() == ix2.borrow().len() => {
          register_assign_srr_b2!([<$fxn_name VBB>], $value_kind, $value_string, DMatrix, DMatrix, DVector, Vector2);
          box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vector3"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::Vector3(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) if sink.borrow().ncols() == 3 && ix1.borrow().len() == sink.borrow().nrows() && source.borrow().nrows() == ix1.borrow().len() && source.borrow().ncols() == ix2.borrow().len() => {
          register_assign_srr_b2!([<$fxn_name VBB>], $value_kind, $value_string, DMatrix, DMatrix, DVector, Vector3);
          box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vector4"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::Vector4(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) if sink.borrow().ncols() == 4 && ix1.borrow().len() == sink.borrow().nrows() && source.borrow().nrows() == ix1.borrow().len() && source.borrow().ncols() == ix2.borrow().len() => {
          register_assign_srr_b2!([<$fxn_name VBB>], $value_kind, $value_string, DMatrix, DMatrix, DVector, Vector4);
          box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::DVector(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) if ix1.borrow().len() == sink.borrow().nrows() && ix2.borrow().len() == sink.borrow().ncols() && source.borrow().nrows() == ix1.borrow().len() && source.borrow().ncols() == ix2.borrow().len() => {
          register_assign_srr_b2!([<$fxn_name VBB>], $value_kind, $value_string, DMatrix, DMatrix, DVector, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        (source, ixes, sink) => Err(MechError2::new(
          UnhandledFunctionArgumentIxes { arg: (source.clone(), ixes.to_vec(), sink.clone()), fxn_name: stringify!($fxn_name).to_string() },
          None
        ).with_compiler_loc()),
      }
    }
  }
}

#[macro_export]
macro_rules! impl_assign_range_range_arms_bu {
  ($fxn_name:ident, $shape:tt, $arg:expr, $value_kind:ident, $value_string:tt) => {
    paste! {
      match $arg {
        // Scalar source
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))], Value::[<$value_kind:camel>](source)) if ix1.borrow().len() == sink.borrow().nrows() => {
          register_assign_srr_bu!([<$fxn_name BU>], $value_kind, $value_string, DMatrix, DVector, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name BU>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        // Vector source
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [Value::MatrixBool(Matrix::DVector(ix1)), Value::MatrixIndex(Matrix::DVector(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) if ix1.borrow().len() == sink.borrow().nrows() && ix1.borrow().len() == source.borrow().nrows() => {
          register_assign_srr_bu2!([<$fxn_name VBU>], $value_kind, $value_string, DMatrix, DMatrix, DVector, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name VBU>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        (source, ixes, sink) => Err(MechError2::new(
          UnhandledFunctionArgumentIxes { arg: (source.clone(), ixes.to_vec(), sink.clone()), fxn_name: stringify!($fxn_name).to_string() },
          None
        ).with_compiler_loc()),
      }
    }
  }
}

#[macro_export]
macro_rules! impl_assign_range_range_arms_ub {
  ($fxn_name:ident, $shape:tt, $arg:expr, $value_kind:ident, $value_string:tt) => {
    paste! {
      match $arg {
        // Scalar-per-column source (UB)
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::DVector(ix2))], Value::[<$value_kind:camel>](source)) if ix2.borrow().len() == sink.borrow().ncols() => {
          register_assign_srr_ub!([<$fxn_name UB>], $value_kind, $value_string, DMatrix, DVector, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name UB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        // Vector-per-column source (VUB)
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vectord"))]
        (Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [Value::MatrixIndex(Matrix::DVector(ix1)), Value::MatrixBool(Matrix::DVector(ix2))], Value::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) if ix2.borrow().len() == sink.borrow().ncols() && ix2.borrow().len() == source.borrow().ncols() => {
          register_assign_srr_ub2!([<$fxn_name VUB>], $value_kind, $value_string, DMatrix, DMatrix, DVector, DVector);
          box_mech_fxn(Ok(Box::new([<$fxn_name VUB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        (source, ixes, sink) => Err(MechError2::new(
          UnhandledFunctionArgumentIxes { arg: (source.clone(), ixes.to_vec(), sink.clone()), fxn_name: stringify!($fxn_name).to_string() },
          None
        ).with_compiler_loc()),
      }
    }
  }
}

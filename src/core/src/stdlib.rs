pub use crate::*;

// The Standard Library
// ----------------------------------------------------------------------------

// These macros are used by various libraries to generate function impl and
// match arms. They're gated on feature flags so although there's a lot of
// code here to account for all the different combinations, but only
// the relevant code will be compiled in any given build.

#[cfg(feature = "semantic-compiler")]
#[macro_export]
macro_rules! compile_register_brrw {
    ($reg:expr, $ctx:ident) => {{
        let addr = $reg.addr();
        let (reg, needs_initialization) = $ctx.register_for_ptr_with_initialization_status(addr);
        if needs_initialization {
            let borrow = $reg.borrow();
            let const_id = borrow.compile_const($ctx)?;
            $ctx.record_register_constant_kind(reg, const_id)?;
            $ctx.emit_const_load(reg, const_id);
        }
        reg
    }};
}

#[cfg(feature = "semantic-compiler")]
#[macro_export]
macro_rules! compile_register {
    ($reg:expr, $ctx:ident) => {{
        let addr = $reg.addr();
        let (reg, needs_initialization) = $ctx.register_for_ptr_with_initialization_status(addr);
        if needs_initialization {
            let const_id = $reg.compile_const($ctx)?;
            $ctx.record_register_constant_kind(reg, const_id)?;
            $ctx.emit_const_load(reg, const_id);
        }
        reg
    }};
}

#[cfg(feature = "semantic-compiler")]
#[macro_export]
macro_rules! compile_register_mat {
    ($reg:expr, $ctx:ident) => {{
        let addr = $reg.addr();
        let (reg, needs_initialization) = $ctx.register_for_ptr_with_initialization_status(addr);
        if needs_initialization {
            let const_id = $reg.compile_const_mat($ctx)?;
            $ctx.record_register_constant_kind(reg, const_id)?;
            $ctx.emit_const_load(reg, const_id);
        }
        reg
    }};
}

#[cfg(feature = "semantic-compiler")]
#[macro_export]
macro_rules! compile_register_initial {
    ($reg:expr, $initial:expr, $ctx:ident) => {{
        let addr = $reg.addr();
        let (reg, needs_initialization) = $ctx.register_for_ptr_with_initialization_status(addr);
        if needs_initialization {
            let const_id = $initial.compile_const($ctx)?;
            $ctx.record_register_constant_kind(reg, const_id)?;
            $ctx.emit_const_load(reg, const_id);
        }
        reg
    }};
}

#[cfg(feature = "semantic-compiler")]
#[macro_export]
macro_rules! compile_nullop {
    ($name:tt, $out:expr, $ctx:ident) => {
        // allocate one register as an array
        let mut registers = [0];

        // Compile out
        registers[0] = compile_register_brrw!($out, $ctx);

        let function = $ctx.function_id(&$name)?;
        $ctx.emit_nullop(function, registers[0]);

        return Ok(registers[0]);
    };
}

#[cfg(feature = "semantic-compiler")]
#[macro_export]
macro_rules! compile_unop {
    ($name:tt, $out:expr, $arg:expr, $ctx:ident) => {
        // Allocate three registers as an array
        let mut registers = [0, 0];

        // Allocate registers
        registers[0] = compile_register_brrw!($out, $ctx);
        registers[1] = compile_register_brrw!($arg, $ctx);

        let function = $ctx.function_id(&$name)?;
        $ctx.emit_unop(function, registers[0], registers[1]);

        return Ok(registers[0]);
    };
}

#[cfg(feature = "semantic-compiler")]
#[macro_export]
macro_rules! compile_binop {
    ($name:tt, $out:expr, $arg1:expr, $arg2:expr, $ctx:ident) => {
        let mut registers = [0, 0, 0];

        registers[0] = compile_register_brrw!($out, $ctx);
        registers[1] = compile_register_brrw!($arg1, $ctx);
        registers[2] = compile_register_brrw!($arg2, $ctx);
        let function = $ctx.function_id(&$name)?;
        $ctx.emit_binop(function, registers[0], registers[1], registers[2]);

        return Ok(registers[0])
    };
}

#[cfg(feature = "semantic-compiler")]
#[macro_export]
macro_rules! compile_ternop {
    ($name:tt, $out:expr, $arg1:expr, $arg2:expr, $arg3:expr, $ctx:ident) => {
        let mut registers = [0, 0, 0, 0];

        registers[0] = compile_register_brrw!($out, $ctx);
        registers[1] = compile_register_brrw!($arg1, $ctx);
        registers[2] = compile_register_brrw!($arg2, $ctx);
        registers[3] = compile_register_brrw!($arg3, $ctx);
        let function = $ctx.function_id(&$name)?;
        $ctx.emit_ternop(
            function,
            registers[0],
            registers[1],
            registers[2],
            registers[3],
        );

        return Ok(registers[0])
    };
}

#[cfg(feature = "semantic-compiler")]
#[macro_export]
macro_rules! compile_quadop {
    ($name:tt, $out:expr, $arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr, $ctx:ident) => {
        let mut registers = [0, 0, 0, 0, 0];

        registers[0] = compile_register_brrw!($out, $ctx);
        registers[1] = compile_register_brrw!($arg1, $ctx);
        registers[2] = compile_register_brrw!($arg2, $ctx);
        registers[3] = compile_register_brrw!($arg3, $ctx);
        registers[4] = compile_register_brrw!($arg4, $ctx);
        let function = $ctx.function_id(&$name)?;
        $ctx.emit_quadop(
            function,
            registers[0],
            registers[1],
            registers[2],
            registers[3],
            registers[4],
        );
        return Ok(registers[0])
    };
}

#[cfg(feature = "semantic-compiler")]
#[macro_export]
macro_rules! compile_varop {
    ($name:tt, $out:expr, $args:expr, $ctx:ident) => {
        let arg_count = $args.len();
        let mut registers = vec![0; arg_count + 1];
        registers[0] = compile_register_brrw!($out, $ctx);
        for i in 0..arg_count {
            registers[i + 1] = compile_register_brrw!($args[i], $ctx);
        }
        let function = $ctx.function_id(&$name)?;
        $ctx.emit_varop(function, registers[0], (&registers[1..]).to_vec());
        return Ok(registers[0])
    };
}
#[macro_export]
macro_rules! impl_binop {
    ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $op:ident) => {
        #[derive(Debug)]
        pub struct $struct_name<T> {
            pub lhs: Ref<$arg1_type>,
            pub rhs: Ref<$arg2_type>,
            pub out: Ref<$out_type>,
        }
        impl<T> MechFunctionFactory for $struct_name<T>
        where
            #[cfg(feature = "semantic-compiler")]
            T: Copy
                + Debug
                + Display
                + Clone
                + Sync
                + Send
                + 'static
                + PartialEq
                + PartialOrd
                + ConstElem
                + CompileConst
                + AsValueKind
                + Add<Output = T>
                + AddAssign
                + Sub<Output = T>
                + SubAssign
                + Mul<Output = T>
                + MulAssign
                + Div<Output = T>
                + DivAssign
                + Zero
                + One,
            #[cfg(not(feature = "semantic-compiler"))]
            T: Copy
                + Debug
                + Display
                + Clone
                + Sync
                + Send
                + 'static
                + PartialEq
                + PartialOrd
                + AsValueKind
                + Add<Output = T>
                + AddAssign
                + Sub<Output = T>
                + SubAssign
                + Mul<Output = T>
                + MulAssign
                + Div<Output = T>
                + DivAssign
                + Zero
                + One,
            Ref<$out_type>: ToValue,
            $arg1_type: FunctionRuntimeType + FunctionPortBacking,
            $arg2_type: FunctionRuntimeType + FunctionPortBacking,
            $out_type: FunctionRuntimeType + FunctionPortBacking,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                <$out_type as FunctionRuntimeType>::REPRESENTATION,
                <$arg1_type as FunctionRuntimeType>::REPRESENTATION,
                <$arg2_type as FunctionRuntimeType>::REPRESENTATION,
            );

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, lhs, rhs) = invocation.expect_binary()?;
                let lhs: Ref<$arg1_type> = lhs.try_ref()?;
                let rhs: Ref<$arg2_type> = rhs.try_ref()?;
                let out: Ref<$out_type> = out.try_ref()?;
                Ok(Box::new(Self { lhs, rhs, out }))
            }

            fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
                Self::new_invocation(args.into())
            }
        }
        impl<T> MechFunctionImpl for $struct_name<T>
        where
            T: Copy
                + Debug
                + Display
                + Clone
                + Sync
                + Send
                + 'static
                + PartialEq
                + PartialOrd
                + Add<Output = T>
                + AddAssign
                + Sub<Output = T>
                + SubAssign
                + Mul<Output = T>
                + MulAssign
                + Div<Output = T>
                + DivAssign
                + Zero
                + One,
            Ref<$out_type>: ToValue,
        {
            fn solve_result(&self) -> MResult<()> {
                let lhs_ptr = self.lhs.as_ptr();
                let rhs_ptr = self.rhs.as_ptr();
                let out_ptr = self.out.as_mut_ptr();
                $op!(lhs_ptr, rhs_ptr, out_ptr);
                Ok(())
            }
            fn out(&self) -> LegacyValue {
                self.out.to_value()
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }

            fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
                Ok(self.reactive_output_values())
            }
        }
        #[cfg(feature = "semantic-compiler")]
        impl<T> MechFunctionCompiler for $struct_name<T>
        where
            T: ConstElem + CompileConst + AsValueKind,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!("{}<{}>", stringify!($struct_name), T::as_value_kind());
                compile_binop!(name, self.out, self.lhs, self.rhs, ctx);
            }
        }
    };
}

#[macro_export]
macro_rules! impl_unop {
    ($struct_name:ident, $arg_type:ty, $out_type:ty, $op:ident $(, $semantic_contract:path)?) => {
        #[derive(Debug)]
        pub(crate) struct $struct_name {
            arg: Ref<$arg_type>,
            out: Ref<$out_type>,
        }
        impl MechFunctionFactory for $struct_name
        where
            $arg_type: FunctionPortBacking,
            $out_type: FunctionPortBacking,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::unary(
                <$out_type as FunctionRuntimeType>::REPRESENTATION,
                <$arg_type as FunctionRuntimeType>::REPRESENTATION,
            );

            fn new_invocation(
                invocation: FunctionInvocation,
            ) -> MResult<Box<dyn MechFunction>> {
                let (out, arg) = invocation.expect_unary()?;
                let arg: Ref<$arg_type> = arg.try_ref()?;
                let out: Ref<$out_type> = out.try_ref()?;
                Ok(Box::new(Self { arg, out }))
            }

            fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
                Self::new_invocation(args.into())
            }
        }
        impl MechFunctionImpl for $struct_name {
            fn solve_result(&self) -> MResult<()> {
                let arg_ptr = self.arg.as_ptr();
                let out_ptr = self.out.as_mut_ptr();
                $op!(arg_ptr, out_ptr);
                Ok(())
            }
            fn out(&self) -> LegacyValue {
                self.out.to_value()
            }
            fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
                let contract: Option<&'static OperationContractDeclaration> = None;
                $(let contract = Some($semantic_contract(
                    <$out_type as FunctionRuntimeType>::REPRESENTATION,
                ));)?
                contract
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }

            fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
                Ok(self.reactive_output_values())
            }
        }
        #[cfg(feature = "semantic-compiler")]
        impl MechFunctionCompiler for $struct_name {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!("{}", stringify!($struct_name));
                compile_unop!(name, self.out, self.arg, ctx);
            }
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __mech_for_each_exact_binop_runtime_factory_group {
    (
        $callback:path,
        $context:tt,
        $lib:ident,
        $scalar:ty,
        $scalar_name:literal,
        $scalar_token:ident;
        $cfg:meta;
        $shape_features:tt;
        $($suffix:ident),+ $(,)?
    ) => {
        $(
            #[cfg($cfg)]
            $callback!(
                $context,
                $lib,
                $suffix,
                $shape_features,
                $scalar,
                $scalar_name,
                $scalar_token
            );
        )+
    };
}

/// Enumerates the exact concrete binary-factory surface shared by native
/// declarations, aggregate registration, and hidden installer exports. The
/// feature vector records every storage feature retained by a concrete type.
#[doc(hidden)]
#[macro_export]
macro_rules! __mech_for_each_exact_binop_runtime_factory_for_type {
    ($callback:path, $context:tt, $lib:ident, $scalar:ty, $scalar_name:literal, $scalar_token:ident) => {
        $callback!($context, $lib, SS, [], $scalar, $scalar_name, $scalar_token);

        $crate::__mech_for_each_exact_binop_runtime_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; feature = "matrix1"; ["matrix1"]; SM1, M1S, M1M1);
        $crate::__mech_for_each_exact_binop_runtime_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; feature = "matrix2"; ["matrix2"]; SM2, M2S, M2M2);
        $crate::__mech_for_each_exact_binop_runtime_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; feature = "matrix3"; ["matrix3"]; SM3, M3S, M3M3);
        $crate::__mech_for_each_exact_binop_runtime_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; feature = "matrix4"; ["matrix4"]; SM4, M4S, M4M4);
        $crate::__mech_for_each_exact_binop_runtime_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; feature = "matrix2x3"; ["matrix2x3"]; SM2x3, M2x3S, M2x3M2x3);
        $crate::__mech_for_each_exact_binop_runtime_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; feature = "matrix3x2"; ["matrix3x2"]; SM3x2, M3x2S, M3x2M3x2);
        $crate::__mech_for_each_exact_binop_runtime_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; feature = "matrixd"; ["matrixd"]; SMD, MDS, MDMD);

        $crate::__mech_for_each_exact_binop_runtime_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; feature = "row_vector2"; ["row_vector2"]; SR2, R2S, R2R2);
        $crate::__mech_for_each_exact_binop_runtime_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; feature = "row_vector3"; ["row_vector3"]; SR3, R3S, R3R3);
        $crate::__mech_for_each_exact_binop_runtime_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; feature = "row_vector4"; ["row_vector4"]; SR4, R4S, R4R4);
        $crate::__mech_for_each_exact_binop_runtime_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; feature = "row_vectord"; ["row_vectord"]; SRD, RDS, RDRD);

        $crate::__mech_for_each_exact_binop_runtime_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; feature = "vector2"; ["vector2"]; SV2, V2S, V2V2);
        $crate::__mech_for_each_exact_binop_runtime_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; feature = "vector3"; ["vector3"]; SV3, V3S, V3V3);
        $crate::__mech_for_each_exact_binop_runtime_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; feature = "vector4"; ["vector4"]; SV4, V4S, V4V4);
        $crate::__mech_for_each_exact_binop_runtime_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; feature = "vectord"; ["vectord"]; SVD, VDS, VDVD);

        $crate::__mech_for_each_exact_binop_runtime_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrix2", feature = "vector2"); ["matrix2", "vector2"]; M2V2, V2M2);
        $crate::__mech_for_each_exact_binop_runtime_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrix3", feature = "vector3"); ["matrix3", "vector3"]; M3V3, V3M3);
        $crate::__mech_for_each_exact_binop_runtime_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrix4", feature = "vector4"); ["matrix4", "vector4"]; M4V4, V4M4);
        $crate::__mech_for_each_exact_binop_runtime_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrix2x3", feature = "vector2"); ["matrix2x3", "vector2"]; M2x3V2, V2M2x3);
        $crate::__mech_for_each_exact_binop_runtime_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrix3x2", feature = "vector3"); ["matrix3x2", "vector3"]; M3x2V3, V3M3x2);
        $crate::__mech_for_each_exact_binop_runtime_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrixd", feature = "vectord"); ["matrixd", "vectord"]; MDVD, VDMD);
        $crate::__mech_for_each_exact_binop_runtime_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrixd", feature = "vector2"); ["matrixd", "vector2"]; MDV2, V2MD);
        $crate::__mech_for_each_exact_binop_runtime_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrixd", feature = "vector3"); ["matrixd", "vector3"]; MDV3, V3MD);
        $crate::__mech_for_each_exact_binop_runtime_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrixd", feature = "vector4"); ["matrixd", "vector4"]; MDV4, V4MD);

        $crate::__mech_for_each_exact_binop_runtime_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrix2", feature = "row_vector2"); ["matrix2", "row_vector2"]; M2R2, R2M2);
        $crate::__mech_for_each_exact_binop_runtime_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrix3", feature = "row_vector3"); ["matrix3", "row_vector3"]; M3R3, R3M3);
        $crate::__mech_for_each_exact_binop_runtime_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrix4", feature = "row_vector4"); ["matrix4", "row_vector4"]; M4R4, R4M4);
        $crate::__mech_for_each_exact_binop_runtime_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrix2x3", feature = "row_vector3"); ["matrix2x3", "row_vector3"]; M2x3R3, R3M2x3);
        $crate::__mech_for_each_exact_binop_runtime_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrix3x2", feature = "row_vector2"); ["matrix3x2", "row_vector2"]; M3x2R2, R2M3x2);
        $crate::__mech_for_each_exact_binop_runtime_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrixd", feature = "row_vectord"); ["matrixd", "row_vectord"]; MDRD, RDMD);
        $crate::__mech_for_each_exact_binop_runtime_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrixd", feature = "row_vector2"); ["matrixd", "row_vector2"]; MDR2, R2MD);
        $crate::__mech_for_each_exact_binop_runtime_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrixd", feature = "row_vector3"); ["matrixd", "row_vector3"]; MDR3, R3MD);
        $crate::__mech_for_each_exact_binop_runtime_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrixd", feature = "row_vector4"); ["matrixd", "row_vector4"]; MDR4, R4MD);
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __mech_for_each_binop_runtime_factory_for_type {
    ($callback:path, $context:tt, $lib:ident, $scalar:ty, $scalar_name:literal, $scalar_token:ident) => {
        $callback!(
            $context,
            $lib,
            SS,
            none,
            $scalar,
            $scalar_name,
            $scalar_token
        );

        #[cfg(feature = "matrix1")]
        $callback!(
            $context,
            $lib,
            SM1,
            "matrix1",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(feature = "matrix1")]
        $callback!(
            $context,
            $lib,
            M1S,
            "matrix1",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(feature = "matrix1")]
        $callback!(
            $context,
            $lib,
            M1M1,
            "matrix1",
            $scalar,
            $scalar_name,
            $scalar_token
        );

        #[cfg(feature = "matrix2")]
        $callback!(
            $context,
            $lib,
            SM2,
            "matrix2",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(feature = "matrix2")]
        $callback!(
            $context,
            $lib,
            M2S,
            "matrix2",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(feature = "matrix2")]
        $callback!(
            $context,
            $lib,
            M2M2,
            "matrix2",
            $scalar,
            $scalar_name,
            $scalar_token
        );

        #[cfg(feature = "matrix3")]
        $callback!(
            $context,
            $lib,
            SM3,
            "matrix3",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(feature = "matrix3")]
        $callback!(
            $context,
            $lib,
            M3S,
            "matrix3",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(feature = "matrix3")]
        $callback!(
            $context,
            $lib,
            M3M3,
            "matrix3",
            $scalar,
            $scalar_name,
            $scalar_token
        );

        #[cfg(feature = "matrix4")]
        $callback!(
            $context,
            $lib,
            SM4,
            "matrix4",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(feature = "matrix4")]
        $callback!(
            $context,
            $lib,
            M4S,
            "matrix4",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(feature = "matrix4")]
        $callback!(
            $context,
            $lib,
            M4M4,
            "matrix4",
            $scalar,
            $scalar_name,
            $scalar_token
        );

        #[cfg(feature = "matrix2x3")]
        $callback!(
            $context,
            $lib,
            SM2x3,
            "matrix2x3",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(feature = "matrix2x3")]
        $callback!(
            $context,
            $lib,
            M2x3S,
            "matrix2x3",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(feature = "matrix2x3")]
        $callback!(
            $context,
            $lib,
            M2x3M2x3,
            "matrix2x3",
            $scalar,
            $scalar_name,
            $scalar_token
        );

        #[cfg(feature = "matrix3x2")]
        $callback!(
            $context,
            $lib,
            SM3x2,
            "matrix3x2",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(feature = "matrix3x2")]
        $callback!(
            $context,
            $lib,
            M3x2S,
            "matrix3x2",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(feature = "matrix3x2")]
        $callback!(
            $context,
            $lib,
            M3x2M3x2,
            "matrix3x2",
            $scalar,
            $scalar_name,
            $scalar_token
        );

        #[cfg(feature = "matrixd")]
        $callback!(
            $context,
            $lib,
            SMD,
            "matrixd",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(feature = "matrixd")]
        $callback!(
            $context,
            $lib,
            MDS,
            "matrixd",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(feature = "matrixd")]
        $callback!(
            $context,
            $lib,
            MDMD,
            "matrixd",
            $scalar,
            $scalar_name,
            $scalar_token
        );

        #[cfg(feature = "row_vector2")]
        $callback!(
            $context,
            $lib,
            SR2,
            "row_vector2",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(feature = "row_vector2")]
        $callback!(
            $context,
            $lib,
            R2S,
            "row_vector2",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(feature = "row_vector2")]
        $callback!(
            $context,
            $lib,
            R2R2,
            "row_vector2",
            $scalar,
            $scalar_name,
            $scalar_token
        );

        #[cfg(feature = "row_vector3")]
        $callback!(
            $context,
            $lib,
            SR3,
            "row_vector3",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(feature = "row_vector3")]
        $callback!(
            $context,
            $lib,
            R3S,
            "row_vector3",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(feature = "row_vector3")]
        $callback!(
            $context,
            $lib,
            R3R3,
            "row_vector3",
            $scalar,
            $scalar_name,
            $scalar_token
        );

        #[cfg(feature = "row_vector4")]
        $callback!(
            $context,
            $lib,
            SR4,
            "row_vector4",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(feature = "row_vector4")]
        $callback!(
            $context,
            $lib,
            R4S,
            "row_vector4",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(feature = "row_vector4")]
        $callback!(
            $context,
            $lib,
            R4R4,
            "row_vector4",
            $scalar,
            $scalar_name,
            $scalar_token
        );

        #[cfg(feature = "row_vectord")]
        $callback!(
            $context,
            $lib,
            SRD,
            "row_vectord",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(feature = "row_vectord")]
        $callback!(
            $context,
            $lib,
            RDS,
            "row_vectord",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(feature = "row_vectord")]
        $callback!(
            $context,
            $lib,
            RDRD,
            "row_vectord",
            $scalar,
            $scalar_name,
            $scalar_token
        );

        #[cfg(feature = "vector2")]
        $callback!(
            $context,
            $lib,
            SV2,
            "vector2",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(feature = "vector2")]
        $callback!(
            $context,
            $lib,
            V2S,
            "vector2",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(feature = "vector2")]
        $callback!(
            $context,
            $lib,
            V2V2,
            "vector2",
            $scalar,
            $scalar_name,
            $scalar_token
        );

        #[cfg(feature = "vector3")]
        $callback!(
            $context,
            $lib,
            SV3,
            "vector3",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(feature = "vector3")]
        $callback!(
            $context,
            $lib,
            V3S,
            "vector3",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(feature = "vector3")]
        $callback!(
            $context,
            $lib,
            V3V3,
            "vector3",
            $scalar,
            $scalar_name,
            $scalar_token
        );

        #[cfg(feature = "vector4")]
        $callback!(
            $context,
            $lib,
            SV4,
            "vector4",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(feature = "vector4")]
        $callback!(
            $context,
            $lib,
            V4S,
            "vector4",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(feature = "vector4")]
        $callback!(
            $context,
            $lib,
            V4V4,
            "vector4",
            $scalar,
            $scalar_name,
            $scalar_token
        );

        #[cfg(feature = "vectord")]
        $callback!(
            $context,
            $lib,
            SVD,
            "vectord",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(feature = "vectord")]
        $callback!(
            $context,
            $lib,
            VDS,
            "vectord",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(feature = "vectord")]
        $callback!(
            $context,
            $lib,
            VDVD,
            "vectord",
            $scalar,
            $scalar_name,
            $scalar_token
        );

        #[cfg(all(feature = "matrix2", feature = "vector2"))]
        $callback!(
            $context,
            $lib,
            M2V2,
            "matrix2",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(all(feature = "matrix2", feature = "vector2"))]
        $callback!(
            $context,
            $lib,
            V2M2,
            "matrix2",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(all(feature = "matrix3", feature = "vector3"))]
        $callback!(
            $context,
            $lib,
            M3V3,
            "matrix3",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(all(feature = "matrix3", feature = "vector3"))]
        $callback!(
            $context,
            $lib,
            V3M3,
            "matrix3",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(all(feature = "matrix4", feature = "vector4"))]
        $callback!(
            $context,
            $lib,
            M4V4,
            "matrix4",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(all(feature = "matrix4", feature = "vector4"))]
        $callback!(
            $context,
            $lib,
            V4M4,
            "matrix4",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(all(feature = "matrix2x3", feature = "vector2"))]
        $callback!(
            $context,
            $lib,
            M2x3V2,
            "matrix2x3",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(all(feature = "matrix2x3", feature = "vector2"))]
        $callback!(
            $context,
            $lib,
            V2M2x3,
            "matrix2x3",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(all(feature = "matrix3x2", feature = "vector3"))]
        $callback!(
            $context,
            $lib,
            M3x2V3,
            "matrix3x2",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(all(feature = "matrix3x2", feature = "vector3"))]
        $callback!(
            $context,
            $lib,
            V3M3x2,
            "matrix3x2",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(all(feature = "matrixd", feature = "vectord"))]
        $callback!(
            $context,
            $lib,
            MDVD,
            "matrixd",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(all(feature = "matrixd", feature = "vectord"))]
        $callback!(
            $context,
            $lib,
            VDMD,
            "matrixd",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(all(feature = "matrixd", feature = "vector2"))]
        $callback!(
            $context,
            $lib,
            MDV2,
            "matrixd",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(all(feature = "matrixd", feature = "vector2"))]
        $callback!(
            $context,
            $lib,
            V2MD,
            "matrixd",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(all(feature = "matrixd", feature = "vector3"))]
        $callback!(
            $context,
            $lib,
            MDV3,
            "matrixd",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(all(feature = "matrixd", feature = "vector3"))]
        $callback!(
            $context,
            $lib,
            V3MD,
            "matrixd",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(all(feature = "matrixd", feature = "vector4"))]
        $callback!(
            $context,
            $lib,
            MDV4,
            "matrixd",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(all(feature = "matrixd", feature = "vector4"))]
        $callback!(
            $context,
            $lib,
            V4MD,
            "matrixd",
            $scalar,
            $scalar_name,
            $scalar_token
        );

        #[cfg(all(feature = "matrix2", feature = "row_vector2"))]
        $callback!(
            $context,
            $lib,
            M2R2,
            "matrix2",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(all(feature = "matrix2", feature = "row_vector2"))]
        $callback!(
            $context,
            $lib,
            R2M2,
            "matrix2",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(all(feature = "matrix3", feature = "row_vector3"))]
        $callback!(
            $context,
            $lib,
            M3R3,
            "matrix3",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(all(feature = "matrix3", feature = "row_vector3"))]
        $callback!(
            $context,
            $lib,
            R3M3,
            "matrix3",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(all(feature = "matrix4", feature = "row_vector4"))]
        $callback!(
            $context,
            $lib,
            M4R4,
            "matrix4",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(all(feature = "matrix4", feature = "row_vector4"))]
        $callback!(
            $context,
            $lib,
            R4M4,
            "matrix4",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(all(feature = "matrix2x3", feature = "row_vector3"))]
        $callback!(
            $context,
            $lib,
            M2x3R3,
            "matrix2x3",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(all(feature = "matrix2x3", feature = "row_vector3"))]
        $callback!(
            $context,
            $lib,
            R3M2x3,
            "matrix2x3",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(all(feature = "matrix3x2", feature = "row_vector2"))]
        $callback!(
            $context,
            $lib,
            M3x2R2,
            "matrix3x2",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(all(feature = "matrix3x2", feature = "row_vector2"))]
        $callback!(
            $context,
            $lib,
            R2M3x2,
            "matrix3x2",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(all(feature = "matrixd", feature = "row_vectord"))]
        $callback!(
            $context,
            $lib,
            MDRD,
            "matrixd",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(all(feature = "matrixd", feature = "row_vectord"))]
        $callback!(
            $context,
            $lib,
            RDMD,
            "matrixd",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(all(feature = "matrixd", feature = "row_vector2"))]
        $callback!(
            $context,
            $lib,
            MDR2,
            "matrixd",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(all(feature = "matrixd", feature = "row_vector2"))]
        $callback!(
            $context,
            $lib,
            R2MD,
            "matrixd",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(all(feature = "matrixd", feature = "row_vector3"))]
        $callback!(
            $context,
            $lib,
            MDR3,
            "matrixd",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(all(feature = "matrixd", feature = "row_vector3"))]
        $callback!(
            $context,
            $lib,
            R3MD,
            "matrixd",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(all(feature = "matrixd", feature = "row_vector4"))]
        $callback!(
            $context,
            $lib,
            MDR4,
            "matrixd",
            $scalar,
            $scalar_name,
            $scalar_token
        );
        #[cfg(all(feature = "matrixd", feature = "row_vector4"))]
        $callback!(
            $context,
            $lib,
            R4MD,
            "matrixd",
            $scalar,
            $scalar_name,
            $scalar_token
        );
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __mech_elementwise_binop_contract {
    (SS) => {
        $crate::RuntimeFunctionContract::no_matrix(
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };

    (SM1) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            1,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (SM2) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            1,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (SM3) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            1,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (SM4) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            1,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (SM2x3) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            1,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (SM3x2) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            1,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (SMD) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            1,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (SR2) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            1,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (SR3) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            1,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (SR4) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            1,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (SRD) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            1,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (SV2) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            1,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (SV3) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            1,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (SV4) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            1,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (SVD) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            1,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };

    (M1S) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            0,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (M2S) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            0,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (M3S) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            0,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (M4S) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            0,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (M2x3S) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            0,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (M3x2S) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            0,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (MDS) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            0,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (R2S) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            0,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (R3S) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            0,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (R4S) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            0,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (RDS) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            0,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (V2S) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            0,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (V3S) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            0,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (V4S) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            0,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (VDS) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            0,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };

    (M1M1) => {
        $crate::RuntimeFunctionContract::same_shape(
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (M2M2) => {
        $crate::RuntimeFunctionContract::same_shape(
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (M3M3) => {
        $crate::RuntimeFunctionContract::same_shape(
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (M4M4) => {
        $crate::RuntimeFunctionContract::same_shape(
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (M2x3M2x3) => {
        $crate::RuntimeFunctionContract::same_shape(
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (M3x2M3x2) => {
        $crate::RuntimeFunctionContract::same_shape(
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (MDMD) => {
        $crate::RuntimeFunctionContract::same_shape(
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (R2R2) => {
        $crate::RuntimeFunctionContract::same_shape(
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (R3R3) => {
        $crate::RuntimeFunctionContract::same_shape(
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (R4R4) => {
        $crate::RuntimeFunctionContract::same_shape(
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (RDRD) => {
        $crate::RuntimeFunctionContract::same_shape(
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (V2V2) => {
        $crate::RuntimeFunctionContract::same_shape(
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (V3V3) => {
        $crate::RuntimeFunctionContract::same_shape(
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (V4V4) => {
        $crate::RuntimeFunctionContract::same_shape(
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (VDVD) => {
        $crate::RuntimeFunctionContract::same_shape(
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };

    (M2V2) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            0,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (M3V3) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            0,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (M4V4) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            0,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (M2x3V2) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            0,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (M3x2V3) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            0,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (MDVD) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            0,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (MDV2) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            0,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (MDV3) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            0,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (MDV4) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            0,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (M2R2) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            0,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (M3R3) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            0,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (M4R4) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            0,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (M2x3R3) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            0,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (M3x2R2) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            0,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (MDRD) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            0,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (MDR2) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            0,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (MDR3) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            0,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (MDR4) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            0,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };

    (V2M2) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            1,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (V3M3) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            1,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (V4M4) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            1,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (V2M2x3) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            1,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (V3M3x2) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            1,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (VDMD) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            1,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (V2MD) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            1,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (V3MD) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            1,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (V4MD) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            1,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (R2M2) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            1,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (R3M3) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            1,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (R4M4) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            1,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (R3M2x3) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            1,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (R2M3x2) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            1,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (RDMD) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            1,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (R2MD) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            1,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (R3MD) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            1,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    (R4MD) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            1,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __mech_declare_native_binop_runtime_factory {
    (
        ($cfg:meta; $package:literal; $crate_name:literal; [$($base_feature:literal),* $(,)?]),
        $lib:ident,
        $suffix:ident,
        [$($_shape_feature:literal),* $(,)?],
        $scalar:ty,
        $scalar_name:literal,
        $scalar_token:ident
    ) => {
        $crate::paste::paste! {
            $crate::declare_native_runtime_factory! {
                cfg: $cfg,
                registration: [<register_ $lib:snake _ $suffix:lower _ $scalar_token>],
                installer: [<install_ $lib:snake _ $suffix:lower _ $scalar_token>],
                name: concat!(stringify!($lib), stringify!($suffix), "<", $scalar_name, ">"),
                factory_type: [<$lib $suffix>]<$scalar>,
                contract: $crate::__mech_elementwise_binop_contract!($suffix),
                package: $package,
                crate_name: $crate_name,
                installer_path: concat!(
                    $crate_name,
                    "::__mech_native::",
                    stringify!([<install_ $lib:snake _ $suffix:lower _ $scalar_token>])
                ),
                extra_cargo_features: [$($base_feature),*],
            }
        }
    };
}

/// Declares every concrete binary runtime factory together with its native
/// linkage. The shared traversal owns the shape list, so the declaration,
/// aggregate registration, and native installer export cannot drift.
#[doc(hidden)]
#[macro_export]
macro_rules! __mech_declare_native_binop_runtime_factories_for_scalar {
    (
        package: $package:literal,
        crate_name: $crate_name:literal,
        operation: $operation:ident,
        operation_feature: $operation_feature:literal,
        additional_features: [$($additional_feature:literal),* $(,)?],
        scalar: ($scalar_feature:literal, $scalar:ty, $scalar_name:literal, $scalar_token:ident),
    ) => {
        $crate::__mech_for_each_exact_binop_runtime_factory_for_type!(
            $crate::__mech_declare_native_binop_runtime_factory,
            (
                all(feature = $operation_feature, feature = $scalar_feature);
                $package;
                $crate_name;
                [
                    $operation_feature,
                ]
            ),
            $operation,
            $scalar,
            $scalar_name,
            $scalar_token
        );
    };
}

/// Declares every concrete binary runtime factory together with its native
/// linkage. The shared traversal owns the shape list, so the declaration,
/// aggregate registration, and native installer export cannot drift.
#[doc(hidden)]
#[macro_export]
macro_rules! __mech_declare_native_binop_runtime_factories_each {
    (
        package: $package:literal,
        crate_name: $crate_name:literal,
        operation: $operation:ident,
        operation_feature: $operation_feature:literal,
        additional_features: [$($additional_feature:literal),* $(,)?],
        scalars:
            ($scalar_feature:literal, $scalar:ty, $scalar_name:literal, $scalar_token:ident)
            $(, ($remaining_feature:literal, $remaining_scalar:ty, $remaining_name:literal, $remaining_token:ident))*
            $(,)?,
    ) => {
        $crate::__mech_declare_native_binop_runtime_factories_for_scalar! {
            package: $package,
            crate_name: $crate_name,
            operation: $operation,
            operation_feature: $operation_feature,
            additional_features: [$($additional_feature),*],
            scalar: ($scalar_feature, $scalar, $scalar_name, $scalar_token),
        }
        $crate::__mech_declare_native_binop_runtime_factories_each! {
            package: $package,
            crate_name: $crate_name,
            operation: $operation,
            operation_feature: $operation_feature,
            additional_features: [$($additional_feature),*],
            scalars: $(( $remaining_feature, $remaining_scalar, $remaining_name, $remaining_token )),*,
        }
    };
    (
        package: $package:literal,
        crate_name: $crate_name:literal,
        operation: $operation:ident,
        operation_feature: $operation_feature:literal,
        additional_features: [$($additional_feature:literal),* $(,)?],
        scalars:,
    ) => {};
}

#[doc(hidden)]
#[macro_export]
macro_rules! declare_native_binop_runtime_factories {
    (
        package: $package:literal,
        crate_name: $crate_name:literal,
        operation: $operation:ident,
        operation_feature: $operation_feature:literal,
        additional_features: [$($additional_feature:literal),* $(,)?],
        scalars: $(
            ($scalar_feature:literal, $scalar:ty, $scalar_name:literal, $scalar_token:ident)
        ),+ $(,)?
    ) => {
        $crate::__mech_declare_native_binop_runtime_factories_each! {
            package: $package,
            crate_name: $crate_name,
            operation: $operation,
            operation_feature: $operation_feature,
            additional_features: [$($additional_feature),*],
            scalars: $(($scalar_feature, $scalar, $scalar_name, $scalar_token)),+,
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __mech_register_native_binop_runtime_factory {
    (
        $builder:expr,
        $lib:ident,
        $suffix:ident,
        [$($_shape_feature:literal),* $(,)?],
        $scalar:ty,
        $scalar_name:literal,
        $scalar_token:ident
    ) => {
        $crate::paste::paste! {
            [<register_ $lib:snake _ $suffix:lower _ $scalar_token>]($builder)?;
        }
    };
}

/// Registers a binary factory family previously declared with
/// [`declare_native_binop_runtime_factories!`].
#[doc(hidden)]
#[macro_export]
macro_rules! install_native_binop_runtime_factories {
    (
        $builder:expr,
        $operation:ident;
        $(($scalar_feature:literal, $scalar:ty, $scalar_name:literal, $scalar_token:ident)),+
        $(,)?
    ) => {{
        $(
            #[cfg(feature = $scalar_feature)]
        $crate::__mech_for_each_exact_binop_runtime_factory_for_type!(
                $crate::__mech_register_native_binop_runtime_factory,
                $builder,
                $operation,
                $scalar,
                $scalar_name,
                $scalar_token
            );
        )+
        Ok::<(), $crate::MechError>(())
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __mech_export_native_binop_runtime_factory {
    (
        $_context:tt,
        $lib:ident,
        $suffix:ident,
        [$($_shape_feature:literal),* $(,)?],
        $scalar:ty,
        $scalar_name:literal,
        $scalar_token:ident
    ) => {
        $crate::paste::paste! {
            pub use super::[<install_ $lib:snake _ $suffix:lower _ $scalar_token>];
        }
    };
}

/// Reexports exactly the per-factory native installers declared by a binary
/// family. Invoke this inside an owner's hidden `__mech_native` module.
#[doc(hidden)]
#[macro_export]
macro_rules! export_native_binop_runtime_factories {
    (
        operation_feature: $operation_feature:literal,
        operation: $operation:ident;
        $(($scalar_feature:literal, $scalar:ty, $scalar_name:literal, $scalar_token:ident)),+
        $(,)?
    ) => {
        $(
            #[cfg(all(feature = $operation_feature, feature = $scalar_feature))]
            $crate::__mech_for_each_exact_binop_runtime_factory_for_type!(
                $crate::__mech_export_native_binop_runtime_factory,
                (),
                $operation,
                $scalar,
                $scalar_name,
                $scalar_token
            );
        )+
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __mech_install_binop_runtime_factory {
    ($builder:expr, $lib:ident, $suffix:ident, $scalar:ty, $scalar_name:literal) => {
        paste! {
            $builder.insert_runtime_factory::<[<$lib $suffix>]<$scalar>>(
                concat!(
                    stringify!($lib),
                    stringify!($suffix),
                    "<",
                    $scalar_name,
                    ">"
                ),
                $crate::__mech_elementwise_binop_contract!($suffix),
            )?;
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __mech_install_binop_runtime_factory_group {
    ($builder:expr, $lib:ident, $scalar:ty, $scalar_name:literal; $($suffix:ident),+ $(,)?) => {
        $(
            $crate::__mech_install_binop_runtime_factory!(
                $builder,
                $lib,
                $suffix,
                $scalar,
                $scalar_name
            );
        )+
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __mech_install_binop_runtime_factories_for_type {
    ($builder:expr, $lib:ident, $scalar:ty, $scalar_name:literal) => {{
        $crate::__mech_install_binop_runtime_factory_group!(
            $builder, $lib, $scalar, $scalar_name; SS
        );

        #[cfg(feature = "matrix1")]
        $crate::__mech_install_binop_runtime_factory_group!(
            $builder, $lib, $scalar, $scalar_name; SM1, M1S, M1M1
        );
        #[cfg(feature = "matrix2")]
        $crate::__mech_install_binop_runtime_factory_group!(
            $builder, $lib, $scalar, $scalar_name; SM2, M2S, M2M2
        );
        #[cfg(feature = "matrix3")]
        $crate::__mech_install_binop_runtime_factory_group!(
            $builder, $lib, $scalar, $scalar_name; SM3, M3S, M3M3
        );
        #[cfg(feature = "matrix4")]
        $crate::__mech_install_binop_runtime_factory_group!(
            $builder, $lib, $scalar, $scalar_name; SM4, M4S, M4M4
        );
        #[cfg(feature = "matrix2x3")]
        $crate::__mech_install_binop_runtime_factory_group!(
            $builder, $lib, $scalar, $scalar_name; SM2x3, M2x3S, M2x3M2x3
        );
        #[cfg(feature = "matrix3x2")]
        $crate::__mech_install_binop_runtime_factory_group!(
            $builder, $lib, $scalar, $scalar_name; SM3x2, M3x2S, M3x2M3x2
        );
        #[cfg(feature = "matrixd")]
        $crate::__mech_install_binop_runtime_factory_group!(
            $builder, $lib, $scalar, $scalar_name; SMD, MDS, MDMD
        );

        #[cfg(feature = "row_vector2")]
        $crate::__mech_install_binop_runtime_factory_group!(
            $builder, $lib, $scalar, $scalar_name; SR2, R2S, R2R2
        );
        #[cfg(feature = "row_vector3")]
        $crate::__mech_install_binop_runtime_factory_group!(
            $builder, $lib, $scalar, $scalar_name; SR3, R3S, R3R3
        );
        #[cfg(feature = "row_vector4")]
        $crate::__mech_install_binop_runtime_factory_group!(
            $builder, $lib, $scalar, $scalar_name; SR4, R4S, R4R4
        );
        #[cfg(feature = "row_vectord")]
        $crate::__mech_install_binop_runtime_factory_group!(
            $builder, $lib, $scalar, $scalar_name; SRD, RDS, RDRD
        );

        #[cfg(feature = "vector2")]
        $crate::__mech_install_binop_runtime_factory_group!(
            $builder, $lib, $scalar, $scalar_name; SV2, V2S, V2V2
        );
        #[cfg(feature = "vector3")]
        $crate::__mech_install_binop_runtime_factory_group!(
            $builder, $lib, $scalar, $scalar_name; SV3, V3S, V3V3
        );
        #[cfg(feature = "vector4")]
        $crate::__mech_install_binop_runtime_factory_group!(
            $builder, $lib, $scalar, $scalar_name; SV4, V4S, V4V4
        );
        #[cfg(feature = "vectord")]
        $crate::__mech_install_binop_runtime_factory_group!(
            $builder, $lib, $scalar, $scalar_name; SVD, VDS, VDVD
        );

        #[cfg(all(feature = "matrix2", feature = "vector2"))]
        $crate::__mech_install_binop_runtime_factory_group!(
            $builder, $lib, $scalar, $scalar_name; M2V2, V2M2
        );
        #[cfg(all(feature = "matrix3", feature = "vector3"))]
        $crate::__mech_install_binop_runtime_factory_group!(
            $builder, $lib, $scalar, $scalar_name; M3V3, V3M3
        );
        #[cfg(all(feature = "matrix4", feature = "vector4"))]
        $crate::__mech_install_binop_runtime_factory_group!(
            $builder, $lib, $scalar, $scalar_name; M4V4, V4M4
        );
        #[cfg(all(feature = "matrix2x3", feature = "vector2"))]
        $crate::__mech_install_binop_runtime_factory_group!(
            $builder, $lib, $scalar, $scalar_name; M2x3V2, V2M2x3
        );
        #[cfg(all(feature = "matrix3x2", feature = "vector3"))]
        $crate::__mech_install_binop_runtime_factory_group!(
            $builder, $lib, $scalar, $scalar_name; M3x2V3, V3M3x2
        );
        #[cfg(all(feature = "matrixd", feature = "vectord"))]
        $crate::__mech_install_binop_runtime_factory_group!(
            $builder, $lib, $scalar, $scalar_name; MDVD, VDMD
        );
        #[cfg(all(feature = "matrixd", feature = "vector2"))]
        $crate::__mech_install_binop_runtime_factory_group!(
            $builder, $lib, $scalar, $scalar_name; MDV2, V2MD
        );
        #[cfg(all(feature = "matrixd", feature = "vector3"))]
        $crate::__mech_install_binop_runtime_factory_group!(
            $builder, $lib, $scalar, $scalar_name; MDV3, V3MD
        );
        #[cfg(all(feature = "matrixd", feature = "vector4"))]
        $crate::__mech_install_binop_runtime_factory_group!(
            $builder, $lib, $scalar, $scalar_name; MDV4, V4MD
        );

        #[cfg(all(feature = "matrix2", feature = "row_vector2"))]
        $crate::__mech_install_binop_runtime_factory_group!(
            $builder, $lib, $scalar, $scalar_name; M2R2, R2M2
        );
        #[cfg(all(feature = "matrix3", feature = "row_vector3"))]
        $crate::__mech_install_binop_runtime_factory_group!(
            $builder, $lib, $scalar, $scalar_name; M3R3, R3M3
        );
        #[cfg(all(feature = "matrix4", feature = "row_vector4"))]
        $crate::__mech_install_binop_runtime_factory_group!(
            $builder, $lib, $scalar, $scalar_name; M4R4, R4M4
        );
        #[cfg(all(feature = "matrix2x3", feature = "row_vector3"))]
        $crate::__mech_install_binop_runtime_factory_group!(
            $builder, $lib, $scalar, $scalar_name; M2x3R3, R3M2x3
        );
        #[cfg(all(feature = "matrix3x2", feature = "row_vector2"))]
        $crate::__mech_install_binop_runtime_factory_group!(
            $builder, $lib, $scalar, $scalar_name; M3x2R2, R2M3x2
        );
        #[cfg(all(feature = "matrixd", feature = "row_vectord"))]
        $crate::__mech_install_binop_runtime_factory_group!(
            $builder, $lib, $scalar, $scalar_name; MDRD, RDMD
        );
        #[cfg(all(feature = "matrixd", feature = "row_vector2"))]
        $crate::__mech_install_binop_runtime_factory_group!(
            $builder, $lib, $scalar, $scalar_name; MDR2, R2MD
        );
        #[cfg(all(feature = "matrixd", feature = "row_vector3"))]
        $crate::__mech_install_binop_runtime_factory_group!(
            $builder, $lib, $scalar, $scalar_name; MDR3, R3MD
        );
        #[cfg(all(feature = "matrixd", feature = "row_vector4"))]
        $crate::__mech_install_binop_runtime_factory_group!(
            $builder, $lib, $scalar, $scalar_name; MDR4, R4MD
        );

        Ok::<(), $crate::MechError>(())
    }};
}

/// Installs every enabled concrete runtime factory generated for a binary
/// operation across the supplied scalar kinds.
#[macro_export]
macro_rules! install_binop_runtime_factories {
    ($builder:expr, $lib:ident; $(($feature:literal, $scalar:ty, $scalar_name:literal)),+ $(,)?) => {{
        $(
            #[cfg(feature = $feature)]
            {
                #[inline(never)]
                fn install(
                    builder: &mut $crate::FunctionCatalogBuilder,
                ) -> $crate::MResult<()> {
                    $crate::__mech_install_binop_runtime_factories_for_type!(
                        builder,
                        $lib,
                        $scalar,
                        $scalar_name
                    )
                }

                install($builder)?;
            }
        )+

        Ok::<(), $crate::MechError>(())
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __mech_elementwise_unop_contract {
    (S) => {
        $crate::RuntimeFunctionContract::no_matrix(
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
    ($suffix:ident) => {
        $crate::RuntimeFunctionContract::output_matches_input(
            0,
            $crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
        )
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __mech_install_unop_runtime_factory {
    ($builder:expr, $lib:ident, $scalar:ident, $suffix:ident) => {
        paste! {
            $builder.insert_runtime_factory::<[<$lib $scalar:camel $suffix>]>(
                stringify!([<$lib $scalar:camel $suffix>]),
                $crate::__mech_elementwise_unop_contract!($suffix),
            )?;
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __mech_install_unop_runtime_factories_for_type {
    ($builder:expr, $lib:ident, $scalar:ident) => {{
        $crate::__mech_install_unop_runtime_factory!($builder, $lib, $scalar, S);

        #[cfg(feature = "matrix1")]
        $crate::__mech_install_unop_runtime_factory!($builder, $lib, $scalar, M1);
        #[cfg(feature = "matrix2")]
        $crate::__mech_install_unop_runtime_factory!($builder, $lib, $scalar, M2);
        #[cfg(feature = "matrix3")]
        $crate::__mech_install_unop_runtime_factory!($builder, $lib, $scalar, M3);
        #[cfg(feature = "matrix4")]
        $crate::__mech_install_unop_runtime_factory!($builder, $lib, $scalar, M4);
        #[cfg(feature = "matrix2x3")]
        $crate::__mech_install_unop_runtime_factory!($builder, $lib, $scalar, M2x3);
        #[cfg(feature = "matrix3x2")]
        $crate::__mech_install_unop_runtime_factory!($builder, $lib, $scalar, M3x2);
        #[cfg(feature = "matrixd")]
        $crate::__mech_install_unop_runtime_factory!($builder, $lib, $scalar, MD);

        #[cfg(feature = "row_vector2")]
        $crate::__mech_install_unop_runtime_factory!($builder, $lib, $scalar, R2);
        #[cfg(feature = "row_vector3")]
        $crate::__mech_install_unop_runtime_factory!($builder, $lib, $scalar, R3);
        #[cfg(feature = "row_vector4")]
        $crate::__mech_install_unop_runtime_factory!($builder, $lib, $scalar, R4);
        #[cfg(feature = "row_vectord")]
        $crate::__mech_install_unop_runtime_factory!($builder, $lib, $scalar, RD);

        #[cfg(feature = "vector2")]
        $crate::__mech_install_unop_runtime_factory!($builder, $lib, $scalar, V2);
        #[cfg(feature = "vector3")]
        $crate::__mech_install_unop_runtime_factory!($builder, $lib, $scalar, V3);
        #[cfg(feature = "vector4")]
        $crate::__mech_install_unop_runtime_factory!($builder, $lib, $scalar, V4);
        #[cfg(feature = "vectord")]
        $crate::__mech_install_unop_runtime_factory!($builder, $lib, $scalar, VD);

        Ok::<(), $crate::MechError>(())
    }};
}

/// Installs every enabled concrete runtime factory generated for a unary
/// operation across the supplied scalar kinds.
#[macro_export]
macro_rules! install_unop_runtime_factories {
    ($builder:expr, $lib:ident; $(($feature:literal, $scalar:ident)),+ $(,)?) => {{
        $(
            #[cfg(feature = $feature)]
            {
                #[inline(never)]
                fn install(
                    builder: &mut $crate::FunctionCatalogBuilder,
                ) -> $crate::MResult<()> {
                    $crate::__mech_install_unop_runtime_factories_for_type!(
                        builder,
                        $lib,
                        $scalar
                    )
                }

                install($builder)?;
            }
        )+

        Ok::<(), $crate::MechError>(())
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __mech_install_typed_runtime_factory {
    ($builder:expr, $factory:ident, $scalar:ty, $scalar_name:literal, $contract:expr) => {
        $builder.insert_runtime_factory::<$factory<$scalar>>(
            concat!(stringify!($factory), "<", $scalar_name, ">"),
            $contract,
        )?;
    };
}

/// Installs enabled one-type generic factories whose legacy names have the
/// form `Factory<scalar>`.
#[macro_export]
macro_rules! install_typed_runtime_factories {
    ($builder:expr, $factory:ident, contract: $contract:expr; $(($feature:literal, $scalar:ty, $scalar_name:literal)),+ $(,)?) => {{
        $(
            #[cfg(feature = $feature)]
            $crate::__mech_install_typed_runtime_factory!(
                $builder,
                $factory,
                $scalar,
                $scalar_name,
                $contract
            );
        )+

        Ok::<(), $crate::MechError>(())
    }};
}

#[macro_export]
macro_rules! impl_fxns {
  ($lib:ident, $in:ident, $out:ident, $op:ident) => {
    paste!{
      // Scalar
      $op!([<$lib SS>], $in, $in, $out, [<$lib:lower _op>]);
      // Scalar Matrix
      #[cfg(feature = "matrix1")]
      $op!([<$lib SM1>], $in, Matrix1<$in>, Matrix1<$out>,[<$lib:lower _scalar_rhs_op>]);
      #[cfg(feature = "matrix2")]
      $op!([<$lib SM2>], $in, Matrix2<$in>, Matrix2<$out>,[<$lib:lower _scalar_rhs_op>]);
      #[cfg(feature = "matrix3")]
      $op!([<$lib SM3>], $in, Matrix3<$in>, Matrix3<$out>,[<$lib:lower _scalar_rhs_op>]);

      #[cfg(feature = "matrix4")]
      $op!([<$lib SM4>], $in, Matrix4<$in>, Matrix4<$out>,[<$lib:lower _scalar_rhs_op>]);
      #[cfg(feature = "matrix2x3")]
      $op!([<$lib SM2x3>], $in, Matrix2x3<$in>, Matrix2x3<$out>,[<$lib:lower _scalar_rhs_op>]);
      #[cfg(feature = "matrix3x2")]
      $op!([<$lib SM3x2>], $in, Matrix3x2<$in>, Matrix3x2<$out>,[<$lib:lower _scalar_rhs_op>]);
      #[cfg(feature = "matrixd")]
      $op!([<$lib SMD>], $in, DMatrix<$in>, DMatrix<$out>,[<$lib:lower _scalar_rhs_op>]);
      // Scalar Row
      #[cfg(feature = "row_vector2")]
      $op!([<$lib SR2>], $in, RowVector2<$in>, RowVector2<$out>,[<$lib:lower _scalar_rhs_op>]);
      #[cfg(feature = "row_vector3")]
      $op!([<$lib SR3>], $in, RowVector3<$in>, RowVector3<$out>,[<$lib:lower _scalar_rhs_op>]);
      #[cfg(feature = "row_vector4")]
      $op!([<$lib SR4>], $in, RowVector4<$in>, RowVector4<$out>,[<$lib:lower _scalar_rhs_op>]);
      #[cfg(feature = "row_vectord")]
      $op!([<$lib SRD>], $in, RowDVector<$in>, RowDVector<$out>,[<$lib:lower _scalar_rhs_op>]);
      // Scalar Vector
      #[cfg(feature = "vector2")]
      $op!([<$lib SV2>], $in, Vector2<$in>, Vector2<$out>,[<$lib:lower _scalar_rhs_op>]);
      #[cfg(feature = "vector3")]
      $op!([<$lib SV3>], $in, Vector3<$in>, Vector3<$out>,[<$lib:lower _scalar_rhs_op>]);
      #[cfg(feature = "vector4")]
      $op!([<$lib SV4>], $in, Vector4<$in>, Vector4<$out>,[<$lib:lower _scalar_rhs_op>]);
      #[cfg(feature = "vectord")]
      $op!([<$lib SVD>], $in, DVector<$in>, DVector<$out>,[<$lib:lower _scalar_rhs_op>]);
      // Matrix Scalar
      #[cfg(feature = "matrix1")]
      $op!([<$lib M1S>], Matrix1<$in>, $in, Matrix1<$out>,[<$lib:lower _scalar_lhs_op>]);
      #[cfg(feature = "matrix2")]
      $op!([<$lib M2S>], Matrix2<$in>, $in, Matrix2<$out>,[<$lib:lower _scalar_lhs_op>]);
      #[cfg(feature = "matrix3")]
      $op!([<$lib M3S>], Matrix3<$in>, $in, Matrix3<$out>,[<$lib:lower _scalar_lhs_op>]);
      #[cfg(feature = "matrix4")]
      $op!([<$lib M4S>], Matrix4<$in>, $in, Matrix4<$out>,[<$lib:lower _scalar_lhs_op>]);
      #[cfg(feature = "matrix2x3")]
      $op!([<$lib M2x3S>], Matrix2x3<$in>, $in, Matrix2x3<$out>,[<$lib:lower _scalar_lhs_op>]);
      #[cfg(feature = "matrix3x2")]
      $op!([<$lib M3x2S>], Matrix3x2<$in>, $in, Matrix3x2<$out>,[<$lib:lower _scalar_lhs_op>]);
      #[cfg(feature = "matrixd")]
      $op!([<$lib MDS>], DMatrix<$in>, $in, DMatrix<$out>,[<$lib:lower _scalar_lhs_op>]);
      // Row Scalar
      #[cfg(feature = "row_vector2")]
      $op!([<$lib R2S>], RowVector2<$in>, $in, RowVector2<$out>,[<$lib:lower _scalar_lhs_op>]);
      #[cfg(feature = "row_vector3")]
      $op!([<$lib R3S>], RowVector3<$in>, $in, RowVector3<$out>,[<$lib:lower _scalar_lhs_op>]);
      #[cfg(feature = "row_vector4")]
      $op!([<$lib R4S>], RowVector4<$in>, $in, RowVector4<$out>,[<$lib:lower _scalar_lhs_op>]);
      #[cfg(feature = "row_vectord")]
      $op!([<$lib RDS>], RowDVector<$in>, $in, RowDVector<$out>,[<$lib:lower _scalar_lhs_op>]);
      // Vector Scalar
      #[cfg(feature = "vector2")]
      $op!([<$lib V2S>], Vector2<$in>, $in, Vector2<$out>,[<$lib:lower _scalar_lhs_op>]);
      #[cfg(feature = "vector3")]
      $op!([<$lib V3S>], Vector3<$in>, $in, Vector3<$out>,[<$lib:lower _scalar_lhs_op>]);
      #[cfg(feature = "vector4")]
      $op!([<$lib V4S>], Vector4<$in>, $in, Vector4<$out>,[<$lib:lower _scalar_lhs_op>]);
      #[cfg(feature = "vectord")]
      $op!([<$lib VDS>], DVector<$in>, $in, DVector<$out>,[<$lib:lower _scalar_lhs_op>]);
      // Matrix Matrix
      #[cfg(feature = "matrix1")]
      $op!([<$lib M1M1>], Matrix1<$in>, Matrix1<$in>, Matrix1<$out>, [<$lib:lower _vec_op>]);
      #[cfg(feature = "matrix2")]
      $op!([<$lib M2M2>], Matrix2<$in>, Matrix2<$in>, Matrix2<$out>, [<$lib:lower _vec_op>]);
      #[cfg(feature = "matrix3")]
      $op!([<$lib M3M3>], Matrix3<$in>, Matrix3<$in>, Matrix3<$out>, [<$lib:lower _vec_op>]);
      #[cfg(feature = "matrix4")]
      $op!([<$lib M4M4>], Matrix4<$in>, Matrix4<$in>, Matrix4<$out>, [<$lib:lower _vec_op>]);
      #[cfg(feature = "matrix2x3")]
      $op!([<$lib M2x3M2x3>], Matrix2x3<$in>, Matrix2x3<$in>, Matrix2x3<$out>, [<$lib:lower _vec_op>]);
      #[cfg(feature = "matrix3x2")]
      $op!([<$lib M3x2M3x2>], Matrix3x2<$in>, Matrix3x2<$in>, Matrix3x2<$out>, [<$lib:lower _vec_op>]);
      #[cfg(feature = "matrixd")]
      $op!([<$lib MDMD>], DMatrix<$in>, DMatrix<$in>, DMatrix<$out>, [<$lib:lower _vec_op>]);
      // Matrix Vector
      #[cfg(all(feature = "matrix2", feature = "vector2"))]
      $op!([<$lib M2V2>], Matrix2<$in>, Vector2<$in>, Matrix2<$out>, [<$lib:lower _mat_vec_op>]);
      #[cfg(all(feature = "matrix3", feature = "vector3"))]
      $op!([<$lib M3V3>], Matrix3<$in>, Vector3<$in>, Matrix3<$out>, [<$lib:lower _mat_vec_op>]);
      #[cfg(all(feature = "matrix4", feature = "vector4"))]
      $op!([<$lib M4V4>], Matrix4<$in>, Vector4<$in>, Matrix4<$out>, [<$lib:lower _mat_vec_op>]);
      #[cfg(all(feature = "matrix2x3", feature = "vector2"))]
      $op!([<$lib M2x3V2>], Matrix2x3<$in>, Vector2<$in>, Matrix2x3<$out>, [<$lib:lower _mat_vec_op>]);
      #[cfg(all(feature = "matrix3x2", feature = "vector3"))]
      $op!([<$lib M3x2V3>], Matrix3x2<$in>, Vector3<$in>, Matrix3x2<$out>, [<$lib:lower _mat_vec_op>]);
      #[cfg(all(feature = "matrixd", feature = "vectord"))]
      $op!([<$lib MDVD>], DMatrix<$in>, DVector<$in>, DMatrix<$out>, [<$lib:lower _mat_vec_op>]);
      #[cfg(all(feature = "matrixd", feature = "vector2"))]
      $op!([<$lib MDV2>], DMatrix<$in>, Vector2<$in>, DMatrix<$out>, [<$lib:lower _mat_vec_op>]);
      #[cfg(all(feature = "matrixd", feature = "vector3"))]
      $op!([<$lib MDV3>], DMatrix<$in>, Vector3<$in>, DMatrix<$out>, [<$lib:lower _mat_vec_op>]);
      #[cfg(all(feature = "matrixd", feature = "vector4"))]
      $op!([<$lib MDV4>], DMatrix<$in>, Vector4<$in>, DMatrix<$out>, [<$lib:lower _mat_vec_op>]);
      // Vector Matrix
      #[cfg(all(feature = "vector2", feature = "matrix2"))]
      $op!([<$lib V2M2>], Vector2<$in>, Matrix2<$in>, Matrix2<$out>, [<$lib:lower _vec_mat_op>]);
      #[cfg(all(feature = "vector3", feature = "matrix3"))]
      $op!([<$lib V3M3>], Vector3<$in>, Matrix3<$in>, Matrix3<$out>, [<$lib:lower _vec_mat_op>]);
      #[cfg(all(feature = "vector4", feature = "matrix4"))]
      $op!([<$lib V4M4>], Vector4<$in>, Matrix4<$in>, Matrix4<$out>, [<$lib:lower _vec_mat_op>]);
      #[cfg(all(feature = "vector2", feature = "matrix2x3"))]
      $op!([<$lib V2M2x3>], Vector2<$in>, Matrix2x3<$in>, Matrix2x3<$out>, [<$lib:lower _vec_mat_op>]);
      #[cfg(all(feature = "vector3", feature = "matrix3x2"))]
      $op!([<$lib V3M3x2>], Vector3<$in>, Matrix3x2<$in>, Matrix3x2<$out>, [<$lib:lower _vec_mat_op>]);
      #[cfg(all(feature = "vectord", feature = "matrixd"))]
      $op!([<$lib VDMD>], DVector<$in>, DMatrix<$in>, DMatrix<$out>, [<$lib:lower _vec_mat_op>]);
      #[cfg(all(feature = "vector2", feature = "matrixd"))]
      $op!([<$lib V2MD>], Vector2<$in>, DMatrix<$in>, DMatrix<$out>, [<$lib:lower _vec_mat_op>]);
      #[cfg(all(feature = "vector3", feature = "matrixd"))]
      $op!([<$lib V3MD>], Vector3<$in>, DMatrix<$in>, DMatrix<$out>, [<$lib:lower _vec_mat_op>]);
      #[cfg(all(feature = "vector4", feature = "matrixd"))]
      $op!([<$lib V4MD>], Vector4<$in>, DMatrix<$in>, DMatrix<$out>, [<$lib:lower _vec_mat_op>]);
      // Matrix Row
      #[cfg(all(feature = "matrix2", feature = "row_vector2"))]
      $op!([<$lib M2R2>], Matrix2<$in>, RowVector2<$in>, Matrix2<$out>, [<$lib:lower _mat_row_op>]);
      #[cfg(all(feature = "matrix3", feature = "row_vector3"))]
      $op!([<$lib M3R3>], Matrix3<$in>, RowVector3<$in>, Matrix3<$out>, [<$lib:lower _mat_row_op>]);
      #[cfg(all(feature = "matrix4", feature = "row_vector4"))]
      $op!([<$lib M4R4>], Matrix4<$in>, RowVector4<$in>, Matrix4<$out>, [<$lib:lower _mat_row_op>]);
      #[cfg(all(feature = "matrix2x3", feature = "row_vector3"))]
      $op!([<$lib M2x3R3>], Matrix2x3<$in>, RowVector3<$in>, Matrix2x3<$out>, [<$lib:lower _mat_row_op>]);
      #[cfg(all(feature = "matrix3x2", feature = "row_vector2"))]
      $op!([<$lib M3x2R2>], Matrix3x2<$in>, RowVector2<$in>, Matrix3x2<$out>, [<$lib:lower _mat_row_op>]);
      #[cfg(all(feature = "matrixd", feature = "row_vectord"))]
      $op!([<$lib MDRD>], DMatrix<$in>, RowDVector<$in>, DMatrix<$out>, [<$lib:lower _mat_row_op>]);
      #[cfg(all(feature = "matrixd", feature = "row_vector2"))]
      $op!([<$lib MDR2>], DMatrix<$in>, RowVector2<$in>, DMatrix<$out>, [<$lib:lower _mat_row_op>]);
      #[cfg(all(feature = "matrixd", feature = "row_vector3"))]
      $op!([<$lib MDR3>], DMatrix<$in>, RowVector3<$in>, DMatrix<$out>, [<$lib:lower _mat_row_op>]);
      #[cfg(all(feature = "matrixd", feature = "row_vector4"))]
      $op!([<$lib MDR4>], DMatrix<$in>, RowVector4<$in>, DMatrix<$out>, [<$lib:lower _mat_row_op>]);
      // Row Matrix
      #[cfg(all(feature = "row_vector2", feature = "matrix2"))]
      $op!([<$lib R2M2>], RowVector2<$in>, Matrix2<$in>, Matrix2<$out>, [<$lib:lower _row_mat_op>]);
      #[cfg(all(feature = "row_vector3", feature = "matrix3"))]
      $op!([<$lib R3M3>], RowVector3<$in>, Matrix3<$in>, Matrix3<$out>, [<$lib:lower _row_mat_op>]);
      #[cfg(all(feature = "row_vector4", feature = "matrix4"))]
      $op!([<$lib R4M4>], RowVector4<$in>, Matrix4<$in>, Matrix4<$out>, [<$lib:lower _row_mat_op>]);
      #[cfg(all(feature = "row_vector3", feature = "matrix2x3"))]
      $op!([<$lib R3M2x3>], RowVector3<$in>, Matrix2x3<$in>, Matrix2x3<$out>, [<$lib:lower _row_mat_op>]);
      #[cfg(all(feature = "row_vector2", feature = "matrix3x2"))]
      $op!([<$lib R2M3x2>], RowVector2<$in>, Matrix3x2<$in>, Matrix3x2<$out>, [<$lib:lower _row_mat_op>]);
      #[cfg(all(feature = "row_vectord", feature = "matrixd"))]
      $op!([<$lib RDMD>], RowDVector<$in>, DMatrix<$in>, DMatrix<$out>, [<$lib:lower _row_mat_op>]);
      #[cfg(all(feature = "row_vector2", feature = "matrixd"))]
      $op!([<$lib R2MD>], RowVector2<$in>, DMatrix<$in>, DMatrix<$out>, [<$lib:lower _row_mat_op>]);
      #[cfg(all(feature = "row_vector3", feature = "matrixd"))]
      $op!([<$lib R3MD>], RowVector3<$in>, DMatrix<$in>, DMatrix<$out>, [<$lib:lower _row_mat_op>]);
      #[cfg(all(feature = "row_vector4", feature = "matrixd"))]
      $op!([<$lib R4MD>], RowVector4<$in>, DMatrix<$in>, DMatrix<$out>, [<$lib:lower _row_mat_op>]);
      // Row Row
      #[cfg(feature = "row_vector2")]
      $op!([<$lib R2R2>], RowVector2<$in>, RowVector2<$in>, RowVector2<$out>, [<$lib:lower _vec_op>]);
      #[cfg(feature = "row_vector3")]
      $op!([<$lib R3R3>], RowVector3<$in>, RowVector3<$in>, RowVector3<$out>, [<$lib:lower _vec_op>]);
      #[cfg(feature = "row_vector4")]
      $op!([<$lib R4R4>], RowVector4<$in>, RowVector4<$in>, RowVector4<$out>, [<$lib:lower _vec_op>]);
      #[cfg(feature = "row_vectord")]
      $op!([<$lib RDRD>], RowDVector<$in>, RowDVector<$in>, RowDVector<$out>, [<$lib:lower _vec_op>]);
      // Vector Vector
      #[cfg(feature = "vector2")]
      $op!([<$lib V2V2>], Vector2<$in>, Vector2<$in>, Vector2<$out>, [<$lib:lower _vec_op>]);
      #[cfg(feature = "vector3")]
      $op!([<$lib V3V3>], Vector3<$in>, Vector3<$in>, Vector3<$out>, [<$lib:lower _vec_op>]);
      #[cfg(feature = "vector4")]
      $op!([<$lib V4V4>], Vector4<$in>, Vector4<$in>, Vector4<$out>, [<$lib:lower _vec_op>]);
      #[cfg(feature = "vectord")]
      $op!([<$lib VDVD>], DVector<$in>, DVector<$in>, DVector<$out>, [<$lib:lower _vec_op>]);
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
            (LegacyValue::$lhs_type(lhs), LegacyValue::$lhs_type(rhs)) => {
              Ok(Box::new([<$lib SS>]{lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new($target_type::default()) }))
            },
            // Scalar Matrix
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            (LegacyValue::$lhs_type(lhs), LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix1(rhs))) => {
              Ok(Box::new([<$lib SM1>]{lhs, rhs, out: Ref::new(Matrix1::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2"))]
            (LegacyValue::$lhs_type(lhs), LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix2(rhs))) => {
              Ok(Box::new([<$lib SM2>]{lhs, rhs, out: Ref::new(Matrix2::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            (LegacyValue::$lhs_type(lhs), LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix3(rhs))) => {
              Ok(Box::new([<$lib SM3>]{lhs, rhs, out: Ref::new(Matrix3::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            (LegacyValue::$lhs_type(lhs), LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix4(rhs))) => {
              Ok(Box::new([<$lib SM4>]{lhs, rhs, out: Ref::new(Matrix4::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            (LegacyValue::$lhs_type(lhs), LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix2x3(rhs))) => {
              Ok(Box::new([<$lib SM2x3>]{lhs, rhs, out: Ref::new(Matrix2x3::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            (LegacyValue::$lhs_type(lhs), LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix3x2(rhs))) => {
              Ok(Box::new([<$lib SM3x2>]{lhs, rhs, out: Ref::new(Matrix3x2::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            (LegacyValue::$lhs_type(lhs), LegacyValue::[<Matrix $lhs_type>](Matrix::DMatrix(rhs))) => {
              let (rows,cols) = {rhs.borrow().shape()};
              Ok(Box::new([<$lib SMD>]{lhs, rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
            },
            // Scalar Row
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            (LegacyValue::$lhs_type(lhs), LegacyValue::[<Matrix $lhs_type>](Matrix::RowVector2(rhs))) => {
              Ok(Box::new([<$lib SR2>]{lhs, rhs, out: Ref::new(RowVector2::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            (LegacyValue::$lhs_type(lhs), LegacyValue::[<Matrix $lhs_type>](Matrix::RowVector3(rhs))) => {
              Ok(Box::new([<$lib SR3>]{lhs, rhs, out: Ref::new(RowVector3::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            (LegacyValue::$lhs_type(lhs), LegacyValue::[<Matrix $lhs_type>](Matrix::RowVector4(rhs))) => {
              Ok(Box::new([<$lib SR4>]{lhs, rhs, out: Ref::new(RowVector4::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            (LegacyValue::$lhs_type(lhs), LegacyValue::[<Matrix $lhs_type>](Matrix::RowDVector(rhs))) => {
              Ok(Box::new([<$lib SRD>]{lhs, rhs: rhs.clone(), out: Ref::new(RowDVector::from_element(rhs.borrow().len(),$target_type::default()))}))
            },
            // Scalar Vector
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            (LegacyValue::$lhs_type(lhs), LegacyValue::[<Matrix $lhs_type>](Matrix::Vector2(rhs))) => {
              Ok(Box::new([<$lib SV2>]{lhs, rhs, out: Ref::new(Vector2::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            (LegacyValue::$lhs_type(lhs), LegacyValue::[<Matrix $lhs_type>](Matrix::Vector3(rhs))) => {
              Ok(Box::new([<$lib SV3>]{lhs, rhs, out: Ref::new(Vector3::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            (LegacyValue::$lhs_type(lhs), LegacyValue::[<Matrix $lhs_type>](Matrix::Vector4(rhs))) => {
              Ok(Box::new([<$lib SV4>]{lhs, rhs, out: Ref::new(Vector4::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            (LegacyValue::$lhs_type(lhs), LegacyValue::[<Matrix $lhs_type>](Matrix::DVector(rhs))) => {
              Ok(Box::new([<$lib SVD>]{lhs, rhs: rhs.clone(), out: Ref::new(DVector::from_element(rhs.borrow().len(),$target_type::default()))}))
            },
            // Matrix Scalar
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix1(lhs)),LegacyValue::$lhs_type(rhs)) => {
              Ok(Box::new([<$lib M1S>]{lhs, rhs, out: Ref::new(Matrix1::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix2(lhs)),LegacyValue::$lhs_type(rhs)) => {
              Ok(Box::new([<$lib M2S>]{lhs, rhs, out: Ref::new(Matrix2::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix3(lhs)),LegacyValue::$lhs_type(rhs)) => {
              Ok(Box::new([<$lib M3S>]{lhs, rhs, out: Ref::new(Matrix3::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix4(lhs)),LegacyValue::$lhs_type(rhs)) => {
              Ok(Box::new([<$lib M4S>]{lhs, rhs, out: Ref::new(Matrix4::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix2x3(lhs)),LegacyValue::$lhs_type(rhs)) => {
              Ok(Box::new([<$lib M2x3S>]{lhs, rhs, out: Ref::new(Matrix2x3::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix3x2(lhs)),LegacyValue::$lhs_type(rhs)) => {
              Ok(Box::new([<$lib M3x2S>]{lhs, rhs, out: Ref::new(Matrix3x2::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::DMatrix(lhs)),LegacyValue::$lhs_type(rhs)) => {
              let (rows,cols) = {lhs.borrow().shape()};
              Ok(Box::new([<$lib MDS>]{lhs: lhs.clone(), rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
            },
            // Row Scalar
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::RowVector2(lhs)),LegacyValue::$lhs_type(rhs)) => {
              Ok(Box::new([<$lib R2S>]{lhs, rhs, out: Ref::new(RowVector2::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::RowVector3(lhs)),LegacyValue::$lhs_type(rhs)) => {
              Ok(Box::new([<$lib R3S>]{lhs, rhs, out: Ref::new(RowVector3::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::RowVector4(lhs)),LegacyValue::$lhs_type(rhs)) => {
              Ok(Box::new([<$lib R4S>]{lhs, rhs, out: Ref::new(RowVector4::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::RowDVector(lhs)),LegacyValue::$lhs_type(rhs)) => {
              Ok(Box::new([<$lib RDS>]{lhs: lhs.clone(), rhs, out: Ref::new(RowDVector::from_element(lhs.borrow().len(),$target_type::default()))}))
            },
            // Vector Scalar
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::Vector2(lhs)),LegacyValue::$lhs_type(rhs)) => {
              Ok(Box::new([<$lib V2S>]{lhs, rhs, out: Ref::new(Vector2::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::Vector3(lhs)),LegacyValue::$lhs_type(rhs)) => {
              Ok(Box::new([<$lib V3S>]{lhs, rhs, out: Ref::new(Vector3::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::Vector4(lhs)),LegacyValue::$lhs_type(rhs)) => {
              Ok(Box::new([<$lib V4S>]{lhs, rhs, out: Ref::new(Vector4::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::DVector(lhs)),LegacyValue::$lhs_type(rhs)) => {
              Ok(Box::new([<$lib VDS>]{lhs: lhs.clone(), rhs, out: Ref::new(DVector::from_element(lhs.borrow().len(),$target_type::default()))}))
            },
            // Matrix Matrix
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix1(lhs)), LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix1(rhs))) => {
              Ok(Box::new([<$lib M1M1>]{lhs, rhs, out: Ref::new(Matrix1::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix2(lhs)), LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix2(rhs))) => {
              Ok(Box::new([<$lib M2M2>]{lhs, rhs, out: Ref::new(Matrix2::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix3(lhs)), LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix3(rhs))) => {
              Ok(Box::new([<$lib M3M3>]{lhs, rhs, out: Ref::new(Matrix3::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix4(lhs)), LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix4(rhs))) => {
              Ok(Box::new([<$lib M4M4>]{lhs, rhs, out: Ref::new(Matrix4::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix2x3(lhs)), LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix2x3(rhs))) => {
              Ok(Box::new([<$lib M2x3M2x3>]{lhs, rhs, out: Ref::new(Matrix2x3::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix3x2(lhs)), LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix3x2(rhs))) => {
              Ok(Box::new([<$lib M3x2M3x2>]{lhs, rhs, out: Ref::new(Matrix3x2::from_element($target_type::default()))}))
            },
              #[cfg(all(feature = $value_string, feature = "matrixd"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::DMatrix(lhs)), LegacyValue::[<Matrix $lhs_type>](Matrix::DMatrix(rhs))) => {
              let (rows,cols) = {lhs.borrow().shape()};
              Ok(Box::new([<$lib MDMD>]{lhs, rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
            },
            // Row Row
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::RowVector2(lhs)), LegacyValue::[<Matrix $lhs_type>](Matrix::RowVector2(rhs))) => {
              Ok(Box::new([<$lib R2R2>]{lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(RowVector2::from_element($target_type::default())) }))
            },
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::RowVector3(lhs)), LegacyValue::[<Matrix $lhs_type>](Matrix::RowVector3(rhs))) => {
              Ok(Box::new([<$lib R3R3>]{lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(RowVector3::from_element($target_type::default())) }))
            },
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::RowVector4(lhs)), LegacyValue::[<Matrix $lhs_type>](Matrix::RowVector4(rhs))) => {
              Ok(Box::new([<$lib R4R4>]{lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(RowVector4::from_element($target_type::default())) }))
            },
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::RowDVector(lhs)), LegacyValue::[<Matrix $lhs_type>](Matrix::RowDVector(rhs))) => {
              Ok(Box::new([<$lib RDRD>]{lhs: lhs.clone(), rhs, out: Ref::new(RowDVector::from_element(lhs.borrow().len(),$target_type::default())) }))
            },
            // Vector Vector
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::Vector2(lhs)), LegacyValue::[<Matrix $lhs_type>](Matrix::Vector2(rhs))) => {
              Ok(Box::new([<$lib V2V2>]{lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Vector2::from_element($target_type::default())) }))
            },
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::Vector3(lhs)), LegacyValue::[<Matrix $lhs_type>](Matrix::Vector3(rhs))) => {
              Ok(Box::new([<$lib V3V3>]{lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Vector3::from_element($target_type::default())) }))
            },
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::Vector4(lhs)), LegacyValue::[<Matrix $lhs_type>](Matrix::Vector4(rhs))) => {
              Ok(Box::new([<$lib V4V4>]{lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Vector4::from_element($target_type::default())) }))
            },
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::DVector(lhs)), LegacyValue::[<Matrix $lhs_type>](Matrix::DVector(rhs))) => {
              Ok(Box::new([<$lib VDVD>]{lhs: lhs.clone(), rhs, out: Ref::new(DVector::from_element(lhs.borrow().len(),$target_type::default())) }))
            },
            // Matrix Vector
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector2"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix2(lhs)),LegacyValue::[<Matrix $lhs_type>](Matrix::Vector2(rhs))) => {
              Ok(Box::new([<$lib M2V2>]{lhs, rhs, out: Ref::new(Matrix2::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3", feature = "vector3"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix3(lhs)),LegacyValue::[<Matrix $lhs_type>](Matrix::Vector3(rhs))) => {
              Ok(Box::new([<$lib M3V3>]{lhs, rhs, out: Ref::new(Matrix3::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "vector2"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix2x3(lhs)),LegacyValue::[<Matrix $lhs_type>](Matrix::Vector2(rhs))) => {
              Ok(Box::new([<$lib M2x3V2>]{lhs, rhs, out: Ref::new(Matrix2x3::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "vector3"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix3x2(lhs)),LegacyValue::[<Matrix $lhs_type>](Matrix::Vector3(rhs))) => {
              Ok(Box::new([<$lib M3x2V3>]{lhs, rhs, out: Ref::new(Matrix3x2::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix4", feature = "vector4"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix4(lhs)),LegacyValue::[<Matrix $lhs_type>](Matrix::Vector4(rhs))) => {
              Ok(Box::new([<$lib M4V4>]{lhs, rhs, out: Ref::new(Matrix4::from_element($target_type::default()))}))
            },
            // Vector Matrix
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "row_vector2"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix2(lhs)),LegacyValue::[<Matrix $lhs_type>](Matrix::RowVector2(rhs))) => {
              Ok(Box::new([<$lib M2R2>]{lhs, rhs, out: Ref::new(Matrix2::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3", feature = "row_vector3"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix3(lhs)),LegacyValue::[<Matrix $lhs_type>](Matrix::RowVector3(rhs))) => {
              Ok(Box::new([<$lib M3R3>]{lhs, rhs, out: Ref::new(Matrix3::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "row_vector3"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix2x3(lhs)),LegacyValue::[<Matrix $lhs_type>](Matrix::RowVector3(rhs))) => {
              Ok(Box::new([<$lib M2x3R3>]{lhs, rhs, out: Ref::new(Matrix2x3::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "row_vector2"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix3x2(lhs)),LegacyValue::[<Matrix $lhs_type>](Matrix::RowVector2(rhs))) => {
              Ok(Box::new([<$lib M3x2R2>]{lhs, rhs, out: Ref::new(Matrix3x2::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix4", feature = "row_vector4"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix4(lhs)),LegacyValue::[<Matrix $lhs_type>](Matrix::RowVector4(rhs))) => {
              Ok(Box::new([<$lib M4R4>]{lhs, rhs, out: Ref::new(Matrix4::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::DMatrix(lhs)),LegacyValue::[<Matrix $lhs_type>](rhs)) => {
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
                    MechError::new(
                      DimensionMismatch { dims: vec![rows, cols, rhs_shape[0], rhs_shape[1]] },
                      None
                    ).with_compiler_loc()
                  );
                }
              }
              match rhs {
                #[cfg(feature = "vector2")]
                Matrix::Vector2(rhs) => {
                  Ok(Box::new([<$lib MDV2>]{lhs: lhs.clone(), rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
                },
                #[cfg(feature = "vector3")]
                Matrix::Vector3(rhs) => {
                  Ok(Box::new([<$lib MDV3>]{lhs: lhs.clone(), rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
                },
                #[cfg(feature = "vector4")]
                Matrix::Vector4(rhs) => {
                  Ok(Box::new([<$lib MDV4>]{lhs: lhs.clone(), rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
                },
                #[cfg(feature = "vectord")]
                Matrix::DVector(rhs) => {
                  Ok(Box::new([<$lib MDVD>]{lhs: lhs.clone(), rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
                },
                #[cfg(feature = "row_vector2")]
                Matrix::RowVector2(rhs) => {
                  Ok(Box::new([<$lib MDR2>]{lhs: lhs.clone(), rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
                },
                #[cfg(feature = "row_vector3")]
                Matrix::RowVector3(rhs) => {
                  Ok(Box::new([<$lib MDR3>]{lhs: lhs.clone(), rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
                },
                #[cfg(feature = "row_vector4")]
                Matrix::RowVector4(rhs) => {
                  Ok(Box::new([<$lib MDR4>]{lhs: lhs.clone(), rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
                },
                #[cfg(feature = "row_vectord")]
                Matrix::RowDVector(rhs) => {
                  Ok(Box::new([<$lib MDRD>]{lhs: lhs.clone(), rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
                },
                x => {
                  return Err(
                    MechError::new(
                      DimensionMismatch { dims: vec![rows, cols, rhs_shape[0], rhs_shape[1]] },
                      None
                    ).with_compiler_loc()
                  );
                }
              }
            },
            // Vector Matrix
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector2"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::Vector2(lhs)), LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix2(rhs))) => {
              Ok(Box::new([<$lib V2M2>]{lhs, rhs, out: Ref::new(Matrix2::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3", feature = "vector3"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::Vector3(lhs)), LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix3(rhs))) => {
              Ok(Box::new([<$lib V3M3>]{lhs, rhs, out: Ref::new(Matrix3::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "vector2"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::Vector2(lhs)), LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix2x3(rhs))) => {
              Ok(Box::new([<$lib V2M2x3>]{lhs, rhs, out: Ref::new(Matrix2x3::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "vector3"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::Vector3(lhs)), LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix3x2(rhs))) => {
              Ok(Box::new([<$lib V3M3x2>]{lhs, rhs, out: Ref::new(Matrix3x2::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrix4", feature = "vector4"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::Vector4(lhs)), LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix4(rhs))) => {
              Ok(Box::new([<$lib V4M4>]{lhs, rhs, out: Ref::new(Matrix4::from_element($target_type::default()))}))
            },
            // Row Matrix
            #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "matrix2"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::RowVector2(lhs)), LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix2(rhs))) => {
              Ok(Box::new([<$lib R2M2>]{lhs, rhs, out: Ref::new(Matrix2::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "matrix3"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::RowVector3(lhs)), LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix3(rhs))) => {
              Ok(Box::new([<$lib R3M3>]{lhs, rhs, out: Ref::new(Matrix3::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "matrix2x3"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::RowVector3(lhs)), LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix2x3(rhs))) => {
              Ok(Box::new([<$lib R3M2x3>]{lhs, rhs, out: Ref::new(Matrix2x3::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "matrix3x2"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::RowVector2(lhs)), LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix3x2(rhs))) => {
              Ok(Box::new([<$lib R2M3x2>]{lhs, rhs, out: Ref::new(Matrix3x2::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "row_vector4", feature = "matrix4"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::RowVector4(lhs)), LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix4(rhs))) => {
              Ok(Box::new([<$lib R4M4>]{lhs, rhs, out: Ref::new(Matrix4::from_element($target_type::default()))}))
            },
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            (LegacyValue::[<Matrix $lhs_type>](lhs),LegacyValue::[<Matrix $lhs_type>](Matrix::DMatrix(rhs))) => {
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
                    MechError::new(
                      DimensionMismatch { dims: vec![lhs_shape[0], lhs_shape[1], rows, cols] },
                      None
                    ).with_compiler_loc()
                  );
                }
              }
              match lhs {
                #[cfg(feature = "vector2")]
                Matrix::Vector2(lhs) => {
                  Ok(Box::new([<$lib V2MD>]{lhs, rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
                },
                #[cfg(feature = "vector3")]
                Matrix::Vector3(lhs) => {
                  Ok(Box::new([<$lib V3MD>]{lhs, rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
                },
                #[cfg(feature = "vector4")]
                Matrix::Vector4(lhs) => {
                  Ok(Box::new([<$lib V4MD>]{lhs, rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
                },
                #[cfg(feature = "vectord")]
                Matrix::DVector(lhs) => {
                  Ok(Box::new([<$lib VDMD>]{lhs, rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
                },
                #[cfg(feature = "row_vector2")]
                Matrix::RowVector2(lhs) => {
                  Ok(Box::new([<$lib R2MD>]{lhs, rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
                },
                #[cfg(feature = "row_vector3")]
                Matrix::RowVector3(lhs) => {
                  Ok(Box::new([<$lib R3MD>]{lhs, rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
                },
                #[cfg(feature = "row_vector4")]
                Matrix::RowVector4(lhs) => {
                  Ok(Box::new([<$lib R4MD>]{lhs, rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
                },
                #[cfg(feature = "row_vectord")]
                Matrix::RowDVector(lhs) => {
                  Ok(Box::new([<$lib RDMD>]{lhs, rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
                },
                _ => {
                  return Err(
                    MechError::new(
                      DimensionMismatch { dims: vec![lhs_shape[0], lhs_shape[1], rows, cols] },
                      None
                    ).with_compiler_loc()
                  );
                }
              }
            }
          )+
        )+
        (lhs,rhs) => Err(MechError::new(
          UnhandledFunctionArgumentKind2{arg: (lhs.kind(), rhs.kind()), fxn_name: stringify!($lib).to_string()},
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
            (LegacyValue::$lhs_type(arg)) => Ok(Box::new([<$lib S>]{arg: arg.clone(), out: Ref::new($target_type::default()), _marker: PhantomData::default() })),
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix1(arg))) => Ok(Box::new([<$lib V>]{arg, out: Ref::new(Matrix1::from_element($target_type::default())), _marker: PhantomData::default() })),
            #[cfg(all(feature = $value_string, feature = "matrix2"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix2(arg))) => Ok(Box::new([<$lib V>]{arg, out: Ref::new(Matrix2::from_element($target_type::default())), _marker: PhantomData::default() })),
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix3(arg))) => Ok(Box::new([<$lib V>]{arg, out: Ref::new(Matrix3::from_element($target_type::default())), _marker: PhantomData::default() })),
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix4(arg))) => Ok(Box::new([<$lib V>]{arg, out: Ref::new(Matrix4::from_element($target_type::default())), _marker: PhantomData::default() })),
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix2x3(arg))) => Ok(Box::new([<$lib V>]{arg, out: Ref::new(Matrix2x3::from_element($target_type::default())), _marker: PhantomData::default() })),
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::Matrix3x2(arg))) => Ok(Box::new([<$lib V>]{arg, out: Ref::new(Matrix3x2::from_element($target_type::default())), _marker: PhantomData::default() })),
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::RowVector2(arg))) => Ok(Box::new([<$lib V>]{arg: arg.clone(), out: Ref::new(RowVector2::from_element($target_type::default())), _marker: PhantomData::default() })),
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::RowVector3(arg))) => Ok(Box::new([<$lib V>]{arg: arg.clone(), out: Ref::new(RowVector3::from_element($target_type::default())), _marker: PhantomData::default() })),
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::RowVector4(arg))) => Ok(Box::new([<$lib V>]{arg: arg.clone(), out: Ref::new(RowVector4::from_element($target_type::default())), _marker: PhantomData::default() })),
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::RowDVector(arg))) => Ok(Box::new([<$lib V>]{arg: arg.clone(), out: Ref::new(RowDVector::from_element(arg.borrow().len(),$target_type::default())), _marker: PhantomData::default() })),
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::Vector2(arg))) => Ok(Box::new([<$lib V>]{arg: arg.clone(), out: Ref::new(Vector2::from_element($target_type::default())), _marker: PhantomData::default() })),
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::Vector3(arg))) => Ok(Box::new([<$lib V>]{arg: arg.clone(), out: Ref::new(Vector3::from_element($target_type::default())), _marker: PhantomData::default() })),
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::Vector4(arg))) => Ok(Box::new([<$lib V>]{arg: arg.clone(), out: Ref::new(Vector4::from_element($target_type::default())), _marker: PhantomData::default() })),
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::DVector(arg))) => Ok(Box::new([<$lib V>]{arg: arg.clone(), out: Ref::new(DVector::from_element(arg.borrow().len(),$target_type::default())), _marker: PhantomData::default() })),
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            (LegacyValue::[<Matrix $lhs_type>](Matrix::DMatrix(arg))) => {
              let (rows,cols) = {arg.borrow().shape()};
              Ok(Box::new([<$lib V>]{arg, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default())), _marker: PhantomData::default() }))},
          )+
        )+
        x => Err(MechError::new(
          UnhandledFunctionArgumentKind1{arg: x.kind(), fxn_name: stringify!($lib).to_string()},
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
        impl FunctionSpecializer for $fxn_name {
            fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
                if arguments.len() != 2 {
                    return Err(MechError::new(
                        IncorrectNumberOfArguments {
                            expected: 2,
                            found: arguments.len(),
                        },
                        None,
                    )
                    .with_compiler_loc());
                }
                let lhs_value = arguments[0].clone();
                let rhs_value = arguments[1].clone();
                match $gen_fxn(lhs_value.clone(), rhs_value.clone()) {
                    Ok(fxn) => Ok(fxn),
                    Err(_) => match (lhs_value, rhs_value) {
                        (
                            LegacyValue::MutableReference(lhs),
                            LegacyValue::MutableReference(rhs),
                        ) => $gen_fxn(lhs.borrow().clone(), rhs.borrow().clone()),
                        (lhs_value, LegacyValue::MutableReference(rhs)) => {
                            $gen_fxn(lhs_value.clone(), rhs.borrow().clone())
                        }
                        (LegacyValue::MutableReference(lhs), rhs_value) => {
                            $gen_fxn(lhs.borrow().clone(), rhs_value.clone())
                        }
                        (lhs, rhs) => {
                            if let Some(rhs_converted) = rhs.convert_to(&lhs.kind()) {
                                if let Ok(fxn) = $gen_fxn(lhs.clone(), rhs_converted) {
                                    return Ok(fxn);
                                }
                            }
                            if let Some(lhs_converted) = lhs.convert_to(&rhs.kind()) {
                                if let Ok(fxn) = $gen_fxn(lhs_converted, rhs.clone()) {
                                    return Ok(fxn);
                                }
                            }
                            Err(MechError::new(
                                UnhandledFunctionArgumentKind2 {
                                    arg: (lhs.kind(), rhs.kind()),
                                    fxn_name: stringify!($fxn_name).to_string(),
                                },
                                None,
                            )
                            .with_compiler_loc())
                        }
                    },
                }
            }
        }
    };
}

#[macro_export]
macro_rules! impl_mech_urnop_fxn {
    ($fxn_name:ident, $gen_fxn:tt, $fxn_string:tt) => {
        pub struct $fxn_name {}
        impl FunctionSpecializer for $fxn_name {
            fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
                if arguments.len() != 1 {
                    return Err(MechError::new(
                        IncorrectNumberOfArguments {
                            expected: 1,
                            found: arguments.len(),
                        },
                        None,
                    )
                    .with_compiler_loc());
                }
                let input = arguments[0].clone();
                match $gen_fxn(input.clone()) {
                    Ok(fxn) => Ok(fxn),
                    Err(_) => match (input) {
                        (LegacyValue::MutableReference(input)) => $gen_fxn(input.borrow().clone()),
                        x => Err(MechError::new(
                            UnhandledFunctionArgumentKind1 {
                                arg: x.kind(),
                                fxn_name: stringify!($fxn_name).to_string(),
                            },
                            None,
                        )
                        .with_compiler_loc()),
                    },
                }
            }
        }
    };
}

#[cfg(feature = "functions")]
pub fn box_mech_fxn<T>(r: MResult<Box<T>>) -> MResult<Box<dyn MechFunction>>
where
    T: MechFunction + 'static,
{
    r.map(|x| x as Box<dyn MechFunction>)
}

#[macro_export]
macro_rules! impl_assign_fxn {
    ($op:tt, $fxn_name:ident, $arg:expr, $value_kind:ident, $value_string:tt) => {{
        let mut res: MResult<_> = Err(MechError::new(
            GenericError {
                msg: "No matching types found".to_string(),
            },
            None,
        )
        .with_compiler_loc());

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
        let &(ref source, ref ixes, ref sink) = &$arg;
        res.map_err(|_| {
            MechError::new(
                UnhandledFunctionArgumentIxes {
                    arg: (
                        sink.kind(),
                        ixes.iter().map(|x| x.kind()).collect(),
                        source.kind(),
                    ),
                    fxn_name: stringify!($fxn_name).to_string(),
                },
                None,
            )
            .with_compiler_loc()
        })
    }};
}

#[macro_export]
macro_rules! impl_assign_scalar_arms {
  ($fxn_name:ident, $shape:tt, $arg:expr, $value_kind:ident, $value_string:tt) => {
    paste! {
      match $arg {
        // Scalar source
        #[cfg(all(feature = $value_string))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[LegacyValue::Index(ix)], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        (sink, ixes, source) => Err(MechError::new(
          UnhandledFunctionArgumentIxes { arg: (sink.kind(), ixes.iter().map(|x| x.kind()).collect(), source.kind()), fxn_name: stringify!($fxn_name).to_string() },
          None
        ).with_compiler_loc()),
      }
    }
  };
}

#[macro_export]
macro_rules! impl_assign_all_arms {
  ($fxn_name:ident, $shape:tt, $arg:expr, $value_kind:ident, $value_string:tt) => {
    paste! {
      match $arg {
        #[cfg(feature = $value_string)]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[LegacyValue::IndexAll], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        (sink, ixes, source) => Err(MechError::new(
          UnhandledFunctionArgumentIxes { arg: (sink.kind(), ixes.iter().map(|x| x.kind()).collect(), source.kind()), fxn_name: stringify!($fxn_name).to_string() },
          None
        ).with_compiler_loc()),
      }
    }
  };
}

#[macro_export]
macro_rules! impl_assign_scalar_scalar_arms {
  ($fxn_name:ident, $shape:tt, $arg:expr, $value_kind:ident, $value_string:tt) => {
    paste! {
      match $arg {
        #[cfg(feature = $value_string)]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[LegacyValue::Index(ix1), LegacyValue::Index(ix2)], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd", not(feature = "matrix1")))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[LegacyValue::MatrixIndex(Matrix::DMatrix(ix1)), LegacyValue::Index(ix2)], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name MD>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        (sink, ixes, source) => Err(MechError::new(
          UnhandledFunctionArgumentIxes { arg: (sink.kind(), ixes.iter().map(|x| x.kind()).collect(), source.kind()), fxn_name: stringify!($fxn_name).to_string() },
          None
        ).with_compiler_loc()),
      }
    }
  };
}

#[macro_export]
macro_rules! impl_set_range_arms {
  ($fxn_name:ident, $shape:tt, $arg:expr, $value_kind:ident, $value_string:tt) => {
    paste! {
      match $arg {
        // Scalar source
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[LegacyValue::MatrixIndex(Matrix::Matrix1(ix))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[LegacyValue::MatrixIndex(Matrix::Vector2(ix))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[LegacyValue::MatrixIndex(Matrix::Vector3(ix))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[LegacyValue::MatrixIndex(Matrix::Vector4(ix))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[LegacyValue::MatrixIndex(Matrix::DVector(ix))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        // Vector source
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Matrix1(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix1(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector4(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::DVector(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::DVector(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix4(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::DVector(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::DVector(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::DVector(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::DVector(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DVector(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::DVector(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector2(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector3(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector4(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector4(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector2(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector3(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector4(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector4(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        (source, ixes, sink) => Err(MechError::new(
          UnhandledFunctionArgumentIxes { arg: (source.kind(), ixes.to_vec().iter().map(|x| x.kind()).collect(), sink.kind()), fxn_name: stringify!($fxn_name).to_string() },
          None
        ).with_compiler_loc()),
      }
    }
  };
}

#[macro_export]
macro_rules! impl_assign_all_arms_b {
  ($fxn_name:ident, $shape:tt, $arg:expr, $value_kind:ident, $value_string:tt) => {
    paste! {
      match $arg {
        #[cfg(all(feature = $value_string, feature = "bool"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[LegacyValue::Bool(ix)], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        // Vector source, must have equal size to output
        #[cfg(all(feature = $value_string, feature = "bool"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::Bool(ix)], LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        (sink, ixes, source) => Err(MechError::new(
          UnhandledFunctionArgumentIxes { arg: (sink.kind(), ixes.iter().map(|x| x.kind()).collect(), source.kind()), fxn_name: stringify!($fxn_name).to_string() },
          None
        ).with_compiler_loc()),
      }
    }
  };
}

#[macro_export]
macro_rules! impl_set_range_all_arms {
  ($fxn_name:ident, $shape:tt, $arg:expr, $value_kind:ident, $value_string:tt) => {
    paste! {
      match $arg {
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[LegacyValue::MatrixIndex(Matrix::Matrix1(ix)),LegacyValue::IndexAll], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[LegacyValue::MatrixIndex(Matrix::Vector2(ix)),LegacyValue::IndexAll], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[LegacyValue::MatrixIndex(Matrix::Vector3(ix)),LegacyValue::IndexAll], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[LegacyValue::MatrixIndex(Matrix::Vector4(ix)),LegacyValue::IndexAll], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[LegacyValue::MatrixIndex(Matrix::DVector(ix)),LegacyValue::IndexAll], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Matrix1(ix)),LegacyValue::IndexAll], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix1(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector4(ix)),LegacyValue::IndexAll], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::DVector(ix)),LegacyValue::IndexAll], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::DVector(ix)),LegacyValue::IndexAll], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix4(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::DVector(ix)),LegacyValue::IndexAll], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::DVector(ix)),LegacyValue::IndexAll], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::DVector(ix)),LegacyValue::IndexAll], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::DVector(ix)),LegacyValue::IndexAll], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DVector(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::DVector(ix)),LegacyValue::IndexAll], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector2(ix)),LegacyValue::IndexAll], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector3(ix)),LegacyValue::IndexAll], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector4(ix)),LegacyValue::IndexAll], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector4(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector2(ix)),LegacyValue::IndexAll], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector3(ix)),LegacyValue::IndexAll], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector4(ix)),LegacyValue::IndexAll], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector4(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        (source, ixes, sink) => Err(MechError::new(
          UnhandledFunctionArgumentIxes { arg: (source.kind(), ixes.to_vec().iter().map(|x| x.kind()).collect(), sink.kind()), fxn_name: stringify!($fxn_name).to_string() },
          None
        ).with_compiler_loc()),
      }
    }
  };
}

#[macro_export]
macro_rules! impl_assign_range_scalar_arms {
  ($fxn_name:ident, $shape:tt, $arg:expr, $value_kind:ident, $value_string:tt) => {
    paste! {
      match $arg {
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[LegacyValue::MatrixIndex(Matrix::Matrix1(ix1)), LegacyValue::Index(ix2)], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[LegacyValue::MatrixIndex(Matrix::Vector2(ix1)), LegacyValue::Index(ix2)], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[LegacyValue::MatrixIndex(Matrix::Vector3(ix1)), LegacyValue::Index(ix2)], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[LegacyValue::MatrixIndex(Matrix::Vector4(ix1)), LegacyValue::Index(ix2)], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[LegacyValue::MatrixIndex(Matrix::DMatrix(ix1)), LegacyValue::Index(ix2)], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[LegacyValue::MatrixIndex(Matrix::DVector(ix1)), LegacyValue::Index(ix2)], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Matrix1(ix1)), LegacyValue::Index(ix2)], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix1(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector4(ix1)), LegacyValue::Index(ix2)], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::DVector(ix1)), LegacyValue::Index(ix2)], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3(source))) if ix1.borrow().len() == source.borrow().len() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::DVector(ix1)), LegacyValue::Index(ix2)], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix4(source))) if ix1.borrow().len() == source.borrow().len() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::DVector(ix1)), LegacyValue::Index(ix2)], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(source))) if ix1.borrow().len() == source.borrow().len() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::DVector(ix1)), LegacyValue::Index(ix2)], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(source))) if ix1.borrow().len() == source.borrow().len() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector2(ix1)), LegacyValue::Index(ix2)], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector3(ix1)), LegacyValue::Index(ix2)], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector4(ix1)), LegacyValue::Index(ix2)], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector4(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector2(ix1)), LegacyValue::Index(ix2)], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector3(ix1)), LegacyValue::Index(ix2)], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector4(ix1)), LegacyValue::Index(ix2)], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector4(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::DVector(ix1)), LegacyValue::Index(ix2)], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) if ix1.borrow().len() == source.borrow().len() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::DVector(ix1)), LegacyValue::Index(ix2)], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DVector(source))) if ix1.borrow().len() == source.borrow().len() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::DVector(ix1)), LegacyValue::Index(ix2)], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) if ix1.borrow().len() == source.borrow().len() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        (source, ixes, sink) => Err(MechError::new(
          UnhandledFunctionArgumentIxes { arg: (source.kind(), ixes.to_vec().iter().map(|x| x.kind()).collect(), sink.kind()), fxn_name: stringify!($fxn_name).to_string() },
          None
        ).with_compiler_loc()),
      }
    }
  };
}

#[macro_export]
macro_rules! impl_assign_scalar_range_arms {
  ($fxn_name:ident, $shape:tt, $arg:expr, $value_kind:ident, $value_string:tt) => {
    paste! {
      match $arg {
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[LegacyValue::Index(ix1), LegacyValue::MatrixIndex(Matrix::Matrix1(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[LegacyValue::Index(ix1), LegacyValue::MatrixIndex(Matrix::Vector2(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[LegacyValue::Index(ix1), LegacyValue::MatrixIndex(Matrix::Vector3(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[LegacyValue::Index(ix1), LegacyValue::MatrixIndex(Matrix::Vector4(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd", not(feature = "matrix1")))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[LegacyValue::Index(ix1), LegacyValue::MatrixIndex(Matrix::DMatrix(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[LegacyValue::Index(ix1), LegacyValue::MatrixIndex(Matrix::DVector(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::Index(ix1), LegacyValue::MatrixIndex(Matrix::Matrix1(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix1(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::Index(ix1), LegacyValue::MatrixIndex(Matrix::Vector4(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::Index(ix1), LegacyValue::MatrixIndex(Matrix::DVector(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::Index(ix1), LegacyValue::MatrixIndex(Matrix::DVector(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix4(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::Index(ix1), LegacyValue::MatrixIndex(Matrix::DVector(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(source))) if ix2.borrow().len() == source.borrow().len() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::Index(ix1), LegacyValue::MatrixIndex(Matrix::DVector(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(source))) if ix2.borrow().len() == source.borrow().len() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::Index(ix1), LegacyValue::MatrixIndex(Matrix::Vector2(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::Index(ix1), LegacyValue::MatrixIndex(Matrix::Vector3(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::Index(ix1), LegacyValue::MatrixIndex(Matrix::Vector4(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector4(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::Index(ix1), LegacyValue::MatrixIndex(Matrix::Vector2(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::Index(ix1), LegacyValue::MatrixIndex(Matrix::Vector3(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::Index(ix1), LegacyValue::MatrixIndex(Matrix::Vector4(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector4(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::Index(ix1), LegacyValue::MatrixIndex(Matrix::DVector(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) if ix2.borrow().len() == source.borrow().len() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::Index(ix1), LegacyValue::MatrixIndex(Matrix::DVector(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DVector(source))) if ix2.borrow().len() == source.borrow().len() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::Index(ix1), LegacyValue::MatrixIndex(Matrix::DVector(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) if ix2.borrow().len() == source.borrow().len() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        (sink, ixes, source) => Err(MechError::new(
          UnhandledFunctionArgumentIxes { arg: (source.kind(), ixes.to_vec().iter().map(|x| x.kind()).collect(), sink.kind()), fxn_name: stringify!($fxn_name).to_string() },
          None
        ).with_compiler_loc()),
      }
    }
  };
}

pub fn check_index_lengths<Ix1, Ix2, Source, A, B, C>(
    ix1: &Ref<Ix1>,
    ix2: &Ref<Ix2>,
    source: &Ref<Source>,
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
        return Err(MechError::new(
            MismatchedIndexLengthsError {
                ix1_len,
                ix2_len,
                source_len,
            },
            None,
        )
        .with_compiler_loc());
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct MismatchedIndexLengthsError {
    pub ix1_len: usize,
    pub ix2_len: usize,
    pub source_len: usize,
}
impl MechErrorKind for MismatchedIndexLengthsError {
    fn name(&self) -> &str {
        "MismatchedIndexLengths"
    }

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
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Matrix1(ix1)), LegacyValue::MatrixIndex(Matrix::Matrix1(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix1", feature = "vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Matrix1(ix1)), LegacyValue::MatrixIndex(Matrix::Vector2(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix1", feature = "vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Matrix1(ix1)), LegacyValue::MatrixIndex(Matrix::Vector3(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix1", feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Matrix1(ix1)), LegacyValue::MatrixIndex(Matrix::Vector4(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix1", feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Matrix1(ix1)), LegacyValue::MatrixIndex(Matrix::DVector(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "matrix1"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector2(ix1)), LegacyValue::MatrixIndex(Matrix::Matrix1(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector2(ix1)), LegacyValue::MatrixIndex(Matrix::Vector2(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector2(ix1)), LegacyValue::MatrixIndex(Matrix::Vector3(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector2(ix1)), LegacyValue::MatrixIndex(Matrix::Vector4(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector2(ix1)), LegacyValue::MatrixIndex(Matrix::DVector(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "matrix1"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector3(ix1)), LegacyValue::MatrixIndex(Matrix::Matrix1(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector3(ix1)), LegacyValue::MatrixIndex(Matrix::Vector2(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector3(ix1)), LegacyValue::MatrixIndex(Matrix::Vector3(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector3(ix1)), LegacyValue::MatrixIndex(Matrix::Vector4(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector3(ix1)), LegacyValue::MatrixIndex(Matrix::DVector(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "matrix1"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector4(ix1)), LegacyValue::MatrixIndex(Matrix::Matrix1(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector4(ix1)), LegacyValue::MatrixIndex(Matrix::Vector2(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector4(ix1)), LegacyValue::MatrixIndex(Matrix::Vector3(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector4(ix1)), LegacyValue::MatrixIndex(Matrix::Vector4(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector4(ix1)), LegacyValue::MatrixIndex(Matrix::DVector(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord", feature = "matrix1"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::DVector(ix1)), LegacyValue::MatrixIndex(Matrix::Matrix1(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::DVector(ix1)), LegacyValue::MatrixIndex(Matrix::Vector2(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::DVector(ix1)), LegacyValue::MatrixIndex(Matrix::Vector3(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::DVector(ix1)), LegacyValue::MatrixIndex(Matrix::Vector4(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::DVector(ix1)), LegacyValue::MatrixIndex(Matrix::DVector(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        // Vector source
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Matrix1(ix1)), LegacyValue::MatrixIndex(Matrix::Matrix1(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix1(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix1", feature = "vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Matrix1(ix1)), LegacyValue::MatrixIndex(Matrix::Vector2(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix1", feature = "row_vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Matrix1(ix1)), LegacyValue::MatrixIndex(Matrix::Vector2(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix1", feature = "vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Matrix1(ix1)), LegacyValue::MatrixIndex(Matrix::Vector3(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix1", feature = "row_vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Matrix1(ix1)), LegacyValue::MatrixIndex(Matrix::Vector3(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix1", feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Matrix1(ix1)), LegacyValue::MatrixIndex(Matrix::Vector4(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector4(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix1", feature = "vector2", feature = "matrix2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Matrix1(ix1)), LegacyValue::MatrixIndex(Matrix::Vector2(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix1", feature = "row_vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Matrix1(ix1)), LegacyValue::MatrixIndex(Matrix::Vector4(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector4(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix1", feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Matrix1(ix1)), LegacyValue::MatrixIndex(Matrix::DVector(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix1", feature = "row_vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Matrix1(ix1)), LegacyValue::MatrixIndex(Matrix::DVector(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix1", feature = "matrixd"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Matrix1(ix1)), LegacyValue::MatrixIndex(Matrix::DVector(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "matrix1", feature = "vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector2(ix1)), LegacyValue::MatrixIndex(Matrix::Matrix1(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "matrix1", feature = "row_vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector2(ix1)), LegacyValue::MatrixIndex(Matrix::Matrix1(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector2(ix1)), LegacyValue::MatrixIndex(Matrix::Vector2(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector4(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "row_vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector2(ix1)), LegacyValue::MatrixIndex(Matrix::Vector2(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector4(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector3", feature = "matrix2x3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector2(ix1)), LegacyValue::MatrixIndex(Matrix::Vector3(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector3", feature = "matrix3x2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector2(ix1)), LegacyValue::MatrixIndex(Matrix::Vector3(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector3", feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector2(ix1)), LegacyValue::MatrixIndex(Matrix::Vector3(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector3", feature = "row_vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector2(ix1)), LegacyValue::MatrixIndex(Matrix::Vector3(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector4", feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector2(ix1)), LegacyValue::MatrixIndex(Matrix::Vector4(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector4", feature = "row_vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector2(ix1)), LegacyValue::MatrixIndex(Matrix::Vector4(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector4", feature = "matrixd"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector2(ix1)), LegacyValue::MatrixIndex(Matrix::Vector4(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "vectord", feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector2(ix1)), LegacyValue::MatrixIndex(Matrix::DVector(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "vectord", feature = "row_vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector2(ix1)), LegacyValue::MatrixIndex(Matrix::DVector(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "vectord", feature = "matrixd"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector2(ix1)), LegacyValue::MatrixIndex(Matrix::DVector(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "matrix1", feature = "vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector3(ix1)), LegacyValue::MatrixIndex(Matrix::Matrix1(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "matrix1", feature = "row_vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector3(ix1)), LegacyValue::MatrixIndex(Matrix::Matrix1(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector2", feature = "matrix2x3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector3(ix1)), LegacyValue::MatrixIndex(Matrix::Vector2(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector2", feature = "matrix3x2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector3(ix1)), LegacyValue::MatrixIndex(Matrix::Vector2(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector2", feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector3(ix1)), LegacyValue::MatrixIndex(Matrix::Vector2(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector2", feature = "row_vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector3(ix1)), LegacyValue::MatrixIndex(Matrix::Vector2(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "matrix3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector3(ix1)), LegacyValue::MatrixIndex(Matrix::Vector3(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector3(ix1)), LegacyValue::MatrixIndex(Matrix::DVector(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "vectord", feature = "row_vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector3(ix1)), LegacyValue::MatrixIndex(Matrix::DVector(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector4", feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector3(ix1)), LegacyValue::MatrixIndex(Matrix::Vector4(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector4", feature = "row_vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector3(ix1)), LegacyValue::MatrixIndex(Matrix::Vector4(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector4", feature = "matrixd"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector3(ix1)), LegacyValue::MatrixIndex(Matrix::Vector4(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "vectord", feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector3(ix1)), LegacyValue::MatrixIndex(Matrix::DVector(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "vectord", feature = "row_vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector3(ix1)), LegacyValue::MatrixIndex(Matrix::DVector(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "vectord", feature = "matrixd"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector3(ix1)), LegacyValue::MatrixIndex(Matrix::DVector(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "matrix1", feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector4(ix1)), LegacyValue::MatrixIndex(Matrix::Matrix1(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector4(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "matrix1", feature = "row_vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector4(ix1)), LegacyValue::MatrixIndex(Matrix::Matrix1(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector4(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "matrix1", feature = "matrix2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector4(ix1)), LegacyValue::MatrixIndex(Matrix::Matrix1(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector2", feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector4(ix1)), LegacyValue::MatrixIndex(Matrix::Vector2(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector2", feature = "row_vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector4(ix1)), LegacyValue::MatrixIndex(Matrix::Vector2(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector2", feature = "matrixd"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector4(ix1)), LegacyValue::MatrixIndex(Matrix::Vector2(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector3", feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector4(ix1)), LegacyValue::MatrixIndex(Matrix::Vector3(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector3", feature = "row_vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector4(ix1)), LegacyValue::MatrixIndex(Matrix::Vector3(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector3", feature = "matrixd"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector4(ix1)), LegacyValue::MatrixIndex(Matrix::Vector3(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector4(ix1)), LegacyValue::MatrixIndex(Matrix::Vector4(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "row_vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector4(ix1)), LegacyValue::MatrixIndex(Matrix::Vector4(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "matrixd"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector4(ix1)), LegacyValue::MatrixIndex(Matrix::DVector(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector4(ix1)), LegacyValue::MatrixIndex(Matrix::DVector(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "row_vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector4(ix1)), LegacyValue::MatrixIndex(Matrix::DVector(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "matrix4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::Vector4(ix1)), LegacyValue::MatrixIndex(Matrix::Vector4(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix4(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord", feature = "matrix1", feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::DVector(ix1)), LegacyValue::MatrixIndex(Matrix::Matrix1(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DVector(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord", feature = "matrix1", feature = "row_vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::DVector(ix1)), LegacyValue::MatrixIndex(Matrix::Matrix1(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord", feature = "matrix1", feature = "matrixd"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::DVector(ix1)), LegacyValue::MatrixIndex(Matrix::Matrix1(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector2", feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::DVector(ix1)), LegacyValue::MatrixIndex(Matrix::Vector2(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector2", feature = "row_vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::DVector(ix1)), LegacyValue::MatrixIndex(Matrix::Vector2(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector2", feature = "matrixd"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::DVector(ix1)), LegacyValue::MatrixIndex(Matrix::Vector2(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector3", feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::DVector(ix1)), LegacyValue::MatrixIndex(Matrix::Vector3(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector3", feature = "row_vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::DVector(ix1)), LegacyValue::MatrixIndex(Matrix::Vector3(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector3", feature = "matrixd"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::DVector(ix1)), LegacyValue::MatrixIndex(Matrix::Vector3(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector4", feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::DVector(ix1)), LegacyValue::MatrixIndex(Matrix::Vector4(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector4", feature = "row_vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::DVector(ix1)), LegacyValue::MatrixIndex(Matrix::Vector4(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord", feature = "vector4", feature = "matrixd"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::DVector(ix1)), LegacyValue::MatrixIndex(Matrix::Vector4(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::DVector(ix1)), LegacyValue::MatrixIndex(Matrix::DVector(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord", feature = "matrixd"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::DVector(ix1)), LegacyValue::MatrixIndex(Matrix::DVector(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord", feature = "row_vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixIndex(Matrix::DVector(ix1)), LegacyValue::MatrixIndex(Matrix::DVector(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) => {
          check_index_lengths(&ix1, &ix2, &source)?;
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        (source, ixes, sink) => Err(MechError::new(
          UnhandledFunctionArgumentIxes { arg: (source.kind(), ixes.to_vec().iter().map(|x| x.kind()).collect(), sink.kind()), fxn_name: stringify!($fxn_name).to_string() },
          None
        ).with_compiler_loc()),
      }
    }
  };
}

#[macro_export]
macro_rules! impl_assign_all_range_arms {
  ($fxn_name:ident, $shape:tt, $arg:expr, $value_kind:ident, $value_string:tt) => {
    paste! {
      match $arg {
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[LegacyValue::IndexAll, LegacyValue::MatrixIndex(Matrix::Matrix1(ix))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[LegacyValue::IndexAll, LegacyValue::MatrixIndex(Matrix::Vector2(ix))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[LegacyValue::IndexAll, LegacyValue::MatrixIndex(Matrix::Vector3(ix))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[LegacyValue::IndexAll, LegacyValue::MatrixIndex(Matrix::Vector4(ix))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[LegacyValue::IndexAll, LegacyValue::MatrixIndex(Matrix::DVector(ix))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::IndexAll, LegacyValue::MatrixIndex(Matrix::Matrix1(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix1(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::IndexAll, LegacyValue::MatrixIndex(Matrix::Vector4(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::IndexAll, LegacyValue::MatrixIndex(Matrix::DVector(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::IndexAll, LegacyValue::MatrixIndex(Matrix::DVector(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix4(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::IndexAll, LegacyValue::MatrixIndex(Matrix::DVector(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::IndexAll, LegacyValue::MatrixIndex(Matrix::DVector(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::IndexAll, LegacyValue::MatrixIndex(Matrix::DVector(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::IndexAll, LegacyValue::MatrixIndex(Matrix::DVector(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DVector(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::IndexAll, LegacyValue::MatrixIndex(Matrix::DVector(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::IndexAll, LegacyValue::MatrixIndex(Matrix::Vector2(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::IndexAll, LegacyValue::MatrixIndex(Matrix::Vector3(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::IndexAll, LegacyValue::MatrixIndex(Matrix::Vector4(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector4(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::IndexAll, LegacyValue::MatrixIndex(Matrix::Vector2(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::IndexAll, LegacyValue::MatrixIndex(Matrix::Vector3(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::IndexAll, LegacyValue::MatrixIndex(Matrix::Vector4(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector4(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        (source, ixes, sink) => Err(MechError::new(
          UnhandledFunctionArgumentIxes { arg: (source.kind(), ixes.to_vec().iter().map(|x| x.kind()).collect(), sink.kind()), fxn_name: stringify!($fxn_name).to_string() },
          None
        ).with_compiler_loc()),
      }
    }
  };
}

#[macro_export]
macro_rules! impl_assign_all_scalar_arms {
  ($fxn_name:ident, $arg:expr, $value_kind:ident, $value_string:tt) => {
    paste! {
      match $arg {
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix1(sink)),[LegacyValue::IndexAll, LegacyValue::Index(ix)], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2(sink)),[LegacyValue::IndexAll, LegacyValue::Index(ix)], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3(sink)),[LegacyValue::IndexAll, LegacyValue::Index(ix)], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix4(sink)),[LegacyValue::IndexAll, LegacyValue::Index(ix)], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(sink)),[LegacyValue::IndexAll, LegacyValue::Index(ix)], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(sink)),[LegacyValue::IndexAll, LegacyValue::Index(ix)], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)),[LegacyValue::IndexAll, LegacyValue::Index(ix)], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::DVector(sink)),[LegacyValue::IndexAll, LegacyValue::Index(ix)], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowDVector(sink)),[LegacyValue::IndexAll, LegacyValue::Index(ix)], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector2(sink)),[LegacyValue::IndexAll, LegacyValue::Index(ix)], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector3(sink)),[LegacyValue::IndexAll, LegacyValue::Index(ix)], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector4(sink)),[LegacyValue::IndexAll, LegacyValue::Index(ix)], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector2(sink)),[LegacyValue::IndexAll, LegacyValue::Index(ix)], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector3(sink)),[LegacyValue::IndexAll, LegacyValue::Index(ix)], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector4(sink)),[LegacyValue::IndexAll, LegacyValue::Index(ix)], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        // Vector source
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix1(sink)), [LegacyValue::IndexAll, LegacyValue::Index(ix)], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix1(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "matrix2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2(sink)), [LegacyValue::IndexAll, LegacyValue::Index(ix)], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "matrix2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2(sink)), [LegacyValue::IndexAll, LegacyValue::Index(ix)], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "matrix3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3(sink)), [LegacyValue::IndexAll, LegacyValue::Index(ix)], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "matrix3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3(sink)), [LegacyValue::IndexAll, LegacyValue::Index(ix)], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector4", feature = "matrix4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix4(sink)), [LegacyValue::IndexAll, LegacyValue::Index(ix)], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector4(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "matrix4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix4(sink)), [LegacyValue::IndexAll, LegacyValue::Index(ix)], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector4(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "matrix2x3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(sink)), [LegacyValue::IndexAll, LegacyValue::Index(ix)], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "matrix2x3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(sink)), [LegacyValue::IndexAll, LegacyValue::Index(ix)], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "matrix3x2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(sink)), [LegacyValue::IndexAll, LegacyValue::Index(ix)], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "matrix3x2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(sink)), [LegacyValue::IndexAll, LegacyValue::Index(ix)], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord", feature = "matrixd"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [LegacyValue::IndexAll, LegacyValue::Index(ix)], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DVector(source))) if sink.borrow().nrows() == source.borrow().len() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "matrixd"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [LegacyValue::IndexAll, LegacyValue::Index(ix)], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) if sink.borrow().nrows() == source.borrow().len() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        (source, ixes, sink) => Err(MechError::new(
          UnhandledFunctionArgumentIxes { arg: (source.kind(), ixes.to_vec().iter().map(|x| x.kind()).collect(), sink.kind()), fxn_name: stringify!($fxn_name).to_string() },
          None
        ).with_compiler_loc()),
      }
    }
  };
}

#[macro_export]
macro_rules! impl_assign_scalar_all_arms {
  ($fxn_name:ident, $arg:expr, $value_kind:ident, $value_string:tt) => {
    paste! {
      match $arg {
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix1(sink)),[LegacyValue::Index(ix), LegacyValue::IndexAll], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2(sink)),[LegacyValue::Index(ix), LegacyValue::IndexAll], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3(sink)),[LegacyValue::Index(ix), LegacyValue::IndexAll], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix4(sink)),[LegacyValue::Index(ix), LegacyValue::IndexAll], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(sink)),[LegacyValue::Index(ix), LegacyValue::IndexAll], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(sink)),[LegacyValue::Index(ix), LegacyValue::IndexAll], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)),[LegacyValue::Index(ix), LegacyValue::IndexAll], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::DVector(sink)),[LegacyValue::Index(ix), LegacyValue::IndexAll], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowDVector(sink)),[LegacyValue::Index(ix), LegacyValue::IndexAll], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector2(sink)),[LegacyValue::Index(ix), LegacyValue::IndexAll], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector3(sink)),[LegacyValue::Index(ix), LegacyValue::IndexAll], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector4(sink)),[LegacyValue::Index(ix), LegacyValue::IndexAll], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector2(sink)),[LegacyValue::Index(ix), LegacyValue::IndexAll], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector3(sink)),[LegacyValue::Index(ix), LegacyValue::IndexAll], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector4(sink)),[LegacyValue::Index(ix), LegacyValue::IndexAll], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        // Vector source
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix1(sink)), [LegacyValue::Index(ix), LegacyValue::IndexAll], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix1(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "matrix2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2(sink)), [LegacyValue::Index(ix), LegacyValue::IndexAll], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "matrix2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2(sink)), [LegacyValue::Index(ix), LegacyValue::IndexAll], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "matrix3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3(sink)), [LegacyValue::Index(ix), LegacyValue::IndexAll], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "matrix3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3(sink)), [LegacyValue::Index(ix), LegacyValue::IndexAll], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector4", feature = "matrix4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix4(sink)), [LegacyValue::Index(ix), LegacyValue::IndexAll], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector4(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "matrix4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix4(sink)), [LegacyValue::Index(ix), LegacyValue::IndexAll], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector4(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "matrix2x3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(sink)), [LegacyValue::Index(ix), LegacyValue::IndexAll], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "matrix2x3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(sink)), [LegacyValue::Index(ix), LegacyValue::IndexAll], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "matrix3x2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(sink)), [LegacyValue::Index(ix), LegacyValue::IndexAll], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "matrix3x2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(sink)), [LegacyValue::Index(ix), LegacyValue::IndexAll], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord", feature = "matrixd"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [LegacyValue::Index(ix), LegacyValue::IndexAll], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DVector(source))) if sink.borrow().ncols() == source.borrow().len() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "matrixd"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [LegacyValue::Index(ix), LegacyValue::IndexAll], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) if sink.borrow().ncols() == source.borrow().len() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        (source, ixes, sink) => Err(MechError::new(
          UnhandledFunctionArgumentIxes { arg: (source.kind(), ixes.to_vec().iter().map(|x| x.kind()).collect(), sink.kind()), fxn_name: stringify!($fxn_name).to_string() },
          None
        ).with_compiler_loc()),
      }
    }
  };
}

#[macro_export]
macro_rules! impl_set_all_range_arms_b {
  ($fxn_name:ident, $arg:expr, $value_kind:ident, $value_string:tt) => {
    paste! {
      match $arg {
        // Scalar source
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix1(sink)),[LegacyValue::IndexAll, LegacyValue::MatrixBool(Matrix::Matrix1(ix))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2(sink)),[LegacyValue::IndexAll, LegacyValue::MatrixBool(Matrix::Vector4(ix))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3(sink)),[LegacyValue::IndexAll, LegacyValue::MatrixBool(Matrix::DVector(ix))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix4(sink)),[LegacyValue::IndexAll, LegacyValue::MatrixBool(Matrix::DVector(ix))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(sink)),[LegacyValue::IndexAll, LegacyValue::MatrixBool(Matrix::DVector(ix))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(sink)),[LegacyValue::IndexAll, LegacyValue::MatrixBool(Matrix::DVector(ix))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)),[LegacyValue::IndexAll, LegacyValue::MatrixBool(Matrix::DVector(ix))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector2(sink)),[LegacyValue::IndexAll, LegacyValue::MatrixBool(Matrix::Vector2(ix))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector3(sink)),[LegacyValue::IndexAll, LegacyValue::MatrixBool(Matrix::Vector3(ix))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector4(sink)),[LegacyValue::IndexAll, LegacyValue::MatrixBool(Matrix::Vector4(ix))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::DVector(sink)),[LegacyValue::IndexAll, LegacyValue::MatrixBool(Matrix::DVector(ix))], LegacyValue::[<$value_kind:camel>](source)) if sink.borrow().len() == ix.borrow().len() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector2(sink)),[LegacyValue::IndexAll, LegacyValue::MatrixBool(Matrix::Vector2(ix))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector3(sink)),[LegacyValue::IndexAll, LegacyValue::MatrixBool(Matrix::Vector3(ix))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector4(sink)),[LegacyValue::IndexAll, LegacyValue::MatrixBool(Matrix::Vector4(ix))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowDVector(sink)),[LegacyValue::IndexAll, LegacyValue::MatrixBool(Matrix::DVector(ix))], LegacyValue::[<$value_kind:camel>](source)) if sink.borrow().len() == ix.borrow().len() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        // Vector source
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix1(sink)), [LegacyValue::IndexAll, LegacyValue::MatrixBool(Matrix::Matrix1(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix1(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2(sink)), [LegacyValue::IndexAll, LegacyValue::MatrixBool(Matrix::Vector2(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3(sink)), [LegacyValue::IndexAll, LegacyValue::MatrixBool(Matrix::Vector3(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix4(sink)), [LegacyValue::IndexAll, LegacyValue::MatrixBool(Matrix::Vector4(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix4(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(sink)), [LegacyValue::IndexAll, LegacyValue::MatrixBool(Matrix::Vector3(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(sink)), [LegacyValue::IndexAll, LegacyValue::MatrixBool(Matrix::Vector2(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [LegacyValue::IndexAll, LegacyValue::MatrixBool(Matrix::DVector(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) if sink.borrow().ncols() == source.borrow().ncols() && sink.borrow().nrows() == source.borrow().nrows() && ix.borrow().len() == source.borrow().ncols() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::DVector(sink)), [LegacyValue::IndexAll, LegacyValue::MatrixBool(Matrix::DVector(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DVector(source))) if sink.borrow().len() == source.borrow().len() && sink.borrow().len() == ix.borrow().len() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowDVector(sink)), [LegacyValue::IndexAll, LegacyValue::MatrixBool(Matrix::DVector(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) if sink.borrow().len() == source.borrow().len() && sink.borrow().len() == ix.borrow().len() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector2(sink)), [LegacyValue::IndexAll, LegacyValue::MatrixBool(Matrix::Vector2(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector3(sink)), [LegacyValue::IndexAll, LegacyValue::MatrixBool(Matrix::Vector3(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector4(sink)), [LegacyValue::IndexAll, LegacyValue::MatrixBool(Matrix::Vector4(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector4(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector2(sink)), [LegacyValue::IndexAll, LegacyValue::MatrixBool(Matrix::Vector2(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector3(sink)), [LegacyValue::IndexAll, LegacyValue::MatrixBool(Matrix::Vector3(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector4(sink)), [LegacyValue::IndexAll, LegacyValue::MatrixBool(Matrix::Vector4(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector4(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        (source, ixes, sink) => Err(MechError::new(
          UnhandledFunctionArgumentIxes { arg: (source.kind(), ixes.to_vec().iter().map(|x| x.kind()).collect(), sink.kind()), fxn_name: stringify!($fxn_name).to_string() },
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
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix1(sink)),[LegacyValue::MatrixBool(Matrix::Matrix1(ix)), LegacyValue::IndexAll], LegacyValue::[<$value_kind:camel>](source)) if ix.borrow().len() == sink.borrow().nrows() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2(sink)),[LegacyValue::MatrixBool(Matrix::Vector2(ix)), LegacyValue::IndexAll], LegacyValue::[<$value_kind:camel>](source)) if ix.borrow().len() == sink.borrow().nrows() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3(sink)),[LegacyValue::MatrixBool(Matrix::Vector3(ix)), LegacyValue::IndexAll], LegacyValue::[<$value_kind:camel>](source)) if ix.borrow().len() == sink.borrow().nrows() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix4(sink)),[LegacyValue::MatrixBool(Matrix::Vector4(ix)), LegacyValue::IndexAll], LegacyValue::[<$value_kind:camel>](source)) if ix.borrow().len() == sink.borrow().nrows() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(sink)),[LegacyValue::MatrixBool(Matrix::Vector2(ix)), LegacyValue::IndexAll], LegacyValue::[<$value_kind:camel>](source)) if ix.borrow().len() == sink.borrow().nrows() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(sink)),[LegacyValue::MatrixBool(Matrix::Vector3(ix)), LegacyValue::IndexAll], LegacyValue::[<$value_kind:camel>](source)) if ix.borrow().len() == sink.borrow().nrows() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)),[LegacyValue::MatrixBool(Matrix::DVector(ix)), LegacyValue::IndexAll], LegacyValue::[<$value_kind:camel>](source)) if ix.borrow().len() == sink.borrow().nrows() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector2(sink)),[LegacyValue::MatrixBool(Matrix::Vector2(ix)), LegacyValue::IndexAll], LegacyValue::[<$value_kind:camel>](source)) if ix.borrow().len() == sink.borrow().nrows() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector3(sink)),[LegacyValue::MatrixBool(Matrix::Vector3(ix)), LegacyValue::IndexAll], LegacyValue::[<$value_kind:camel>](source)) if ix.borrow().len() == sink.borrow().nrows() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector4(sink)),[LegacyValue::MatrixBool(Matrix::Vector4(ix)), LegacyValue::IndexAll], LegacyValue::[<$value_kind:camel>](source)) if ix.borrow().len() == sink.borrow().nrows() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::DVector(sink)),[LegacyValue::MatrixBool(Matrix::DVector(ix)), LegacyValue::IndexAll], LegacyValue::[<$value_kind:camel>](source)) if sink.borrow().len() == ix.borrow().len() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector2(sink)),[LegacyValue::MatrixBool(Matrix::Vector2(ix)), LegacyValue::IndexAll], LegacyValue::[<$value_kind:camel>](source)) if ix.borrow().len() == sink.borrow().nrows() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector3(sink)),[LegacyValue::MatrixBool(Matrix::Vector3(ix)), LegacyValue::IndexAll], LegacyValue::[<$value_kind:camel>](source)) if ix.borrow().len() == sink.borrow().nrows() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector4(sink)),[LegacyValue::MatrixBool(Matrix::Vector4(ix)), LegacyValue::IndexAll], LegacyValue::[<$value_kind:camel>](source)) if ix.borrow().len() == sink.borrow().nrows() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowDVector(sink)),[LegacyValue::MatrixBool(Matrix::DVector(ix)), LegacyValue::IndexAll], LegacyValue::[<$value_kind:camel>](source)) if sink.borrow().len() == ix.borrow().len() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        // Vector source
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix1(sink)), [LegacyValue::MatrixBool(Matrix::Matrix1(ix)), LegacyValue::IndexAll], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix1(source))) if ix.borrow().len() == sink.borrow().nrows() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2(sink)), [LegacyValue::MatrixBool(Matrix::Vector2(ix)), LegacyValue::IndexAll], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2(source))) if ix.borrow().len() == sink.borrow().nrows() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3(sink)), [LegacyValue::MatrixBool(Matrix::Vector3(ix)), LegacyValue::IndexAll], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3(source))) if ix.borrow().len() == sink.borrow().nrows() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix4(sink)), [LegacyValue::MatrixBool(Matrix::Vector4(ix)), LegacyValue::IndexAll], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix4(source))) if ix.borrow().len() == sink.borrow().nrows() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(sink)), [LegacyValue::MatrixBool(Matrix::Vector2(ix)), LegacyValue::IndexAll], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(source))) if ix.borrow().len() == sink.borrow().nrows() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(sink)), [LegacyValue::MatrixBool(Matrix::Vector3(ix)), LegacyValue::IndexAll], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(source))) if ix.borrow().len() == sink.borrow().nrows() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [LegacyValue::MatrixBool(Matrix::DVector(ix)), LegacyValue::IndexAll], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) if sink.borrow().ncols() == source.borrow().ncols() && sink.borrow().nrows() == source.borrow().nrows() && ix.borrow().len() == sink.borrow().nrows() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::DVector(sink)), [LegacyValue::MatrixBool(Matrix::DVector(ix)), LegacyValue::IndexAll], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DVector(source))) if sink.borrow().len() == source.borrow().len() && sink.borrow().len() == ix.borrow().len() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowDVector(sink)), [LegacyValue::MatrixBool(Matrix::DVector(ix)), LegacyValue::IndexAll], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) if sink.borrow().len() == source.borrow().len() && sink.borrow().len() == ix.borrow().len() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        (source, ixes, sink) => Err(MechError::new(
          UnhandledFunctionArgumentIxes { arg: (source.kind(), ixes.to_vec().iter().map(|x| x.kind()).collect(), sink.kind()), fxn_name: stringify!($fxn_name).to_string() },
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
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix1(sink)),[LegacyValue::MatrixBool(Matrix::Matrix1(ix))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2(sink)),[LegacyValue::MatrixBool(Matrix::Vector4(ix))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3(sink)),[LegacyValue::MatrixBool(Matrix::DVector(ix))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix4(sink)),[LegacyValue::MatrixBool(Matrix::DVector(ix))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(sink)),[LegacyValue::MatrixBool(Matrix::DVector(ix))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(sink)),[LegacyValue::MatrixBool(Matrix::DVector(ix))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)),[LegacyValue::MatrixBool(Matrix::DVector(ix))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector2(sink)),[LegacyValue::MatrixBool(Matrix::Vector2(ix))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector3(sink)),[LegacyValue::MatrixBool(Matrix::Vector3(ix))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector4(sink)),[LegacyValue::MatrixBool(Matrix::Vector4(ix))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::DVector(sink)),[LegacyValue::MatrixBool(Matrix::DVector(ix))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector2(sink)),[LegacyValue::MatrixBool(Matrix::Vector2(ix))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector3(sink)),[LegacyValue::MatrixBool(Matrix::Vector3(ix))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector4(sink)),[LegacyValue::MatrixBool(Matrix::Vector4(ix))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowDVector(sink)),[LegacyValue::MatrixBool(Matrix::DVector(ix))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        // Vector source, must have equal size to output
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix1(sink)), [LegacyValue::MatrixBool(Matrix::Matrix1(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix1(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2(sink)), [LegacyValue::MatrixBool(Matrix::Vector4(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3(sink)), [LegacyValue::MatrixBool(Matrix::DVector(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix4(sink)), [LegacyValue::MatrixBool(Matrix::DVector(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix4(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(sink)), [LegacyValue::MatrixBool(Matrix::DVector(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(sink)), [LegacyValue::MatrixBool(Matrix::DVector(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [LegacyValue::MatrixBool(Matrix::DVector(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::DVector(sink)), [LegacyValue::MatrixBool(Matrix::DVector(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DVector(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowDVector(sink)), [LegacyValue::MatrixBool(Matrix::DVector(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector2(sink)), [LegacyValue::MatrixBool(Matrix::Vector2(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector3(sink)), [LegacyValue::MatrixBool(Matrix::Vector3(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector4(sink)), [LegacyValue::MatrixBool(Matrix::Vector4(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector4(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector2(sink)), [LegacyValue::MatrixBool(Matrix::Vector2(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector3(sink)), [LegacyValue::MatrixBool(Matrix::Vector3(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector4(sink)), [LegacyValue::MatrixBool(Matrix::Vector4(ix))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector4(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        (source, ixes, sink) => Err(MechError::new(
          UnhandledFunctionArgumentIxes { arg: (source.kind(), ixes.to_vec().iter().map(|x| x.kind()).collect(), sink.kind()), fxn_name: stringify!($fxn_name).to_string() },
          None
        ).with_compiler_loc()),
      }
    }
  };
}

#[macro_export]
macro_rules! impl_assign_scalar_arms_b {
  ($fxn_name:ident, $shape:tt, $arg:expr, $value_kind:ident, $value_string:tt) => {
    paste! {
      match $arg {
        #[cfg(all(feature = $value_string, feature = "bool"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[LegacyValue::Bool(ix)], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })))
        },
        // Vector source, must have equal size to output
        #[cfg(all(feature = $value_string, feature = "bool"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::Bool(ix)], LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: ix.clone(), _marker: PhantomData::default() })))
        },
        (source, ixes, sink) => Err(MechError::new(
          UnhandledFunctionArgumentIxes { arg: (source.kind(), ixes.to_vec().iter().map(|x| x.kind()).collect(), sink.kind()), fxn_name: stringify!($fxn_name).to_string() },
          None
        ).with_compiler_loc()),
      }
    }
  };
}

#[macro_export]
macro_rules! impl_assign_range_scalar_arms_b {
  ($fxn_name:ident, $shape:tt, $arg:expr, $value_kind:ident, $value_string:tt) => {
    paste! {
      match $arg {
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[LegacyValue::MatrixBool(Matrix::Matrix1(ix1)), LegacyValue::Index(ix2)], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[LegacyValue::MatrixBool(Matrix::Vector2(ix1)), LegacyValue::Index(ix2)], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[LegacyValue::MatrixBool(Matrix::Vector3(ix1)), LegacyValue::Index(ix2)], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[LegacyValue::MatrixBool(Matrix::Vector4(ix1)), LegacyValue::Index(ix2)], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[LegacyValue::MatrixBool(Matrix::DVector(ix1)), LegacyValue::Index(ix2)], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixBool(Matrix::Matrix1(ix1)), LegacyValue::Index(ix2)], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix1(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixBool(Matrix::Vector4(ix1)), LegacyValue::Index(ix2)], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixBool(Matrix::DVector(ix1)), LegacyValue::Index(ix2)], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3(source))) if ix1.borrow().len() == source.borrow().len() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixBool(Matrix::DVector(ix1)), LegacyValue::Index(ix2)], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix4(source))) if ix1.borrow().len() == source.borrow().len() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixBool(Matrix::DVector(ix1)), LegacyValue::Index(ix2)], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(source))) if ix1.borrow().len() == source.borrow().len() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixBool(Matrix::DVector(ix1)), LegacyValue::Index(ix2)], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(source))) if ix1.borrow().len() == source.borrow().len() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixBool(Matrix::Vector2(ix1)), LegacyValue::Index(ix2)], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixBool(Matrix::Vector3(ix1)), LegacyValue::Index(ix2)], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixBool(Matrix::Vector4(ix1)), LegacyValue::Index(ix2)], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector4(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixBool(Matrix::Vector2(ix1)), LegacyValue::Index(ix2)], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixBool(Matrix::Vector3(ix1)), LegacyValue::Index(ix2)], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixBool(Matrix::Vector4(ix1)), LegacyValue::Index(ix2)], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector4(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixBool(Matrix::DVector(ix1)), LegacyValue::Index(ix2)], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DVector(source))) if ix1.borrow().len() == source.borrow().len() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixBool(Matrix::DVector(ix1)), LegacyValue::Index(ix2)], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) if ix1.borrow().len() == source.borrow().len() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::MatrixBool(Matrix::DVector(ix1)), LegacyValue::Index(ix2)], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) if ix1.borrow().len() == source.borrow().len() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(),ix2.clone()), _marker: PhantomData::default() })))
        },
        (source, ixes, sink) => Err(MechError::new(
          UnhandledFunctionArgumentIxes { arg: (source.kind(), ixes.to_vec().iter().map(|x| x.kind()).collect(), sink.kind()), fxn_name: stringify!($fxn_name).to_string() },
          None
        ).with_compiler_loc()),
      }
    }
  };
}

#[macro_export]
macro_rules! impl_assign_scalar_range_arms_b {
  ($fxn_name:ident, $shape:tt, $arg:expr, $value_kind:ident, $value_string:tt) => {
    paste! {
      match $arg {
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[LegacyValue::Index(ix1), LegacyValue::MatrixBool(Matrix::Matrix1(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[LegacyValue::Index(ix1), LegacyValue::MatrixBool(Matrix::Vector2(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[LegacyValue::Index(ix1), LegacyValue::MatrixBool(Matrix::Vector3(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[LegacyValue::Index(ix1), LegacyValue::MatrixBool(Matrix::Vector4(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)),[LegacyValue::Index(ix1), LegacyValue::MatrixBool(Matrix::DVector(ix2))], LegacyValue::[<$value_kind:camel>](source)) if ix2.borrow().len() == sink.borrow().ncols() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::Index(ix1), LegacyValue::MatrixBool(Matrix::Matrix1(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix1(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::Index(ix1), LegacyValue::MatrixBool(Matrix::Vector4(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::Index(ix1), LegacyValue::MatrixBool(Matrix::DVector(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3(source))) if ix2.borrow().len() == source.borrow().len() && ix2.borrow().len() == sink.borrow().ncols() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::Index(ix1), LegacyValue::MatrixBool(Matrix::DVector(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix4(source))) if ix2.borrow().len() == source.borrow().len() && ix2.borrow().len() == sink.borrow().ncols() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::Index(ix1), LegacyValue::MatrixBool(Matrix::DVector(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(source))) if ix2.borrow().len() == source.borrow().len() && ix2.borrow().len() == sink.borrow().ncols() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::Index(ix1), LegacyValue::MatrixBool(Matrix::DVector(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(source))) if ix2.borrow().len() == source.borrow().len() && ix2.borrow().len() == sink.borrow().ncols() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::Index(ix1), LegacyValue::MatrixBool(Matrix::Vector2(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::Index(ix1), LegacyValue::MatrixBool(Matrix::Vector3(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::Index(ix1), LegacyValue::MatrixBool(Matrix::Vector4(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector4(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::Index(ix1), LegacyValue::MatrixBool(Matrix::Vector2(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::Index(ix1), LegacyValue::MatrixBool(Matrix::Vector3(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::Index(ix1), LegacyValue::MatrixBool(Matrix::Vector4(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector4(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::Index(ix1), LegacyValue::MatrixBool(Matrix::DVector(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DVector(source))) if ix2.borrow().len() == sink.borrow().ncols() && ix2.borrow().len() == source.borrow().len() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::Index(ix1), LegacyValue::MatrixBool(Matrix::DVector(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) if ix2.borrow().len() == sink.borrow().ncols() && ix2.borrow().len() == source.borrow().len() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::$shape(sink)), [LegacyValue::Index(ix1), LegacyValue::MatrixBool(Matrix::DVector(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) if ix2.borrow().len() == sink.borrow().ncols() && ix2.borrow().len() == source.borrow().len() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        (source, ixes, sink) => Err(MechError::new(
          UnhandledFunctionArgumentIxes { arg: (source.kind(), ixes.to_vec().iter().map(|x| x.kind()).collect(), sink.kind()), fxn_name: stringify!($fxn_name).to_string() },
          None
        ).with_compiler_loc()),
      }
    }
  };
}

#[macro_export]
macro_rules! impl_assign_range_range_arms_b {
  ($fxn_name:ident, $shape:tt, $arg:expr, $value_kind:ident, $value_string:tt) => {
    paste! {
      match $arg {
        // Scalar source
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix1(sink)), [LegacyValue::MatrixBool(Matrix::Matrix1(ix1)), LegacyValue::MatrixBool(Matrix::Matrix1(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name BB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "matrix1", feature = "row_vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector2(sink)), [LegacyValue::MatrixBool(Matrix::Matrix1(ix1)), LegacyValue::MatrixBool(Matrix::Vector2(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name BB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "matrix1", feature = "row_vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector3(sink)), [LegacyValue::MatrixBool(Matrix::Matrix1(ix1)), LegacyValue::MatrixBool(Matrix::Vector3(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name BB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector4", feature = "matrix1", feature = "row_vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector4(sink)), [LegacyValue::MatrixBool(Matrix::Matrix1(ix1)), LegacyValue::MatrixBool(Matrix::Vector4(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name BB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "matrix1", feature = "row_vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowDVector(sink)), [LegacyValue::MatrixBool(Matrix::Matrix1(ix1)), LegacyValue::MatrixBool(Matrix::DVector(ix2))], LegacyValue::[<$value_kind:camel>](source)) if ix2.borrow().len() == sink.borrow().len() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name BB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector2", feature = "matrix1"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector2(sink)), [LegacyValue::MatrixBool(Matrix::Vector2(ix1)), LegacyValue::MatrixBool(Matrix::Matrix1(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name BB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector3", feature = "matrix1"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector3(sink)), [LegacyValue::MatrixBool(Matrix::Vector3(ix1)), LegacyValue::MatrixBool(Matrix::Matrix1(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name BB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector4", feature = "matrix1"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector4(sink)), [LegacyValue::MatrixBool(Matrix::Vector4(ix1)), LegacyValue::MatrixBool(Matrix::Matrix1(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name BB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord", feature = "vectord", feature = "matrix1"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::DVector(sink)), [LegacyValue::MatrixBool(Matrix::DVector(ix1)), LegacyValue::MatrixBool(Matrix::Matrix1(ix2))], LegacyValue::[<$value_kind:camel>](source)) if ix1.borrow().len() == sink.borrow().len() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name BB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2(sink)), [LegacyValue::MatrixBool(Matrix::Vector2(ix1)), LegacyValue::MatrixBool(Matrix::Vector2(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name BB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3", feature = "vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3(sink)), [LegacyValue::MatrixBool(Matrix::Vector3(ix1)), LegacyValue::MatrixBool(Matrix::Vector3(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name BB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix4", feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix4(sink)), [LegacyValue::MatrixBool(Matrix::Vector4(ix1)), LegacyValue::MatrixBool(Matrix::Vector4(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name BB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "vector3", feature = "vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(sink)), [LegacyValue::MatrixBool(Matrix::Vector2(ix1)), LegacyValue::MatrixBool(Matrix::Vector3(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name BB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "vector2", feature = "vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(sink)), [LegacyValue::MatrixBool(Matrix::Vector3(ix1)), LegacyValue::MatrixBool(Matrix::Vector2(ix2))], LegacyValue::[<$value_kind:camel>](source)) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name BB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vector4", feature = "vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [LegacyValue::MatrixBool(Matrix::Vector4(ix1)), LegacyValue::MatrixBool(Matrix::Vector2(ix2))], LegacyValue::[<$value_kind:camel>](source)) if sink.borrow().nrows() == 4 && sink.borrow().ncols() == 2 => {
          box_mech_fxn(Ok(Box::new([<$fxn_name BB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vector2", feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [LegacyValue::MatrixBool(Matrix::Vector2(ix1)), LegacyValue::MatrixBool(Matrix::Vector4(ix2))], LegacyValue::[<$value_kind:camel>](source)) if sink.borrow().nrows() == 2 && sink.borrow().ncols() == 4 => {
          box_mech_fxn(Ok(Box::new([<$fxn_name BB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [LegacyValue::MatrixBool(Matrix::DVector(ix1)), LegacyValue::MatrixBool(Matrix::Vector2(ix2))], LegacyValue::[<$value_kind:camel>](source)) if sink.borrow().ncols() == 2 && ix1.borrow().len() == sink.borrow().nrows() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name BB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [LegacyValue::MatrixBool(Matrix::DVector(ix1)), LegacyValue::MatrixBool(Matrix::Vector3(ix2))], LegacyValue::[<$value_kind:camel>](source)) if sink.borrow().ncols() == 3 && ix1.borrow().len() == sink.borrow().nrows() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name BB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [LegacyValue::MatrixBool(Matrix::DVector(ix1)), LegacyValue::MatrixBool(Matrix::Vector4(ix2))], LegacyValue::[<$value_kind:camel>](source)) if sink.borrow().ncols() == 4 && ix1.borrow().len() == sink.borrow().nrows() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name BB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [LegacyValue::MatrixBool(Matrix::DVector(ix1)), LegacyValue::MatrixBool(Matrix::DVector(ix2))], LegacyValue::[<$value_kind:camel>](source)) if ix1.borrow().len() == sink.borrow().nrows() && ix2.borrow().len() == sink.borrow().ncols() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name BB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        // Vector source
        #[cfg(all(feature = $value_string, feature = "matrix1"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix1(sink)), [LegacyValue::MatrixBool(Matrix::Matrix1(ix1)), LegacyValue::MatrixBool(Matrix::Matrix1(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix1(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "matrix1", feature = "row_vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector2(sink)), [LegacyValue::MatrixBool(Matrix::Matrix1(ix1)), LegacyValue::MatrixBool(Matrix::Vector2(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "matrix1", feature = "row_vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector3(sink)), [LegacyValue::MatrixBool(Matrix::Matrix1(ix1)), LegacyValue::MatrixBool(Matrix::Vector3(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vector4", feature = "matrix1", feature = "row_vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector4(sink)), [LegacyValue::MatrixBool(Matrix::Matrix1(ix1)), LegacyValue::MatrixBool(Matrix::Vector4(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowVector4(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "matrix1", feature = "row_vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowDVector(sink)), [LegacyValue::MatrixBool(Matrix::Matrix1(ix1)), LegacyValue::MatrixBool(Matrix::DVector(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::RowDVector(source))) if ix2.borrow().len() == sink.borrow().len() && ix2.borrow().len() == source.borrow().len() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector2", feature = "matrix1"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector2(sink)), [LegacyValue::MatrixBool(Matrix::Vector2(ix1)), LegacyValue::MatrixBool(Matrix::Matrix1(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector3", feature = "matrix1"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector3(sink)), [LegacyValue::MatrixBool(Matrix::Vector3(ix1)), LegacyValue::MatrixBool(Matrix::Matrix1(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector4", feature = "matrix1"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector4(sink)), [LegacyValue::MatrixBool(Matrix::Vector4(ix1)), LegacyValue::MatrixBool(Matrix::Matrix1(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Vector4(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "vectord", feature = "vectord", feature = "matrix1"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::DVector(sink)), [LegacyValue::MatrixBool(Matrix::DVector(ix1)), LegacyValue::MatrixBool(Matrix::Matrix1(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DVector(source))) if ix1.borrow().len() == sink.borrow().len() && ix2.borrow().len() == source.borrow().len() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2(sink)), [LegacyValue::MatrixBool(Matrix::Vector2(ix1)), LegacyValue::MatrixBool(Matrix::Vector2(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3", feature = "vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3(sink)), [LegacyValue::MatrixBool(Matrix::Vector3(ix1)), LegacyValue::MatrixBool(Matrix::Vector3(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix4", feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix4(sink)), [LegacyValue::MatrixBool(Matrix::Vector4(ix1)), LegacyValue::MatrixBool(Matrix::Vector4(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix4(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "vector3", feature = "vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(sink)), [LegacyValue::MatrixBool(Matrix::Vector2(ix1)), LegacyValue::MatrixBool(Matrix::Vector3(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix2x3(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "vector2", feature = "vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(sink)), [LegacyValue::MatrixBool(Matrix::Vector3(ix1)), LegacyValue::MatrixBool(Matrix::Vector2(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::Matrix3x2(source))) => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vector4", feature = "vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [LegacyValue::MatrixBool(Matrix::Vector4(ix1)), LegacyValue::MatrixBool(Matrix::Vector2(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) if sink.borrow().nrows() == 4 && sink.borrow().ncols() == 2 && source.borrow().nrows() == ix1.borrow().len() && source.borrow().ncols() == ix2.borrow().len() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vector2", feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [LegacyValue::MatrixBool(Matrix::Vector2(ix1)), LegacyValue::MatrixBool(Matrix::Vector4(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) if sink.borrow().nrows() == 2 && sink.borrow().ncols() == 4 && source.borrow().nrows() == ix1.borrow().len() && source.borrow().ncols() == ix2.borrow().len() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vector2"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [LegacyValue::MatrixBool(Matrix::DVector(ix1)), LegacyValue::MatrixBool(Matrix::Vector2(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) if sink.borrow().ncols() == 2 && ix1.borrow().len() == sink.borrow().nrows() && source.borrow().nrows() == ix1.borrow().len() && source.borrow().ncols() == ix2.borrow().len() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vector3"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [LegacyValue::MatrixBool(Matrix::DVector(ix1)), LegacyValue::MatrixBool(Matrix::Vector3(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) if sink.borrow().ncols() == 3 && ix1.borrow().len() == sink.borrow().nrows() && source.borrow().nrows() == ix1.borrow().len() && source.borrow().ncols() == ix2.borrow().len() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vector4"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [LegacyValue::MatrixBool(Matrix::DVector(ix1)), LegacyValue::MatrixBool(Matrix::Vector4(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) if sink.borrow().ncols() == 4 && ix1.borrow().len() == sink.borrow().nrows() && source.borrow().nrows() == ix1.borrow().len() && source.borrow().ncols() == ix2.borrow().len() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [LegacyValue::MatrixBool(Matrix::DVector(ix1)), LegacyValue::MatrixBool(Matrix::DVector(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) if ix1.borrow().len() == sink.borrow().nrows() && ix2.borrow().len() == sink.borrow().ncols() && source.borrow().nrows() == ix1.borrow().len() && source.borrow().ncols() == ix2.borrow().len() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VBB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        (source, ixes, sink) => Err(MechError::new(
          UnhandledFunctionArgumentIxes { arg: (source.kind(), ixes.to_vec().iter().map(|x| x.kind()).collect(), sink.kind()), fxn_name: stringify!($fxn_name).to_string() },
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
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [LegacyValue::MatrixBool(Matrix::DVector(ix1)), LegacyValue::MatrixIndex(Matrix::DVector(ix2))], LegacyValue::[<$value_kind:camel>](source)) if ix1.borrow().len() == sink.borrow().nrows() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name BU>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        // Vector source
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [LegacyValue::MatrixBool(Matrix::DVector(ix1)), LegacyValue::MatrixIndex(Matrix::DVector(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) if ix1.borrow().len() == sink.borrow().nrows() && ix1.borrow().len() == source.borrow().nrows() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VBU>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        (source, ixes, sink) => Err(MechError::new(
          UnhandledFunctionArgumentIxes { arg: (source.kind(), ixes.to_vec().iter().map(|x| x.kind()).collect(), sink.kind()), fxn_name: stringify!($fxn_name).to_string() },
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
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [LegacyValue::MatrixIndex(Matrix::DVector(ix1)), LegacyValue::MatrixBool(Matrix::DVector(ix2))], LegacyValue::[<$value_kind:camel>](source)) if ix2.borrow().len() == sink.borrow().ncols() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name UB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        // Vector-per-column source (VUB)
        #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord", feature = "vectord"))]
        (LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(sink)), [LegacyValue::MatrixIndex(Matrix::DVector(ix1)), LegacyValue::MatrixBool(Matrix::DVector(ix2))], LegacyValue::[<Matrix $value_kind:camel>](Matrix::DMatrix(source))) if ix2.borrow().len() == sink.borrow().ncols() && ix2.borrow().len() == source.borrow().ncols() => {
          box_mech_fxn(Ok(Box::new([<$fxn_name VUB>] { sink: sink.clone(), source: source.clone(), ixes: (ix1.clone(), ix2.clone()), _marker: PhantomData::default() })))
        },
        (source, ixes, sink) => Err(MechError::new(
          UnhandledFunctionArgumentIxes { arg: (source.kind(), ixes.to_vec().iter().map(|x| x.kind()).collect(), sink.kind()), fxn_name: stringify!($fxn_name).to_string() },
          None
        ).with_compiler_loc()),
      }
    }
  }
}

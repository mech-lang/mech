use crate::*;
use libm::{yn, ynf};

// Yn ------------------------------------------------------------------------

macro_rules! yn_op {
    ($arg1:expr, $arg2:expr, $out:expr) => {
        unsafe {
            (*$out) = yn((*$arg1) as i32, (*$arg2));
        }
    };
}

macro_rules! yn_vec_op {
    ($arg1:expr, $arg2:expr, $out:expr) => {
        unsafe {
            let arg1_deref = &(*$arg1);
            let arg2_deref = &(*$arg2);
            let out_deref = &mut *$out;
            for i in 0..arg1_deref.len() {
                (out_deref[i]) = yn(arg1_deref[i] as i32, arg2_deref[i]);
            }
        }
    };
}

macro_rules! ynf_op {
    ($arg1:expr, $arg2:expr, $out:expr) => {
        unsafe {
            (*$out) = ynf((*$arg1) as i32, (*$arg2));
        }
    };
}

macro_rules! ynf_vec_op {
    ($arg1:expr, $arg2:expr, $out:expr) => {
        unsafe {
            let arg1_deref = &(*$arg1);
            let arg2_deref = &(*$arg2);
            let out_deref = &mut *$out;
            for i in 0..arg1_deref.len() {
                (out_deref[i]) = ynf(arg1_deref[i] as i32, arg2_deref[i]);
            }
        }
    };
}

macro_rules! impl_two_arg_fxn {
    ($struct_name:ident, $kind1:ty, $kind2:ty, $out_kind:ty, $op:ident) => {
        #[derive(Debug)]
        pub(crate) struct $struct_name {
            arg1: Ref<$kind1>,
            arg2: Ref<$kind2>,
            out: Ref<$out_kind>,
        }
        impl MechFunctionFactory for $struct_name {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                <$out_kind as FunctionRuntimeType>::REPRESENTATION,
                <$kind1 as FunctionRuntimeType>::REPRESENTATION,
                <$kind2 as FunctionRuntimeType>::REPRESENTATION,
            );

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, arg1, arg2) = invocation.expect_binary()?;
                Ok(Box::new(Self {
                    arg1: arg1.try_ref()?,
                    arg2: arg2.try_ref()?,
                    out: out.try_ref()?,
                }))
            }

            fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
                Some(crate::ops::arithmetic_full_write_contract(
                    <$out_kind as FunctionRuntimeType>::REPRESENTATION,
                ))
            }
        }
        impl MechFunctionImpl for $struct_name {
            fn solve_result(&self) -> MResult<()> {
                let arg1_ptr = self.arg1.as_ptr();
                let arg2_ptr = self.arg2.as_ptr();
                let out_ptr = self.out.as_mut_ptr();
                $op!(arg1_ptr, arg2_ptr, out_ptr);
                Ok(())
            }
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_ref(&self.out))
            }
            fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
                Some(crate::ops::arithmetic_full_write_contract(
                    <$out_kind as FunctionRuntimeType>::REPRESENTATION,
                ))
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }

            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_ref(&self.out)]))
            }
        }
        #[cfg(feature = "semantic-compiler")]
        impl MechFunctionCompiler for $struct_name {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = stringify!($struct_name);
                compile_binop!(name, self.out, self.arg1, self.arg2, ctx);
            }
        }
    };
}

#[cfg(all(feature = "f32", feature = "matrix1"))]
impl_two_arg_fxn!(
    YnM1F32,
    Matrix1<f32>,
    Matrix1<f32>,
    Matrix1<f32>,
    ynf_vec_op
);
#[cfg(all(feature = "f32", feature = "matrix2"))]
impl_two_arg_fxn!(
    YnM2F32,
    Matrix2<f32>,
    Matrix2<f32>,
    Matrix2<f32>,
    ynf_vec_op
);
#[cfg(all(feature = "f32", feature = "matrix3"))]
impl_two_arg_fxn!(
    YnM3F32,
    Matrix3<f32>,
    Matrix3<f32>,
    Matrix3<f32>,
    ynf_vec_op
);
#[cfg(all(feature = "f32", feature = "matrix2x3"))]
impl_two_arg_fxn!(
    YnM2x3F32,
    Matrix2x3<f32>,
    Matrix2x3<f32>,
    Matrix2x3<f32>,
    ynf_vec_op
);
#[cfg(all(feature = "f32", feature = "matrix3"))]
impl_two_arg_fxn!(
    YnM3x2F32,
    Matrix3x2<f32>,
    Matrix3x2<f32>,
    Matrix3x2<f32>,
    ynf_vec_op
);
#[cfg(all(feature = "f32", feature = "matrix4"))]
impl_two_arg_fxn!(
    YnM4F32,
    Matrix4<f32>,
    Matrix4<f32>,
    Matrix4<f32>,
    ynf_vec_op
);
#[cfg(all(feature = "f32", feature = "vector2"))]
impl_two_arg_fxn!(
    YnV2F32,
    Vector2<f32>,
    Vector2<f32>,
    Vector2<f32>,
    ynf_vec_op
);
#[cfg(all(feature = "f32", feature = "vector3"))]
impl_two_arg_fxn!(
    YnV3F32,
    Vector3<f32>,
    Vector3<f32>,
    Vector3<f32>,
    ynf_vec_op
);
#[cfg(all(feature = "f32", feature = "vector4"))]
impl_two_arg_fxn!(
    YnV4F32,
    Vector4<f32>,
    Vector4<f32>,
    Vector4<f32>,
    ynf_vec_op
);
#[cfg(all(feature = "f32", feature = "row_vector2"))]
impl_two_arg_fxn!(
    YnR2F32,
    RowVector2<f32>,
    RowVector2<f32>,
    RowVector2<f32>,
    ynf_vec_op
);
#[cfg(all(feature = "f32", feature = "row_vector3"))]
impl_two_arg_fxn!(
    YnR3F32,
    RowVector3<f32>,
    RowVector3<f32>,
    RowVector3<f32>,
    ynf_vec_op
);
#[cfg(all(feature = "f32", feature = "row_vector4"))]
impl_two_arg_fxn!(
    YnR4F32,
    RowVector4<f32>,
    RowVector4<f32>,
    RowVector4<f32>,
    ynf_vec_op
);
#[cfg(all(feature = "f32", feature = "row_vectord"))]
impl_two_arg_fxn!(
    YnRDF32,
    RowDVector<f32>,
    RowDVector<f32>,
    RowDVector<f32>,
    ynf_vec_op
);
#[cfg(all(feature = "f32", feature = "vectord"))]
impl_two_arg_fxn!(
    YnVDF32,
    DVector<f32>,
    DVector<f32>,
    DVector<f32>,
    ynf_vec_op
);
#[cfg(all(feature = "f32", feature = "matrixd"))]
impl_two_arg_fxn!(
    YnMDF32,
    DMatrix<f32>,
    DMatrix<f32>,
    DMatrix<f32>,
    ynf_vec_op
);

#[cfg(feature = "f32")]
impl_two_arg_fxn!(YnF32, f32, f32, f32, ynf_op);

#[cfg(all(feature = "f64", feature = "matrix1"))]
impl_two_arg_fxn!(YnM1F64, Matrix1<f64>, Matrix1<f64>, Matrix1<f64>, yn_vec_op);
#[cfg(all(feature = "f64", feature = "matrix2"))]
impl_two_arg_fxn!(YnM2F64, Matrix2<f64>, Matrix2<f64>, Matrix2<f64>, yn_vec_op);
#[cfg(all(feature = "f64", feature = "matrix3"))]
impl_two_arg_fxn!(YnM3F64, Matrix3<f64>, Matrix3<f64>, Matrix3<f64>, yn_vec_op);
#[cfg(all(feature = "f64", feature = "matrix2x3"))]
impl_two_arg_fxn!(
    YnM2x3F64,
    Matrix2x3<f64>,
    Matrix2x3<f64>,
    Matrix2x3<f64>,
    yn_vec_op
);
#[cfg(all(feature = "f64", feature = "matrix3"))]
impl_two_arg_fxn!(
    YnM3x2F64,
    Matrix3x2<f64>,
    Matrix3x2<f64>,
    Matrix3x2<f64>,
    yn_vec_op
);
#[cfg(all(feature = "f64", feature = "matrix4"))]
impl_two_arg_fxn!(YnM4F64, Matrix4<f64>, Matrix4<f64>, Matrix4<f64>, yn_vec_op);
#[cfg(all(feature = "f64", feature = "vector2"))]
impl_two_arg_fxn!(YnV2F64, Vector2<f64>, Vector2<f64>, Vector2<f64>, yn_vec_op);
#[cfg(all(feature = "f64", feature = "vector3"))]
impl_two_arg_fxn!(YnV3F64, Vector3<f64>, Vector3<f64>, Vector3<f64>, yn_vec_op);
#[cfg(all(feature = "f64", feature = "vector4"))]
impl_two_arg_fxn!(YnV4F64, Vector4<f64>, Vector4<f64>, Vector4<f64>, yn_vec_op);
#[cfg(all(feature = "f64", feature = "row_vector2"))]
impl_two_arg_fxn!(
    YnR2F64,
    RowVector2<f64>,
    RowVector2<f64>,
    RowVector2<f64>,
    yn_vec_op
);
#[cfg(all(feature = "f64", feature = "row_vector3"))]
impl_two_arg_fxn!(
    YnR3F64,
    RowVector3<f64>,
    RowVector3<f64>,
    RowVector3<f64>,
    yn_vec_op
);
#[cfg(all(feature = "f64", feature = "row_vector4"))]
impl_two_arg_fxn!(
    YnR4F64,
    RowVector4<f64>,
    RowVector4<f64>,
    RowVector4<f64>,
    yn_vec_op
);
#[cfg(all(feature = "f64", feature = "row_vectord"))]
impl_two_arg_fxn!(
    YnRDF64,
    RowDVector<f64>,
    RowDVector<f64>,
    RowDVector<f64>,
    yn_vec_op
);
#[cfg(all(feature = "f64", feature = "vectord"))]
impl_two_arg_fxn!(YnVDF64, DVector<f64>, DVector<f64>, DVector<f64>, yn_vec_op);
#[cfg(all(feature = "f64", feature = "matrixd"))]
impl_two_arg_fxn!(YnMDF64, DMatrix<f64>, DMatrix<f64>, DMatrix<f64>, yn_vec_op);

#[cfg(feature = "f64")]
impl_two_arg_fxn!(YnF64, f64, f64, f64, yn_op);

impl_canonical_math_same_type_binop_specializer!(MathYn, Yn, "math/bessel/yn");

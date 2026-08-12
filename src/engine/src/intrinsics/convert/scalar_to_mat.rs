#[macro_use]
use crate::intrinsics::*;
use nalgebra::Scalar;
use std::marker::PhantomData;

static PURE_SCALAR_BROADCAST_CONTRACT: std::sync::LazyLock<OperationContractDeclaration> =
    std::sync::LazyLock::new(|| OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![InputPortPolicy {
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            }]
            .into_boxed_slice(),
        ),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            alias: AliasPolicy::NoAlias,
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    });

#[derive(Debug)]
pub struct ConvertScalarToMat2<F, T> {
    pub arg: Ref<F>,
    pub out: Ref<T>,
}

impl<F, T> MechFunctionFactory for ConvertScalarToMat2<F, T>
where
    F: Debug + Scalar + Clone + Sync + Send + 'static + FunctionRuntimeType,
    T: Debug + Sync + Send + 'static + FunctionRuntimeType,
    #[cfg(feature = "compiler")]
    F: CompileConst + ConstElem + AsValueKind,
    #[cfg(feature = "compiler")]
    T: CompileConst + ConstElem + AsValueKind,
    Ref<T>: ToValue,
    for<'a> &'a mut T: IntoIterator<Item = &'a mut F>,
{
    const SIGNATURE: RuntimeFunctionSignature =
        RuntimeFunctionSignature::unary(T::REPRESENTATION, F::REPRESENTATION);

    fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
            FunctionArgs::Unary(out, arg) => {
                let arg: Ref<F> = arg.try_function_ref(FunctionArgumentRole::Input(0))?;
                let out: Ref<T> = out.try_function_ref(FunctionArgumentRole::Output)?;
                Ok(Box::new(Self { arg, out }))
            }
            _ => Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 1,
                    found: args.len(),
                },
                None,
            )
            .with_compiler_loc()),
        }
    }
}

impl<F, T> MechFunctionImpl for ConvertScalarToMat2<F, T>
where
    Ref<T>: ToValue,
    F: Debug + Scalar + Clone,
    for<'a> &'a mut T: IntoIterator<Item = &'a mut F>,
    T: Debug,
{
    fn solve_result(&self) -> MResult<()> {
        let arg_ptr = self.arg.as_ptr();
        let out_ptr = self.out.as_mut_ptr();
        unsafe {
            let arg_ref: &F = &*arg_ptr;
            let out_ref: &mut T = &mut *out_ptr;
            for dst in (&mut *out_ref).into_iter() {
                *dst = arg_ref.clone();
            }
        };
        Ok(())
    }
    fn out(&self) -> LegacyValue {
        self.out.to_value()
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_SCALAR_BROADCAST_CONTRACT)
    }

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(self.reactive_output_values())
    }
}
#[cfg(feature = "compiler")]
impl<F, T> MechFunctionCompiler for ConvertScalarToMat2<F, T>
where
    T: CompileConst + ConstElem + AsValueKind,
    F: ConstElem + CompileConst + AsValueKind,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "ConvertScalarToMat2<{},{}>",
            F::as_value_kind(),
            T::as_value_kind()
        );
        compile_unop!(name, self.out, self.arg, ctx);
    }
}

fn validate_scalar_to_matrix(args: &FunctionArgs) -> MResult<()> {
    let contract = "scalar_to_matrix";
    if args.input_count() != 1 {
        return Err(function_shape_contract_violation(
            contract,
            format!("expected one input, found {}", args.input_count()),
        ));
    }
    args.output_value()
        .function_matrix_descriptor(FunctionArgumentRole::Output)?
        .ok_or_else(|| {
            function_shape_contract_violation(contract, "output must be matrix-backed")
        })?;
    if args
        .input_value(0)
        .expect("input count was validated")
        .function_matrix_descriptor(FunctionArgumentRole::Input(0))?
        .is_some()
    {
        return Err(function_shape_contract_violation(
            contract,
            "input must be scalar",
        ));
    }
    Ok(())
}

mech_core::declare_native_runtime_factory! {
    cfg: all(
        feature = "native-kernel-prototype",
        feature = "convert",
        feature = "f64",
        feature = "row_vectord",
    ),
    registration: register_convert_scalar_f64_to_row_vectord,
    installer: install_convert_scalar_f64_to_row_vectord,
    name: "ConvertScalarToMat2<f64,[f64]:1,0>",
    factory_type: ConvertScalarToMat2<f64, RowDVector<f64>>,
    contract: RuntimeFunctionContract::custom(
        "scalar_to_matrix",
        RuntimeOutputAliasPolicy::DisallowInputAlias,
        validate_scalar_to_matrix,
    ),
    package: "mech-engine", crate_name: "mech_engine",
    installer_path: "mech_engine::__mech_native::install_convert_scalar_f64_to_row_vectord",
    extra_cargo_features: ["native-kernel-prototype", "convert"],
}

pub(crate) fn install_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(all(
        feature = "native-kernel-prototype",
        feature = "f64",
        feature = "row_vectord"
    ))]
    register_convert_scalar_f64_to_row_vectord(builder)?;
    Ok(())
}

macro_rules! impl_conversion_scalar_to_mat_match_arms {
  ($arg:expr, $($input_type:ident => $($target_type:ident, $type_string:tt),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            #[cfg(all(feature = "matrix", feature = $type_string))]
            (LegacyValue::$input_type(v), ValueKind::Matrix(target_kind, dims)) if matches!(target_kind.as_ref(), ValueKind::$target_type) => {
              match dims[..] {
                #[cfg(feature = "matrix1")]
                [1,1] => {let out = Matrix1::from_element(v.borrow().clone());      return Ok(Box::new(ConvertScalarToMat2{arg: v, out: Ref::new(out)}));},
                #[cfg(feature = "matrix2")]
                [2,2] => {let out = Matrix2::from_element(v.borrow().clone());      return Ok(Box::new(ConvertScalarToMat2{arg: v, out: Ref::new(out)}));},
                #[cfg(feature = "matrix3")]
                [3,3] => {let out = Matrix3::from_element(v.borrow().clone());      return Ok(Box::new(ConvertScalarToMat2{arg: v, out: Ref::new(out)}));},
                #[cfg(feature = "matrix4")]
                [4,4] => {let out = Matrix4::from_element(v.borrow().clone());      return Ok(Box::new(ConvertScalarToMat2{arg: v, out: Ref::new(out)}));},
                #[cfg(feature = "matrix2x3")]
                [2,3] => {let out = Matrix2x3::from_element(v.borrow().clone());    return Ok(Box::new(ConvertScalarToMat2{arg: v, out: Ref::new(out)}));},
                #[cfg(feature = "matrix3x2")]
                [3,2] => {let out = Matrix3x2::from_element(v.borrow().clone());    return Ok(Box::new(ConvertScalarToMat2{arg: v, out: Ref::new(out)}));},
                #[cfg(feature = "row_vector2")]
                [1,2] => {let out = RowVector2::from_element(v.borrow().clone());   return Ok(Box::new(ConvertScalarToMat2{arg: v, out: Ref::new(out)}));},
                #[cfg(feature = "row_vector3")]
                [1,3] => {let out = RowVector3::from_element(v.borrow().clone());   return Ok(Box::new(ConvertScalarToMat2{arg: v, out: Ref::new(out)}));},
                #[cfg(feature = "row_vector4")]
                [1,4] => {let out = RowVector4::from_element(v.borrow().clone());   return Ok(Box::new(ConvertScalarToMat2{arg: v, out: Ref::new(out)}));},
                #[cfg(feature = "vector2")]
                [2,1] => {let out = Vector2::from_element(v.borrow().clone());      return Ok(Box::new(ConvertScalarToMat2{arg: v, out: Ref::new(out)}));},
                #[cfg(feature = "vector3")]
                [3,1] => {let out = Vector3::from_element(v.borrow().clone());      return Ok(Box::new(ConvertScalarToMat2{arg: v, out: Ref::new(out)}));},
                #[cfg(feature = "vector4")]
                [4,1] => {let out = Vector4::from_element(v.borrow().clone());      return Ok(Box::new(ConvertScalarToMat2{arg: v, out: Ref::new(out)}));},
                #[cfg(feature = "row_vectord")]
                [1,n] => {let out = RowDVector::from_element(n,v.borrow().clone()); return Ok(Box::new(ConvertScalarToMat2{arg: v, out: Ref::new(out)}));},
                #[cfg(feature = "vectord")]
                [n,1] => {let out = DVector::from_element(n,v.borrow().clone());    return Ok(Box::new(ConvertScalarToMat2{arg: v, out: Ref::new(out)}));},
                #[cfg(feature = "matrixd")]
                [n,m] => {let out = DMatrix::from_element(n,m,v.borrow().clone());  return Ok(Box::new(ConvertScalarToMat2{arg: v, out: Ref::new(out)}));},
                [] => {return Err(MechError::new(
                  CannotReshapeMatrixToEmpty,
                  None
                ).with_compiler_loc());}
                _ => todo!(),
              }
            }
          )+
        )+
        x => Err(MechError::new(
            UnsupportedConversionError{from: x.0.kind(), to: x.1.clone()},
            None
          ).with_compiler_loc()
        ),
      }
    }
  }
}

fn impl_conversion_scalar_to_mat_fxn(
    source_value: LegacyValue,
    target_kind: ValueKind,
) -> MResult<Box<dyn MechFunction>> {
    impl_conversion_scalar_to_mat_match_arms!(
      (source_value, target_kind),
      Bool => Bool, "bool";
      U8 => U8, "u8";
      U16 => U16, "u16";
      U32 => U32, "u32";
      U64 => U64, "u64";
      U128 => U128, "u128";
      I8 => I8, "i8";
      I16 => I16, "i16";
      I32 => I32, "i32";
      I64 => I64, "i64";
      I128 => I128, "i128";
      F32 => F32, "f32";
      F64 => F64, "f64";
      String => String, "string";
      R64 => R64, "rational";
      C64 => C64, "complex";
    )
}

pub struct ConvertScalarToMat {}

impl FunctionSpecializer for ConvertScalarToMat {
    fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
        if arguments.len() != 2 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 1,
                    found: arguments.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let source_value = arguments[0].clone();
        let source_kind = source_value.kind();
        let target_kind = arguments[1].kind();
        match impl_conversion_scalar_to_mat_fxn(source_value.clone(), target_kind.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match source_value {
                LegacyValue::MutableReference(rhs) => {
                    impl_conversion_scalar_to_mat_fxn(rhs.borrow().clone(), target_kind.clone())
                }
                x => Err(MechError::new(
                    UnhandledFunctionArgumentKind2 {
                        arg: (arguments[0].kind(), arguments[1].kind()),
                        fxn_name: "convert/scalar-to-mat".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct CannotReshapeMatrixToEmpty;

impl MechErrorKind for CannotReshapeMatrixToEmpty {
    fn name(&self) -> &str {
        "CannotReshapeMatrixToEmpty"
    }
    fn message(&self) -> String {
        "Cannot reshape matrix to empty dimensions".to_string()
    }
}

use crate::*;

paste! {
    const _: Option<&dyn Display> = None;
}

use std::sync::LazyLock;

#[cfg(feature = "matrix")]
use nalgebra::{
    Dim,
    base::{Matrix as naMatrix, Storage, StorageMut},
};

#[cfg(all(
    feature = "add_assign",
    any(feature = "matrix", feature = "source")
))]
pub mod add_assign;
#[cfg(all(
    feature = "div_assign",
    any(feature = "matrix", feature = "source")
))]
pub mod div_assign;
#[cfg(all(
    feature = "mul_assign",
    any(feature = "matrix", feature = "source")
))]
pub mod mul_assign;
#[cfg(all(
    feature = "sub_assign",
    any(feature = "matrix", feature = "source")
))]
pub mod sub_assign;

#[cfg(feature = "add_assign")]
pub use self::add_assign::*;
#[cfg(feature = "div_assign")]
pub use self::div_assign::*;
#[cfg(feature = "mul_assign")]
pub use self::mul_assign::*;
#[cfg(feature = "sub_assign")]
pub use self::sub_assign::*;

#[cfg(test)]
mod port_tests;

pub trait RuntimeCheckedOpAssign: Copy {
    fn runtime_checked_add(self, rhs: Self) -> Option<Self>;
    fn runtime_checked_sub(self, rhs: Self) -> Option<Self>;
    fn runtime_checked_mul(self, rhs: Self) -> Option<Self>;
    fn runtime_checked_div(self, rhs: Self) -> Option<Self>;
}

macro_rules! impl_checked_integer_op_assign {
    ($($type:ty),+ $(,)?) => {
        $(
            impl RuntimeCheckedOpAssign for $type {
                fn runtime_checked_add(self, rhs: Self) -> Option<Self> { self.checked_add(rhs) }
                fn runtime_checked_sub(self, rhs: Self) -> Option<Self> { self.checked_sub(rhs) }
                fn runtime_checked_mul(self, rhs: Self) -> Option<Self> { self.checked_mul(rhs) }
                fn runtime_checked_div(self, rhs: Self) -> Option<Self> { self.checked_div(rhs) }
            }
        )+
    };
}

impl_checked_integer_op_assign!(i8, i16, i32, i64, i128, u8, u16, u32, u64, u128);

macro_rules! impl_ieee_op_assign {
    ($($type:ty),+ $(,)?) => {
        $(
            impl RuntimeCheckedOpAssign for $type {
                fn runtime_checked_add(self, rhs: Self) -> Option<Self> { Some(self + rhs) }
                fn runtime_checked_sub(self, rhs: Self) -> Option<Self> { Some(self - rhs) }
                fn runtime_checked_mul(self, rhs: Self) -> Option<Self> { Some(self * rhs) }
                fn runtime_checked_div(self, rhs: Self) -> Option<Self> { Some(self / rhs) }
            }
        )+
    };
}

impl_ieee_op_assign!(f32, f64);
#[cfg(feature = "complex")]
impl_ieee_op_assign!(C64);

#[cfg(feature = "rational")]
impl RuntimeCheckedOpAssign for R64 {
    fn runtime_checked_add(self, rhs: Self) -> Option<Self> { self.checked_add(rhs) }
    fn runtime_checked_sub(self, rhs: Self) -> Option<Self> { self.checked_sub(rhs) }
    fn runtime_checked_mul(self, rhs: Self) -> Option<Self> { self.checked_mul(rhs) }
    fn runtime_checked_div(self, rhs: Self) -> Option<Self> { self.checked_div(rhs) }
}

macro_rules! checked_op_assign {
    ($name:ident, $method:ident, $operation:literal) => {
        fn $name<T: RuntimeCheckedOpAssign>(lhs: T, rhs: T) -> MResult<T> {
            lhs.$method(rhs)
                .ok_or_else(|| arithmetic_overflow::<T>($operation))
        }
    };
}

#[cfg(feature = "add_assign")]
checked_op_assign!(checked_add_assign, runtime_checked_add, "addition assignment");
#[cfg(feature = "sub_assign")]
checked_op_assign!(checked_sub_assign, runtime_checked_sub, "subtraction assignment");
#[cfg(feature = "mul_assign")]
checked_op_assign!(checked_mul_assign, runtime_checked_mul, "multiplication assignment");
#[cfg(feature = "div_assign")]
checked_op_assign!(checked_div_assign, runtime_checked_div, "division assignment");

static PURE_WHOLE_VALUE_RMW_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
            ]
            .into_boxed_slice(),
        ),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::ReadWrite,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::ReadModifyWrite {
                base_input: 0,
                regions: RegionPolicy::WholeValue,
            },
            alias: AliasPolicy::MayAlias { input: 0 },
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    });

#[cfg(feature = "matrix")]
static PURE_INDEXED_AXIS_ZERO_RMW_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
            ]
            .into_boxed_slice(),
        ),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::ReadWrite,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::ReadModifyWrite {
                base_input: 0,
                regions: RegionPolicy::IndexedAxis { axis: 0 },
            },
            alias: AliasPolicy::MayAlias { input: 0 },
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    });

#[cfg(feature = "matrix")]
fn checked_one_based_index(index: usize, len: usize) -> MResult<usize> {
    if index == 0 || index > len {
        return Err(function_shape_contract_violation(
            "op_assign_slice",
            format!("index {index} is outside the valid 1..={len} range"),
        ));
    }
    Ok(index - 1)
}

#[cfg(feature = "matrix")]
fn validate_mask_len(mask_len: usize, sink_len: usize) -> MResult<()> {
    if mask_len > sink_len {
        return Err(function_shape_contract_violation(
            "op_assign_slice",
            format!("boolean index has {mask_len} elements, output has {sink_len}"),
        ));
    }
    Ok(())
}

#[cfg(feature = "matrix")]
fn validate_source_len(source_len: usize, selected_len: usize) -> MResult<()> {
    if source_len < selected_len {
        return Err(function_shape_contract_violation(
            "op_assign_slice",
            format!("source has {source_len} elements, selection requires {selected_len}"),
        ));
    }
    Ok(())
}

#[macro_export]
macro_rules! impl_op_assign_range_fxn_s {
    ($struct_name:ident, $op:ident, $ix:ty) => {
        #[derive(Debug)]
        pub struct $struct_name<T, MatA, IxVec> {
            pub source: Ref<T>,
            pub ixes: Ref<IxVec>,
            pub sink: Ref<MatA>,
            pub _marker: PhantomData<T>,
        }
        impl<T, R1: 'static, C1: 'static, S1: 'static, IxVec: 'static> MechFunctionFactory
            for $struct_name<T, naMatrix<T, R1, C1, S1>, IxVec>
        where
            Ref<naMatrix<T, R1, C1, S1>>: ToValue,
            T: Copy
                + Debug
                + Clone
                + Sync
                + Send
                + 'static
                + Div<Output = T>
                + DivAssign
                + Add<Output = T>
                + AddAssign
                + Sub<Output = T>
                + SubAssign
                + Mul<Output = T>
                + MulAssign
                + Zero
                + One
                + PartialEq
                + PartialOrd
                + AsValueKind,
            T: RuntimeCheckedOpAssign,
            #[cfg(feature = "semantic-compiler")]
            T: CompileConst + ConstElem,
            IxVec: Debug + AsRef<[$ix]> + AsNaKind,
            #[cfg(feature = "semantic-compiler")]
            IxVec: CompileConst + ConstElem,
            R1: Dim,
            C1: Dim,
            S1: StorageMut<T, R1, C1> + Clone + Debug,
            naMatrix<T, R1, C1, S1>: Debug + AsNaKind,
            naMatrix<T, R1, C1, S1>: FunctionRuntimeType,
            IxVec: FunctionRuntimeType,
            T: FunctionRuntimeType,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R1, C1, S1>: CompileConst + ConstElem,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                <naMatrix<T, R1, C1, S1> as FunctionRuntimeType>::REPRESENTATION,
                T::REPRESENTATION,
                IxVec::REPRESENTATION,
            );

            fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
                match args {
                    FunctionArgs::Binary(out, arg1, arg2) => {
                        let source: Ref<T> =
                            arg1.try_function_ref(FunctionArgumentRole::Input(0))?;
                        let ixes: Ref<IxVec> =
                            arg2.try_function_ref(FunctionArgumentRole::Input(1))?;
                        let sink: Ref<naMatrix<T, R1, C1, S1>> =
                            out.try_function_ref(FunctionArgumentRole::Output)?;
                        Ok(Box::new(Self {
                            sink,
                            source,
                            ixes,
                            _marker: PhantomData::default(),
                        }))
                    }
                    _ => Err(MechError::new(
                        IncorrectNumberOfArguments {
                            expected: 3,
                            found: args.len(),
                        },
                        None,
                    )
                    .with_compiler_loc()),
                }
            }
        }
        impl<T, R1, C1, S1, IxVec> MechFunctionImpl
            for $struct_name<T, naMatrix<T, R1, C1, S1>, IxVec>
        where
            Ref<naMatrix<T, R1, C1, S1>>: ToValue,
            T: Copy
                + Debug
                + Clone
                + Sync
                + Send
                + 'static
                + Div<Output = T>
                + DivAssign
                + Add<Output = T>
                + AddAssign
                + Sub<Output = T>
                + SubAssign
                + Mul<Output = T>
                + MulAssign
                + Zero
                + One
                + PartialEq
                + PartialOrd,
            T: RuntimeCheckedOpAssign,
            IxVec: AsRef<[$ix]> + Debug,
            R1: Dim,
            C1: Dim,
            S1: StorageMut<T, R1, C1> + Clone + Debug,
        {
            fn solve_result(&self) -> MResult<()> {
                unsafe {
                    let sink_ptr = &mut *self.sink.as_mut_ptr();
                    let source_ptr = &*self.source.as_ptr();
                    let ix_ptr = &(*self.ixes.as_ptr()).as_ref();
                    let mut next = sink_ptr.clone();
                    $op!(source_ptr, ix_ptr, &mut next)?;
                    *sink_ptr = next;
                };
                Ok(())
            }
            fn out(&self) -> LegacyValue {
                self.sink.to_value()
            }
            fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
                Some(&PURE_INDEXED_AXIS_ZERO_RMW_CONTRACT)
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }

            fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
                Ok(self.reactive_output_values())
            }
        }
        #[cfg(feature = "semantic-compiler")]
        impl<T, R1, C1, S1, IxVec> MechFunctionCompiler
            for $struct_name<T, naMatrix<T, R1, C1, S1>, IxVec>
        where
            T: CompileConst + ConstElem + AsValueKind,
            IxVec: CompileConst + ConstElem + AsNaKind,
            naMatrix<T, R1, C1, S1>: CompileConst + ConstElem + AsNaKind,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}{}{}>",
                    stringify!($struct_name),
                    T::as_value_kind(),
                    naMatrix::<T, R1, C1, S1>::as_na_kind(),
                    IxVec::as_na_kind()
                );
                compile_binop!(name, self.sink, self.source, self.ixes, ctx);
            }
        }
    };
}

#[macro_export]
macro_rules! impl_op_assign_range_fxn_v {
    ($struct_name:ident, $op:ident, $ix:ty) => {
        #[cfg(feature = "matrix")]
        #[derive(Debug)]
        pub struct $struct_name<T, MatA, MatB, IxVec> {
            pub source: Ref<MatB>,
            pub ixes: Ref<IxVec>,
            pub sink: Ref<MatA>,
            pub _marker: PhantomData<T>,
        }
        impl<
            T,
            R1: 'static,
            C1: 'static,
            S1: 'static,
            R2: 'static,
            C2: 'static,
            S2: 'static,
            IxVec: 'static,
        > MechFunctionFactory
            for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec>
        where
            Ref<naMatrix<T, R1, C1, S1>>: ToValue,
            Ref<naMatrix<T, R2, C2, S2>>: ToValue,
            T: Copy
                + Debug
                + Clone
                + Sync
                + Send
                + 'static
                + Div<Output = T>
                + DivAssign
                + Add<Output = T>
                + AddAssign
                + Sub<Output = T>
                + SubAssign
                + Mul<Output = T>
                + MulAssign
                + Zero
                + One
                + PartialEq
                + PartialOrd
                + AsValueKind,
            T: RuntimeCheckedOpAssign,
            #[cfg(feature = "semantic-compiler")]
            T: CompileConst + ConstElem,
            IxVec: AsNaKind + Debug + AsRef<[$ix]>,
            #[cfg(feature = "semantic-compiler")]
            IxVec: CompileConst + ConstElem,
            R1: Dim,
            C1: Dim,
            S1: StorageMut<T, R1, C1> + Clone + Debug,
            R2: Dim,
            C2: Dim,
            S2: Storage<T, R2, C2> + Clone + Debug,
            naMatrix<T, R1, C1, S1>: Debug + AsNaKind,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R1, C1, S1>: CompileConst + ConstElem,
            naMatrix<T, R2, C2, S2>: Debug + AsNaKind,
            naMatrix<T, R1, C1, S1>: FunctionRuntimeType,
            naMatrix<T, R2, C2, S2>: FunctionRuntimeType,
            IxVec: FunctionRuntimeType,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R2, C2, S2>: CompileConst + ConstElem,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                <naMatrix<T, R1, C1, S1> as FunctionRuntimeType>::REPRESENTATION,
                <naMatrix<T, R2, C2, S2> as FunctionRuntimeType>::REPRESENTATION,
                IxVec::REPRESENTATION,
            );

            fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
                match args {
                    FunctionArgs::Binary(out, arg1, arg2) => {
                        let source: Ref<naMatrix<T, R2, C2, S2>> =
                            arg1.try_function_ref(FunctionArgumentRole::Input(0))?;
                        let ixes: Ref<IxVec> =
                            arg2.try_function_ref(FunctionArgumentRole::Input(1))?;
                        let sink: Ref<naMatrix<T, R1, C1, S1>> =
                            out.try_function_ref(FunctionArgumentRole::Output)?;
                        Ok(Box::new(Self {
                            sink,
                            source,
                            ixes,
                            _marker: PhantomData::default(),
                        }))
                    }
                    _ => Err(MechError::new(
                        IncorrectNumberOfArguments {
                            expected: 3,
                            found: args.len(),
                        },
                        None,
                    )
                    .with_compiler_loc()),
                }
            }
        }
        impl<T, R1, C1, S1, R2, C2, S2, IxVec> MechFunctionImpl
            for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec>
        where
            Ref<naMatrix<T, R1, C1, S1>>: ToValue,
            T: Copy
                + Debug
                + Clone
                + Sync
                + Send
                + 'static
                + Div<Output = T>
                + DivAssign
                + Add<Output = T>
                + AddAssign
                + Sub<Output = T>
                + SubAssign
                + Mul<Output = T>
                + MulAssign
                + Zero
                + One
                + PartialEq
                + PartialOrd,
            T: RuntimeCheckedOpAssign,
            IxVec: AsRef<[$ix]> + Debug,
            R1: Dim,
            C1: Dim,
            S1: StorageMut<T, R1, C1> + Clone + Debug,
            R2: Dim,
            C2: Dim,
            S2: Storage<T, R2, C2> + Clone + Debug,
        {
            fn solve_result(&self) -> MResult<()> {
                unsafe {
                    let sink_ptr = &mut *self.sink.as_mut_ptr();
                    let source_ptr = &*self.source.as_ptr();
                    let ix_ptr = &(*self.ixes.as_ptr()).as_ref();
                    let mut next = sink_ptr.clone();
                    $op!(source_ptr, ix_ptr, &mut next)?;
                    *sink_ptr = next;
                };
                Ok(())
            }
            fn out(&self) -> LegacyValue {
                self.sink.to_value()
            }
            fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
                Some(&PURE_INDEXED_AXIS_ZERO_RMW_CONTRACT)
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }

            fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
                Ok(self.reactive_output_values())
            }
        }
        #[cfg(feature = "semantic-compiler")]
        impl<T, R1, C1, S1, R2, C2, S2, IxVec> MechFunctionCompiler
            for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec>
        where
            T: CompileConst + ConstElem + AsValueKind,
            IxVec: CompileConst + ConstElem + AsNaKind,
            naMatrix<T, R1, C1, S1>: CompileConst + ConstElem + AsNaKind,
            naMatrix<T, R2, C2, S2>: CompileConst + ConstElem + AsNaKind,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}{}{}{}>",
                    stringify!($struct_name),
                    T::as_value_kind(),
                    naMatrix::<T, R1, C1, S1>::as_na_kind(),
                    naMatrix::<T, R2, C2, S2>::as_na_kind(),
                    IxVec::as_na_kind()
                );
                compile_binop!(name, self.sink, self.source, self.ixes, ctx);
            }
        }
    };
}

//impl_set_range_arms
#[cfg(feature = "source")]
#[macro_export]
macro_rules! op_assign_range_fxn {
  ($op_fxn_name:tt, $fxn_name:ident) => {
    paste::paste! {
      fn $op_fxn_name(sink: LegacyValue, source: LegacyValue, ixes: Vec<LegacyValue>) -> MResult<Box<dyn MechFunction>> {
        let arg = (sink.clone(), ixes.as_slice(), source.clone());
                     impl_assign_fxn!(impl_set_range_arms, $fxn_name, arg, u8, "u8")
        .or_else(|_| impl_assign_fxn!(impl_set_range_arms, $fxn_name, arg, u16, "u16"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_arms, $fxn_name, arg, u32, "u32"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_arms, $fxn_name, arg, u64, "u64"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_arms, $fxn_name, arg, u128, "u128"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_arms, $fxn_name, arg, i8, "i8"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_arms, $fxn_name, arg, i16, "i16"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_arms, $fxn_name, arg, i32, "i32"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_arms, $fxn_name, arg, i64, "i64"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_arms, $fxn_name, arg, f32, "f32"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_arms, $fxn_name, arg, f64, "f64"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_arms, $fxn_name, arg, R64, "rational"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_arms, $fxn_name, arg, C64, "complex"))
        .map_err(|_| MechError::new(
            UnhandledFunctionArgumentIxes { arg: (sink.kind(), ixes.iter().map(|x| x.kind()).collect(), source.kind()), fxn_name: stringify!($fxn_name).to_string() },
            None
          ).with_compiler_loc()
        )
      }
    }
  }
}

#[cfg(feature = "source")]
#[macro_export]
macro_rules! op_assign_range_all_fxn {
  ($op_fxn_name:tt, $fxn_name:ident) => {
    paste::paste! {
      fn $op_fxn_name(sink: LegacyValue, source: LegacyValue, ixes: Vec<LegacyValue>) -> MResult<Box<dyn MechFunction>> {
        let arg = (sink.clone(), ixes.as_slice(), source.clone());
                     impl_assign_fxn!(impl_set_range_all_arms, $fxn_name, arg, u8, "u8")
        .or_else(|_| impl_assign_fxn!(impl_set_range_all_arms, $fxn_name, arg, u16, "u16"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_all_arms, $fxn_name, arg, u32, "u32"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_all_arms, $fxn_name, arg, u64, "u64"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_all_arms, $fxn_name, arg, u128, "u128"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_all_arms, $fxn_name, arg, i8, "i8"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_all_arms, $fxn_name, arg, i16, "i16"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_all_arms, $fxn_name, arg, i32, "i32"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_all_arms, $fxn_name, arg, i64, "i64"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_all_arms, $fxn_name, arg, f32, "f32"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_all_arms, $fxn_name, arg, f64, "f64"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_all_arms, $fxn_name, arg, R64, "rational"))
        .or_else(|_| impl_assign_fxn!(impl_set_range_all_arms, $fxn_name, arg, C64, "complex"))
        .map_err(|_| MechError::new(
            UnhandledFunctionArgumentIxes { arg: (sink.kind(), ixes.iter().map(|x| x.kind()).collect(), source.kind()), fxn_name: stringify!($fxn_name).to_string() },
            None
          ).with_compiler_loc()
        )
      }
    }
  }
}

#[macro_export]
macro_rules! impl_assign_scalar_scalar {
  ($op_name:tt, $checked_op:ident) => {
    paste::paste! {
      #[derive(Debug)]
      pub(crate) struct [<$op_name AssignSS>]<T> {
        sink: Ref<T>,
        source: Ref<T>,
      }
      impl<T> MechFunctionFactory for [<$op_name AssignSS>]<T>
      where
        T: Debug + Clone + Sync + Send + 'static +
           $op_name<Output = T> + [<$op_name Assign>] +
           PartialEq + PartialOrd + AsValueKind,
        T: RuntimeCheckedOpAssign,
        #[cfg(feature = "semantic-compiler")]
        T: CompileConst + ConstElem,
        Ref<T>: ToValue,
        T: FunctionStateBacking,
      {
        const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::unary(
          T::REPRESENTATION,
          T::REPRESENTATION,
        );

        fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
          let (sink, source) = invocation.expect_unary()?;
          let source: Ref<T> = source.try_ref()?;
          let sink: Ref<T> = sink.try_ref()?;
          Ok(Box::new(Self { sink, source }))
        }

        fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
          Self::new_invocation(args.into())
        }
      }
      impl<T> MechFunctionImpl for [<$op_name AssignSS>]<T>
      where
        T: Debug + Clone + Sync + Send + 'static +
           $op_name<Output = T> + [<$op_name Assign>] +
           PartialEq + PartialOrd,
        T: RuntimeCheckedOpAssign,
        Ref<T>: ToValue,
        T: FunctionStateBacking,
      {
        fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
          Some(FunctionStatePort::from_ref(&self.sink))
        }
        fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
          Ok(Some(vec![FunctionStatePort::from_ref(&self.sink)]))
        }
        fn solve_result(&self) -> MResult<()> {
          let next = $checked_op(*self.sink.borrow(), *self.source.borrow())?;
          *self.sink.borrow_mut() = next;
          Ok(())
        }
        fn stage_register(&self) -> MResult<Box<dyn ReactiveRegisterCommit>> {
          let next = $checked_op(*self.sink.borrow(), *self.source.borrow())?;
          Ok(Box::new(ReactiveRegisterWrite::new(self.sink.clone(), next, self.reactive_output_cell_ids())))
        }
        fn out(&self) -> LegacyValue { self.sink.to_value() }
        fn reactive_node_kind(&self) -> ReactiveNodeKind { ReactiveNodeKind::Register }
        fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
          Some(&PURE_WHOLE_VALUE_RMW_CONTRACT)
        }
        fn to_string(&self) -> String { format!("{:#?}", self) }

        fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
          Ok(self.reactive_output_values())
        }
      }
      #[cfg(feature = "semantic-compiler")]
      impl<T> MechFunctionCompiler for [<$op_name AssignSS>]<T>
      where
        T: CompileConst + ConstElem + AsValueKind,
      {
        fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
          let name = format!("{}AssignSS<{}>", stringify!($op_name), T::as_value_kind());
          compile_unop!(name, self.sink, self.source, ctx );
        }
      }
    }
  };
}

#[macro_export]
macro_rules! impl_assign_vector_vector {
  ($op_name:tt, $checked_op:ident) => {
    paste::paste! {
      #[derive(Debug)]
      pub struct [<$op_name AssignVV>]<T, MatA, MatB> {
        pub sink: Ref<MatA>,
        pub source: Ref<MatB>,
        _marker: PhantomData<T>,
      }
      impl<T, MatA, MatB> MechFunctionFactory for [<$op_name AssignVV>]<T, MatA, MatB>
      where
        Ref<MatA>: ToValue,
        T: Debug + Clone + Sync + Send + 'static + [<$op_name Assign>] +
        AsValueKind,
        T: RuntimeCheckedOpAssign,
        #[cfg(feature = "semantic-compiler")]
        T: CompileConst + ConstElem,
        for<'a> &'a MatA: IntoIterator<Item = &'a T>,
        for<'a> &'a mut MatA: IntoIterator<Item = &'a mut T>,
        for<'a> &'a MatB: IntoIterator<Item = &'a T>,
        MatA: Debug + Clone + AsValueKind + 'static,
        #[cfg(feature = "semantic-compiler")]
        MatA: CompileConst + ConstElem,
        MatB: Debug + AsValueKind + 'static,
        MatA: FunctionStateBacking,
        MatB: FunctionPortBacking,
        #[cfg(feature = "semantic-compiler")]
        MatB: CompileConst + ConstElem,
      {
        const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::unary(
          MatA::REPRESENTATION,
          MatB::REPRESENTATION,
        );

        fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
          let (sink, source) = invocation.expect_unary()?;
          let source: Ref<MatB> = source.try_ref()?;
          let sink: Ref<MatA> = sink.try_ref()?;
          Ok(Box::new(Self { sink, source, _marker: PhantomData::default() }))
        }

        fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
          Self::new_invocation(args.into())
        }
      }
      impl<T, MatA, MatB> MechFunctionImpl for [<$op_name AssignVV>]<T, MatA, MatB>
      where
        Ref<MatA>: ToValue,
        T: Debug + Clone + Sync + Send + 'static + [<$op_name Assign>],
        T: RuntimeCheckedOpAssign,
        for<'a> &'a MatA: IntoIterator<Item = &'a T>,
        for<'a> &'a mut MatA: IntoIterator<Item = &'a mut T>,
        for<'a> &'a MatB: IntoIterator<Item = &'a T>,
        MatA: Debug + Clone + FunctionStateBacking + 'static,
        MatB: Debug,
      {
        fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
          Some(FunctionStatePort::from_ref(&self.sink))
        }
        fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
          Ok(Some(vec![FunctionStatePort::from_ref(&self.sink)]))
        }
        fn solve_result(&self) -> MResult<()> {
          let mut next = self.sink.borrow().clone();
          {
            let source = self.source.borrow();
            for (dst, src) in (&mut next).into_iter().zip((&*source).into_iter()) {
              *dst = $checked_op(*dst, *src)?;
            }
          }
          *self.sink.borrow_mut() = next;
          Ok(())
        }
        fn stage_register(&self) -> MResult<Box<dyn ReactiveRegisterCommit>> {
          let mut next = self.sink.borrow().clone();
          {
            let source = self.source.borrow();
            for (dst, src) in (&mut next).into_iter().zip((&*source).into_iter()) {
              *dst = $checked_op(*dst, *src)?;
            }
          }
          Ok(Box::new(ReactiveRegisterWrite::new(self.sink.clone(), next, self.reactive_output_cell_ids())))
        }
        fn out(&self) -> LegacyValue {self.sink.to_value()}
        fn reactive_node_kind(&self) -> ReactiveNodeKind { ReactiveNodeKind::Register }
        fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
          Some(&PURE_WHOLE_VALUE_RMW_CONTRACT)
        }
        fn to_string(&self) -> String {format!("{:#?}", self)}

        fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
          Ok(self.reactive_output_values())
        }
      }
      #[cfg(feature = "semantic-compiler")]
      impl<T, MatA, MatB> MechFunctionCompiler for [<$op_name AssignVV>]<T, MatA, MatB>
      where
        T: CompileConst + ConstElem + AsValueKind,
        MatA: CompileConst + ConstElem + AsValueKind,
        MatB: CompileConst + ConstElem + AsValueKind,
      {
        fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
          let name = format!("{}AssignVV<{}>", stringify!($op_name), MatA::as_value_kind());
          compile_unop!(name, self.sink, self.source, ctx );
        }
      }
    }
  };
}

#[macro_export]
macro_rules! impl_assign_vector_scalar {
  ($op_name:tt, $checked_op:ident) => {
    paste::paste! {
      #[derive(Debug)]
      pub struct [<$op_name AssignVS>]<T, MatA> {
        pub sink: Ref<MatA>,
        pub source: Ref<T>,
        _marker: PhantomData<T>,
      }
      impl<T, MatA> MechFunctionFactory for [<$op_name AssignVS>]<T, MatA>
      where
        Ref<MatA>: ToValue,
        T: Debug + Clone + Sync + Send + 'static + [<$op_name Assign>] +
        AsValueKind,
        T: RuntimeCheckedOpAssign,
        #[cfg(feature = "semantic-compiler")]
        T: CompileConst + ConstElem,
        for<'a> &'a MatA: IntoIterator<Item = &'a T>,
        for<'a> &'a mut MatA: IntoIterator<Item = &'a mut T>,
        MatA: Debug + Clone + AsValueKind + 'static,
        MatA: FunctionStateBacking,
        T: FunctionPortBacking,
        #[cfg(feature = "semantic-compiler")]
        MatA: CompileConst + ConstElem,
      {
        const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
          MatA::REPRESENTATION,
          FunctionValueRepresentation::AnyValue,
          T::REPRESENTATION,
        );

        fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
          let (sink, _base, source) = invocation.expect_binary()?;
          let source: Ref<T> = source.try_ref()?;
          let sink: Ref<MatA> = sink.try_ref()?;
          Ok(Box::new(Self { sink, source, _marker: PhantomData::default() }))
        }

        fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
          Self::new_invocation(args.into())
        }
      }
      impl<T, MatA> MechFunctionImpl for [<$op_name AssignVS>]<T, MatA>
      where
        Ref<MatA>: ToValue,
        T: Debug + Clone + Sync + Send + 'static + [<$op_name Assign>],
        T: RuntimeCheckedOpAssign,
        for<'a> &'a MatA: IntoIterator<Item = &'a T>,
        for<'a> &'a mut MatA: IntoIterator<Item = &'a mut T>,
        MatA: Debug + Clone + FunctionStateBacking + 'static,
      {
        fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
          Some(FunctionStatePort::from_ref(&self.sink))
        }
        fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
          Ok(Some(vec![FunctionStatePort::from_ref(&self.sink)]))
        }
        fn solve_result(&self) -> MResult<()> {
          let mut next = self.sink.borrow().clone();
          let source = *self.source.borrow();
          for dst in (&mut next).into_iter() {
            *dst = $checked_op(*dst, source)?;
          }
          *self.sink.borrow_mut() = next;
          Ok(())
        }
        fn stage_register(&self) -> MResult<Box<dyn ReactiveRegisterCommit>> {
          let mut next = self.sink.borrow().clone();
          let source = self.source.borrow().clone();
          for dst in (&mut next).into_iter() {
            *dst = $checked_op(*dst, source)?;
          }
          Ok(Box::new(ReactiveRegisterWrite::new(self.sink.clone(), next, self.reactive_output_cell_ids())))
        }
        fn out(&self) -> LegacyValue {self.sink.to_value()}
        fn reactive_node_kind(&self) -> ReactiveNodeKind { ReactiveNodeKind::Register }
        fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
          Some(&PURE_WHOLE_VALUE_RMW_CONTRACT)
        }
        fn to_string(&self) -> String {format!("{:#?}", self)}

        fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
          Ok(self.reactive_output_values())
        }
      }
      #[cfg(feature = "semantic-compiler")]
      impl<T, MatA> MechFunctionCompiler for [<$op_name AssignVS>]<T, MatA>
      where
        T: CompileConst + ConstElem + AsValueKind,
        MatA: CompileConst + ConstElem + AsValueKind,
      {
        fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
          let name = format!("{}AssignVS<{}>", stringify!($op_name), MatA::as_value_kind());
          compile_unop!(name, self.sink, self.source, ctx );
        }
      }
    }
  }
}

#[cfg(not(any(feature = "matrix", feature = "source")))]
macro_rules! impl_scalar_op_assign_module {
    ($module:ident, $op_name:tt, $checked_op:ident) => {
        pub mod $module {
            use super::*;

            impl_assign_scalar_scalar!($op_name, $checked_op);
            impl_assign_vector_vector!($op_name, $checked_op);
            impl_assign_vector_scalar!($op_name, $checked_op);
        }
    };
}

#[cfg(all(
    feature = "add_assign",
    not(any(feature = "matrix", feature = "source"))
))]
impl_scalar_op_assign_module!(add_assign, Add, checked_add_assign);
#[cfg(all(
    feature = "div_assign",
    not(any(feature = "matrix", feature = "source"))
))]
impl_scalar_op_assign_module!(div_assign, Div, checked_div_assign);
#[cfg(all(
    feature = "mul_assign",
    not(any(feature = "matrix", feature = "source"))
))]
impl_scalar_op_assign_module!(mul_assign, Mul, checked_mul_assign);
#[cfg(all(
    feature = "sub_assign",
    not(any(feature = "matrix", feature = "source"))
))]
impl_scalar_op_assign_module!(sub_assign, Sub, checked_sub_assign);

#[cfg(feature = "source")]
#[macro_export]
macro_rules! impl_op_assign_value_match_arms {
  ($op:tt, $arg:expr,$($value_kind:ident, $feature:tt);+ $(;)?) => {
    paste::paste! {
      match $arg {
        $(
          #[cfg(feature = $feature)]
          (LegacyValue::$value_kind(sink), LegacyValue::$value_kind(source)) => Ok(Box::new([<$op AssignSS>]{ sink: sink.clone(), source: source.clone() })),
          #[cfg(all(feature = $feature, feature = "matrix1"))]
          (LegacyValue::[<Matrix $value_kind>](Matrix::Matrix1(sink)), LegacyValue::$value_kind(source)) => Ok(Box::new([<$op AssignVS>]{sink: sink.clone(), source: source.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $feature, feature = "matrix2"))]
          (LegacyValue::[<Matrix $value_kind>](Matrix::Matrix2(sink)), LegacyValue::$value_kind(source)) => Ok(Box::new([<$op AssignVS>]{sink: sink.clone(), source: source.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $feature, feature = "matrix2x3"))]
          (LegacyValue::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), LegacyValue::$value_kind(source)) => Ok(Box::new([<$op AssignVS>]{sink: sink.clone(), source: source.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $feature, feature = "matrix3x2"))]
          (LegacyValue::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), LegacyValue::$value_kind(source)) => Ok(Box::new([<$op AssignVS>]{sink: sink.clone(), source: source.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $feature, feature = "matrix3"))]
          (LegacyValue::[<Matrix $value_kind>](Matrix::Matrix3(sink)), LegacyValue::$value_kind(source)) => Ok(Box::new([<$op AssignVS>]{sink: sink.clone(), source: source.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $feature, feature = "matrix4"))]
          (LegacyValue::[<Matrix $value_kind>](Matrix::Matrix4(sink)), LegacyValue::$value_kind(source)) => Ok(Box::new([<$op AssignVS>]{sink: sink.clone(), source: source.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $feature, feature = "matrixd"))]
          (LegacyValue::[<Matrix $value_kind>](Matrix::DMatrix(sink)), LegacyValue::$value_kind(source)) => Ok(Box::new([<$op AssignVS>]{sink: sink.clone(), source: source.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $feature, feature = "vector2"))]
          (LegacyValue::[<Matrix $value_kind>](Matrix::Vector2(sink)), LegacyValue::$value_kind(source)) => Ok(Box::new([<$op AssignVS>]{sink: sink.clone(), source: source.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $feature, feature = "vector3"))]
          (LegacyValue::[<Matrix $value_kind>](Matrix::Vector3(sink)), LegacyValue::$value_kind(source)) => Ok(Box::new([<$op AssignVS>]{sink: sink.clone(), source: source.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $feature, feature = "vector4"))]
          (LegacyValue::[<Matrix $value_kind>](Matrix::Vector4(sink)), LegacyValue::$value_kind(source)) => Ok(Box::new([<$op AssignVS>]{sink: sink.clone(), source: source.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $feature, feature = "vectord"))]
          (LegacyValue::[<Matrix $value_kind>](Matrix::DVector(sink)), LegacyValue::$value_kind(source)) => Ok(Box::new([<$op AssignVS>]{sink: sink.clone(), source: source.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $feature, feature = "row_vector2"))]
          (LegacyValue::[<Matrix $value_kind>](Matrix::RowVector2(sink)), LegacyValue::$value_kind(source)) => Ok(Box::new([<$op AssignVS>]{sink: sink.clone(), source: source.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $feature, feature = "row_vector3"))]
          (LegacyValue::[<Matrix $value_kind>](Matrix::RowVector3(sink)), LegacyValue::$value_kind(source)) => Ok(Box::new([<$op AssignVS>]{sink: sink.clone(), source: source.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $feature, feature = "row_vector4"))]
          (LegacyValue::[<Matrix $value_kind>](Matrix::RowVector4(sink)), LegacyValue::$value_kind(source)) => Ok(Box::new([<$op AssignVS>]{sink: sink.clone(), source: source.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $feature, feature = "row_vectord"))]
          (LegacyValue::[<Matrix $value_kind>](Matrix::RowDVector(sink)), LegacyValue::$value_kind(source)) => Ok(Box::new([<$op AssignVS>]{sink: sink.clone(), source: source.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $feature, feature = "matrix1"))]
          (LegacyValue::[<Matrix $value_kind>](Matrix::Matrix1(sink)), LegacyValue::[<Matrix $value_kind>](Matrix::Matrix1(source))) => Ok(Box::new([<$op AssignVV>]{sink: sink.clone(), source: source.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $feature, feature = "matrix2"))]
          (LegacyValue::[<Matrix $value_kind>](Matrix::Matrix2(sink)), LegacyValue::[<Matrix $value_kind>](Matrix::Matrix2(source))) => Ok(Box::new([<$op AssignVV>]{sink: sink.clone(), source: source.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $feature, feature = "matrix2x3"))]
          (LegacyValue::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), LegacyValue::[<Matrix $value_kind>](Matrix::Matrix2x3(source))) => Ok(Box::new([<$op AssignVV>]{sink: sink.clone(), source: source.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $feature, feature = "matrix3x2"))]
          (LegacyValue::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), LegacyValue::[<Matrix $value_kind>](Matrix::Matrix3x2(source))) => Ok(Box::new([<$op AssignVV>]{sink: sink.clone(), source: source.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $feature, feature = "matrix3"))]
          (LegacyValue::[<Matrix $value_kind>](Matrix::Matrix3(sink)), LegacyValue::[<Matrix $value_kind>](Matrix::Matrix3(source))) => Ok(Box::new([<$op AssignVV>]{sink: sink.clone(), source: source.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $feature, feature = "matrix4"))]
          (LegacyValue::[<Matrix $value_kind>](Matrix::Matrix4(sink)), LegacyValue::[<Matrix $value_kind>](Matrix::Matrix4(source))) => Ok(Box::new([<$op AssignVV>]{sink: sink.clone(), source: source.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $feature, feature = "matrixd"))]
          (LegacyValue::[<Matrix $value_kind>](Matrix::DMatrix(sink)), LegacyValue::[<Matrix $value_kind>](Matrix::DMatrix(source))) => Ok(Box::new([<$op AssignVV>]{sink: sink.clone(), source: source.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $feature, feature = "vector2"))]
          (LegacyValue::[<Matrix $value_kind>](Matrix::Vector2(sink)), LegacyValue::[<Matrix $value_kind>](Matrix::Vector2(source))) => Ok(Box::new([<$op AssignVV>]{sink: sink.clone(), source: source.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $feature, feature = "vector3"))]
          (LegacyValue::[<Matrix $value_kind>](Matrix::Vector3(sink)), LegacyValue::[<Matrix $value_kind>](Matrix::Vector3(source))) => Ok(Box::new([<$op AssignVV>]{sink: sink.clone(), source: source.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $feature, feature = "vector4"))]
          (LegacyValue::[<Matrix $value_kind>](Matrix::Vector4(sink)), LegacyValue::[<Matrix $value_kind>](Matrix::Vector4(source))) => Ok(Box::new([<$op AssignVV>]{sink: sink.clone(), source: source.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $feature, feature = "vectord"))]
          (LegacyValue::[<Matrix $value_kind>](Matrix::DVector(sink)), LegacyValue::[<Matrix $value_kind>](Matrix::DVector(source))) => Ok(Box::new([<$op AssignVV>]{sink: sink.clone(), source: source.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $feature, feature = "row_vector2"))]
          (LegacyValue::[<Matrix $value_kind>](Matrix::RowVector2(sink)), LegacyValue::[<Matrix $value_kind>](Matrix::RowVector2(source))) => Ok(Box::new([<$op AssignVV>]{sink: sink.clone(), source: source.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $feature, feature = "row_vector3"))]
          (LegacyValue::[<Matrix $value_kind>](Matrix::RowVector3(sink)), LegacyValue::[<Matrix $value_kind>](Matrix::RowVector3(source))) => Ok(Box::new([<$op AssignVV>]{sink: sink.clone(), source: source.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $feature, feature = "row_vector4"))]
          (LegacyValue::[<Matrix $value_kind>](Matrix::RowVector4(sink)), LegacyValue::[<Matrix $value_kind>](Matrix::RowVector4(source))) => Ok(Box::new([<$op AssignVV>]{sink: sink.clone(), source: source.clone(), _marker: PhantomData::default()})),
          #[cfg(all(feature = $feature, feature = "row_vectord"))]
          (LegacyValue::[<Matrix $value_kind>](Matrix::RowDVector(sink)), LegacyValue::[<Matrix $value_kind>](Matrix::RowDVector(source))) => Ok(Box::new([<$op AssignVV>]{sink: sink.clone(), source: source.clone(), _marker: PhantomData::default()})),
        )+
        (arg1,arg2) => Err(MechError::new(
            UnhandledFunctionArgumentKind2 { arg: (arg1.kind(),arg2.kind()), fxn_name: stringify!($op).to_string() },
            None
          ).with_compiler_loc()
        ),
      }
    }
  };
}

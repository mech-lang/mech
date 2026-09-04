use crate::intrinsics::*;
use std::marker::PhantomData;

#[cfg(all(
    feature = "variable_define",
    any(feature = "semantic-compiler", feature = "source")
))]
pub(crate) static PURE_VARIABLE_DEFINITION_CONTRACT: std::sync::LazyLock<
    OperationContractDeclaration,
> = std::sync::LazyLock::new(|| OperationContractDeclaration {
    inputs: InputPortLayout::Fixed(Box::new([])),
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

#[cfg(all(
    feature = "variable_define",
    any(
        feature = "semantic-compiler",
        feature = "table",
        feature = "set",
        feature = "tuple",
        feature = "record",
        feature = "map",
        feature = "atom",
        feature = "enum"
    )
))]
pub(crate) struct CanonicalVariableDefinition {
    pub(crate) value: ValueCell,
    #[cfg(feature = "semantic-compiler")]
    pub(crate) initial: mech_core::Value,
    #[cfg(feature = "semantic-compiler")]
    pub(crate) name: String,
    #[cfg(feature = "semantic-compiler")]
    pub(crate) mutable: bool,
    #[cfg(feature = "semantic-compiler")]
    pub(crate) root_visible: bool,
}

#[cfg(all(
    feature = "variable_define",
    any(
        feature = "semantic-compiler",
        feature = "table",
        feature = "set",
        feature = "tuple",
        feature = "record",
        feature = "map",
        feature = "atom",
        feature = "enum"
    )
))]
impl MechFunctionImpl for CanonicalVariableDefinition {
    fn solve_result(&self) -> MResult<()> {
        Ok(())
    }

    fn reactive_node_kind(&self) -> ReactiveNodeKind {
        ReactiveNodeKind::Combinational
    }

    fn semantic_operation_name(&self) -> Option<&str> {
        Some("var/define")
    }

    fn reactive_output_value_cells(&self) -> Vec<ValueCell> {
        vec![self.value.clone()]
    }

    fn to_string(&self) -> String {
        "VariableDefineCanonical".to_owned()
    }
}

#[cfg(all(feature = "variable_define", feature = "semantic-compiler"))]
impl MechFunctionCompiler for CanonicalVariableDefinition {
    fn compiler_owned_value_cells(&self) -> Vec<ValueCell> {
        vec![self.value.clone()]
    }

    fn reserve_bytecode_registers(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<()> {
        if self.mutable {
            compile_value_cell_initializer_register(&self.value, &self.initial, context)?;
        }
        Ok(())
    }

    fn compile(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let register = compile_value_cell_register(&self.value, context)?;
        let identity = self.value.reactive_cell_id().get() as usize;
        if self.root_visible {
            context.define_symbol(identity, register, &self.name, self.mutable)?;
        } else {
            context.define_local_symbol(identity, register, &self.name, self.mutable)?;
        }
        let name = ValueCell::from_exact(self.name.clone())?;
        let mutable = ValueCell::from_exact(self.mutable)?;
        let name_register = compile_value_cell_register(&name, context)?;
        let mutable_register = compile_value_cell_register(&mutable, context)?;
        context.emit_declaration_binary(
            hash_str(&canonical_variable_definition_runtime_name(
                self.value.representation(),
            )?),
            register,
            name_register,
            mutable_register,
        );
        Ok(register)
    }
}

#[cfg(all(
    feature = "variable_define",
    any(feature = "semantic-compiler", feature = "source")
))]
pub(crate) fn canonical_variable_definition_runtime_name(
    representation: FunctionValueRepresentation,
) -> MResult<String> {
    use crate::{
        FunctionMatrixElement as Element, FunctionMatrixRepresentation as Storage,
        FunctionMatrixStoragePattern as StoragePattern,
        FunctionValueRepresentation as Representation,
    };

    let scalar = match representation {
        Representation::U8 => Some("U8"),
        Representation::U16 => Some("U16"),
        Representation::U32 => Some("U32"),
        Representation::U64 => Some("U64"),
        Representation::U128 => Some("U128"),
        Representation::I8 => Some("I8"),
        Representation::I16 => Some("I16"),
        Representation::I32 => Some("I32"),
        Representation::I64 => Some("I64"),
        Representation::I128 => Some("I128"),
        Representation::F32 => Some("F32"),
        Representation::F64 => Some("F64"),
        Representation::C64 => Some("C64"),
        Representation::R64 => Some("R64"),
        Representation::String => Some("String"),
        Representation::Bool => Some("Bool"),
        Representation::Empty => return Ok("VariableDefineEmpty".to_owned()),
        Representation::Atom => Some("MechAtom"),
        Representation::Enum => Some("MechEnum"),
        Representation::Record => Some("MechRecord"),
        Representation::Map => Some("MechMap"),
        Representation::Set => Some("MechSet"),
        Representation::Table => Some("MechTable"),
        Representation::Tuple => Some("MechTuple"),
        Representation::Matrix {
            element,
            storage: StoragePattern::Exact(storage),
        } => {
            let element = match element {
                Element::Bool => "bool",
                Element::String => "string",
                Element::U8 => "u8",
                Element::U16 => "u16",
                Element::U32 => "u32",
                Element::U64 => "u64",
                Element::U128 => "u128",
                Element::I8 => "i8",
                Element::I16 => "i16",
                Element::I32 => "i32",
                Element::I64 => "i64",
                Element::I128 => "i128",
                Element::F32 => "f32",
                Element::F64 => "f64",
                Element::C64 => "complex",
                Element::R64 => "rational",
                Element::Index | Element::Value => {
                    return Err(MechError::new(
                        GenericError {
                            msg: "canonical matrix declaration has no exact runtime marker"
                                .to_owned(),
                        },
                        None,
                    )
                    .with_compiler_loc());
                }
            };
            let storage = match storage {
                Storage::Matrix1 => "Matrix1",
                Storage::Matrix2 => "Matrix2",
                Storage::Matrix3 => "Matrix3",
                Storage::Matrix4 => "Matrix4",
                Storage::Matrix2x3 => "Matrix2x3",
                Storage::Matrix3x2 => "Matrix3x2",
                Storage::RowVector2 => "RowVector2",
                Storage::RowVector3 => "RowVector3",
                Storage::RowVector4 => "RowVector4",
                Storage::Vector2 => "Vector2",
                Storage::Vector3 => "Vector3",
                Storage::Vector4 => "Vector4",
                Storage::RowVectorD => "RowDVector",
                Storage::VectorD => "DVector",
                Storage::MatrixD => "DMatrix",
            };
            return Ok(format!("VariableDefineMatrix<{element}{storage}>"));
        }
        Representation::Id
        | Representation::Index
        | Representation::Kind
        | Representation::MutableValueCell
        | Representation::AnyValue
        | Representation::Matrix {
            element: Element::Value,
            ..
        }
        | Representation::Matrix {
            storage: StoragePattern::AnyStorage,
            ..
        } => return Ok("VariableDefineEmpty".to_owned()),
    };
    scalar
        .map(|suffix| format!("VariableDefine{suffix}"))
        .ok_or_else(|| {
            MechError::new(
                GenericError {
                    msg: format!(
                        "canonical declaration representation {representation:?} has no exact runtime marker"
                    ),
                },
                None,
            )
            .with_compiler_loc()
        })
}

#[cfg(feature = "semantic-compiler")]
fn define_compiler_symbol(
    ctx: &mut dyn BytecodeCompilerContext,
    pointer: usize,
    register: Register,
    name: &str,
    mutable: bool,
    root_visible: bool,
) -> MResult<()> {
    if root_visible {
        ctx.define_symbol(pointer, register, name, mutable)
    } else {
        ctx.define_local_symbol(pointer, register, name, mutable)
    }
}

/// Bytecode-visible marker that keeps integrity-constraint support in the
/// exact native dependency closure. Constraint identity and its live result
/// cell remain encoded by the immutable `!` symbol bound to the input
/// register; this no-op factory gives contract analysis an explicit linkage
/// requirement without adding a bytecode-v1 opcode or section.
#[cfg(feature = "invariant_define")]
#[derive(Debug)]
pub struct BytecodeIntegrityConstraintMarker {
    out: FunctionValueOutput,
    #[cfg(feature = "semantic-compiler")]
    arguments: Vec<FunctionValueInput>,
}

#[cfg(feature = "invariant_define")]
impl MechFunctionFactory for BytecodeIntegrityConstraintMarker {
    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::variadic(
        FunctionValueRepresentation::Bool,
        FunctionValueRepresentation::AnyValue,
    );

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arguments) = invocation.expect_variadic()?;
        if arguments.len() != 6 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 6,
                    found: arguments.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        Ok(Box::new(Self {
            out: out.value(),
            #[cfg(feature = "semantic-compiler")]
            arguments: arguments.map(FunctionInputPort::value).collect(),
        }))
    }
}

#[cfg(feature = "invariant_define")]
impl MechFunctionImpl for BytecodeIntegrityConstraintMarker {
    fn solve_result(&self) -> MResult<()> {
        Ok(())
    }

    fn reactive_dependency_scopes(
        &self,
        argument_count: usize,
    ) -> Option<Vec<ReactiveDependencyScope>> {
        Some(vec![ReactiveDependencyScope::None; argument_count])
    }

    fn to_string(&self) -> String {
        "integrity/constraint".to_string()
    }

    fn reactive_output_value_cells(&self) -> Vec<ValueCell> {
        vec![self.out.cell().clone()]
    }
}

#[cfg(all(feature = "invariant_define", feature = "semantic-compiler"))]
impl MechFunctionCompiler for BytecodeIntegrityConstraintMarker {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let destination = self.out.compile_register(ctx)?;
        let arguments = self
            .arguments
            .iter()
            .map(|argument| argument.compile_register(ctx))
            .collect::<MResult<Vec<_>>>()?;
        ctx.emit_varop(hash_str("integrity/constraint"), destination, arguments);
        Ok(destination)
    }
}

#[derive(Debug)]
pub struct VariableDefineMatrix<T, MatA> {
    pub name: Ref<String>,
    pub mutable: Ref<bool>,
    pub var: Ref<MatA>,
    pub initial: MatA,
    pub root_visible: bool,
    pub _marker: PhantomData<T>,
}
impl<T, MatA> MechFunctionFactory for VariableDefineMatrix<T, MatA>
where
    T: Debug
        + Clone
        + Sync
        + Send
        + 'static
        + ConstElem
        + FunctionRuntimeType
        + CanonicalMatrixElementBacking,
    #[cfg(feature = "semantic-compiler")]
    T: CompileConst + CanonicalMatrixElementBacking,
    for<'a> &'a MatA: IntoIterator<Item = &'a T>,
    for<'a> &'a mut MatA: IntoIterator<Item = &'a mut T>,
    MatA: Debug + Clone + ConstElem + FunctionStateBacking + 'static,
    #[cfg(feature = "semantic-compiler")]
    MatA: CompileConst,
{
    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
        MatA::REPRESENTATION,
        FunctionValueRepresentation::String,
        FunctionValueRepresentation::Bool,
    );

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (var, name, mutable) = invocation.expect_binary()?;
        let var: Ref<MatA> = var.try_ref()?;
        let name: Ref<String> = name.try_ref()?;
        let mutable: Ref<bool> = mutable.try_ref()?;
        let initial = var.borrow().clone();
        Ok(Box::new(Self {
            name,
            mutable,
            var,
            initial,
            root_visible: true,
            _marker: PhantomData::default(),
        }))
    }
}
impl<T, MatA> MechFunctionImpl for VariableDefineMatrix<T, MatA>
where
    T: Debug
        + Clone
        + Sync
        + Send
        + 'static
        + ConstElem
        + FunctionRuntimeType
        + CanonicalMatrixElementBacking,
    MatA: Debug,
{
    fn solve_result(&self) -> MResult<()> {
        Ok(())
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(feature = "semantic-compiler")]
impl<T, MatA> MechFunctionCompiler for VariableDefineMatrix<T, MatA>
where
    T: CompileConst + ConstElem + FunctionRuntimeType + CanonicalMatrixElementBacking,
    MatA: CompileConst + ConstElem,
{
    fn reserve_bytecode_registers(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<()> {
        if *self.mutable.borrow() {
            compile_register_initial!(self.var, self.initial, ctx);
        }
        Ok(())
    }

    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let variable_register = compile_register_initial!(self.var, self.initial, ctx);
        let variable_name = self.name.borrow().clone();
        let variable_mutable = *self.mutable.borrow();
        if variable_mutable {
            let initializer = self.initial.compile_const(ctx)?;
            ctx.record_state_initializer(variable_register, initializer)?;
        }
        define_compiler_symbol(
            ctx,
            self.var.addr(),
            variable_register,
            &variable_name,
            variable_mutable,
            self.root_visible,
        )?;
        let name = format!(
            "VariableDefineMatrix<{}{}>",
            <T as FunctionRuntimeType>::REPRESENTATION,
            function_matrix_storage_name::<MatA>()
        );
        let name_register = compile_register_brrw!(self.name, ctx);
        let mutable_register = compile_register_brrw!(self.mutable, ctx);
        ctx.emit_declaration_binary(
            hash_str(&name),
            variable_register,
            name_register,
            mutable_register,
        );
        Ok(variable_register)
    }
}

#[macro_export]
macro_rules! impl_variable_define_fxn {
    ($kind:tt) => {
        paste! {
          #[derive(Debug, Clone)]
          pub struct [<VariableDefine $kind:camel>] {
            #[cfg(feature = "semantic-compiler")]
            name: Ref<String>,
            #[cfg(feature = "semantic-compiler")]
            mutable: Ref<bool>,
            output: FunctionValueOutput,
            #[cfg(feature = "semantic-compiler")]
            var: Ref<$kind>,
            #[cfg(feature = "semantic-compiler")]
            initial: $kind,
            #[cfg(feature = "semantic-compiler")]
            root_visible: bool,
          }
          impl MechFunctionFactory for [<VariableDefine $kind:camel>] {
            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::NoAdditionalScratch
            }

          const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
              <$kind as FunctionRuntimeType>::REPRESENTATION,
              FunctionValueRepresentation::String,
              FunctionValueRepresentation::Bool,
            );

          fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
              let (out, name, mutable) = invocation.expect_binary()?;
              let output = out.value();
              let var: Ref<$kind> = out.try_ref()?;
              let name: Ref<String> = name.try_ref()?;
              let mutable: Ref<bool> = mutable.try_ref()?;
              #[cfg(feature = "semantic-compiler")]
              let initial = var.borrow().clone();
              #[cfg(not(feature = "semantic-compiler"))]
              {
                drop(var);
                drop(name);
                drop(mutable);
              }
              Ok(Box::new(Self {
                output,
                #[cfg(feature = "semantic-compiler")]
                name,
                #[cfg(feature = "semantic-compiler")]
                mutable,
                #[cfg(feature = "semantic-compiler")]
                var,
                #[cfg(feature = "semantic-compiler")]
                initial,
                #[cfg(feature = "semantic-compiler")]
                root_visible: true,
              }))
            }

          }
          impl MechFunctionImpl for [<VariableDefine $kind:camel>] {
            fn solve_result(&self) -> MResult<()> {
                Ok(())
            }
            fn to_string(&self) -> String { format!("{:#?}", self) }

            fn reactive_output_value_cells(&self) -> Vec<ValueCell> {
                vec![self.output.cell().clone()]
            }

          }
          #[cfg(feature = "semantic-compiler")]
          impl MechFunctionCompiler for [<VariableDefine $kind:camel>] {
          fn reserve_bytecode_registers(
              &self,
              ctx: &mut dyn BytecodeCompilerContext,
          ) -> MResult<()> {
              if *self.mutable.borrow() {
                compile_register_initial!(self.var, self.initial, ctx);
              }
              Ok(())
            }

          fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
              let variable_register = compile_register_initial!(self.var, self.initial, ctx);
              let variable_name = self.name.borrow().clone();
              let variable_mutable = *self.mutable.borrow();
              if variable_mutable {
                let initializer = self.initial.compile_const(ctx)?;
                ctx.record_state_initializer(variable_register, initializer)?;
              }
              define_compiler_symbol(
                ctx,
                self.var.addr(),
                variable_register,
                &variable_name,
                variable_mutable,
                self.root_visible,
              )?;
              let name = format!(stringify!([<VariableDefine $kind:camel>]));
              let name_register = compile_register_brrw!(self.name, ctx);
              let mutable_register = compile_register_brrw!(self.mutable, ctx);
              ctx.emit_declaration_binary(
                hash_str(&name),
                variable_register,
                name_register,
                mutable_register,
              );
              Ok(variable_register)
            }
          }
        }
    };
}

#[cfg(any(
    feature = "table",
    feature = "set",
    feature = "tuple",
    feature = "record",
    feature = "map",
    feature = "atom",
    feature = "enum"
))]
macro_rules! impl_canonical_variable_define_fxn {
    ($factory:ident, $representation:expr) => {
        #[derive(Debug, Clone, Copy)]
        pub struct $factory;

        impl MechFunctionFactory for $factory {
            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                mech_core::ImplementationMemoryClass::NoAdditionalScratch
            }

            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                $representation,
                FunctionValueRepresentation::String,
                FunctionValueRepresentation::Bool,
            );

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, name, mutable) = invocation.expect_binary()?;
                let value = out.value();
                let name = name.value().snapshot()?;
                let mutable = mutable.value().snapshot()?;
                let ValueData::String(name) = name.data() else {
                    return Err(MechError::new(
                        FunctionArgumentTypeMismatch {
                            role: FunctionArgumentRole::Input(0),
                            expected: "String".to_owned(),
                            found: format!("{:?}", name.data()),
                        },
                        None,
                    )
                    .with_compiler_loc());
                };
                let ValueData::Bool(mutable) = mutable.data() else {
                    return Err(MechError::new(
                        FunctionArgumentTypeMismatch {
                            role: FunctionArgumentRole::Input(1),
                            expected: "Bool".to_owned(),
                            found: format!("{:?}", mutable.data()),
                        },
                        None,
                    )
                    .with_compiler_loc());
                };
                #[cfg(not(feature = "semantic-compiler"))]
                let _ = (name, mutable);
                Ok(Box::new(CanonicalVariableDefinition {
                    #[cfg(feature = "semantic-compiler")]
                    initial: value.snapshot()?,
                    value: value.cell().clone(),
                    #[cfg(feature = "semantic-compiler")]
                    name: name.to_string(),
                    #[cfg(feature = "semantic-compiler")]
                    mutable: *mutable,
                    #[cfg(feature = "semantic-compiler")]
                    root_visible: true,
                }))
            }
        }
    };
}

#[cfg(feature = "f64")]
impl_variable_define_fxn!(f64);

mech_core::declare_native_runtime_factory! {
    cfg: all(feature = "f64", feature = "variable_define"),

    registration: register_variable_define_f64,
    installer: install_variable_define_f64,

    name: "VariableDefineF64",
    factory_type: VariableDefineF64,
    contract: RuntimeFunctionContract::no_matrix(RuntimeOutputAliasPolicy::AllowInputAlias),
    compiler_family: mech_core::RuntimeFamilyId::from_name("VariableDefineF64"),

    package: "mech-engine",
    crate_name: "mech_engine",
    installer_path: "mech_engine::__mech_native::install_variable_define_f64",

    extra_cargo_features: ["variable_define"],
}

#[cfg(feature = "f32")]
impl_variable_define_fxn!(f32);
#[cfg(feature = "u8")]
impl_variable_define_fxn!(u8);
#[cfg(feature = "u16")]
impl_variable_define_fxn!(u16);
#[cfg(feature = "u32")]
impl_variable_define_fxn!(u32);
#[cfg(feature = "u64")]
impl_variable_define_fxn!(u64);
#[cfg(feature = "u128")]
impl_variable_define_fxn!(u128);
#[cfg(feature = "i8")]
impl_variable_define_fxn!(i8);
#[cfg(feature = "i16")]
impl_variable_define_fxn!(i16);
#[cfg(feature = "i32")]
impl_variable_define_fxn!(i32);
#[cfg(feature = "i64")]
impl_variable_define_fxn!(i64);
#[cfg(feature = "i128")]
impl_variable_define_fxn!(i128);
#[cfg(feature = "r64")]
impl_variable_define_fxn!(R64);
#[cfg(feature = "c64")]
impl_variable_define_fxn!(C64);
#[cfg(feature = "bool")]
impl_variable_define_fxn!(bool);
#[cfg(feature = "string")]
impl_variable_define_fxn!(String);
#[cfg(feature = "table")]
impl_canonical_variable_define_fxn!(VariableDefineTable, FunctionValueRepresentation::Table);
#[cfg(feature = "set")]
impl_canonical_variable_define_fxn!(VariableDefineSet, FunctionValueRepresentation::Set);
#[cfg(feature = "tuple")]
impl_canonical_variable_define_fxn!(VariableDefineTuple, FunctionValueRepresentation::Tuple);
#[cfg(feature = "record")]
impl_canonical_variable_define_fxn!(VariableDefineRecord, FunctionValueRepresentation::Record);
#[cfg(feature = "map")]
impl_canonical_variable_define_fxn!(VariableDefineMap, FunctionValueRepresentation::Map);
#[cfg(feature = "atom")]
impl_canonical_variable_define_fxn!(VariableDefineAtom, FunctionValueRepresentation::Atom);
#[cfg(feature = "enum")]
impl_canonical_variable_define_fxn!(VariableDefineEnum, FunctionValueRepresentation::Enum);

macro_rules! declare_variable_define_scalar_native {
    ($feature:literal, $kind:ident) => {
        paste! {
            mech_core::declare_native_runtime_factory! {
                cfg: all(feature = "variable_define", feature = $feature),
                registration: [<register_variable_define_ $kind:lower>],
                installer: [<install_variable_define_ $kind:lower>],
                name: stringify!([<VariableDefine $kind:camel>]),
                factory_type: [<VariableDefine $kind:camel>],
                contract: RuntimeFunctionContract::no_matrix(RuntimeOutputAliasPolicy::AllowInputAlias),
                compiler_family: mech_core::RuntimeFamilyId::from_name(stringify!([<VariableDefine $kind:camel>])),
                package: "mech-engine",
                crate_name: "mech_engine",
                installer_path: concat!(
                    "mech_engine::__mech_native::install_variable_define_",
                    stringify!([<$kind:lower>]),
                ),
                extra_cargo_features: ["variable_define"],
            }
        }
    };
}

declare_variable_define_scalar_native!("f32", f32);
declare_variable_define_scalar_native!("u8", u8);
declare_variable_define_scalar_native!("u16", u16);
declare_variable_define_scalar_native!("u32", u32);
declare_variable_define_scalar_native!("u64", u64);
declare_variable_define_scalar_native!("u128", u128);
declare_variable_define_scalar_native!("i8", i8);
declare_variable_define_scalar_native!("i16", i16);
declare_variable_define_scalar_native!("i32", i32);
declare_variable_define_scalar_native!("i64", i64);
declare_variable_define_scalar_native!("i128", i128);
declare_variable_define_scalar_native!("r64", R64);
declare_variable_define_scalar_native!("c64", C64);
declare_variable_define_scalar_native!("bool", bool);
declare_variable_define_scalar_native!("string", String);

macro_rules! declare_canonical_variable_define_native {
    (
        $feature:literal,
        $token_prefix:ident,
        $token:ident,
        $installer_token:ident,
        $factory:ident,
        $runtime_name:literal
    ) => {
        paste! {
            mech_core::declare_native_runtime_factory! {
                cfg: all(feature = "variable_define", feature = $feature),
                registration: [<register_variable_define_ $token_prefix $token>],
                installer: [<install_variable_define_ $installer_token>],
                name: $runtime_name,
                factory_type: $factory,
                contract: RuntimeFunctionContract::no_matrix(RuntimeOutputAliasPolicy::AllowInputAlias),
                compiler_family: mech_core::RuntimeFamilyId::from_name($runtime_name),
                package: "mech-engine",
                crate_name: "mech_engine",
                installer_path: concat!(
                    "mech_engine::__mech_native::install_variable_define_",
                    stringify!($installer_token),
                ),
                extra_cargo_features: ["variable_define"],
            }
        }
    };
}

declare_canonical_variable_define_native!(
    "table",
    mech_,
    table,
    mechtable,
    VariableDefineTable,
    "VariableDefineMechTable"
);
declare_canonical_variable_define_native!(
    "set",
    mech_,
    set,
    mechset,
    VariableDefineSet,
    "VariableDefineMechSet"
);
declare_canonical_variable_define_native!(
    "tuple",
    mech_,
    tuple,
    mechtuple,
    VariableDefineTuple,
    "VariableDefineMechTuple"
);
declare_canonical_variable_define_native!(
    "record",
    mech_,
    record,
    mechrecord,
    VariableDefineRecord,
    "VariableDefineMechRecord"
);
declare_canonical_variable_define_native!(
    "map",
    mech_,
    map,
    mechmap,
    VariableDefineMap,
    "VariableDefineMechMap"
);
declare_canonical_variable_define_native!(
    "atom",
    mech_,
    atom,
    mechatom,
    VariableDefineAtom,
    "VariableDefineMechAtom"
);
declare_canonical_variable_define_native!(
    "enum",
    mech_,
    enum,
    mechenum,
    VariableDefineEnum,
    "VariableDefineMechEnum"
);

#[doc(hidden)]
#[cfg(feature = "native-link")]
pub mod __mech_native {
    #[cfg(feature = "variable_define")]
    pub use super::install_variable_define_empty;

    macro_rules! export_variable_define_scalar_native {
        ($feature:literal, $kind:ident) => {
            #[cfg(all(feature = "variable_define", feature = $feature))]
            paste::paste! { pub use super::[<install_variable_define_ $kind:lower>]; }
        };
    }
    export_variable_define_scalar_native!("f32", f32);
    export_variable_define_scalar_native!("u8", u8);
    export_variable_define_scalar_native!("u16", u16);
    export_variable_define_scalar_native!("u32", u32);
    export_variable_define_scalar_native!("u64", u64);
    export_variable_define_scalar_native!("u128", u128);
    export_variable_define_scalar_native!("i8", i8);
    export_variable_define_scalar_native!("i16", i16);
    export_variable_define_scalar_native!("i32", i32);
    export_variable_define_scalar_native!("i64", i64);
    export_variable_define_scalar_native!("i128", i128);
    export_variable_define_scalar_native!("r64", R64);
    export_variable_define_scalar_native!("c64", C64);
    export_variable_define_scalar_native!("bool", bool);
    export_variable_define_scalar_native!("string", String);
    #[cfg(all(feature = "variable_define", feature = "atom"))]
    pub use super::install_variable_define_mechatom;
    #[cfg(all(feature = "variable_define", feature = "enum"))]
    pub use super::install_variable_define_mechenum;
    #[cfg(all(feature = "variable_define", feature = "map"))]
    pub use super::install_variable_define_mechmap;
    #[cfg(all(feature = "variable_define", feature = "record"))]
    pub use super::install_variable_define_mechrecord;
    #[cfg(all(feature = "variable_define", feature = "set"))]
    pub use super::install_variable_define_mechset;
    #[cfg(all(feature = "variable_define", feature = "table"))]
    pub use super::install_variable_define_mechtable;
    #[cfg(all(feature = "variable_define", feature = "tuple"))]
    pub use super::install_variable_define_mechtuple;
}

macro_rules! for_each_variable_define_matrix_shape {
    ($callback:path, $context:tt) => {
        $callback!(
            $context,
            Matrix1,
            "matrix1",
            any(feature = "matrix1", feature = "variable_define_matrix1")
        );
        $callback!($context, Matrix2, "matrix2", feature = "matrix2");
        $callback!($context, Matrix2x3, "matrix2x3", feature = "matrix2x3");
        $callback!($context, Matrix3x2, "matrix3x2", feature = "matrix3x2");
        $callback!($context, Matrix3, "matrix3", feature = "matrix3");
        $callback!($context, Matrix4, "matrix4", feature = "matrix4");
        $callback!($context, DMatrix, "matrixd", feature = "matrixd");
        $callback!($context, Vector2, "vector2", feature = "vector2");
        $callback!($context, Vector3, "vector3", feature = "vector3");
        $callback!($context, Vector4, "vector4", feature = "vector4");
        $callback!($context, DVector, "vectord", feature = "vectord");
        $callback!($context, RowVector2, "row_vector2", feature = "row_vector2");
        $callback!($context, RowVector3, "row_vector3", feature = "row_vector3");
        $callback!($context, RowVector4, "row_vector4", feature = "row_vector4");
        $callback!($context, RowDVector, "row_vectord", feature = "row_vectord");
    };
}

macro_rules! declare_variable_define_matrix_native {
    (
        ($value_feature:literal; $kind:ty; $kind_token:ident; $kind_name:literal),
        $shape:ident,
        $shape_feature:literal,
        $shape_cfg:meta
    ) => {
        paste! {
            mech_core::declare_native_runtime_factory! {
                cfg: all(
                    feature = "variable_define",
                    feature = $value_feature,
                    $shape_cfg
                ),
                registration: [<register_variable_define_matrix_ $kind_token:lower _ $shape:lower>],
                installer: [<install_variable_define_matrix_ $kind_token:lower _ $shape:lower>],
                name: concat!("VariableDefineMatrix<", $kind_name, stringify!($shape), ">"),
                factory_type: VariableDefineMatrix<$kind, $shape<$kind>>,
                contract: RuntimeFunctionContract::same_shape(RuntimeOutputAliasPolicy::AllowInputAlias),
                compiler_family: mech_core::RuntimeFamilyId::from_name(concat!("VariableDefineMatrix<", $kind_name, stringify!($shape), ">")),
                package: "mech-engine",
                crate_name: "mech_engine",
                installer_path: concat!(
                    "mech_engine::__mech_native::install_variable_define_matrix_",
                    stringify!([<$kind_token:lower>]), "_", stringify!([<$shape:lower>]),
                ),
                extra_cargo_features: ["variable_define"],
            }
        }
    };
}

macro_rules! declare_variable_define_matrix_for_type {
    ($value_feature:literal, $kind:ty, $kind_token:ident, $kind_name:literal) => {
        for_each_variable_define_matrix_shape!(
            declare_variable_define_matrix_native,
            ($value_feature; $kind; $kind_token; $kind_name)
        );
    };
}

declare_variable_define_matrix_for_type!("u8", u8, u8, "u8");
declare_variable_define_matrix_for_type!("u16", u16, u16, "u16");
declare_variable_define_matrix_for_type!("u32", u32, u32, "u32");
declare_variable_define_matrix_for_type!("u64", u64, u64, "u64");
declare_variable_define_matrix_for_type!("u128", u128, u128, "u128");
declare_variable_define_matrix_for_type!("i8", i8, i8, "i8");
declare_variable_define_matrix_for_type!("i16", i16, i16, "i16");
declare_variable_define_matrix_for_type!("i32", i32, i32, "i32");
declare_variable_define_matrix_for_type!("i64", i64, i64, "i64");
declare_variable_define_matrix_for_type!("i128", i128, i128, "i128");
declare_variable_define_matrix_for_type!("f32", f32, f32, "f32");
declare_variable_define_matrix_for_type!("f64", f64, f64, "f64");
declare_variable_define_matrix_for_type!("r64", R64, R64, "rational");
declare_variable_define_matrix_for_type!("c64", C64, C64, "complex");
declare_variable_define_matrix_for_type!("bool", bool, bool, "bool");
declare_variable_define_matrix_for_type!("string", String, String, "string");

#[cfg(feature = "matrix")]
macro_rules! register_variable_define_matrix_native {
    (
        ($builder:ident; $kind_token:ident),
        Matrix1,
        "matrix1",
        $shape_cfg:meta
    ) => {
        #[cfg(feature = "matrix1")]
        paste! {
            [<register_variable_define_matrix_ $kind_token:lower _matrix1>]($builder)?;
        }
    };
    (
        ($builder:ident; $kind_token:ident),
        $shape:ident,
        $_shape_feature:literal,
        $shape_cfg:meta
    ) => {
        #[cfg($shape_cfg)]
        paste! {
            [<register_variable_define_matrix_ $kind_token:lower _ $shape:lower>]($builder)?;
        }
    };
}

#[cfg(feature = "native-link")]
macro_rules! export_variable_define_matrix_native {
    (($kind_token:ident), $shape:ident, $_shape_feature:literal, $shape_cfg:meta) => {
        #[cfg($shape_cfg)]
        paste::paste! {
            pub use super::[<install_variable_define_matrix_ $kind_token:lower _ $shape:lower>];
        }
    };
}

#[doc(hidden)]
#[cfg(feature = "native-link")]
pub mod __mech_native_matrix {
    macro_rules! export_variable_define_matrix_for_type {
        ($feature:literal, $kind_token:ident) => {
            #[cfg(all(feature = "variable_define", feature = $feature))]
            for_each_variable_define_matrix_shape!(
                export_variable_define_matrix_native,
                ($kind_token)
            );
        };
    }
    export_variable_define_matrix_for_type!("u8", u8);
    export_variable_define_matrix_for_type!("u16", u16);
    export_variable_define_matrix_for_type!("u32", u32);
    export_variable_define_matrix_for_type!("u64", u64);
    export_variable_define_matrix_for_type!("u128", u128);
    export_variable_define_matrix_for_type!("i8", i8);
    export_variable_define_matrix_for_type!("i16", i16);
    export_variable_define_matrix_for_type!("i32", i32);
    export_variable_define_matrix_for_type!("i64", i64);
    export_variable_define_matrix_for_type!("i128", i128);
    export_variable_define_matrix_for_type!("f32", f32);
    export_variable_define_matrix_for_type!("f64", f64);
    export_variable_define_matrix_for_type!("r64", R64);
    export_variable_define_matrix_for_type!("c64", C64);
    export_variable_define_matrix_for_type!("bool", bool);
    export_variable_define_matrix_for_type!("string", String);
}

#[derive(Debug, Clone)]
pub struct VariableDefineEmpty {
    #[cfg(feature = "semantic-compiler")]
    name: Ref<String>,
    #[cfg(feature = "semantic-compiler")]
    mutable: Ref<bool>,
    var: FunctionValueOutput,
    #[cfg(feature = "semantic-compiler")]
    initial: Value,
    #[cfg(feature = "semantic-compiler")]
    root_visible: bool,
}

impl MechFunctionFactory for VariableDefineEmpty {
    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
        FunctionValueRepresentation::MutableValueCell,
        FunctionValueRepresentation::String,
        FunctionValueRepresentation::Bool,
    );

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (var, name, mutable) = invocation.expect_binary()?;
        let var = var.value();
        let name: Ref<String> = name.try_ref()?;
        let mutable: Ref<bool> = mutable.try_ref()?;
        #[cfg(feature = "semantic-compiler")]
        let initial = var.snapshot()?;
        #[cfg(not(feature = "semantic-compiler"))]
        {
            drop(name);
            drop(mutable);
        }
        Ok(Box::new(Self {
            #[cfg(feature = "semantic-compiler")]
            name,
            #[cfg(feature = "semantic-compiler")]
            mutable,
            var,
            #[cfg(feature = "semantic-compiler")]
            initial,
            #[cfg(feature = "semantic-compiler")]
            root_visible: true,
        }))
    }
}

mech_core::declare_native_runtime_factory! {
    cfg: feature = "variable_define",
    registration: register_variable_define_empty,
    installer: install_variable_define_empty,
    name: "VariableDefineEmpty",
    factory_type: VariableDefineEmpty,
    contract: RuntimeFunctionContract::no_matrix(RuntimeOutputAliasPolicy::AllowInputAlias),
    compiler_family: mech_core::RuntimeFamilyId::from_name("VariableDefineEmpty"),
    package: "mech-engine",
    crate_name: "mech_engine",
    installer_path: "mech_engine::__mech_native::install_variable_define_empty",
    extra_cargo_features: ["variable_define"],
}
impl MechFunctionImpl for VariableDefineEmpty {
    fn solve_result(&self) -> MResult<()> {
        Ok(())
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
    fn reactive_output_value_cells(&self) -> Vec<ValueCell> {
        vec![self.var.cell().clone()]
    }
}
#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for VariableDefineEmpty {
    fn reserve_bytecode_registers(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<()> {
        if *self.mutable.borrow() {
            compile_value_cell_initializer_register(self.var.cell(), &self.initial, ctx)?;
        }
        Ok(())
    }

    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let variable_register = self.var.compile_register(ctx)?;
        let variable_name = self.name.borrow().clone();
        let variable_mutable = *self.mutable.borrow();
        define_compiler_symbol(
            ctx,
            self.var.cell().reactive_cell_id().get() as usize,
            variable_register,
            &variable_name,
            variable_mutable,
            self.root_visible,
        )?;
        let name = "VariableDefineEmpty".to_string();
        let name_register = compile_register_brrw!(self.name, ctx);
        let mutable_register = compile_register_brrw!(self.mutable, ctx);
        ctx.emit_declaration_binary(
            hash_str(&name),
            variable_register,
            name_register,
            mutable_register,
        );
        Ok(variable_register)
    }
}

#[cfg(any(
    feature = "u8",
    feature = "u16",
    feature = "u32",
    feature = "u64",
    feature = "u128",
    feature = "i8",
    feature = "i16",
    feature = "i32",
    feature = "i64",
    feature = "i128",
    feature = "f32",
    feature = "r64",
    feature = "c64",
    feature = "bool",
    feature = "string",
    feature = "table",
    feature = "set",
    feature = "tuple",
    feature = "record",
    feature = "map",
    feature = "atom",
    feature = "enum"
))]
macro_rules! install_variable_define_scalar_runtime {
    ($builder:expr, $kind:ident) => {
        paste! {
            [<register_variable_define_ $kind:lower>]($builder)?;
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! install_variable_define_matrix_runtime {
    ($builder:ident, $kind_token:ident) => {
        for_each_variable_define_matrix_shape!(
            register_variable_define_matrix_native,
            ($builder; $kind_token)
        );
    };
}

#[cfg(all(
    feature = "native-plan",
    feature = "variable_define_matrix1",
    not(feature = "matrix1")
))]
pub(super) fn install_native_plan_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    macro_rules! install_matrix1_kind {
        ($feature:literal, $kind_token:ident) => {
            #[cfg(feature = $feature)]
            paste! {
                [<register_variable_define_matrix_ $kind_token:lower _matrix1>](builder)?;
            }
        };
    }

    install_matrix1_kind!("u8", u8);
    install_matrix1_kind!("u16", u16);
    install_matrix1_kind!("u32", u32);
    install_matrix1_kind!("u64", u64);
    install_matrix1_kind!("u128", u128);
    install_matrix1_kind!("i8", i8);
    install_matrix1_kind!("i16", i16);
    install_matrix1_kind!("i32", i32);
    install_matrix1_kind!("i64", i64);
    install_matrix1_kind!("i128", i128);
    install_matrix1_kind!("f32", f32);
    install_matrix1_kind!("f64", f64);
    install_matrix1_kind!("r64", R64);
    install_matrix1_kind!("c64", C64);
    install_matrix1_kind!("bool", bool);
    install_matrix1_kind!("string", String);

    Ok(())
}

pub(crate) fn install_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    macro_rules! install_kind {
        ($feature:literal, $kind:ty, $kind_token:ident, $kind_name:literal) => {
            #[cfg(feature = $feature)]
            {
                install_variable_define_scalar_runtime!(builder, $kind_token);
                #[cfg(feature = "matrix")]
                install_variable_define_matrix_runtime!(builder, $kind_token);
            }
        };
    }

    install_kind!("u8", u8, u8, "u8");
    install_kind!("u16", u16, u16, "u16");
    install_kind!("u32", u32, u32, "u32");
    install_kind!("u64", u64, u64, "u64");
    install_kind!("u128", u128, u128, "u128");
    install_kind!("i8", i8, i8, "i8");
    install_kind!("i16", i16, i16, "i16");
    install_kind!("i32", i32, i32, "i32");
    install_kind!("i64", i64, i64, "i64");
    install_kind!("i128", i128, i128, "i128");
    install_kind!("f32", f32, f32, "f32");
    #[cfg(feature = "f64")]
    {
        register_variable_define_f64(builder)?;
        #[cfg(feature = "matrix")]
        install_variable_define_matrix_runtime!(builder, f64);
    }
    #[cfg(feature = "r64")]
    install_variable_define_scalar_runtime!(builder, R64);
    #[cfg(all(feature = "matrix", feature = "rational"))]
    install_variable_define_matrix_runtime!(builder, R64);

    #[cfg(feature = "c64")]
    install_variable_define_scalar_runtime!(builder, C64);
    #[cfg(all(feature = "matrix", feature = "complex"))]
    install_variable_define_matrix_runtime!(builder, C64);

    install_kind!("bool", bool, bool, "bool");
    install_kind!("string", String, String, "string");

    #[cfg(feature = "table")]
    register_variable_define_mech_table(builder)?;
    #[cfg(feature = "set")]
    register_variable_define_mech_set(builder)?;
    #[cfg(feature = "tuple")]
    register_variable_define_mech_tuple(builder)?;
    #[cfg(feature = "record")]
    register_variable_define_mech_record(builder)?;
    #[cfg(feature = "map")]
    register_variable_define_mech_map(builder)?;
    #[cfg(feature = "atom")]
    register_variable_define_mech_atom(builder)?;
    #[cfg(feature = "enum")]
    register_variable_define_mech_enum(builder)?;

    register_variable_define_empty(builder)?;
    Ok(())
}

#[cfg(feature = "semantic-compiler")]
pub struct VarDefine;

#[cfg(feature = "semantic-compiler")]
impl CanonicalFunctionSpecializer for VarDefine {
    fn specialize_invocation(
        &self,
        invocation: &SpecializationInvocation,
        context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        if invocation.len() != 4 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 4,
                    found: invocation.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let value = invocation
            .input(0)
            .expect("validated value")
            .cell()?
            .clone();
        let name = invocation
            .input(1)
            .expect("validated name")
            .cell()?
            .snapshot()?;
        let mutable = invocation
            .input(2)
            .expect("validated mutability")
            .cell()?
            .snapshot()?;
        let root_visible = invocation
            .input(3)
            .expect("validated visibility")
            .cell()?
            .snapshot()?;
        let ValueData::String(name) = name.data() else {
            return Err(MechError::new(
                GenericError {
                    msg: "variable definition name must be a string".to_owned(),
                },
                None,
            )
            .with_compiler_loc());
        };
        let ValueData::Bool(mutable) = mutable.data() else {
            return Err(MechError::new(
                GenericError {
                    msg: "variable definition mutability must be boolean".to_owned(),
                },
                None,
            )
            .with_compiler_loc());
        };
        let ValueData::Bool(root_visible) = root_visible.data() else {
            return Err(MechError::new(
                GenericError {
                    msg: "variable definition visibility must be boolean".to_owned(),
                },
                None,
            )
            .with_compiler_loc());
        };
        let implementation = CanonicalVariableDefinition {
            #[cfg(feature = "semantic-compiler")]
            initial: value.snapshot()?,
            value: value.clone(),
            name: name.to_string(),
            mutable: *mutable,
            root_visible: *root_visible,
        };
        context.resolve_syntax_operation_contract(&PURE_VARIABLE_DEFINITION_CONTRACT)?;
        let runtime_name = canonical_variable_definition_runtime_name(value.representation())?;
        context.certify_instance(
            FunctionInstance::new(Box::new(implementation), FunctionInvocation::nullary(value)),
            mech_core::RuntimeFunctionId::from_name(&runtime_name),
            mech_core::ExecutionTarget::DirectRuntime,
        )
    }
}

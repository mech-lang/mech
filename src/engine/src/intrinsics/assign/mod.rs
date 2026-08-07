#[macro_use]
use crate::intrinsics::*;

pub mod catalog;
pub use self::catalog::install_runtime;

#[cfg(feature = "map")]
pub mod map;
#[cfg(feature = "matrix")]
pub mod matrix;
#[cfg(feature = "record")]
pub mod record;
#[cfg(feature = "table")]
pub mod table;
#[cfg(feature = "tuple")]
pub mod tuple;

#[cfg(feature = "map")]
pub use self::map::*;
#[cfg(feature = "matrix")]
pub use self::matrix::*;
#[cfg(feature = "record")]
pub use self::record::*;
#[cfg(feature = "table")]
pub use self::table::*;
#[cfg(feature = "tuple")]
pub use self::tuple::*;

// ----------------------------------------------------------------------------
// Assign
// ----------------------------------------------------------------------------

// x = 1 ----------------------------------------------------------------------

trait AssignRuntimeName {
    fn assign_runtime_name() -> String;
}

macro_rules! impl_scalar_assign_runtime_name {
    ($type:ty, $name:literal, $feature:literal) => {
        #[cfg(feature = $feature)]
        impl AssignRuntimeName for $type {
            fn assign_runtime_name() -> String {
                concat!("Assign<", $name, ">").to_string()
            }
        }
    };
}

impl_scalar_assign_runtime_name!(u8, "u8", "u8");
impl_scalar_assign_runtime_name!(u16, "u16", "u16");
impl_scalar_assign_runtime_name!(u32, "u32", "u32");
impl_scalar_assign_runtime_name!(u64, "u64", "u64");
impl_scalar_assign_runtime_name!(u128, "u128", "u128");
impl_scalar_assign_runtime_name!(i8, "i8", "i8");
impl_scalar_assign_runtime_name!(i16, "i16", "i16");
impl_scalar_assign_runtime_name!(i32, "i32", "i32");
impl_scalar_assign_runtime_name!(i64, "i64", "i64");
impl_scalar_assign_runtime_name!(i128, "i128", "i128");
impl_scalar_assign_runtime_name!(f32, "f32", "f32");
impl_scalar_assign_runtime_name!(f64, "f64", "f64");
impl_scalar_assign_runtime_name!(bool, "bool", "bool");
impl_scalar_assign_runtime_name!(String, "string", "string");
impl_scalar_assign_runtime_name!(R64, "r64", "r64");
impl_scalar_assign_runtime_name!(C64, "c64", "c64");

impl AssignRuntimeName for usize {
    fn assign_runtime_name() -> String {
        "Assign<index>".to_string()
    }
}

macro_rules! impl_matrix_assign_runtime_name {
    ($shape:ident, $feature:literal) => {
        #[cfg(feature = $feature)]
        impl<T> AssignRuntimeName for $shape<T>
        where
            T: AsValueKind,
        {
            fn assign_runtime_name() -> String {
                format!("Assign<{}{}>", T::as_value_kind(), stringify!($shape))
            }
        }
    };
}

impl_matrix_assign_runtime_name!(Matrix1, "matrix1");
impl_matrix_assign_runtime_name!(Matrix2, "matrix2");
impl_matrix_assign_runtime_name!(Matrix2x3, "matrix2x3");
impl_matrix_assign_runtime_name!(Matrix3x2, "matrix3x2");
impl_matrix_assign_runtime_name!(Matrix3, "matrix3");
impl_matrix_assign_runtime_name!(Matrix4, "matrix4");
impl_matrix_assign_runtime_name!(DMatrix, "matrixd");
impl_matrix_assign_runtime_name!(Vector2, "vector2");
impl_matrix_assign_runtime_name!(Vector3, "vector3");
impl_matrix_assign_runtime_name!(Vector4, "vector4");
impl_matrix_assign_runtime_name!(DVector, "vectord");
impl_matrix_assign_runtime_name!(RowVector2, "row_vector2");
impl_matrix_assign_runtime_name!(RowVector3, "row_vector3");
impl_matrix_assign_runtime_name!(RowVector4, "row_vector4");
impl_matrix_assign_runtime_name!(RowDVector, "row_vectord");

#[derive(Debug)]
struct Assign<T> {
    sink: Ref<T>,
    source: Ref<T>,
}

/// A whole-value assignment for stable composite cells. The outer [`Ref`]
/// remains unchanged so reactive dependencies keep pointing at the same cell,
/// while the validated composite snapshot replaces its contents atomically.
#[derive(Debug)]
struct AssignComposite<T> {
    sink: Ref<T>,
    source: Ref<T>,
}

impl<T> MechFunctionImpl for AssignComposite<T>
where
    T: Clone + Debug + 'static,
    Ref<T>: ToValue,
{
    fn solve_result(&self) -> MResult<()> {
        let next = self.source.borrow().clone();
        *self.sink.borrow_mut() = next;
        Ok(())
    }

    fn stage_register(&self) -> MResult<Box<dyn ReactiveRegisterCommit>> {
        let next = self.source.borrow().clone();
        Ok(Box::new(ReactiveRegisterWrite::new(
            self.sink.clone(),
            next,
            self.reactive_output_cell_ids(),
        )))
    }

    fn out(&self) -> Value {
        self.sink.to_value()
    }

    fn reactive_node_kind(&self) -> ReactiveNodeKind {
        ReactiveNodeKind::Register
    }

    fn to_string(&self) -> String {
        format!("{self:#?}")
    }

    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        Ok(self.reactive_output_values())
    }
}

#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for AssignComposite<T>
where
    T: Clone + Debug + 'static,
    Ref<T>: ToValue,
{
    fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Err(MechError::new(
            GenericError {
                msg: "stable composite assignments are runtime external-update nodes and cannot be emitted as bytecode instructions"
                    .to_owned(),
            },
            None,
        )
        .with_compiler_loc())
    }
}

/// A stable composite assignment that updates the already-installed child
/// cells instead of replacing them. Source specialization aliases record,
/// map, table, and tuple children directly, so preserving only the outer cell
/// would leave those downstream aliases attached to stale values.
#[derive(Debug)]
struct AssignStructuredComposite {
    sink: Value,
    source: Value,
}

fn collect_structured_assignments(
    sink: Value,
    source: Value,
    assignments: &mut Vec<Box<dyn MechFunction>>,
) -> MResult<()> {
    match (&sink, &source) {
        (Value::Typed(sink, sink_annotation), Value::Typed(source, source_annotation))
            if sink_annotation == source_annotation =>
        {
            return collect_structured_assignments(
                sink.as_ref().clone(),
                source.as_ref().clone(),
                assignments,
            );
        }
        #[cfg(feature = "record")]
        (Value::Record(sink), Value::Record(source)) => {
            let pairs = {
                let sink = sink.borrow();
                let source = source.borrow();
                sink.data
                    .iter()
                    .map(|(id, sink)| {
                        source
                            .data
                            .get(id)
                            .cloned()
                            .map(|source| (sink.clone(), source))
                            .ok_or_else(|| {
                                MechError::new(
                                    GenericError {
                                        msg: format!(
                                            "stable record update lost validated field {id}"
                                        ),
                                    },
                                    None,
                                )
                                .with_compiler_loc()
                            })
                    })
                    .collect::<MResult<Vec<_>>>()?
            };
            for (sink, source) in pairs {
                collect_structured_assignments(sink, source, assignments)?;
            }
            return Ok(());
        }
        #[cfg(feature = "map")]
        (Value::Map(sink), Value::Map(source)) => {
            let pairs = {
                let sink = sink.borrow();
                let source = source.borrow();
                sink.map
                    .iter()
                    .map(|(key, sink)| {
                        source
                            .map
                            .get(key)
                            .cloned()
                            .map(|source| (sink.clone(), source))
                            .ok_or_else(|| {
                                MechError::new(
                                    GenericError {
                                        msg: "stable map update lost a validated key".to_owned(),
                                    },
                                    None,
                                )
                                .with_compiler_loc()
                            })
                    })
                    .collect::<MResult<Vec<_>>>()?
            };
            for (sink, source) in pairs {
                collect_structured_assignments(sink, source, assignments)?;
            }
            return Ok(());
        }
        #[cfg(feature = "table")]
        (Value::Table(sink), Value::Table(source)) => {
            let pairs = {
                let sink = sink.borrow();
                let source = source.borrow();
                sink.data
                    .iter()
                    .map(|(id, (_, sink))| {
                        source
                            .data
                            .get(id)
                            .map(|(_, source)| (sink.as_vec(), source.as_vec()))
                            .ok_or_else(|| {
                                MechError::new(
                                    GenericError {
                                        msg: format!(
                                            "stable table update lost validated column {id}"
                                        ),
                                    },
                                    None,
                                )
                                .with_compiler_loc()
                            })
                    })
                    .collect::<MResult<Vec<_>>>()?
            };
            for (sink, source) in pairs {
                for (sink, source) in sink.into_iter().zip(source) {
                    collect_structured_assignments(sink, source, assignments)?;
                }
            }
            return Ok(());
        }
        #[cfg(feature = "tuple")]
        (Value::Tuple(sink), Value::Tuple(source)) => {
            let pairs = {
                let sink = sink.borrow();
                let source = source.borrow();
                sink.elements
                    .iter()
                    .zip(source.elements.iter())
                    .map(|(sink, source)| (sink.as_ref().clone(), source.as_ref().clone()))
                    .collect::<Vec<_>>()
            };
            for (sink, source) in pairs {
                collect_structured_assignments(sink, source, assignments)?;
            }
            return Ok(());
        }
        _ => {}
    }

    assignments.push(assign_value_fxn(sink, source)?);
    Ok(())
}

impl AssignStructuredComposite {
    fn assignments(&self) -> MResult<Vec<Box<dyn MechFunction>>> {
        let mut assignments = Vec::new();
        collect_structured_assignments(self.sink.clone(), self.source.clone(), &mut assignments)?;
        let mut output_cells = Vec::new();
        for assignment in &assignments {
            for cell in assignment.reactive_output_cell_ids() {
                if output_cells.contains(&cell) {
                    return Err(MechError::new(
                        GenericError {
                            msg: format!(
                                "stable structured update cannot preserve aliased child cell {:?}",
                                cell,
                            ),
                        },
                        None,
                    )
                    .with_compiler_loc());
                }
                output_cells.push(cell);
            }
        }
        Ok(assignments)
    }
}

impl MechFunctionImpl for AssignStructuredComposite {
    fn solve_result(&self) -> MResult<()> {
        let assignments = self.assignments()?;
        for assignment in assignments {
            assignment.solve_result()?;
        }
        Ok(())
    }

    fn stage_register(&self) -> MResult<Box<dyn ReactiveRegisterCommit>> {
        let assignments = self.assignments()?;
        let commits = assignments
            .iter()
            .map(|assignment| assignment.stage_register())
            .collect::<MResult<Vec<_>>>()?;
        Ok(Box::new(ReactiveRegisterCommitBatch::new(
            commits,
            self.reactive_output_cell_ids(),
        )))
    }

    fn out(&self) -> Value {
        self.sink.clone()
    }

    fn reactive_node_kind(&self) -> ReactiveNodeKind {
        ReactiveNodeKind::Register
    }

    fn to_string(&self) -> String {
        format!("{self:#?}")
    }

    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        Ok(self.reactive_output_values())
    }
}

#[cfg(feature = "compiler")]
impl MechFunctionCompiler for AssignStructuredComposite {
    fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Err(MechError::new(
            GenericError {
                msg: "stable structured assignments are runtime external-update nodes and cannot be emitted as bytecode instructions"
                    .to_owned(),
            },
            None,
        )
        .with_compiler_loc())
    }
}

#[cfg(feature = "matrix")]
fn assign_index_matrix_fxn(
    sink: Matrix<usize>,
    source: Matrix<usize>,
) -> MResult<Box<dyn MechFunction>> {
    match (sink, source) {
        #[cfg(feature = "matrix1")]
        (Matrix::Matrix1(sink), Matrix::Matrix1(source)) => Ok(Box::new(AssignComposite {
            sink: sink.clone(),
            source: source.clone(),
        })),
        #[cfg(feature = "matrix2")]
        (Matrix::Matrix2(sink), Matrix::Matrix2(source)) => Ok(Box::new(AssignComposite {
            sink: sink.clone(),
            source: source.clone(),
        })),
        #[cfg(feature = "matrix2x3")]
        (Matrix::Matrix2x3(sink), Matrix::Matrix2x3(source)) => Ok(Box::new(AssignComposite {
            sink: sink.clone(),
            source: source.clone(),
        })),
        #[cfg(feature = "matrix3x2")]
        (Matrix::Matrix3x2(sink), Matrix::Matrix3x2(source)) => Ok(Box::new(AssignComposite {
            sink: sink.clone(),
            source: source.clone(),
        })),
        #[cfg(feature = "matrix3")]
        (Matrix::Matrix3(sink), Matrix::Matrix3(source)) => Ok(Box::new(AssignComposite {
            sink: sink.clone(),
            source: source.clone(),
        })),
        #[cfg(feature = "matrix4")]
        (Matrix::Matrix4(sink), Matrix::Matrix4(source)) => Ok(Box::new(AssignComposite {
            sink: sink.clone(),
            source: source.clone(),
        })),
        #[cfg(feature = "matrixd")]
        (Matrix::DMatrix(sink), Matrix::DMatrix(source)) => Ok(Box::new(AssignComposite {
            sink: sink.clone(),
            source: source.clone(),
        })),
        #[cfg(feature = "vector2")]
        (Matrix::Vector2(sink), Matrix::Vector2(source)) => Ok(Box::new(AssignComposite {
            sink: sink.clone(),
            source: source.clone(),
        })),
        #[cfg(feature = "vector3")]
        (Matrix::Vector3(sink), Matrix::Vector3(source)) => Ok(Box::new(AssignComposite {
            sink: sink.clone(),
            source: source.clone(),
        })),
        #[cfg(feature = "vector4")]
        (Matrix::Vector4(sink), Matrix::Vector4(source)) => Ok(Box::new(AssignComposite {
            sink: sink.clone(),
            source: source.clone(),
        })),
        #[cfg(feature = "vectord")]
        (Matrix::DVector(sink), Matrix::DVector(source)) => Ok(Box::new(AssignComposite {
            sink: sink.clone(),
            source: source.clone(),
        })),
        #[cfg(feature = "row_vector2")]
        (Matrix::RowVector2(sink), Matrix::RowVector2(source)) => Ok(Box::new(AssignComposite {
            sink: sink.clone(),
            source: source.clone(),
        })),
        #[cfg(feature = "row_vector3")]
        (Matrix::RowVector3(sink), Matrix::RowVector3(source)) => Ok(Box::new(AssignComposite {
            sink: sink.clone(),
            source: source.clone(),
        })),
        #[cfg(feature = "row_vector4")]
        (Matrix::RowVector4(sink), Matrix::RowVector4(source)) => Ok(Box::new(AssignComposite {
            sink: sink.clone(),
            source: source.clone(),
        })),
        #[cfg(feature = "row_vectord")]
        (Matrix::RowDVector(sink), Matrix::RowDVector(source)) => Ok(Box::new(AssignComposite {
            sink: sink.clone(),
            source: source.clone(),
        })),
        (sink, source) => Err(MechError::new(
            UnhandledFunctionArgumentKind2 {
                arg: (
                    Value::MatrixIndex(sink).kind(),
                    Value::MatrixIndex(source).kind(),
                ),
                fxn_name: "assign".to_owned(),
            },
            None,
        )
        .with_compiler_loc()),
    }
}

impl<T> MechFunctionFactory for Assign<T>
where
    T: Clone + Debug + Sync + Send + 'static,
    Ref<T>: ToValue,
    #[cfg(feature = "compiler")]
    T: ConstElem + AsValueKind,
    #[cfg(feature = "compiler")]
    T: CompileConst,
    T: FunctionRuntimeType,
    T: AssignRuntimeName,
{
    const SIGNATURE: RuntimeFunctionSignature =
        RuntimeFunctionSignature::unary(T::REPRESENTATION, T::REPRESENTATION);

    fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
            FunctionArgs::Unary(out, source) => {
                let sink: Ref<T> = out.try_function_ref(FunctionArgumentRole::Output)?;
                let source: Ref<T> = source.try_function_ref(FunctionArgumentRole::Input(0))?;

                Ok(Box::new(Self { sink, source }))
            }
            _ => Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 2,
                    found: args.len(),
                },
                None,
            )
            .with_compiler_loc()),
        }
    }
}
impl<T> MechFunctionImpl for Assign<T>
where
    T: Clone + Debug + 'static,
    Ref<T>: ToValue,
{
    fn solve_result(&self) -> MResult<()> {
        let source_ptr = self.source.as_ptr();
        let sink_ptr = self.sink.as_mut_ptr();
        unsafe {
            *sink_ptr = (*source_ptr).clone();
        };
        Ok(())
    }
    fn stage_register(&self) -> MResult<Box<dyn ReactiveRegisterCommit>> {
        let next = self.source.borrow().clone();
        let output_cells = self.reactive_output_cell_ids();
        Ok(Box::new(ReactiveRegisterWrite::new(
            self.sink.clone(),
            next,
            output_cells,
        )))
    }
    fn out(&self) -> Value {
        self.sink.to_value()
    }
    fn reactive_node_kind(&self) -> ReactiveNodeKind {
        ReactiveNodeKind::Register
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        Ok(self.reactive_output_values())
    }
}
#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for Assign<T>
where
    T: CompileConst + ConstElem + AsValueKind + AssignRuntimeName,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = T::assign_runtime_name();
        compile_unop!(name, self.sink, self.source, ctx);
    }
}
#[derive(Debug, Clone)]
pub struct EmptyAssignmentNotBytecodeCompilable;
impl MechErrorKind for EmptyAssignmentNotBytecodeCompilable {
    fn name(&self) -> &str {
        "EmptyAssignmentNotBytecodeCompilable"
    }

    fn message(&self) -> String {
        "empty stable assignment is not currently bytecode-compilable".to_string()
    }
}

#[derive(Debug)]
struct AssignEmpty;
impl MechFunctionImpl for AssignEmpty {
    fn solve_result(&self) -> MResult<()> {
        Ok(())
    }
    fn stage_register(&self) -> MResult<Box<dyn ReactiveRegisterCommit>> {
        Ok(Box::new(ReactiveRegisterNoopCommit::new(
            self.reactive_output_cell_ids(),
        )))
    }
    fn out(&self) -> Value {
        Value::Empty
    }
    fn reactive_node_kind(&self) -> ReactiveNodeKind {
        ReactiveNodeKind::Register
    }
    fn to_string(&self) -> String {
        "AssignEmpty".to_string()
    }

    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        Ok(self.reactive_output_values())
    }
}
#[cfg(feature = "compiler")]
impl MechFunctionCompiler for AssignEmpty {
    fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Err(MechError::new(EmptyAssignmentNotBytecodeCompilable, None).with_compiler_loc())
    }
}

#[macro_export]
macro_rules! impl_assign_value_match_arms {
  ($arg:expr,$($value_kind:ident, $feature:tt);+ $(;)?) => {
    paste::paste! {
      match $arg {
        $(
          #[cfg(feature = $feature)]
          (Value::$value_kind(sink), Value::$value_kind(source)) => Ok(Box::new(Assign{ sink: sink.clone(), source: source.clone() })),
          #[cfg(all(feature = $feature, feature = "matrix1"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)), Value::[<Matrix $value_kind>](Matrix::Matrix1(source))) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
          #[cfg(all(feature = $feature, feature = "matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)), Value::[<Matrix $value_kind>](Matrix::Matrix2(source))) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
          #[cfg(all(feature = $feature, feature = "matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), Value::[<Matrix $value_kind>](Matrix::Matrix2x3(source))) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
          #[cfg(all(feature = $feature, feature = "matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), Value::[<Matrix $value_kind>](Matrix::Matrix3x2(source))) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
          #[cfg(all(feature = $feature, feature = "matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)), Value::[<Matrix $value_kind>](Matrix::Matrix3(source))) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
          #[cfg(all(feature = $feature, feature = "matrix4"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)), Value::[<Matrix $value_kind>](Matrix::Matrix4(source))) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
          #[cfg(all(feature = $feature, feature = "matrixd"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)), Value::[<Matrix $value_kind>](Matrix::DMatrix(source))) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
          #[cfg(all(feature = $feature, feature = "vector2"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector2(sink)), Value::[<Matrix $value_kind>](Matrix::Vector2(source))) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
          #[cfg(all(feature = $feature, feature = "vector3"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector3(sink)), Value::[<Matrix $value_kind>](Matrix::Vector3(source))) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
          #[cfg(all(feature = $feature, feature = "vector4"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector4(sink)), Value::[<Matrix $value_kind>](Matrix::Vector4(source))) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
          #[cfg(all(feature = $feature, feature = "vectord"))]
          (Value::[<Matrix $value_kind>](Matrix::DVector(sink)), Value::[<Matrix $value_kind>](Matrix::DVector(source))) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
          #[cfg(all(feature = $feature, feature = "row_vector2"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector2(sink)), Value::[<Matrix $value_kind>](Matrix::RowVector2(source))) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
          #[cfg(all(feature = $feature, feature = "row_vector3"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector3(sink)), Value::[<Matrix $value_kind>](Matrix::RowVector3(source))) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
          #[cfg(all(feature = $feature, feature = "row_vector4"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector4(sink)), Value::[<Matrix $value_kind>](Matrix::RowVector4(source))) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
          #[cfg(all(feature = $feature, feature = "row_vectord"))]
          (Value::[<Matrix $value_kind>](Matrix::RowDVector(sink)), Value::[<Matrix $value_kind>](Matrix::RowDVector(source))) => Ok(Box::new(Assign{sink: sink.clone(), source: source.clone()})),
        )+
        (sink, source) => Err(MechError::new(
            UnhandledFunctionArgumentKind2 {arg: (sink.kind(), source.kind()), fxn_name: "assign".to_string() },
            None
          ).with_compiler_loc()
        ),
      }
    }
  };
}

fn assign_value_fxn(sink: Value, source: Value) -> MResult<Box<dyn MechFunction>> {
    match (&sink, &source) {
        (
            Value::Typed(sink_inner, sink_annotation),
            Value::Typed(source_inner, source_annotation),
        ) if sink_annotation == source_annotation => {
            return assign_value_fxn(sink_inner.as_ref().clone(), source_inner.as_ref().clone());
        }
        (Value::Empty, Value::Empty) => {
            return Ok(Box::new(AssignEmpty));
        }
        (Value::Index(sink), Value::Index(source)) => {
            return Ok(Box::new(Assign {
                sink: sink.clone(),
                source: source.clone(),
            }));
        }
        #[cfg(feature = "matrix")]
        (Value::MatrixIndex(sink), Value::MatrixIndex(source)) => {
            return assign_index_matrix_fxn(sink.clone(), source.clone());
        }
        #[cfg(feature = "record")]
        (Value::Record(sink), Value::Record(source)) => {
            return Ok(Box::new(AssignStructuredComposite {
                sink: Value::Record(sink.clone()),
                source: Value::Record(source.clone()),
            }));
        }
        #[cfg(feature = "map")]
        (Value::Map(sink), Value::Map(source)) => {
            return Ok(Box::new(AssignStructuredComposite {
                sink: Value::Map(sink.clone()),
                source: Value::Map(source.clone()),
            }));
        }
        #[cfg(feature = "set")]
        (Value::Set(sink), Value::Set(source)) => {
            return Ok(Box::new(AssignComposite {
                sink: sink.clone(),
                source: source.clone(),
            }));
        }
        #[cfg(feature = "table")]
        (Value::Table(sink), Value::Table(source)) => {
            return Ok(Box::new(AssignStructuredComposite {
                sink: Value::Table(sink.clone()),
                source: Value::Table(source.clone()),
            }));
        }
        #[cfg(feature = "tuple")]
        (Value::Tuple(sink), Value::Tuple(source)) => {
            return Ok(Box::new(AssignStructuredComposite {
                sink: Value::Tuple(sink.clone()),
                source: Value::Tuple(source.clone()),
            }));
        }
        #[cfg(feature = "atom")]
        (Value::Atom(sink), Value::Atom(source)) => {
            return Ok(Box::new(AssignComposite {
                sink: sink.clone(),
                source: source.clone(),
            }));
        }
        #[cfg(feature = "enum")]
        (Value::Enum(sink), Value::Enum(source)) => {
            return Ok(Box::new(AssignComposite {
                sink: sink.clone(),
                source: source.clone(),
            }));
        }
        _ => {}
    }
    impl_assign_value_match_arms!(
      (sink, source),
      Bool,   "bool";
      String, "string";
      U8,     "u8";
      U16,    "u16";
      U32,    "u32";
      U64,    "u64";
      U128,   "u128";
      I8,     "i8";
      I16,    "i16";
      I32,    "i32";
      I64,    "i64";
      I128,   "i128";
      F32,    "f32";
      F64,    "f64";
      R64, "rational";
      C64, "complex";
    )
}

pub struct AssignValue {}

#[cfg(feature = "matrix")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AssignmentIndexKind {
    Scalar,
    Range,
    All,
}

#[cfg(feature = "matrix")]
fn assignment_index_kind(value: &Value) -> AssignmentIndexKind {
    match value {
        Value::IndexAll => AssignmentIndexKind::All,
        Value::Index(_) => AssignmentIndexKind::Scalar,
        Value::MatrixIndex(_) => AssignmentIndexKind::Range,
        _ if value.shape() == [1, 1] => AssignmentIndexKind::Scalar,
        _ => AssignmentIndexKind::Range,
    }
}

#[cfg(feature = "matrix")]
fn compile_matrix_assignment(arguments: &[Value]) -> MResult<Box<dyn MechFunction>> {
    match arguments {
        [_, _, index] => match assignment_index_kind(index) {
            AssignmentIndexKind::Scalar => MatrixAssignScalar {}.specialize(arguments),
            AssignmentIndexKind::Range => MatrixAssignRange {}.specialize(arguments),
            AssignmentIndexKind::All => MatrixAssignAll {}.specialize(arguments),
        },
        [_, _, row, column] => match (assignment_index_kind(row), assignment_index_kind(column)) {
            (AssignmentIndexKind::Scalar, AssignmentIndexKind::Scalar) => {
                MatrixAssignScalarScalar {}.specialize(arguments)
            }
            (AssignmentIndexKind::Scalar, AssignmentIndexKind::Range) => {
                MatrixAssignScalarRange {}.specialize(arguments)
            }
            (AssignmentIndexKind::Range, AssignmentIndexKind::Scalar) => {
                MatrixAssignRangeScalar {}.specialize(arguments)
            }
            (AssignmentIndexKind::Range, AssignmentIndexKind::Range) => {
                MatrixAssignRangeRange {}.specialize(arguments)
            }
            (AssignmentIndexKind::All, AssignmentIndexKind::Scalar) => {
                MatrixAssignAllScalar {}.specialize(arguments)
            }
            (AssignmentIndexKind::All, AssignmentIndexKind::Range) => {
                MatrixAssignAllRange {}.specialize(arguments)
            }
            (AssignmentIndexKind::Scalar, AssignmentIndexKind::All) => {
                MatrixAssignScalarAll {}.specialize(arguments)
            }
            (AssignmentIndexKind::Range, AssignmentIndexKind::All) => {
                MatrixAssignRangeAll {}.specialize(arguments)
            }
            (AssignmentIndexKind::All, AssignmentIndexKind::All) => Err(MechError::new(
                GenericError {
                    msg: "two-dimensional all/all assignment is not implemented".to_string(),
                },
                None,
            )
            .with_compiler_loc()),
        },
        _ => Err(MechError::new(
            IncorrectNumberOfArguments {
                expected: 3,
                found: arguments.len(),
            },
            None,
        )
        .with_compiler_loc()),
    }
}

impl FunctionSpecializer for AssignValue {
    fn specialize(&self, arguments: &[Value]) -> MResult<Box<dyn MechFunction>> {
        if arguments.len() <= 1 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 1,
                    found: arguments.len(),
                },
                None,
            )
            .with_compiler_loc());
        }

        if arguments.len() > 2 {
            let sink_kind = arguments[0].kind().deref_kind();
            #[cfg(feature = "matrix")]
            if matches!(sink_kind, ValueKind::Matrix(_, _)) {
                return compile_matrix_assignment(arguments);
            }
            #[cfg(feature = "map")]
            if matches!(sink_kind, ValueKind::Map(_, _)) {
                return MapAssignScalar {}.specialize(arguments);
            }
            #[cfg(feature = "tuple")]
            if matches!(sink_kind, ValueKind::Tuple(_)) {
                return TupleAssignScalar {}.specialize(arguments);
            }
        }

        let sink = arguments[0].clone();
        let source = arguments[1].clone();
        match assign_value_fxn(sink.clone(), source.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(x) => match (sink, source) {
                (Value::MutableReference(sink), Value::MutableReference(source)) => {
                    assign_value_fxn(sink.borrow().clone(), source.borrow().clone())
                }
                (sink, Value::MutableReference(source)) => {
                    assign_value_fxn(sink.clone(), source.borrow().clone())
                }
                (Value::MutableReference(sink), source) => {
                    assign_value_fxn(sink.borrow().clone(), source.clone())
                }
                (sink, source) => Err(MechError::new(
                    UnhandledFunctionArgumentKind2 {
                        arg: (sink.kind(), source.kind()),
                        fxn_name: "assign".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}

pub struct AssignColumn {}
impl FunctionSpecializer for AssignColumn {
    fn specialize(&self, arguments: &[Value]) -> MResult<Box<dyn MechFunction>> {
        if arguments.len() < 1 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 1,
                    found: arguments.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let src = &arguments[0];
        match src.kind().deref_kind() {
            #[cfg(feature = "table")]
            ValueKind::Table(_, _) => AssignTableColumn {}.specialize(&arguments),
            #[cfg(feature = "record")]
            ValueKind::Record(_) => AssignRecordField {}.specialize(&arguments),
            _ => Err(MechError::new(
                UnhandledFunctionArgumentKind1 {
                    arg: src.kind(),
                    fxn_name: "assign/column".to_string(),
                },
                None,
            )
            .with_compiler_loc()),
        }
    }
}

// x += y ----------------------------------------------------------------------

pub fn add_assign_value_fxn(sink: Value, source: Value) -> MResult<Box<dyn MechFunction>> {
    match sink {
        #[cfg(feature = "table")]
        Value::Table(_) => add_assign_table_fxn(sink, source),
        _ => Err(MechError::new(
            UnhandledFunctionArgumentKind2 {
                arg: (sink.kind(), source.kind()),
                fxn_name: "assign/add".to_string(),
            },
            None,
        )
        .with_compiler_loc()),
    }
}

pub struct AddAssignValue {}
impl FunctionSpecializer for AddAssignValue {
    fn specialize(&self, arguments: &[Value]) -> MResult<Box<dyn MechFunction>> {
        if arguments.len() <= 1 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 1,
                    found: arguments.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let sink = arguments[0].clone();
        let source = arguments[1].clone();
        match add_assign_value_fxn(sink.clone(), source.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(x) => match (sink, source) {
                (Value::MutableReference(sink), Value::MutableReference(source)) => {
                    add_assign_value_fxn(sink.borrow().clone(), source.borrow().clone())
                }
                (sink, Value::MutableReference(source)) => {
                    add_assign_value_fxn(sink.clone(), source.borrow().clone())
                }
                (Value::MutableReference(sink), source) => {
                    add_assign_value_fxn(sink.borrow().clone(), source.clone())
                }
                (sink, source) => Err(MechError::new(
                    UnhandledFunctionArgumentKind2 {
                        arg: (sink.kind(), source.kind()),
                        fxn_name: "assign/add".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "compiler")]
    #[test]
    fn empty_stable_assignment_bytecode_compile_returns_error() {
        use crate::test_support::bytecode_compiler::RecordingBytecodeCompilerContext;

        let assignment = AssignEmpty;
        let mut context = RecordingBytecodeCompilerContext::default();
        let error = assignment.compile(&mut context).unwrap_err();
        let rendered = format!("{error:?}");
        assert!(
            rendered.contains("EmptyAssignmentNotBytecodeCompilable"),
            "{rendered}",
        );
    }
}

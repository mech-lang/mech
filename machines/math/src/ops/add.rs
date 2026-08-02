use crate::*;
use num_traits::*;
#[cfg(feature = "source")]
use std::sync::Arc;

#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Add ------------------------------------------------------------------------

macro_rules! add_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            *$out = *$lhs + *$rhs;
        }
    };
}

macro_rules! add_vec_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe { (*$lhs).add_to(&*$rhs, &mut *$out) }
    };
}

macro_rules! add_mat_vec_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut out_deref = &mut (*$out);
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut col, lhs_col) in out_deref.column_iter_mut().zip(lhs_deref.column_iter()) {
                lhs_col.add_to(&rhs_deref, &mut col);
            }
        }
    };
}

macro_rules! add_vec_mat_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut out_deref = &mut (*$out);
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut col, rhs_col) in out_deref.column_iter_mut().zip(rhs_deref.column_iter()) {
                lhs_deref.add_to(&rhs_col, &mut col);
            }
        }
    };
}

macro_rules! add_mat_row_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut out_deref = &mut (*$out);
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut row, lhs_row) in out_deref.row_iter_mut().zip(lhs_deref.row_iter()) {
                lhs_row.add_to(&rhs_deref, &mut row);
            }
        }
    };
}

macro_rules! add_row_mat_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let mut out_deref = &mut (*$out);
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut row, rhs_row) in out_deref.row_iter_mut().zip(rhs_deref.row_iter()) {
                lhs_deref.add_to(&rhs_row, &mut row);
            }
        }
    };
}

macro_rules! add_scalar_lhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            *$out = (*$lhs).add_scalar(*$rhs);
        }
    };
}

macro_rules! add_scalar_rhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            *$out = (*$rhs).add_scalar(*$lhs);
        }
    };
}

impl_math_fxns!(Add);

#[cfg(feature = "source")]
fn impl_add_fxn(lhs_value: Value, rhs_value: Value) -> MResult<Box<dyn MechFunction>> {
    #[cfg(feature = "c64")]
    match (&lhs_value, &rhs_value) {
        (Value::C64(lhs), rhs) if !matches!(rhs, Value::C64(_)) => {
            if let Ok(rhs_c64) = rhs.as_c64() {
                return impl_add_fxn(Value::C64(lhs.clone()), Value::C64(rhs_c64));
            }
        }
        (lhs, Value::C64(rhs)) if !matches!(lhs, Value::C64(_)) => {
            if let Ok(lhs_c64) = lhs.as_c64() {
                return impl_add_fxn(Value::C64(lhs_c64), Value::C64(rhs.clone()));
            }
        }
        _ => {}
    }

    impl_binop_match_arms!(
      Add,
      (lhs_value, rhs_value),
      I8,   i8,   "i8";
      I16,  i16,  "i16";
      I32,  i32,  "i32";
      I64,  i64,  "i64";
      I128, i128, "i128";
      U8,   u8,   "u8";
      U16,  u16,  "u16";
      U32,  u32,  "u32";
      U64,  u64,  "u64";
      U128, u128, "u128";
      F32,  f32,  "f32";
      F64,  f64,  "f64";
      R64, R64, "rational";
      C64, C64, "complex";
    )
}

#[cfg(feature = "source")]
fn specialize_math_add(arguments: &[Value]) -> MResult<Box<dyn MechFunction>> {
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
    match impl_add_fxn(lhs_value.clone(), rhs_value.clone()) {
        Ok(fxn) => Ok(fxn),
        Err(_) => match (lhs_value, rhs_value) {
            (Value::MutableReference(lhs), Value::MutableReference(rhs)) => {
                impl_add_fxn(lhs.borrow().clone(), rhs.borrow().clone())
            }
            (lhs_value, Value::MutableReference(rhs)) => {
                impl_add_fxn(lhs_value, rhs.borrow().clone())
            }
            (Value::MutableReference(lhs), rhs_value) => {
                impl_add_fxn(lhs.borrow().clone(), rhs_value)
            }
            (lhs, rhs) => {
                if let Some(rhs_converted) = rhs.convert_to(&lhs.kind()) {
                    if let Ok(fxn) = impl_add_fxn(lhs.clone(), rhs_converted) {
                        return Ok(fxn);
                    }
                }
                if let Some(lhs_converted) = lhs.convert_to(&rhs.kind()) {
                    if let Ok(fxn) = impl_add_fxn(lhs_converted, rhs.clone()) {
                        return Ok(fxn);
                    }
                }
                Err(MechError::new(
                    UnhandledFunctionArgumentKind2 {
                        arg: (lhs.kind(), rhs.kind()),
                        fxn_name: "MathAdd".to_string(),
                    },
                    None,
                )
                .with_compiler_loc())
            }
        },
    }
}

#[cfg(feature = "source")]
pub struct MathAdd {}

#[cfg(feature = "source")]
impl FunctionSpecializer for MathAdd {
    fn specialize(&self, arguments: &[Value]) -> MResult<Box<dyn MechFunction>> {
        specialize_math_add(arguments)
    }

    fn guard_safety(&self) -> GuardFunctionSafety {
        // Mixed-kind coercion reads live values, so this cannot honestly claim
        // the `PureStatic` contract even though same-kind selection is static.
        GuardFunctionSafety::Unsupported
    }
}

pub fn install_math_add_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    install_binop_runtime_factories!(
        builder,
        Add;
        ("i8", i8, "i8"),
        ("i16", i16, "i16"),
        ("i32", i32, "i32"),
        ("i64", i64, "i64"),
        ("i128", i128, "i128"),
        ("u8", u8, "u8"),
        ("u16", u16, "u16"),
        ("u32", u32, "u32"),
        ("u64", u64, "u64"),
        ("u128", u128, "u128"),
        ("f32", f32, "f32"),
        ("f64", f64, "f64"),
        ("rational", R64, "r64"),
        ("complex", C64, "c64"),
    )
}

#[cfg(feature = "source")]
pub fn install_math_add_source(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    let operation = builder.insert_specializer("math/add", Arc::new(MathAdd {}))?;
    builder.insert_export(FunctionExport {
        operation,
        canonical_name: "math/add".to_string(),
        module: None,
        item: None,
        exposure: FunctionExposure::Prelude,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn explicit_runtime_catalog() -> FunctionCatalog {
        let mut builder = FunctionCatalogBuilder::new();
        install_math_add_runtime(&mut builder).unwrap();
        builder.build().unwrap()
    }

    fn enabled_scalar_type_count() -> usize {
        [
            cfg!(feature = "i8"),
            cfg!(feature = "i16"),
            cfg!(feature = "i32"),
            cfg!(feature = "i64"),
            cfg!(feature = "i128"),
            cfg!(feature = "u8"),
            cfg!(feature = "u16"),
            cfg!(feature = "u32"),
            cfg!(feature = "u64"),
            cfg!(feature = "u128"),
            cfg!(feature = "f32"),
            cfg!(feature = "f64"),
            cfg!(feature = "rational"),
            cfg!(feature = "complex"),
        ]
        .into_iter()
        .filter(|enabled| *enabled)
        .count()
    }

    fn enabled_shape_family_count() -> usize {
        let direct_shapes = [
            cfg!(feature = "matrix1"),
            cfg!(feature = "matrix2"),
            cfg!(feature = "matrix3"),
            cfg!(feature = "matrix4"),
            cfg!(feature = "matrix2x3"),
            cfg!(feature = "matrix3x2"),
            cfg!(feature = "matrixd"),
            cfg!(feature = "row_vector2"),
            cfg!(feature = "row_vector3"),
            cfg!(feature = "row_vector4"),
            cfg!(feature = "row_vectord"),
            cfg!(feature = "vector2"),
            cfg!(feature = "vector3"),
            cfg!(feature = "vector4"),
            cfg!(feature = "vectord"),
        ]
        .into_iter()
        .filter(|enabled| *enabled)
        .count();

        let matrix_vector_pairs = [
            cfg!(all(feature = "matrix2", feature = "vector2")),
            cfg!(all(feature = "matrix3", feature = "vector3")),
            cfg!(all(feature = "matrix4", feature = "vector4")),
            cfg!(all(feature = "matrix2x3", feature = "vector2")),
            cfg!(all(feature = "matrix3x2", feature = "vector3")),
            cfg!(all(feature = "matrixd", feature = "vectord")),
            cfg!(all(feature = "matrixd", feature = "vector2")),
            cfg!(all(feature = "matrixd", feature = "vector3")),
            cfg!(all(feature = "matrixd", feature = "vector4")),
        ]
        .into_iter()
        .filter(|enabled| *enabled)
        .count();

        let matrix_row_pairs = [
            cfg!(all(feature = "matrix2", feature = "row_vector2")),
            cfg!(all(feature = "matrix3", feature = "row_vector3")),
            cfg!(all(feature = "matrix4", feature = "row_vector4")),
            cfg!(all(feature = "matrix2x3", feature = "row_vector3")),
            cfg!(all(feature = "matrix3x2", feature = "row_vector2")),
            cfg!(all(feature = "matrixd", feature = "row_vectord")),
            cfg!(all(feature = "matrixd", feature = "row_vector2")),
            cfg!(all(feature = "matrixd", feature = "row_vector3")),
            cfg!(all(feature = "matrixd", feature = "row_vector4")),
        ]
        .into_iter()
        .filter(|enabled| *enabled)
        .count();

        1 + direct_shapes * 3 + matrix_vector_pairs * 2 + matrix_row_pairs * 2
    }

    fn assert_catalog_name_and_id(catalog: &FunctionCatalog, name: &str, raw: u64) {
        let id = RuntimeFunctionId::from_name(name);
        assert_eq!(id.raw(), raw, "unexpected runtime ID for {name}");
        let entry = catalog.runtime_entry(id).unwrap_or_else(|| {
            panic!("explicit math/add catalog is missing runtime factory {name}")
        });
        assert_eq!(entry.name, name);
        assert_eq!(entry.id, id);
    }

    #[cfg(feature = "compiler")]
    #[derive(Default)]
    struct RuntimeFactoryRecorder {
        next_register: Register,
        runtime_ids: Vec<u64>,
    }

    #[cfg(feature = "compiler")]
    impl BytecodeCompilerContext for RuntimeFactoryRecorder {
        fn register_for_ptr_with_initialization_status(
            &mut self,
            _pointer: usize,
        ) -> (Register, bool) {
            let register = self.next_register;
            self.next_register += 1;
            (register, false)
        }

        fn intern_constant(&mut self, _constant: EncodedConstant) -> MResult<u32> {
            unreachable!("runtime selection does not initialize constants")
        }

        fn define_symbol(
            &mut self,
            _pointer: usize,
            _register: Register,
            _name: &str,
            _mutable: bool,
        ) -> MResult<()> {
            unreachable!("binary function compilation does not define symbols")
        }

        fn intern_requirement(&mut self, _requirement: ApplicationRequirement) -> MResult<u32> {
            unreachable!("binary function compilation does not require external services")
        }

        fn emit_const_load(&mut self, _destination: Register, _constant: u32) {
            unreachable!("runtime selection does not initialize constants")
        }

        fn emit_nullop(&mut self, _function: u64, _destination: Register) {
            unreachable!("expected binary function compilation")
        }

        fn emit_unop(&mut self, _function: u64, _destination: Register, _source: Register) {
            unreachable!("expected binary function compilation")
        }

        fn emit_binop(
            &mut self,
            function: u64,
            _destination: Register,
            _lhs: Register,
            _rhs: Register,
        ) {
            self.runtime_ids.push(function);
        }

        fn emit_ternop(
            &mut self,
            _function: u64,
            _destination: Register,
            _a: Register,
            _b: Register,
            _c: Register,
        ) {
            unreachable!("expected binary function compilation")
        }

        fn emit_quadop(
            &mut self,
            _function: u64,
            _destination: Register,
            _a: Register,
            _b: Register,
            _c: Register,
            _d: Register,
        ) {
            unreachable!("expected binary function compilation")
        }

        fn emit_varop(
            &mut self,
            _function: u64,
            _destination: Register,
            _arguments: Vec<Register>,
        ) {
            unreachable!("expected binary function compilation")
        }

        fn emit_host_call(
            &mut self,
            _requirement: u32,
            _destination: Register,
            _arguments: Vec<Register>,
        ) {
            unreachable!("expected binary function compilation")
        }

        fn emit_resource_read(&mut self, _requirement: u32, _destination: Register) {
            unreachable!("expected binary function compilation")
        }

        fn emit_resource_write(
            &mut self,
            _requirement: u32,
            _destination: Register,
            _source: Register,
        ) {
            unreachable!("expected binary function compilation")
        }

        fn emit_resource_send(
            &mut self,
            _requirement: u32,
            _destination: Register,
            _source: Register,
        ) {
            unreachable!("expected binary function compilation")
        }
    }

    #[cfg(all(
        feature = "source",
        feature = "f64",
        feature = "matrixd",
        feature = "vector2"
    ))]
    fn assert_catalog_specializes_to(
        specializer: &dyn FunctionSpecializer,
        arguments: [Value; 2],
        expected_family: &str,
        expected_runtime_name: &str,
    ) {
        let function = specializer.specialize(&arguments).unwrap();
        assert!(
            function.to_string().starts_with(expected_family),
            "expected {expected_family}, got {}",
            function.to_string(),
        );

        #[cfg(feature = "compiler")]
        {
            let mut recorder = RuntimeFactoryRecorder::default();
            function.compile(&mut recorder).unwrap();
            assert_eq!(
                recorder.runtime_ids,
                [RuntimeFunctionId::from_name(expected_runtime_name).raw()],
            );
        }

        #[cfg(not(feature = "compiler"))]
        let _ = expected_runtime_name;
    }

    #[test]
    fn installs_every_factory_enabled_by_the_current_feature_set() {
        let catalog = explicit_runtime_catalog();
        assert_eq!(
            catalog.runtime_factory_count(),
            enabled_scalar_type_count() * enabled_shape_family_count()
        );
    }

    #[cfg(all(feature = "f64", feature = "matrixd"))]
    #[test]
    fn pr0_dynamic_add_names_and_ids_are_unchanged() {
        let catalog = explicit_runtime_catalog();
        assert_catalog_name_and_id(&catalog, "AddSS<f64>", 0x000a_2c77_6884_86f3);
        assert_catalog_name_and_id(&catalog, "AddSMD<f64>", 0x0000_6564_dae2_2a47);
        assert_catalog_name_and_id(&catalog, "AddMDS<f64>", 0x003a_baf2_6ed7_4f43);
        assert_catalog_name_and_id(&catalog, "AddMDMD<f64>", 0x008f_a755_537d_c395);
    }

    #[cfg(all(feature = "f64", feature = "vector2"))]
    #[test]
    fn pr0_fixed_vector_add_name_and_id_are_unchanged() {
        let catalog = explicit_runtime_catalog();
        assert_catalog_name_and_id(&catalog, "AddV2S<f64>", 0x0023_38c5_7864_6419);
    }

    #[cfg(all(
        feature = "source",
        feature = "f64",
        feature = "matrixd",
        feature = "vector2"
    ))]
    #[test]
    fn catalog_specializer_selects_all_five_pr0_add_families() {
        let mut builder = FunctionCatalogBuilder::new();
        install_math_add_source(&mut builder).unwrap();
        let catalog = builder.build().unwrap();
        let specializer = catalog
            .specializer(OperationId::from_name("math/add"))
            .unwrap()
            .specializer
            .as_ref();

        let scalar = Value::from(1.0_f64);
        let matrix = Value::MatrixF64(Matrix::DMatrix(Ref::new(DMatrix::from_row_slice(
            2,
            2,
            &[1.0, 2.0, 3.0, 4.0],
        ))));
        let vector = Value::MatrixF64(Matrix::Vector2(Ref::new(Vector2::new(1.0, 2.0))));

        assert_catalog_specializes_to(
            specializer,
            [scalar.clone(), scalar.clone()],
            "AddSS",
            "AddSS<f64>",
        );
        assert_catalog_specializes_to(
            specializer,
            [scalar.clone(), matrix.clone()],
            "AddSMD",
            "AddSMD<f64>",
        );
        assert_catalog_specializes_to(
            specializer,
            [matrix.clone(), scalar.clone()],
            "AddMDS",
            "AddMDS<f64>",
        );
        assert_catalog_specializes_to(
            specializer,
            [matrix.clone(), matrix],
            "AddMDMD",
            "AddMDMD<f64>",
        );
        assert_catalog_specializes_to(specializer, [vector, scalar], "AddV2S", "AddV2S<f64>");
    }

    #[cfg(feature = "source")]
    #[test]
    fn source_installation_exports_only_the_prelude_operation() {
        let mut builder = FunctionCatalogBuilder::new();
        install_math_add_source(&mut builder).unwrap();
        let catalog = builder.build().unwrap();
        let operation = OperationId::from_name("math/add");

        assert_eq!(operation.raw(), 0x00cc_5290_41cb_60c3);
        assert_eq!(catalog.specializer_count(), 1);
        assert_eq!(
            catalog
                .specializer(operation)
                .unwrap()
                .specializer
                .guard_safety(),
            GuardFunctionSafety::Unsupported
        );
        let exports = catalog.exports_for_operation(operation);
        assert_eq!(exports.len(), 1);
        let export = &exports[0];
        assert_eq!(export.operation, operation);
        assert_eq!(export.canonical_name, "math/add");
        assert_eq!(export.module, None);
        assert_eq!(export.item, None);
        assert_eq!(export.exposure, FunctionExposure::Prelude);
        assert!(catalog.module_export("math", "add").is_none());
    }

    #[cfg(all(feature = "source", feature = "f64", feature = "i32"))]
    #[test]
    fn catalog_specializer_preserves_mixed_kind_behavior_without_claiming_purity() {
        let mut builder = FunctionCatalogBuilder::new();
        install_math_add_source(&mut builder).unwrap();
        let catalog = builder.build().unwrap();
        let specializer = &catalog
            .specializer(OperationId::from_name("math/add"))
            .unwrap()
            .specializer;
        let arguments = [Value::from(1.5_f64), Value::from(2_i32)];

        assert_eq!(specializer.guard_safety(), GuardFunctionSafety::Unsupported);
        let fxn = specializer.specialize(&arguments).unwrap();
        fxn.solve();
        assert_eq!(fxn.out().as_f64().unwrap().borrow().clone(), 3.5);
    }

    #[cfg(all(feature = "source", feature = "f64"))]
    #[test]
    fn catalog_specializer_preserves_mutable_reference_behavior() {
        let left = Value::MutableReference(Ref::new(Value::from(1.0_f64)));
        let right = Value::MutableReference(Ref::new(Value::from(2.0_f64)));
        let arguments = vec![left, right];

        let function = FunctionSpecializer::specialize(&MathAdd {}, &arguments).unwrap();
        function.solve();
        assert_eq!(function.out().as_f64().unwrap().borrow().clone(), 3.0);
    }
}

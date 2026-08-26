use mech_core::{
    BytecodeCompilerContext, BytecodeRegisterIdentity, LegacyValue, MResult, Register, ValueCell,
    ValueKind, bytecode_composite_children, bytecode_register_identity,
    compile_annotated_value_cell_register, compile_annotated_value_register,
    compile_value_cell_register, compile_value_register, compiler_value_cell_from_legacy,
};

/// Short-lived source-planning ownership retained until artifact compilation.
///
/// This type is deliberately engine-private and non-serialized. `Cell` keeps
/// location identity explicit, `Typed` preserves every source annotation, and
/// `Immediate` carries values that do not denote a compiler-owned location.
#[derive(Clone, Debug)]
pub(super) enum CompilerValueSource {
    Cell(ValueCell),
    Typed {
        source: Box<CompilerValueSource>,
        annotation: ValueKind,
    },
    Immediate(LegacyValue),
}

impl CompilerValueSource {
    pub(super) fn from_legacy(value: LegacyValue) -> Self {
        // Cell ownership follows the actual outer value; register identity may
        // transparently inspect the payload and must not reorder that boundary.
        if let Some(cell) = compiler_value_cell_from_legacy(&value) {
            return Self::Cell(cell);
        }

        let fallback = core::ptr::from_ref(&value).addr();
        if let BytecodeRegisterIdentity::Typed { annotation, .. } =
            bytecode_register_identity(&value, fallback)
        {
            let mut children = bytecode_composite_children(&value)
                .expect("typed compiler values expose their wrapped value");
            let inner = children
                .pop()
                .expect("typed compiler values have exactly one wrapped value");
            return Self::Typed {
                source: Box::new(Self::from_legacy(inner)),
                annotation,
            };
        }
        Self::Immediate(value)
    }

    pub(super) fn compile_register(
        &self,
        context: &mut dyn BytecodeCompilerContext,
    ) -> MResult<Register> {
        let mut annotations = Vec::new();
        if let Some(cell) = self.typed_cell(&mut annotations) {
            return compile_annotated_value_cell_register(cell, &annotations, context);
        }
        annotations.clear();
        if let Some(value) = self.typed_immediate(&mut annotations) {
            return compile_annotated_value_register(
                value,
                &annotations,
                core::ptr::from_ref(value).addr(),
                context,
            );
        }

        match self {
            Self::Cell(cell) => compile_value_cell_register(cell, context),
            Self::Immediate(value) => {
                compile_value_register(value, core::ptr::from_ref(value).addr(), context)
            }
            Self::Typed { .. } => unreachable!("typed compiler sources were resolved above"),
        }
    }

    fn typed_cell<'a>(&'a self, annotations: &mut Vec<ValueKind>) -> Option<&'a ValueCell> {
        match self {
            Self::Cell(cell) => Some(cell),
            Self::Typed { source, annotation } => {
                annotations.push(annotation.clone());
                source.typed_cell(annotations)
            }
            Self::Immediate(_) => None,
        }
    }

    fn typed_immediate<'a>(&'a self, annotations: &mut Vec<ValueKind>) -> Option<&'a LegacyValue> {
        match self {
            Self::Immediate(value) => Some(value),
            Self::Typed { source, annotation } => {
                annotations.push(annotation.clone());
                source.typed_immediate(annotations)
            }
            Self::Cell(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CompileCtx;
    use mech_core::{BytecodeInstruction, Ref};

    #[test]
    fn mutable_reference_normalization_retains_the_original_cell() {
        let reference = Ref::new(LegacyValue::F64(Ref::new(7.0)));
        let source =
            CompilerValueSource::from_legacy(LegacyValue::MutableReference(reference.clone()));

        let CompilerValueSource::Cell(cell) = source else {
            panic!("mutable references must normalize to explicit cells")
        };
        assert!(cell.same_cell(&ValueCell::from_legacy_ref(reference)));
    }

    #[test]
    fn typed_cell_sources_preserve_annotations_and_underlying_identity() {
        let reference = Ref::new(LegacyValue::F64(Ref::new(7.0)));
        let annotation = ValueKind::Option(Box::new(ValueKind::F64));
        let source = CompilerValueSource::from_legacy(LegacyValue::Typed(
            Box::new(LegacyValue::MutableReference(reference.clone())),
            annotation,
        ));
        let cell = ValueCell::from_legacy_ref(reference);
        let mut context = CompileCtx::new();

        let cell_register = compile_value_cell_register(&cell, &mut context).unwrap();
        let typed_register = source.compile_register(&mut context).unwrap();
        let typed_register_again = source.compile_register(&mut context).unwrap();
        let compiled = context.finish_program(typed_register).unwrap();

        assert_eq!(typed_register, typed_register_again);
        assert_ne!(typed_register, cell_register);
        assert!(
            compiled
                .program
                .instructions
                .iter()
                .any(|instruction| matches!(
                    instruction,
                    BytecodeInstruction::CompositePack { dst, children, .. }
                        if *dst == typed_register && children == &[cell_register]
                ))
        );
    }

    #[test]
    fn mutable_reference_to_typed_payload_retains_the_outer_cell_and_register() {
        let annotation = ValueKind::Option(Box::new(ValueKind::F64));
        let reference = Ref::new(LegacyValue::Typed(
            Box::new(LegacyValue::F64(Ref::new(7.0))),
            annotation.clone(),
        ));
        let cell = ValueCell::from_legacy_ref(reference.clone());
        let source =
            CompilerValueSource::from_legacy(LegacyValue::MutableReference(reference.clone()));

        let CompilerValueSource::Cell(normalized) = &source else {
            panic!("an outer mutable reference must normalize before its typed payload")
        };
        assert!(normalized.same_cell(&cell));

        let payload = normalized.borrow();
        let LegacyValue::Typed(inner, retained_annotation) = &*payload else {
            panic!("the typed payload must remain inside the retained cell")
        };
        assert_eq!(retained_annotation, &annotation);
        assert!(matches!(inner.as_ref(), LegacyValue::F64(_)));
        drop(payload);

        let mut context = CompileCtx::new();
        let cell_register = compile_value_cell_register(&cell, &mut context).unwrap();
        let source_register = source.compile_register(&mut context).unwrap();
        assert_eq!(source_register, cell_register);
    }

    #[test]
    fn mutable_reference_normalization_preserves_nested_typed_payloads() {
        let inner_annotation = ValueKind::Option(Box::new(ValueKind::F64));
        let outer_annotation = ValueKind::Option(Box::new(inner_annotation.clone()));
        let reference = Ref::new(LegacyValue::Typed(
            Box::new(LegacyValue::Typed(
                Box::new(LegacyValue::F64(Ref::new(7.0))),
                inner_annotation.clone(),
            )),
            outer_annotation.clone(),
        ));
        let source =
            CompilerValueSource::from_legacy(LegacyValue::MutableReference(reference.clone()));

        let CompilerValueSource::Cell(normalized) = source else {
            panic!("an outer mutable reference must retain its complete payload")
        };
        assert!(normalized.same_cell(&ValueCell::from_legacy_ref(reference)));

        let payload = normalized.borrow();
        let LegacyValue::Typed(inner, retained_outer_annotation) = &*payload else {
            panic!("the outer typed payload must remain intact")
        };
        assert_eq!(retained_outer_annotation, &outer_annotation);
        let LegacyValue::Typed(value, retained_inner_annotation) = inner.as_ref() else {
            panic!("the nested typed payload must remain intact")
        };
        assert_eq!(retained_inner_annotation, &inner_annotation);
        assert!(matches!(value.as_ref(), LegacyValue::F64(_)));
    }
}

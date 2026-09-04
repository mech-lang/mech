#[cfg(feature = "variable_define")]
use super::{AddressedAssignmentUnsupported, VariableAlreadyDefinedError};
#[cfg(feature = "variable_define")]
use crate::intrinsics::define::{
    CanonicalVariableDefinition, PURE_VARIABLE_DEFINITION_CONTRACT,
    canonical_variable_definition_runtime_name,
};
#[cfg(feature = "variable_define")]
use crate::{
    CanonicalAggregateSourceAbsence, ExecutionTarget, FunctionInstance, FunctionInvocation,
    InterpreterExecution, MResult, MechError, ResolvedOperationDescriptor, RuntimeFunctionId,
    SpecializationInput, SpecializedFunction, ValueCell, VariableDefine, expression,
};
#[cfg(all(
    feature = "variable_define",
    feature = "kind_annotation",
    feature = "enum",
    feature = "atom"
))]
use crate::{CanonicalNominalPath, NominalKey, NominalKind};
#[cfg(all(feature = "variable_define", feature = "subscript_formula"))]
use crate::{
    mark_string_access_value_live, reset_current_string_access_expression_live,
    take_current_string_access_expression_live,
};
#[cfg(all(feature = "variable_define", feature = "kind_annotation"))]
use mech_core::snapshot::{OptionDraft, ValueDataDraft};

#[cfg(all(
    feature = "variable_define",
    feature = "kind_annotation",
    feature = "enum",
    feature = "atom"
))]
fn canonical_atom_enum_conversion(
    value: &ValueCell,
    target: &crate::SchemaBody,
) -> MResult<Option<ValueCell>> {
    let crate::SchemaBody::Enum { variants, .. } = target else {
        return Ok(None);
    };
    let crate::SchemaBody::Atom(atom_key) = value.closed_schema_body()? else {
        return Ok(None);
    };
    let ordinal = variants.iter().position(|variant| {
        CanonicalNominalPath::new(
            variant
                .name
                .split('/')
                .filter(|segment| !segment.is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>(),
        )
        .is_ok_and(|path| NominalKey::from_path(NominalKind::Atom, &path) == atom_key)
    });
    let Some(ordinal) = ordinal else {
        return Ok(None);
    };
    if variants[ordinal].payload.is_some() {
        return Ok(None);
    }
    ValueCell::from_schema_data(
        target.clone(),
        ValueDataDraft::Enum(mech_core::snapshot::EnumDraft {
            ordinal: ordinal as u32,
            payload: None,
        }),
    )
    .map(Some)
}

#[cfg(feature = "variable_define")]
pub fn variable_define(
    definition: &VariableDefine,
    p: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    if definition.var.context.is_some() {
        return Err(MechError::new(AddressedAssignmentUnsupported, None)
            .with_compiler_loc()
            .with_tokens(definition.var.tokens()));
    }
    let id = definition.var.name.hash();
    let name = definition.var.name.to_string();
    if p.symbols().borrow().contains(id) {
        return Err(MechError::new(VariableAlreadyDefinedError { id }, None)
            .with_compiler_loc()
            .with_tokens(definition.var.name.tokens()));
    }

    #[cfg(feature = "subscript_formula")]
    reset_current_string_access_expression_live(p);
    let input = expression(&definition.expression, None, p)?;
    #[cfg(feature = "subscript_formula")]
    let string_access_result_is_live = take_current_string_access_expression_live(p);

    let value = match input {
        SpecializationInput::Cell(value) => {
            #[cfg(all(feature = "kind_annotation", feature = "convert"))]
            if let Some(annotation) = &definition.var.kind {
                let target_schema = crate::structures::schema_body_from_kind(&annotation.kind, p)?;
                if value.closed_schema_body()? == target_schema {
                    value
                } else if let Some(converted) = {
                    #[cfg(all(feature = "enum", feature = "atom"))]
                    {
                        canonical_atom_enum_conversion(&value, &target_schema)?
                    }
                    #[cfg(not(all(feature = "enum", feature = "atom")))]
                    {
                        None
                    }
                } {
                    converted
                } else {
                    crate::literals::convert_literal_cell(value, &target_schema)?
                }
            } else {
                value
            }
            #[cfg(not(all(feature = "kind_annotation", feature = "convert")))]
            value
        }
        SpecializationInput::Absent => {
            #[cfg(feature = "kind_annotation")]
            if let Some(annotation) = &definition.var.kind {
                let schema = crate::structures::schema_body_from_kind(&annotation.kind, p)?;
                let mech_core::SchemaBody::Option(_) = schema else {
                    return Err(MechError::new(
                        CanonicalAggregateSourceAbsence {
                            context: "non-option variable definition",
                        },
                        None,
                    )
                    .with_compiler_loc()
                    .with_tokens(definition.expression.tokens()));
                };
                ValueCell::from_schema_data(
                    schema,
                    ValueDataDraft::Option(OptionDraft {
                        present: false,
                        value: None,
                    }),
                )?
            } else {
                return Err(MechError::new(
                    CanonicalAggregateSourceAbsence {
                        context: "untyped variable definition",
                    },
                    None,
                )
                .with_compiler_loc()
                .with_tokens(definition.expression.tokens()));
            }
            #[cfg(not(feature = "kind_annotation"))]
            return Err(MechError::new(
                CanonicalAggregateSourceAbsence {
                    context: "untyped variable definition",
                },
                None,
            )
            .with_compiler_loc()
            .with_tokens(definition.expression.tokens()));
        }
        SpecializationInput::MatrixAllSelection => {
            return Err(MechError::new(
                CanonicalAggregateSourceAbsence {
                    context: "variable definition",
                },
                Some("matrix all-selection is only valid in a selector position".to_owned()),
            )
            .with_compiler_loc()
            .with_tokens(definition.expression.tokens()));
        }
    };

    #[cfg(feature = "subscript_formula")]
    if string_access_result_is_live {
        mark_string_access_value_live(p, &value);
    }
    p.state
        .borrow()
        .save_symbol(id, name.clone(), value.clone(), definition.mutable);
    let root_visible = !p.in_user_function_scope() && !p.plan().activation_registration_active();
    #[cfg(feature = "semantic-compiler")]
    let initial = value.snapshot()?;
    let runtime_name = canonical_variable_definition_runtime_name(value.representation())?;
    p.plan()
        .register_specialized(SpecializedFunction::syntax_directed(
            FunctionInstance::new(
                Box::new(CanonicalVariableDefinition {
                    value: value.clone(),
                    #[cfg(feature = "semantic-compiler")]
                    initial,
                    name,
                    mutable: definition.mutable,
                    root_visible,
                }),
                FunctionInvocation::nullary(value.clone()),
            ),
            ResolvedOperationDescriptor::from_name(
                "var/define",
                PURE_VARIABLE_DEFINITION_CONTRACT.clone(),
            )?,
            RuntimeFunctionId::from_name(&runtime_name),
            ExecutionTarget::DirectRuntime,
        )?)?;
    Ok(value)
}

#[cfg(feature = "invariant_define")]
pub(super) fn detach_variable_value(value: &ValueCell) -> ValueCell {
    value.clone()
}

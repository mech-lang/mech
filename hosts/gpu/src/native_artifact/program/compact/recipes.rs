//! Source-derived native operation recipes, with no source-name eligibility.
use super::layout::{Kind, Layout, Type};
use crate::native_artifact::NativeCompileError;
use crate::native_artifact::compiler::backend;
use mech_core::ValueData;
use mech_engine::native_artifact::{
    NativeAggregateWord as Word, NativeCollectionPrimitive as Primitive,
    NativeIterationAction as Action, NativeMatchAction as Scalar, NativeMatchValue as Value,
    NativeProgramArtifactPlan as Plan, NativeScalarAggregateAction as Aggregate,
    NativeStructuralObservation as Observation, NativeStructuralStep as Step,
};

pub(super) enum Recipe {
    StringEqual {
        source: Value,
        offset: usize,
        peer: Value,
        guards: Box<[(usize, u32)]>,
        negate: bool,
        output: Value,
    },
    Aggregate(Aggregate),
    Length {
        source: Value,
        output: Value,
    },
    Read {
        source: Value,
        index: Value,
        output: Value,
    },
    Buffer {
        operation: Primitive,
        operands: Box<[Value]>,
        output: Value,
    },
    Pending(Primitive),
    None,
}
fn words(layout: &Layout, value: Value) -> Vec<Word> {
    if matches!(layout.ty(value.get()).kind, Kind::Scalar { .. }) {
        vec![Word::Scalar(value)]
    } else {
        (0..layout.ty(value.get()).words)
            .map(|i| Word::Field(value, i))
            .collect()
    }
}
fn check_width(layout: &Layout, output: Value, count: usize) -> Result<(), NativeCompileError> {
    if layout.ty(output.get()).words != count {
        return Err(backend(
            "compact semantic result width differs from its source recipe",
        ));
    }
    Ok(())
}
pub(super) fn derive(
    plan: &Plan,
    layout: &Layout,
    site: usize,
) -> Result<Recipe, NativeCompileError> {
    let recipe = match plan.blocks()[site].action() {
        Action::Scalar(Scalar::Copy { source, output }) => {
            check_width(layout, *output, layout.ty(source.get()).words)?;
            Recipe::Aggregate(Aggregate::Copy {
                source: *source,
                output: *output,
            })
        }
        Action::LibraryCall {
            template,
            operands,
            output,
        } => match template.primitive() {
            operation @ (Primitive::StringEqual | Primitive::StringNotEqual) => {
                if operands.len() != 2
                    || operands
                        .iter()
                        .any(|v| !matches!(layout.ty(v.get()).kind, Kind::Utf8))
                {
                    return Err(backend("native String comparison requires two UTF8 spans"));
                }
                check_width(layout, *output, 1)?;
                Recipe::StringEqual {
                    source: operands[0],
                    offset: 0,
                    peer: operands[1],
                    guards: Box::new([]),
                    negate: operation == Primitive::StringNotEqual,
                    output: *output,
                }
            }
            Primitive::CompositePack(arity) => {
                let fields = layout.ty(output.get()).fields()?;
                if arity != fields.len() || arity != operands.len() {
                    return Err(backend("compact product source arity mismatch"));
                }
                let mut result = vec![];
                for (field, value) in fields.iter().zip(operands) {
                    if field.ty.words != layout.ty(value.get()).words {
                        return Err(backend("compact product field width mismatch"));
                    }
                    result.extend(words(layout, *value));
                }
                check_width(layout, *output, result.len())?;
                Recipe::Aggregate(Aggregate::Pack {
                    output: *output,
                    words: result.into(),
                })
            }
            Primitive::RecordFieldAccess => {
                if operands.len() != 2 {
                    return Err(backend("compact record selector arity differs"));
                }
                let selected = plan
                    .constants()
                    .iter()
                    .find(|c| c.value() == operands[1])
                    .ok_or_else(|| backend("compact record selector must be a source constant"))?;
                let ValueData::Id(id) = selected.constant().data() else {
                    return Err(backend("compact record selector is not an Id"));
                };
                let fields = layout.ty(operands[0].get()).fields()?;
                let mut matching = fields.iter().filter(|f| {
                    f.name
                        .as_ref()
                        .is_some_and(|n| mech_core::hash_str(n) == *id)
                });
                let field = matching
                    .next()
                    .ok_or_else(|| backend("compact selected record field is absent"))?;
                if matching.next().is_some() {
                    return Err(backend("compact record field selector is ambiguous"));
                }
                check_width(layout, *output, field.ty.words)?;
                Recipe::Aggregate(Aggregate::Project {
                    source: operands[0],
                    offset: field.offset,
                    guards: Box::new([]),
                    output: *output,
                })
            }
            Primitive::EnumPack => {
                if operands.len() != 2 {
                    return Err(backend("compact enum source arity differs"));
                }
                let ordinal = plan
                    .constants()
                    .iter()
                    .find_map(|c| {
                        if c.value() == operands[0] {
                            if let ValueData::Index(n) = c.constant().data() {
                                n.checked_sub(1).and_then(|n| usize::try_from(n).ok())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .ok_or_else(|| backend("compact enum requires an exact source ordinal"))?;
                let ty = layout.ty(output.get());
                let Kind::Tagged(variants) = &ty.kind else {
                    return Err(backend("compact enum source lacks a tagged result"));
                };
                let variant = variants
                    .get(ordinal)
                    .ok_or_else(|| backend("compact enum ordinal outside variants"))?;
                let payload = variant
                    .ty
                    .as_ref()
                    .ok_or_else(|| backend("compact enum pack requires its actual payload"))?;
                let inputs = words(layout, operands[1]);
                if inputs.len() != payload.words {
                    return Err(backend("compact enum payload width mismatch"));
                }
                let mut result = vec![Word::Constant(0); ty.words];
                result[0] = Word::Constant(ordinal as u64);
                result[variant.offset..variant.offset + payload.words].copy_from_slice(&inputs);
                Recipe::Aggregate(Aggregate::Pack {
                    output: *output,
                    words: result.into(),
                })
            }
            Primitive::SequenceLength => {
                if operands.len() != 1 {
                    return Err(backend("compact length source arity differs"));
                }
                // String length counts Unicode graphemes, rather than bytes.
                // Keep that separate lowering gap explicit until its complete
                // Unicode state machine is compiled natively.
                if matches!(layout.ty(operands[0].get()).kind, Kind::Utf8) {
                    return Ok(Recipe::Pending(Primitive::SequenceLength));
                }
                layout.ty(operands[0].get()).element()?;
                Recipe::Length {
                    source: operands[0],
                    output: *output,
                }
            }
            Primitive::MatrixScalarAccess => {
                if operands.len() != 2 {
                    return Err(backend("compact scalar access source arity differs"));
                }
                let element = layout.ty(operands[0].get()).element()?;
                check_width(layout, *output, element.words)?;
                Recipe::Read {
                    source: operands[0],
                    index: operands[1],
                    output: *output,
                }
            }
            operation @ (Primitive::MapNew
            | Primitive::MapInsert
            | Primitive::MapContainsKey
            | Primitive::MapKeyAccess
            | Primitive::StringHistogram
            | Primitive::StringMapKeys
            | Primitive::StringMapValues) => Recipe::Buffer {
                operation,
                operands: operands.clone(),
                output: *output,
            },
            operation @ (Primitive::SequenceWithCapacity
            | Primitive::SequenceReserve
            | Primitive::SequenceAppend
            | Primitive::SequenceAppendLeBytes
            | Primitive::SequenceAppendSublist
            | Primitive::SequenceClear
            | Primitive::MatrixIndexedReplace
            | Primitive::MatrixHorizontal(1..=2)
            | Primitive::MatrixConversion) => Recipe::Buffer {
                operation,
                operands: operands.clone(),
                output: *output,
            },
            primitive => Recipe::Pending(primitive),
        },
        Action::Structural {
            source,
            path,
            observation,
            output,
        } => {
            let mut ty: &Type = layout.ty(source.get());
            let mut offset = 0;
            let mut guards = vec![];
            for step in path {
                match *step {
                    Step::TupleItem { ordinal } => {
                        let field = ty
                            .fields()?
                            .get(ordinal as usize)
                            .ok_or_else(|| backend("compact tuple projection is outside fields"))?;
                        offset += field.offset;
                        ty = &field.ty;
                    }
                    Step::EnumPayload { ordinal } => {
                        let Kind::Tagged(variants) = &ty.kind else {
                            return Err(backend("compact enum projection lacks tagged input"));
                        };
                        let variant = variants.get(ordinal as usize).ok_or_else(|| {
                            backend("compact payload projection is outside variants")
                        })?;
                        guards.push((offset, ordinal));
                        offset += variant.offset;
                        ty = variant
                            .ty
                            .as_deref()
                            .ok_or_else(|| backend("compact selected enum payload is absent"))?;
                    }
                }
            }
            match observation {
                Observation::EnumOrdinal { ordinal } => {
                    if !matches!(ty.kind, Kind::Tagged(_)) {
                        return Err(backend("compact enum observation requires tag"));
                    }
                    Recipe::Aggregate(Aggregate::Observe {
                        source: *source,
                        offset,
                        ordinal: Some(*ordinal),
                        guards: guards.into(),
                        output: *output,
                    })
                }
                Observation::U64 | Observation::CanonicalValue => {
                    check_width(layout, *output, ty.words)?;
                    Recipe::Aggregate(Aggregate::Project {
                        source: *source,
                        offset,
                        guards: guards.into(),
                        output: *output,
                    })
                }
                Observation::StringEqual { peer } => {
                    if !matches!(ty.kind, Kind::Utf8)
                        || !matches!(layout.ty(peer.get()).kind, Kind::Utf8)
                    {
                        return Err(backend("native String observation requires UTF8 spans"));
                    }
                    check_width(layout, *output, 1)?;
                    Recipe::StringEqual {
                        source: *source,
                        offset,
                        peer: *peer,
                        guards: guards.into(),
                        negate: false,
                        output: *output,
                    }
                }
            }
        }
        _ => Recipe::None,
    };
    Ok(recipe)
}

//! Transfers reuse the canonical scalar and aggregate action vocabulary.
use super::*;
pub(super) fn aggregate(
    state: &mut State,
    terms: &mut Terms,
    layout: &NativeScalarAggregates,
    action: &Aggregate,
) -> Option<()> {
    let at = |source, offset| field(layout, source, offset);
    let (output, values) = match action {
        Aggregate::Pack { output, words } => (
            *output,
            words
                .iter()
                .map(|word| match word {
                    Word::Scalar(value) => state.read(value.get() as usize),
                    Word::Field(value, offset) => state.read(at(*value, *offset)?),
                    Word::Constant(word) => terms.constant(*word),
                })
                .collect::<Option<Vec<_>>>()?,
        ),
        Aggregate::Copy { source, output } => {
            let count = layout.slot(*output)?.1;
            (
                *output,
                (0..count)
                    .map(|offset| state.read(at(*source, offset)?))
                    .collect::<Option<Vec<_>>>()?,
            )
        }
        Aggregate::Project {
            source,
            offset,
            output,
            ..
        } => {
            let count = layout.slot(*output)?.1;
            (
                *output,
                (0..count)
                    .map(|i| state.read(at(*source, offset.checked_add(i)?)?))
                    .collect::<Option<Vec<_>>>()?,
            )
        }
        Aggregate::Observe {
            source,
            offset,
            ordinal,
            output,
            ..
        } => {
            let value = state.read(at(*source, *offset)?)?;
            let value = if let Some(ordinal) = ordinal {
                let tag = terms.constant(u64::from(*ordinal))?;
                terms.intern(Expr::Compare(Relation::Eq, value, tag))?
            } else {
                value
            };
            state.words.insert(output.get() as usize, value);
            return Some(());
        }
    };
    for (i, value) in values.into_iter().enumerate() {
        state.words.insert(at(output, i)?, value);
    }
    Some(())
}

pub(super) fn scalar(
    state: &mut State,
    terms: &mut Terms,
    site: usize,
    action: &Scalar,
) -> Option<()> {
    match action {
        Scalar::None => {}
        Scalar::Copy { source, output } => {
            state
                .words
                .insert(output.get() as usize, state.read(source.get() as usize)?);
        }
        Scalar::Primitive {
            op,
            operands,
            output,
        } => {
            let args = operands
                .iter()
                .map(|v| state.read(v.get() as usize))
                .collect::<Option<Vec<_>>>()?;
            let value = match *op {
                Op::U64WrappingAdd | Op::U64WrappingSub => {
                    let [a, b] = args.as_slice() else {
                        return None;
                    };
                    let arithmetic = if *op == Op::U64WrappingAdd {
                        Arithmetic::Add
                    } else {
                        Arithmetic::Sub
                    };
                    if state.safe(terms, arithmetic, *a, *b) {
                        terms.intern(Expr::Arithmetic(arithmetic, *a, *b))?
                    } else {
                        // A ring expression alone is never a mathematical
                        // range proof. It only preserves exact source identity.
                        terms.intern(Expr::Wrapping(arithmetic, *a, *b))?
                    }
                }
                Op::U8Add | Op::U32Add | Op::U32Sub | Op::U64Add | Op::U64Sub => {
                    // Facts describe the successful continuation of the source
                    // operation. A checked narrow result is the same exact
                    // mathematical sum/difference in our U64 fact domain.
                    // This does not prove that a later narrow operation fits:
                    // flow's arithmetic-guard removal remains U64-only.
                    let op = if matches!(op, Op::U8Add | Op::U32Add | Op::U64Add) {
                        Arithmetic::Add
                    } else {
                        Arithmetic::Sub
                    };
                    let [a, b] = args.as_slice() else {
                        return None;
                    };
                    if state.successful.len() == MAX_RELATIONS {
                        return None;
                    }
                    state.successful.insert((op, *a, *b));
                    terms.intern(Expr::Arithmetic(op, *a, *b))?
                }
                Op::U64Eq | Op::U64Ne | Op::U64Lt | Op::U64Le | Op::U64Gt | Op::U64Ge => {
                    let [mut a, mut b]: [Id; 2] = args.as_slice().try_into().ok()?;
                    let relation = match *op {
                        Op::U64Eq => Relation::Eq,
                        Op::U64Ne => Relation::Ne,
                        Op::U64Lt => Relation::Lt,
                        Op::U64Le => Relation::Le,
                        Op::U64Gt => {
                            core::mem::swap(&mut a, &mut b);
                            Relation::Lt
                        }
                        _ => {
                            core::mem::swap(&mut a, &mut b);
                            Relation::Le
                        }
                    };
                    terms.intern(Expr::Compare(relation, a, b))?
                }
                Op::BoolNot if terms.boolean(*args.first()?) => {
                    terms.intern(Expr::Not(*args.first()?))?
                }
                Op::U64BitAnd => {
                    use crate::native_artifact::program::prepare::word_collections::bounds::Range;
                    let bound = Range(0, state.upper(terms, args[0]))
                        .bit_and(Range(0, state.upper(terms, args[1])));
                    terms.intern(Expr::Bounded(site, output.get() as usize, bound.1))?
                }
                Op::BoolAnd | Op::BoolOr if args.iter().all(|&value| terms.boolean(value)) => {
                    let [a, b] = args.as_slice() else {
                        return None;
                    };
                    terms.intern(if *op == Op::BoolAnd {
                        Expr::And(*a, *b)
                    } else {
                        Expr::Or(*a, *b)
                    })?
                }
                Op::BoolXor if args.iter().all(|&value| terms.boolean(value)) => {
                    terms.intern(Expr::Boolean(site, output.get() as usize))?
                }
                _ if op.signature().1 == NativeScalarKind::Bool
                    && !matches!(op, Op::BoolNot | Op::BoolAnd | Op::BoolOr | Op::BoolXor) =>
                {
                    terms.intern(Expr::Boolean(site, output.get() as usize))?
                }
                _ => terms.intern(Expr::Definition(site, output.get() as usize))?,
            };
            if let Some(recording) = &mut terms.recording {
                recording.record(value, Node::Scalar(*op, args));
            }
            state.words.insert(output.get() as usize, value);
        }
    }
    Some(())
}

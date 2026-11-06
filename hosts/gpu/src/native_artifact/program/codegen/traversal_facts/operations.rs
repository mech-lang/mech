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
                Op::U64WrappingAdd => {
                    let [a, b] = args.as_slice() else {
                        return None;
                    };
                    if state.safe(terms, Arithmetic::Add, *a, *b) {
                        terms.intern(Expr::Arithmetic(Arithmetic::Add, *a, *b))?
                    } else {
                        terms.intern(Expr::Definition(site, output.get() as usize))?
                    }
                }
                Op::U64Add | Op::U64Sub => {
                    let op = if *op == Op::U64Add {
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
                _ => terms.intern(Expr::Definition(site, output.get() as usize))?,
            };
            state.words.insert(output.get() as usize, value);
        }
    }
    Some(())
}

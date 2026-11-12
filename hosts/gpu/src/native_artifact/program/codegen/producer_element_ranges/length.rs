//! A conditional result length, reconstructed from all successful paths of one
//! counted append loop. False-tag partial returns confer no length authority.
use super::*;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum Expr {
    Atom(Atom),
    Induction(u32, usize),
    Length(Ref),
    Add(Box<Expr>, Box<Expr>),
    Lt(Box<Expr>, Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Not(Box<Expr>),
    Unknown(usize, u32),
}
impl Expr {
    fn work(&self) -> Option<usize> {
        let mut work = 0usize;
        let mut pending = vec![self];
        while let Some(expr) = pending.pop() {
            work = work.checked_add(1)?;
            if work > 4096 {
                return None;
            }
            match expr {
                Self::Add(a, b) | Self::Lt(a, b) | Self::And(a, b) | Self::Or(a, b) => {
                    pending.push(a);
                    pending.push(b);
                }
                Self::Not(a) => pending.push(a),
                _ => (),
            }
        }
        Some(work)
    }
    fn word(v: u64) -> Self {
        Self::Atom(Atom::Word(v))
    }
    fn add(a: Self, b: Self) -> Option<Self> {
        match (&a, &b) {
            (Self::Atom(Atom::Word(0)), _) => Some(b),
            (_, Self::Atom(Atom::Word(0))) => Some(a),
            (Self::Atom(Atom::Word(a)), Self::Atom(Atom::Word(b))) => {
                Some(Self::word(a.checked_add(*b)?))
            }
            _ => Some(Self::Add(Box::new(a), Box::new(b))),
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
enum Value {
    Scalar(Expr),
    Array(Expr),
    Tuple(Vec<Value>),
    Unknown,
}
impl Value {
    fn work(&self) -> Option<usize> {
        let mut work = 0usize;
        let mut pending = vec![self];
        while let Some(value) = pending.pop() {
            work = work.checked_add(1)?;
            if work > 4096 {
                return None;
            }
            match value {
                Self::Scalar(expr) | Self::Array(expr) => work = work.checked_add(expr.work()?)?,
                Self::Tuple(fields) => pending.extend(fields),
                _ => (),
            }
        }
        Some(work)
    }
    fn item(&self, path: &[usize]) -> Option<&Self> {
        let mut value = self;
        for &field in path {
            let Self::Tuple(fields) = value else {
                return None;
            };
            value = fields.get(field)?;
        }
        Some(value)
    }
    fn scalar(&self) -> Option<&Expr> {
        if let Self::Scalar(v) = self {
            Some(v)
        } else {
            None
        }
    }
}
type Predicates = BTreeMap<Expr, bool>;
type Env = BTreeMap<u32, Value>;
fn truth(expr: &Expr, predicates: &Predicates, ignore: Option<&Expr>) -> Option<bool> {
    let result = match expr {
        Expr::Atom(Atom::Bool(v)) => Some(*v),
        Expr::Not(v) => truth(v, predicates, ignore).map(|v| !v),
        Expr::And(a, b) => match (truth(a, predicates, ignore), truth(b, predicates, ignore)) {
            (Some(false), _) | (_, Some(false)) => Some(false),
            (Some(true), Some(true)) => Some(true),
            _ => None,
        },
        Expr::Or(a, b) => match (truth(a, predicates, ignore), truth(b, predicates, ignore)) {
            (Some(true), _) | (_, Some(true)) => Some(true),
            (Some(false), Some(false)) => Some(false),
            _ => None,
        },
        _ => None,
    };
    result.or_else(|| {
        if Some(expr) == ignore {
            None
        } else {
            predicates.get(expr).copied()
        }
    })
}
fn assume(predicates: &Predicates, expr: Expr, want: bool) -> Option<Predicates> {
    if let Some(old) = truth(&expr, predicates, None) {
        return (old == want).then(|| predicates.clone());
    }
    let mut predicates = predicates.clone();
    predicates.insert(expr, want);
    if predicates
        .iter()
        .any(|(key, &value)| truth(key, &predicates, Some(key)).is_some_and(|v| v != value))
    {
        return None;
    }
    Some(predicates)
}
fn reference(graph: &Graph, r: &Ref) -> Option<Value> {
    Some(match graph.ty(r)? {
        Ty::Array => Value::Array(Expr::Length(r.clone())),
        Ty::Tuple(fields) => Value::Tuple(
            (0..fields.len())
                .map(|i| {
                    let mut child = r.clone();
                    child.1.push(i);
                    reference(graph, &child)
                })
                .collect::<Option<Vec<_>>>()?,
        ),
        Ty::Word | Ty::Bool => Value::Scalar(Expr::Atom(
            graph.atom(r).unwrap_or_else(|| Atom::Source(r.clone())),
        )),
        _ => Value::Unknown,
    })
}
fn get(graph: &Graph, env: &Env, value: u32) -> Option<Value> {
    graph.charge(1)?;
    if let Some(value) = env.get(&value) {
        graph.charge(value.work()?)?;
        Some(value.clone())
    } else {
        reference(graph, &Ref::value(value))
    }
}
fn action(plan: &Plan, graph: &Graph, site: usize, env: &mut Env) -> Option<()> {
    match plan.blocks()[site].action() {
        Action::Scalar(Scalar::None) => (),
        Action::Scalar(Scalar::Copy { source, output }) => {
            env.insert(output.get(), get(graph, env, source.get())?);
        }
        Action::Structural {
            source,
            path,
            output,
            ..
        } => {
            let path = path
                .iter()
                .map(|step| match step {
                    NativeStructuralStep::TupleItem { ordinal } => Some(*ordinal as usize),
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()?;
            let value = get(graph, env, source.get())?.item(&path)?.clone();
            env.insert(output.get(), value);
        }
        Action::Scalar(Scalar::Primitive {
            op,
            operands,
            output,
        }) => {
            let args = operands
                .iter()
                .map(|v| Some(get(graph, env, v.get())?.scalar()?.clone()))
                .collect::<Option<Vec<_>>>()?;
            let result = match (op, args.as_slice()) {
                (op, [a]) if unsigned_cast(*op) => a.clone(), // source checked cast still executes
                (Op::U64Add, [a, b]) => Expr::add(a.clone(), b.clone())?,
                (Op::U64Lt, [a, b]) => Expr::Lt(Box::new(a.clone()), Box::new(b.clone())),
                (Op::BoolAnd, [a, b]) => Expr::And(Box::new(a.clone()), Box::new(b.clone())),
                (Op::BoolOr, [a, b]) => Expr::Or(Box::new(a.clone()), Box::new(b.clone())),
                (Op::BoolNot, [a]) => Expr::Not(Box::new(a.clone())),
                _ => Expr::Unknown(site, output.get()),
            };
            env.insert(output.get(), Value::Scalar(result));
        }
        Action::LibraryCall {
            template,
            operands,
            output,
        } => {
            let args = operands
                .iter()
                .map(|v| get(graph, env, v.get()))
                .collect::<Option<Vec<_>>>()?;
            let result = match (template.primitive(), args.as_slice()) {
                (Primitive::CompositePack(n), _) if usize::try_from(n).ok() == Some(args.len()) => {
                    Value::Tuple(args)
                }
                (Primitive::SequenceAppend, [Value::Array(length), _]) => {
                    Value::Array(Expr::add(length.clone(), Expr::word(1))?)
                }
                (Primitive::SequenceWithCapacity, _) => Value::Array(Expr::word(0)),
                (Primitive::SequenceLength, [Value::Array(length)]) => {
                    Value::Scalar(length.clone())
                }
                _ => reference(graph, &Ref::value(output.get()))?,
            };
            env.insert(output.get(), result);
        }
        _ => return None,
    }
    Some(())
}
struct End {
    site: usize,
    env: Env,
    predicates: Predicates,
}
fn paths(
    plan: &Plan,
    graph: &Graph,
    tail: &Tail,
    induction: usize,
    array: usize,
    limit: usize,
) -> Option<(Expr, Expr, Vec<End>)> {
    let i = Expr::Induction(tail.carrier, induction);
    let bound = Expr::Atom(graph.atom(&Ref::field(tail.carrier, limit))?);
    let Value::Tuple(mut fields) = reference(graph, &Ref::value(tail.carrier))? else {
        return None;
    };
    *fields.get_mut(induction)? = Value::Scalar(i.clone());
    *fields.get_mut(array)? = Value::Array(i.clone());
    *fields.get_mut(limit)? = Value::Scalar(bound.clone());
    let mut pending = vec![(
        tail.body,
        BTreeMap::from([(tail.carrier, Value::Tuple(fields))]),
        BTreeMap::new(),
        BTreeSet::new(),
    )];
    let mut complete = vec![];
    let mut visits = 0usize;
    while let Some((site, mut env, predicates, mut seen)) = pending.pop() {
        graph.charge(
            env.len()
                .checked_add(predicates.len())?
                .checked_add(seen.len())?
                .checked_add(1)?,
        )?;
        visits = visits.checked_add(1)?;
        if visits > 4096 {
            return None;
        }
        if site == tail.latch || site == tail.finish {
            complete.push(End {
                site,
                env,
                predicates,
            });
            continue;
        }
        if !seen.insert(site) {
            return None;
        }
        if let Action::Initialize { iteration } = plan.blocks()[site].action() {
            let child = graph.tail(*iteration)?;
            if child.parent != Some(tail.id) {
                return None;
            }
            // Child output is arbitrary. Canonical immutable value scopes
            // prevent it from redefining any parent local/carrier home.
            env.insert(
                child.returned,
                reference(graph, &Ref::value(child.returned))?,
            );
            env.insert(child.result, reference(graph, &Ref::value(child.result))?);
            pending.push((child.exit, env, predicates, seen));
            continue;
        }
        action(plan, graph, site, &mut env)?;
        match plan.blocks()[site].terminator() {
            Term::Scalar(Branch::Reject | Branch::IntegrityFailure) => (),
            Term::Scalar(Branch::Jump(next)) => {
                pending.push((next as usize, env, predicates, seen))
            }
            Term::Scalar(Branch::LiteralBranch {
                left,
                right,
                yes,
                no,
            }) => {
                let left = get(graph, &env, left.get())?.scalar()?.clone();
                let right = get(graph, &env, right.get())?;
                let Expr::Atom(Atom::Bool(want)) = right.scalar()? else {
                    return None;
                };
                for (expected, next) in [(*want, yes), (!want, no)] {
                    if let Some(selected) = assume(&predicates, left.clone(), expected) {
                        let mut work = seen.len().checked_add(1)?;
                        for value in env.values() {
                            work = work.checked_add(value.work()?)?;
                        }
                        for key in selected.keys() {
                            work = work.checked_add(key.work()?)?;
                        }
                        graph.charge(work)?;
                        pending.push((next as usize, env.clone(), selected, seen.clone()));
                    }
                }
            }
            _ => return None,
        }
    }
    Some((i, bound, complete))
}
fn empty(graph: &Graph, r: &Ref) -> bool {
    let Some(alternatives) = graph.resolve(r, &BTreeSet::new()) else {
        return false;
    };
    !alternatives.is_empty()
        && alternatives.iter().all(|atom| {
            let Atom::Source(r) = atom else { return false };
            r.1.is_empty()
                && matches!(
                    graph.definition(r.0),
                    Some((_, Def::Library(Primitive::SequenceWithCapacity, _)))
                )
        })
}
pub(super) struct Fact {
    pub result: u32,
    pub array: usize,
    pub success: usize,
    pub bound: Ref,
}
pub(super) fn derive(plan: &Plan, graph: &Graph) -> Vec<Fact> {
    let mut facts = vec![];
    for tail in graph.tails.values() {
        if graph.charge(1).is_none() {
            return vec![];
        }
        let Ty::Tuple(fields) = &graph.types[tail.carrier as usize] else {
            continue;
        };
        // Flat carriers suffice for this first general counted-append rule.
        if fields.iter().any(|v| matches!(v, Ty::Tuple(_))) {
            continue;
        }
        let mut guards = BTreeSet::new();
        for block in plan.blocks() {
            if graph.charge(1).is_none() {
                return vec![];
            }
            if block.iteration() != Some(tail.id) {
                continue;
            }
            let Action::Scalar(Scalar::Primitive {
                op: Op::U64Lt,
                operands,
                ..
            }) = block.action()
            else {
                continue;
            };
            if operands.len() != 2 {
                continue;
            }
            let Some(Atom::Source(x)) = graph.atom(&Ref::value(operands[0].get())) else {
                continue;
            };
            if x.0 != tail.carrier || x.1.len() != 1 {
                continue;
            }
            let bound = graph.atom(&Ref::value(operands[1].get()));
            for (limit, kind) in fields.iter().enumerate() {
                let r = Ref::field(tail.carrier, limit);
                if kind == &Ty::Word && graph.invariant.contains_key(&r) && graph.atom(&r) == bound
                {
                    guards.insert((x.1[0], limit));
                }
            }
        }
        for (induction, limit) in guards {
            if graph.charge(1).is_none() {
                return vec![];
            }
            if graph.atom(&Ref::field(tail.seed, induction)) != Some(Atom::Word(0)) {
                continue;
            }
            for (array, kind) in fields.iter().enumerate() {
                if graph.charge(1).is_none() {
                    return vec![];
                }
                if kind != &Ty::Array || !empty(graph, &Ref::field(tail.seed, array)) {
                    continue;
                }
                let Some((i, bound, ends)) = paths(plan, graph, tail, induction, array, limit)
                else {
                    continue;
                };
                let Some(increment) = Expr::add(i.clone(), Expr::word(1)) else {
                    continue;
                };
                let guard = Expr::Lt(Box::new(i.clone()), Box::new(bound.clone()));
                let latches = ends
                    .iter()
                    .filter(|end| end.site == tail.latch)
                    .collect::<Vec<_>>();
                if latches.is_empty()
                    || latches.iter().any(|end| {
                        let Some(next) = get(graph, &end.env, tail.next) else {
                            return true;
                        };
                        next.item(&[induction]) != Some(&Value::Scalar(increment.clone()))
                            || next.item(&[array]) != Some(&Value::Array(increment.clone()))
                            || truth(&guard, &end.predicates, None) != Some(true)
                    })
                {
                    continue;
                }
                let returns = ends
                    .iter()
                    .filter(|end| end.site == tail.finish)
                    .collect::<Vec<_>>();
                if returns.is_empty() {
                    continue;
                }
                let Ty::Tuple(result_fields) = &graph.types[tail.returned as usize] else {
                    continue;
                };
                for (out_array, kind) in result_fields.iter().enumerate() {
                    if kind != &Ty::Array {
                        continue;
                    }
                    for (success, kind) in result_fields.iter().enumerate() {
                        if kind != &Ty::Bool {
                            continue;
                        }
                        let mut successes = 0;
                        let valid = returns.iter().all(|end| {
                            let Some(result) = get(graph, &end.env, tail.returned) else {
                                return false;
                            };
                            let Some(flag) = result.item(&[success]).and_then(Value::scalar) else {
                                return false;
                            };
                            match truth(flag, &end.predicates, None) {
                                Some(false) => true,
                                Some(true) => {
                                    successes += 1;
                                    result.item(&[out_array]) == Some(&Value::Array(i.clone()))
                                        && truth(&guard, &end.predicates, None) == Some(false)
                                }
                                None => false,
                            }
                        });
                        if valid
                            && successes > 0
                            && let Expr::Atom(Atom::Source(bound)) = &bound
                        {
                            facts.push(Fact {
                                result: tail.result,
                                array: out_array,
                                success,
                                bound: bound.clone(),
                            });
                        }
                    }
                }
            }
        }
    }
    facts
}

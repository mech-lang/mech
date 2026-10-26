//! A fixed-width key candidate derived from the original checked U64 additions.
//! This proof neither admits storage nor removes endpoint arithmetic. Runtime
//! entry must snapshot both invariant offsets, check the maintained range shape,
//! and decline unsupported widths before selecting a packed representation.
use super::iteration_abi::NativeIterationVisit;
use mech_core::{IntegerWidth, SchemaBody};
use mech_engine::native_artifact::{
    NativeCollectionPrimitive as Primitive, NativeControlLoop, NativeIterationAction as Action,
    NativeIterationValueScope as Scope, NativeMapOwnerWords, NativeMatchAction as Scalar,
    NativeMatchValue as Value, NativeProgramArtifactPlan as Plan, NativeScalarOp as Op,
};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Proof {
    // These indices come only from derive or the enclosing unsafe build-owned
    // descriptor reader. No public source/image constructor grants this proof.
    start_offset: Option<u32>,
    end_offset: Option<u32>,
    inclusive: bool,
    range_site: u32,
    selection_site: u32,
    domain: Option<super::program_key_domain::Domain>,
}
impl Proof {
    /// None denotes the literal zero in an unmodified base endpoint.
    pub fn start_offset(&self) -> Option<u32> {
        self.start_offset
    }
    pub fn end_offset(&self) -> Option<u32> {
        self.end_offset
    }
    pub fn inclusive(&self) -> bool {
        self.inclusive
    }
    #[cfg(any(test, feature = "canonical-native-program"))]
    pub fn range_site(&self) -> u32 {
        self.range_site
    }
    #[cfg(all(test, feature = "canonical-native-program"))]
    pub fn selection_site(&self) -> u32 {
        self.selection_site
    }
    #[cfg(test)]
    pub fn domain(&self) -> Option<super::program_key_domain::Domain> {
        self.domain
    }
    pub fn packing_domain(&self) -> super::program_key_domain::Domain {
        self.domain
            .unwrap_or_else(super::program_key_domain::Domain::ascii)
            .packing()
    }
}

fn u64_value(plan: &Plan, value: Value) -> bool {
    plan.values().get(value.get() as usize).is_some_and(|ty| {
        ty.definition().dimension_parameters().is_empty()
            && ty.definition().body() == &SchemaBody::UnsignedInteger(IntegerWidth::W64)
    })
}

fn strict_ancestor(plan: &Plan, scope: Option<u32>, iteration: u32) -> bool {
    if scope == Some(iteration) {
        return false;
    }
    let Some(control) = plan.iterations().get(iteration as usize) else {
        return false;
    };
    let mut parent = control.as_ref().parent();
    // Bounds malformed ancestry even though the enclosing plan was validated.
    for _ in 0..=plan.iterations().len() {
        if parent == scope {
            return true;
        }
        let Some(id) = parent else {
            return false;
        };
        let Some(control) = plan.iterations().get(id as usize) else {
            return false;
        };
        if id == iteration {
            return false;
        }
        parent = control.as_ref().parent();
    }
    false
}

fn output(plan: &Plan, action: &Action) -> Option<Value> {
    match action {
        Action::Scalar(Scalar::Primitive { output, .. } | Scalar::Copy { output, .. })
        | Action::LibraryCall { output, .. }
        | Action::Structural { output, .. } => Some(*output),
        Action::CollectionElement { iteration } => {
            match plan.iterations().get(*iteration as usize)? {
                NativeControlLoop::Collection(collection) => collection.element(),
                _ => None,
            }
        }
        _ => None,
    }
}

fn definition(plan: &Plan, value: Value) -> Option<usize> {
    let mut sites = plan
        .blocks()
        .iter()
        .enumerate()
        .filter_map(|(site, block)| (output(plan, block.action()) == Some(value)).then_some(site));
    let site = sites.next()?;
    sites.next().is_none().then_some(site)
}

fn reachable(
    visits: &[NativeIterationVisit],
    entry: u32,
    target: u32,
    omit: Option<usize>,
) -> bool {
    let mut seen = vec![false; visits.len()];
    let mut pending = vec![entry];
    while let Some(site) = pending.pop() {
        let site = site as usize;
        if Some(site) == omit {
            continue;
        }
        let Some(seen) = seen.get_mut(site) else {
            return false;
        };
        if core::mem::replace(seen, true) {
            continue;
        }
        if site == target as usize {
            return true;
        }
        pending.extend(visits[site].successors.into_iter().flatten());
    }
    false
}

fn dominates(
    visits: &[NativeIterationVisit],
    entry: u32,
    definition: usize,
    use_site: u32,
) -> bool {
    definition != use_site as usize
        && reachable(visits, entry, use_site, None)
        && !reachable(visits, entry, use_site, Some(definition))
}

pub(super) fn invariant(
    plan: &Plan,
    visits: &[NativeIterationVisit],
    value: Value,
    iteration: u32,
) -> bool {
    if !u64_value(plan, value) {
        return false;
    }
    let Some(Scope::Immutable { iteration: scope }) = plan.value_scopes().get(value.get() as usize)
    else {
        return false;
    };
    if !strict_ancestor(plan, *scope, iteration) {
        return false;
    }
    if plan.inputs().iter().any(|input| input.value() == value)
        || plan
            .constants()
            .iter()
            .any(|constant| constant.value() == value)
    {
        return scope.is_none();
    }
    let Some(site) = definition(plan, value) else {
        return false;
    };
    let Some(control) = plan.iterations().get(iteration as usize) else {
        return false;
    };
    // Lexical ancestry alone is insufficient: the producer must be completed
    // on every path into THIS invocation, not merely belong to an outer scope.
    plan.blocks()[site].iteration() == *scope
        && dominates(visits, plan.entry(), site, control.as_ref().initialize())
}

fn endpoints(
    plan: &Plan,
    visits: &[NativeIterationVisit],
    value: Value,
    iteration: u32,
    range_site: u32,
) -> Vec<(Value, Option<Value>)> {
    if !u64_value(plan, value) {
        return Vec::new();
    }
    let mut candidates = vec![(value, None)];
    let Some(site) = definition(plan, value) else {
        return candidates;
    };
    let Action::Scalar(Scalar::Primitive {
        op: Op::U64Add,
        operands,
        output,
    }) = plan.blocks()[site].action()
    else {
        return candidates;
    };
    if *output != value
        || operands.len() != 2
        || !dominates(visits, plan.entry(), site, range_site)
        || operands.iter().any(|&operand| !u64_value(plan, operand))
    {
        return candidates;
    }
    // U64Add is commutative, but its checked source operation still executes.
    for (base, offset) in [(operands[0], operands[1]), (operands[1], operands[0])] {
        if invariant(plan, visits, offset, iteration) {
            candidates.push((base, Some(offset)));
        }
    }
    candidates
}

/// Derive from the validated same-plan owner/span inventory, never spelling,
/// constant key contents, an expected result, or a benchmark-specific width.
pub(super) fn derive(
    plan: &Plan,
    owners: &NativeMapOwnerWords,
    owner: usize,
    visits: &[NativeIterationVisit],
) -> Option<Proof> {
    if visits.len() != plan.blocks().len() {
        return None;
    }
    let region = owners.regions().get(owner)?;
    let mut spans = owners.spans().iter().filter(|span| span.region() == owner);
    let span = spans.next()?;
    if spans.next().is_some() {
        return None;
    }
    let Action::LibraryCall {
        template, operands, ..
    } = plan.blocks().get(span.range_site() as usize)?.action()
    else {
        return None;
    };
    let inclusive = match template.primitive() {
        Primitive::RangeInclusive => true,
        Primitive::RangeExclusive => false,
        _ => return None,
    };
    if operands.as_ref() != span.operands().as_slice() {
        return None;
    }
    let start = endpoints(
        plan,
        visits,
        operands[0],
        region.iteration(),
        span.range_site(),
    );
    let end = endpoints(
        plan,
        visits,
        operands[1],
        region.iteration(),
        span.range_site(),
    );
    for &(base, start_offset) in &start {
        if let Some(&(_, end_offset)) = end.iter().find(|&&(other, _)| other == base) {
            return Some(Proof {
                start_offset: start_offset.map(Value::get),
                end_offset: end_offset.map(Value::get),
                inclusive,
                range_site: span.range_site(),
                selection_site: span.selection_site(),
                domain: super::program_key_domain::derive(plan, span.source()),
            });
        }
    }
    None
}

#[cfg(all(test, feature = "canonical-native-program"))]
mod tests {
    use super::*;
    use mech_syntax::document::{
        AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
        parse_canonical_document,
    };

    const COUNT: &str = r#"
+> map
increment(h<{string:u64}>, key<string>) => (out<{string:u64}>) :=
  previous := (key ∈ h)? | true => h{key} | false => 0u64.
  out := map/insert(h, key, previous + 1u64).
#Count(s<string>, width<u64>, n<u64>, initial<{string:u64}>) => <{string:u64}>
  | :Next(index<u64>, counts<{string:u64}>)
  | :Done(counts<{string:u64}>).
#Count(s, width, n, initial) -> :Next(0u64, initial)
  :Next(index, counts)
    | width == 0u64 -> :Done(counts)
    | index >= n -> :Done(counts)
    | width > n - index -> :Done(counts)
    | * -> :Next(index + 1u64, increment(counts, s[(index + 1u64)..=(index + width)]))
  :Done(counts) => counts.
empty<{string:u64}> := map/new()
"#;

    fn each_plan(
        source: &str,
        mut check: impl FnMut(&Plan, &NativeMapOwnerWords, &[NativeIterationVisit]),
    ) {
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(0x5041434b), Revision(1), source).unwrap(),
            ParseConfig::default(),
        );
        assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
        let catalog = mech_stdlib::source_catalog();
        let original = mech_engine::CanonicalSourceFrontend
            .compile_document_with_catalog(
                &DocumentSyntax::cast(parsed.syntax()).unwrap(),
                catalog.clone(),
            )
            .unwrap()
            .compile_artifact()
            .unwrap();
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(
            &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
        )
        .unwrap();
        for artifact in [original, decoded] {
            let plan =
                mech_engine::native_artifact::lower_native_program_artifact(&artifact, &catalog)
                    .unwrap();
            let owners = plan.map_owner_words().unwrap();
            let visits = crate::native_artifact::iteration_abi::control_visits(
                plan.blocks(),
                plan.iterations().iter().map(Into::into),
                0,
            )
            .unwrap();
            check(&plan, &owners, &visits);
        }
    }

    #[test]
    fn packed_width_requires_same_base_and_invariant_offsets() {
        for (start, end, expected) in [
            ("(index + 1u64)", "(index + width)", true),
            ("(1u64 + index)", "(width + index)", true),
            ("index", "(index + width)", true),
            ("(index + 1u64)", "(index + index + 1u64)", false),
            ("(index + 1u64)", "(index + (width + index))", false),
            ("(index + 1u64)", "(width + width)", false),
        ] {
            let source = format!(
                "{}\n#Count(text<string>, width<u64>, n<u64>, empty)\n",
                COUNT.replace(
                    "s[(index + 1u64)..=(index + width)]",
                    &format!("s[{start}..={end}]")
                )
            );
            each_plan(&source, |plan, owners, visits| {
                assert_eq!(owners.regions().len(), 1);
                let proof = derive(plan, owners, 0, visits);
                assert_eq!(proof.is_some(), expected, "{start}..={end}");
                if let Some(proof) = proof {
                    assert!(proof.inclusive());
                    assert_eq!(proof.range_site(), owners.spans()[0].range_site());
                    assert_eq!(proof.selection_site(), owners.spans()[0].selection_site());
                    if start == "index" {
                        assert!(proof.start_offset().is_none());
                    }
                    assert!(proof.end_offset().is_some());
                    let image = serde_json::to_vec(&proof).unwrap();
                    let restored: Proof = serde_json::from_slice(&image).unwrap();
                    assert_eq!(serde_json::to_vec(&restored).unwrap(), image);
                }
            });
        }
    }

    #[test]
    fn packed_width_accepts_current_ancestor_element_and_excludes_child_scope() {
        let source = format!(
            r#"{COUNT}
+> number
+> text
+> sequence
count-text(width<u64>) => (output<string>) :=
  h := #Count("aaaaaa", width, 6u64, empty)
  output := number/format(sequence/length(map/values(h)), 0u64).
text/join([count-text(width) | width <- [1u64 2u64]], "")
"#
        );
        each_plan(&source, |plan, owners, visits| {
            assert_eq!(owners.regions().len(), 1);
            let owner = &owners.regions()[0];
            let parent = owner.tail().parent().expect("enclosing comprehension");
            assert!(derive(plan, owners, 0, visits).is_some());
            assert!(strict_ancestor(plan, Some(parent), owner.iteration()));
            assert!(!strict_ancestor(
                plan,
                Some(owner.iteration()),
                owner.iteration()
            ));
            assert!(!strict_ancestor(plan, Some(owner.iteration()), parent));
        });
    }

    #[test]
    fn packed_offset_producer_must_dominate_entry_on_every_branch() {
        let source = format!("{COUNT}\n#Count(text<string>, width<u64>, n<u64>, empty)\n");
        each_plan(&source, |_, _, original| {
            let mut visits = vec![original[0]; 4];
            visits[0].successors = [Some(1), Some(2)];
            visits[1].successors = [Some(3), None];
            visits[2].successors = [Some(3), None];
            visits[3].successors = [None, None];
            assert!(!dominates(&visits, 0, 1, 3));
            assert!(!dominates(&visits, 0, 2, 3));
            assert!(dominates(&visits, 0, 0, 3));
            assert!(!dominates(&visits, 0, 3, 3));
            visits[0].successors = [Some(1), None];
            assert!(dominates(&visits, 0, 1, 3));
            assert!(!dominates(&visits, 0, 2, 3));
        });
    }

    #[test]
    fn packed_domain_comes_from_source_map_values_and_join_separator() {
        for (mapping, separator, symbols) in [
            (
                r#"{"a": "A", "c": "C", "g": "G", "t": "T"}"#,
                "",
                Some("ACGT"),
            ),
            (
                r#"{"a": "A", "c": "C", "g": "G", "t": "T"}"#,
                ":",
                Some(":ACGT"),
            ),
            (r#"{"a": "ab", "b": "bc"}"#, "", Some("abc")),
            (r#"{"a": "é", "b": "b"}"#, "", None),
            (r#"{"a": replacement<string>, "b": "b"}"#, "", None),
        ] {
            let source = format!(
                r#"{COUNT}
+> text
mapping := {mapping}
copy := mapping
sequence := text/join([copy{{char}} | char <- input<string>], "{separator}")
#Count(sequence, 2u64, 4u64, empty)
"#
            );
            each_plan(&source, |plan, owners, visits| {
                let domain = derive(plan, owners, 0, visits).unwrap().domain();
                match symbols {
                    Some(symbols) => {
                        let domain = domain.expect("proved finite output alphabet");
                        for byte in 0..128u8 {
                            assert_eq!(domain.contains(byte), symbols.as_bytes().contains(&byte));
                        }
                        if symbols == "ACGT" {
                            assert_eq!((domain.bits(), domain.shift()), (2, Some(1)));
                        }
                    }
                    None => assert!(domain.is_none()),
                }
            });
        }
    }
}

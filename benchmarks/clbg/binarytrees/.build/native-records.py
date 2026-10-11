from pathlib import Path
p=Path('src/engine/src/native_sequence/control.rs');s=p.read_text();s=s.replace('    StringLiteralSyntax, SyntaxKind, SyntaxNode, TextSnapshot, TupleDestructureSyntax,','    KindAnnotationSyntax, KindDefineSyntax, KindValueSyntax, RecordSyntax, SliceSyntax,\n    StringLiteralSyntax, SyntaxKind, SyntaxNode, TextSnapshot, TupleDestructureSyntax,')
s=s.replace('    Tuple(Vec<Kind>),','    Tuple(Vec<Kind>),\n    Record(Vec<(String, Kind)>),\n    Type(Box<Kind>),',1)
s=s.replace('    Text(String),','    Text(String),\n    Type,',1)
s=s.replace('        arity: usize,\n    },\n    Arena', '        arity: usize,\n        layout: Option<Kind>,\n    },\n    Arena',1)
s=s.replace('        arity: usize,\n        function: String,','        arity: usize,\n        layout: Kind,\n        function: String,',1)
a=s.index('fn parameter(');b=s.index('    let kind = match s.as_str()',a);s=s[:a]+s[a:b].replace('fn parameter(node: &SyntaxNode)', 'fn parameter(node: &SyntaxNode, aliases: &BTreeMap<String, SyntaxNode>)')+s[b:]
s=s.replace('    let s = text(&annotation)?.replace(\' \', "");\n    let kind = match s.as_str() {','    let kind = native_kind(&annotation, aliases, &mut BTreeSet::new())?;\n    Ok((text(&name)?, kind))\n}\nfn native_kind(annotation: &SyntaxNode, aliases: &BTreeMap<String, SyntaxNode>, active: &mut BTreeSet<String>) -> Result<Kind, String> {\n    let view = KindAnnotationSyntax::cast(annotation.clone()).ok_or("kind annotation")?;\n    let with_option = view.kind().ok_or("kind")?;\n    if with_option.question_mark().is_some() { return Err("native options require lowering support".into()); }\n    if let Some(KindValueSyntax::Record(record)) = with_option.kind().and_then(|k| k.value()) {\n        let names = record.fields();\n        let kinds = record.field_kinds();\n        if names.len() != kinds.len() { return Err("record field annotations required".into()); }\n        let mut seen = BTreeSet::new();\n        let fields = names.iter().zip(kinds).map(|(name, kind)| {\n            let name = text(name.syntax())?;\n            if !seen.insert(name.clone()) { return Err("duplicate record field".into()); }\n            Ok((name, native_kind(kind.syntax(), aliases, active)?))\n        }).collect::<Result<Vec<_>, String>>()?;\n        return Ok(Kind::Record(fields));\n    }\n    let s = text(annotation)?.replace(\' \', "");\n    if let Some(name) = s.strip_prefix(\'<\').and_then(|s| s.strip_suffix(\'>\')) {\n        if let Some(def) = aliases.get(name) {\n            if !active.insert(name.to_owned()) { return Err("recursive kind aliases require explicit managed links".into()); }\n            let kind = native_kind(def, aliases, active);\n            active.remove(name);\n            return kind;\n        }\n    }\n    let kind = match s.as_str() {',1)
s=s.replace('    Ok((text(&name)?, kind))\n}\nfn args', '    Ok(kind)\n}\nfn args',1)
s=s.replace('    source: &\'a BTreeMap<String, SourceFunction>,','    source: &\'a BTreeMap<String, SourceFunction>,\n    aliases: &\'a BTreeMap<String, SyntaxNode>,',1)
# Record literals, kinds-as-values, and ordinary dot fields.
marker='        if node.kind() == SyntaxKind::Tuple {'
extra='''        if let Some(annotation) = KindAnnotationSyntax::cast(node.clone()) {
            return Ok(Expr { kind: Kind::Type(Box::new(native_kind(annotation.syntax(), self.aliases, &mut BTreeSet::new())?)), op: Op::Type });
        }
        if let Some(record) = RecordSyntax::cast(node.clone()) {
            let mut fields = Vec::new();
            let mut values = Vec::new();
            let mut seen = BTreeSet::new();
            for binding in record.bindings() {
                let name = text(binding.name().ok_or("record field name")?.syntax())?;
                if !seen.insert(name.clone()) { return Err("duplicate record field".into()); }
                let value = self.expr(binding.value().ok_or("record field value")?.syntax(), vars)?;
                if let Some(annotation) = binding.annotation() {
                    if native_kind(annotation.syntax(), self.aliases, &mut BTreeSet::new())? != value.kind {
                        return Err("record field annotation mismatch".into());
                    }
                }
                fields.push((name, value.kind.clone()));
                values.push(value);
            }
            return Ok(Expr { kind: Kind::Record(fields), op: Op::Tuple(values) });
        }
        if let Some(slice) = SliceSyntax::cast(node.clone()) {
            let mut value = self.expr(slice.stem().ok_or("field stem")?.syntax(), vars)?;
            for item in slice.subscripts().ok_or("field selectors")?.items() {
                let selection = text(item.syntax())?;
                let name = selection.trim().strip_prefix('.').ok_or("native record requires dot fields")?;
                let Kind::Record(fields) = &value.kind else { return Err("dot field requires record".into()); };
                let index = fields.iter().position(|(field, _)| field == name).ok_or("unknown record field")?;
                value = Expr { kind: fields[index].1.clone(), op: Op::Field(Box::new(value), index) };
            }
            return Ok(value);
        }
'''
s=s.replace(marker,extra+marker,1)
s=s.replace('if name == "memory/with-ref-arena" || name == "memory/with-frame"','if matches!(name.as_str(), "memory/with-ref-arena" | "memory/with-arena" | "memory/with-frame")',1)
s=s.replace('let region = name == "memory/with-ref-arena";','let region = name != "memory/with-frame";')
a=s.index('                let arity = if region {');b=s.index('                if capacity.as_ref()',a)
s=s[:a]+'''                let layout = if region {
                    let e = self.expr(&a[1], vars)?;
                    if name == "memory/with-arena" {
                        let Kind::Type(layout) = e.kind else { return Err("arena record kind required".into()); };
                        record_lanes(&layout)?;
                        *layout
                    } else {
                        let Op::Integer(n) = e.op else { return Err("arena arity must be literal".into()); };
                        if !(1..=32).contains(&n) { return Err("arena arity 1..32".into()); }
                        Kind::Tuple(vec![Kind::Reference; n as usize])
                    }
                } else { Kind::Tuple(vec![]) };
                let arity = scalar_lanes(&layout).len();
'''+s[b:]
s=s.replace('                    || f.inputs.iter().any(|k| k != &Kind::Integer)\n                    || flat(&f.output.kind).iter().any(|k| k != &Kind::Integer)','                    || f.inputs.len() > 32\n                    || f.inputs.iter().any(|k| !scope_scalar(k))\n                    || flat(&f.output.kind).len() > 32\n                    || flat(&f.output.kind).iter().any(|k| !scope_scalar(k))',1)
s=s.replace('memory scope callbacks require scalar u64 inputs and outputs;', 'memory scope callbacks require scalar inputs and outputs;')
s=s.replace('                        arity,\n                        function,','                        arity,\n                        layout,\n                        function,',1)
# Assign layouts to old ref operations; introduce typed ordinary-record primitives.
s=s.replace('                        arity: 0,','                        arity: 0,\n                        layout: None,',1)
s=s.replace('                            arity: fields.len(),','                            arity: fields.len(),\n                            layout: Some(Kind::Tuple(fields.clone())),',1)
s=s.replace('                            arity: arity as usize,','                            arity: arity as usize,\n                            layout: Some(Kind::Tuple(vec![Kind::Reference; arity as usize])),',1)
s=s.replace('                "heap/null" if values.is_empty()', '                "heap/null" | "memory/null" if values.is_empty()',1)
marker='                "heap/new-refs" if values.len() == 1 => {'
extra='''                "memory/allocate" if values.len() == 1 => {
                    let fields = record_lanes(&values[0].kind)?;
                    (Kind::Reference, Op::Heap { id, action: 1, arity: fields.len(), layout: Some(values[0].kind.clone()), arguments: values })
                }
                "memory/read" if values.len() == 2 && values[0].kind == Kind::Reference => {
                    let Kind::Type(layout) = &values[1].kind else { return Err("record read kind required".into()); };
                    let fields = record_lanes(layout)?;
                    (*layout.clone(), Op::Heap { id, action: 2, arity: fields.len(), layout: Some(*layout.clone()), arguments: vec![values[0].clone()] })
                }
'''
s=s.replace(marker,extra+marker)
# General scalar-lane flattening is shared with storage admission.
marker='struct Lower<\'a> {'
extra='''fn scope_scalar(kind: &Kind) -> bool { matches!(kind, Kind::Integer | Kind::Float | Kind::Boolean) }
pub fn scalar_lanes(kind: &Kind) -> Vec<Kind> {
    match kind {
        Kind::Tuple(items) => items.iter().flat_map(scalar_lanes).collect(),
        Kind::Record(fields) => fields.iter().flat_map(|(_, k)| scalar_lanes(k)).collect(),
        k => vec![k.clone()],
    }
}
fn record_lanes(kind: &Kind) -> Result<Vec<Kind>, String> {
    if !matches!(kind, Kind::Record(_)) { return Err("ordinary record required".into()); }
    let lanes = scalar_lanes(kind);
    if lanes.is_empty() || lanes.len() > 32 || lanes.iter().any(|k| !scope_scalar(k) && k != &Kind::Reference) {
        return Err("managed record requires 1..32 scalar or reference lanes".into());
    }
    Ok(lanes)
}
'''
s=s.replace(marker,extra+marker,1)
# Kind definitions are parsed independently, never executed as value definitions.
marker='    let mut defs = Vec::new();\n    visit(&parsed.syntax(), SyntaxKind::FunctionDefine, &mut defs);'
extra='''    let mut kind_defs = Vec::new();
    visit(&parsed.syntax(), SyntaxKind::KindDefine, &mut kind_defs);
    let mut aliases = BTreeMap::new();
    for node in kind_defs {
        let definition = KindDefineSyntax::cast(node).ok_or("kind definition")?;
        let name = text(definition.name().ok_or("kind name")?.syntax())?;
        let annotation = definition.annotation().ok_or("kind definition annotation")?.syntax().clone();
        if aliases.insert(name, annotation).is_some() { return Err("duplicate kind definition".into()); }
    }
    for annotation in aliases.values() { native_kind(annotation, &aliases, &mut BTreeSet::new())?; }
'''
s=s.replace(marker,extra+marker,1).replace('.map(|n| parameter(&n))','.map(|n| parameter(&n, &aliases))',1).replace('.map(parameter)','.map(|n| parameter(n, &aliases))',1)
s=s.replace('        source: &functions,','        source: &functions,\n        aliases: &aliases,',1)
s=s.replace('.any(|n| n.kind() == SyntaxKind::FunctionDefine)','.any(|n| matches!(n.kind(), SyntaxKind::FunctionDefine | SyntaxKind::KindDefine))',1)
# Name now denotes native source functions, not a separate heap memory model.
s=s.replace('heap_functions','native_functions')
p.write_text(s)
p=Path('src/stdlib/src/native_sequence.rs');s=p.read_text().replace('        "memory/with-frame",','        "memory/with-frame",\n        "memory/null",\n        "memory/allocate",\n        "memory/read",\n        "memory/with-arena",');p.write_text(s)

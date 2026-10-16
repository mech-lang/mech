//! Canonical activated-port admission and checked native resource manifests.
use super::*;
use mech_syntax::document::{
    CanonicalModuleImportBodySyntax, ContextSendSyntax, ModuleImportSyntax,
    PrefixedContextPathSyntax, VariableStemSyntax, VariableSyntax,
};

pub const RESOURCE_READ_OP: u64 = 81;
pub const PARSE_U64_OP: u64 = 82;
pub const RESOURCE_SEND_OP: u64 = 83;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum NativeResourceIntent {
    Read,
    Send,
}

/// A requirement requests authority from the launcher; it never grants it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NativeResourceRequirement {
    pub base_uri: String,
    pub context_name: String,
    pub path: String,
    pub intent: NativeResourceIntent,
}

impl NativeResourceRequirement {
    pub fn validate(&self) -> Result<(), String> {
        match (
            self.base_uri.as_str(),
            self.context_name.as_str(),
            self.intent,
        ) {
            ("cli://cli/args", "args", NativeResourceIntent::Read)
                if !self.path.is_empty()
                    && !self.path.starts_with('0')
                    && self.path.bytes().all(|byte| byte.is_ascii_digit())
                    && self.path.parse::<u64>().is_ok_and(|index| index > 0) =>
            {
                Ok(())
            }
            ("cli://cli/stdout", "stdout", NativeResourceIntent::Send)
                if matches!(self.path.as_str(), "line" | "text") =>
            {
                Ok(())
            }
            _ => Err("unsupported native resource base/context/path/intent".into()),
        }
    }

    pub(super) fn arguments(&self, program: &Program) -> Result<Vec<usize>, String> {
        [&self.base_uri, &self.context_name, &self.path]
            .into_iter()
            .map(|value| {
                program
                    .constants
                    .iter()
                    .position(|constant| matches!(constant, Constant::Text(text) if text == value))
                    .ok_or_else(|| "resource metadata constant is missing".into())
            })
            .collect()
    }
}

impl Program {
    /// Recover one exact request for an effect node, independent of manifest
    /// deduplication or instruction/constant renumbering.
    pub fn resource_requirement_for_node(
        &self,
        index: usize,
    ) -> Result<Option<NativeResourceRequirement>, String> {
        let node = self
            .nodes
            .get(index)
            .ok_or("resource node index out of bounds")?;
        let (intent, arity, output) = match node.op {
            RESOURCE_READ_OP => (NativeResourceIntent::Read, 3, SequenceKind::Text),
            RESOURCE_SEND_OP => (NativeResourceIntent::Send, 4, SequenceKind::Unit),
            _ => return Ok(None),
        };
        if node.args.len() != arity
            || node.kernel.is_some()
            || self.kinds.get(self.constants.len() + index) != Some(&output)
        {
            return Err("invalid native resource node shape".into());
        }
        let text_constant = |position: usize| -> Result<String, String> {
            let id = node.args[position];
            if self.kinds.get(id) != Some(&SequenceKind::Text) {
                return Err("native resource metadata must have text kind".into());
            }
            match self.constants.get(id) {
                Some(Constant::Text(value)) => Ok(value.clone()),
                _ => Err("native resource metadata must be a static text constant".into()),
            }
        };
        if intent == NativeResourceIntent::Send
            && (self.kinds.get(node.args[3]) != Some(&SequenceKind::Text)
                || node.args[3] >= self.constants.len() + index)
        {
            return Err("native resource payload must be an earlier text value".into());
        }
        let request = NativeResourceRequirement {
            base_uri: text_constant(0)?,
            context_name: text_constant(1)?,
            path: text_constant(2)?,
            intent,
        };
        request.validate()?;
        Ok(Some(request))
    }

    /// Unique requirements in first-use node order. Every resource node still
    /// resolves its own exact request through `resource_requirement_for_node`.
    pub fn resource_requirements(&self) -> Result<Vec<NativeResourceRequirement>, String> {
        for constant in &self.constants {
            if let Constant::Function(function) = constant
                && (!function.program.resource_requirements()?.is_empty()
                    || function
                        .program
                        .nodes
                        .iter()
                        .any(|node| matches!(node.op, 1 | 8 | 9 | 10 | 12 | 32 | 41 | 45 | 52)))
            {
                return Err("native callable must be pure and cannot access resources".into());
            }
        }
        let mut requirements = Vec::new();
        for index in 0..self.nodes.len() {
            if let Some(request) = self.resource_requirement_for_node(index)?
                && !requirements.contains(&request)
            {
                requirements.push(request);
            }
        }
        if !requirements.is_empty()
            && self
                .nodes
                .iter()
                .any(|node| matches!(node.op, 1 | 8 | 9 | 10 | 12 | 32 | 41 | 45 | 52))
        {
            return Err("activated resources cannot be mixed with trusted legacy IO".into());
        }
        Ok(requirements)
    }
}

/// Document parsing deliberately preserves unsupported declarations as prose.
/// Admission and frontend selection must agree about these attempted forms.
pub(super) fn rejected_declaration(node: &SyntaxNode) -> bool {
    matches!(
        node.kind(),
        SyntaxKind::ImportDeclaration | SyntaxKind::ContextDeclaration
    ) || (node.kind() == SyntaxKind::Paragraph
        && node
            .text()
            .is_ok_and(|text| text.trim_start().starts_with("+>")))
}

#[derive(Default)]
pub(super) struct SourceBindings {
    contexts: BTreeMap<String, String>,
    aliases: BTreeMap<String, String>,
    requirements: Vec<NativeResourceRequirement>,
}

impl SourceBindings {
    pub(super) fn resolve_name<'a>(&'a self, name: &'a str) -> &'a str {
        self.aliases.get(name).map(String::as_str).unwrap_or(name)
    }

    pub(super) fn validate_function_names<'a>(
        &self,
        names: impl Iterator<Item = &'a String>,
    ) -> Result<(), String> {
        for name in names {
            if self.aliases.contains_key(name) {
                return Err(format!(
                    "source function conflicts with native import {name}"
                ));
            }
        }
        Ok(())
    }

    pub(super) fn collect(syntax: &SyntaxNode, catalog: Catalog) -> Result<Self, String> {
        // Canonical document parsing also admits prose. An unrecognized import
        // such as the currently unsupported root glob can therefore become a
        // Paragraph instead of a parser diagnostic. Fail that native admission
        // boundary explicitly rather than silently discarding an import.
        fn check_document(node: &SyntaxNode) -> Result<(), String> {
            if matches!(
                node.kind(),
                SyntaxKind::ImportDeclaration | SyntaxKind::ContextDeclaration
            ) {
                return Err(
                    "native source imports and source-owned context declarations are unsupported"
                        .into(),
                );
            }
            if rejected_declaration(node) {
                return Err("unsupported native import syntax; use canonical module/item, alias or module/* imports".into());
            }
            for child in node.children() {
                check_document(&child)?;
            }
            Ok(())
        }
        check_document(syntax)?;
        let mut result = Self::default();
        let mut imports = Vec::new();
        visit(syntax, SyntaxKind::ModuleImport, &mut imports);
        for import in imports {
            result.import(
                ModuleImportSyntax::cast(import).ok_or("invalid native import")?,
                catalog,
            )?;
        }
        fn scan(
            node: &SyntaxNode,
            bindings: &mut SourceBindings,
            callable: bool,
        ) -> Result<(), String> {
            let callable = callable || node.kind() == SyntaxKind::FunctionDefine;
            if let Some(send) = ContextSendSyntax::cast(node.clone()) {
                if callable {
                    return Err("native callable cannot send resources".into());
                }
                let target = send.target().ok_or("missing resource send target")?;
                let Some(VariableStemSyntax::Context(path)) = target.stem() else {
                    return Err("resource send requires a context-addressed port".into());
                };
                let request = bindings.request(&path, NativeResourceIntent::Send)?;
                bindings.requirements.push(request);
                return scan(
                    send.expression()
                        .ok_or("missing resource payload")?
                        .syntax(),
                    bindings,
                    callable,
                );
            }
            if let Some(path) = PrefixedContextPathSyntax::cast(node.clone()) {
                if callable {
                    return Err("native callable cannot read resources".into());
                }
                let request = bindings.request(&path, NativeResourceIntent::Read)?;
                bindings.requirements.push(request);
                return Ok(());
            }
            if let Some(call) = FunctionCallSyntax::cast(node.clone()) {
                let name = text(call.function().ok_or("missing function")?.syntax())?;
                if !bindings.contexts.is_empty() && bindings.resolve_name(&name).starts_with("io/")
                {
                    return Err("activated resources cannot be mixed with trusted legacy IO".into());
                }
            }
            for child in node.children() {
                scan(&child, bindings, callable)?;
            }
            Ok(())
        }
        scan(syntax, &mut result, false)?;
        Ok(result)
    }

    /// Resolve only callee identifier ranges supplied by the canonical AST.
    /// This precedes matrix purity checks so aliases retain their catalog
    /// identity there, without a source-text recognizer or evaluation rewrite.
    pub(super) fn canonical_calls(
        &self,
        source: &str,
        syntax: &SyntaxNode,
    ) -> Result<String, String> {
        fn collect_calls(node: &SyntaxNode, calls: &mut Vec<SyntaxNode>) {
            if node.kind() == SyntaxKind::FunctionCall {
                calls.push(node.clone());
            }
            for child in node.children() {
                collect_calls(&child, calls);
            }
        }
        let mut calls = Vec::new();
        collect_calls(syntax, &mut calls);
        let mut edits = Vec::new();
        for node in calls {
            let call = FunctionCallSyntax::cast(node).ok_or("invalid native call")?;
            let name = call.function().ok_or("missing native callee")?;
            let original = text(name.syntax())?;
            let resolved = self.resolve_name(&original);
            if resolved != original {
                edits.push((
                    name.syntax().range().start.0 as usize,
                    name.syntax().range().end.0 as usize,
                    resolved.to_string(),
                ));
            }
        }
        edits.sort_by_key(|edit| edit.0);
        let mut output = String::new();
        let mut cursor = 0;
        for (start, end, name) in edits {
            if start < cursor || end > source.len() {
                return Err("overlapping native callee ranges".into());
            }
            output.push_str(&source[cursor..start]);
            output.push_str(&name);
            cursor = end;
        }
        output.push_str(&source[cursor..]);
        Ok(output)
    }

    pub(super) fn append_constants(&self, program: &mut Program) {
        for request in &self.requirements {
            for value in [&request.base_uri, &request.context_name, &request.path] {
                let constant = Constant::Text(value.clone());
                if !program.constants.contains(&constant) {
                    program.constants.push(constant);
                    program.kinds.push(SequenceKind::Text);
                }
            }
        }
    }

    pub(super) fn read(
        &self,
        variable: &VariableSyntax,
    ) -> Result<Option<NativeResourceRequirement>, String> {
        match variable.stem() {
            Some(VariableStemSyntax::Context(path)) => {
                self.request(&path, NativeResourceIntent::Read).map(Some)
            }
            _ => Ok(None),
        }
    }

    pub(super) fn send(
        &self,
        send: &ContextSendSyntax,
    ) -> Result<NativeResourceRequirement, String> {
        let target = send.target().ok_or("missing resource send target")?;
        match target.stem() {
            Some(VariableStemSyntax::Context(path)) => {
                self.request(&path, NativeResourceIntent::Send)
            }
            _ => Err("resource send requires a context-addressed port".into()),
        }
    }

    fn request(
        &self,
        path: &PrefixedContextPathSyntax,
        intent: NativeResourceIntent,
    ) -> Result<NativeResourceRequirement, String> {
        let alias = text(path.context().ok_or("missing resource context")?.syntax())?;
        let context_name = self
            .contexts
            .get(&alias)
            .ok_or_else(|| format!("unbound native resource context @{alias}"))?;
        let request = NativeResourceRequirement {
            base_uri: format!("cli://cli/{context_name}"),
            context_name: context_name.clone(),
            path: text(path.address().ok_or("missing resource path")?.syntax())?,
            intent,
        };
        request.validate()?;
        Ok(request)
    }

    fn import(&mut self, import: ModuleImportSyntax, catalog: Catalog) -> Result<(), String> {
        let mut add = |module: String,
                       item: Option<String>,
                       alias: Option<String>,
                       namespace: bool|
         -> Result<(), String> {
            let prefix = format!("{module}/");
            let names: Vec<_> = catalog
                .iter()
                .filter(|op| {
                    op.name.starts_with(&prefix)
                        && item
                            .as_ref()
                            .is_none_or(|item| op.name == format!("{prefix}{item}"))
                })
                .map(|op| op.name)
                .collect();
            if names.is_empty() {
                return Err(format!("unsupported native module import {module}"));
            }
            for name in names {
                let key = alias.clone().unwrap_or_else(|| {
                    if namespace {
                        name.to_string()
                    } else {
                        name.rsplit('/').next().unwrap().to_string()
                    }
                });
                if self.aliases.get(&key).is_some_and(|old| old != name) {
                    return Err(format!("conflicting native function import {key}"));
                }
                self.aliases.insert(key, name.to_string());
            }
            Ok(())
        };
        match import.body().ok_or("missing native module import body")? {
            CanonicalModuleImportBodySyntax::AliasedItem(body) => {
                let alias = body.alias().ok_or("missing native import alias")?;
                let module = text(
                    body.module()
                        .ok_or("missing native import module")?
                        .syntax(),
                )?;
                let item = text(body.item().ok_or("missing native import item")?.syntax())?;
                if let Some(context) = alias.context() {
                    let alias = text(context.name().ok_or("missing context alias")?.syntax())?;
                    if module != "cli" || !matches!(item.as_str(), "args" | "stdout") {
                        return Err("unsupported native activated context target".into());
                    }
                    if self.contexts.insert(alias.clone(), item).is_some() {
                        return Err(format!("duplicate native context binding @{alias}"));
                    }
                    Ok(())
                } else {
                    let alias = text(
                        alias
                            .value()
                            .and_then(|value| value.path())
                            .ok_or("missing native value alias")?
                            .syntax(),
                    )?;
                    add(module, Some(item), Some(alias), false)
                }
            }
            CanonicalModuleImportBodySyntax::Module(body) => add(
                text(body.module().ok_or("missing native module")?.syntax())?,
                None,
                None,
                true,
            ),
            CanonicalModuleImportBodySyntax::Suffix(body) => {
                let module = text(body.module().ok_or("missing native module")?.syntax())?;
                if body.is_glob() {
                    return add(module, None, None, false);
                }
                if let Some(group) = body.group() {
                    for item in group.items() {
                        add(
                            module.clone(),
                            Some(text(item.path().ok_or("missing imported item")?.syntax())?),
                            None,
                            false,
                        )?;
                    }
                    Ok(())
                } else {
                    add(
                        module,
                        Some(text(body.item().ok_or("missing imported item")?.syntax())?),
                        None,
                        false,
                    )
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use OutputKind::Fixed;
    use SequenceKind::{Float, Integer, Text, Unit};
    const CATALOG: Catalog = &[
        OperationSignature {
            name: "number/parse-u64",
            op: PARSE_U64_OP,
            inputs: &[&[Text]],
            output: Fixed(Integer),
        },
        OperationSignature {
            name: "number/format",
            op: 24,
            inputs: &[&[Integer, Float], &[Integer]],
            output: Fixed(Text),
        },
        OperationSignature {
            name: "io/write",
            op: 8,
            inputs: &[&[Text]],
            output: Fixed(Unit),
        },
        OperationSignature {
            name: "math/sqrt",
            op: 38,
            inputs: &[&[Float]],
            output: Fixed(Float),
        },
    ];
    fn compile(source: &str) -> Result<Program, String> {
        super::super::compile(source, 1, CATALOG)
    }
    const SOURCE: &str = "+> @input := cli/args\n+> @printer := cli/stdout\nn := number/parse-u64(@input/1)\n@printer/line <- number/format(n, 0)\n";

    #[test]
    fn activated_source_detection_includes_unbound_reads_and_sends() {
        for source in [
            "+> @a := cli/args\nx := 1\n",
            "x := @missing/1\n",
            "@missing/line <- \"x\"\n",
            "+> *\nx := 1\n",
        ] {
            assert!(
                ParsedSource::parse(source).unwrap().has_activated_ports(),
                "{source}"
            );
        }
        assert!(
            !ParsedSource::parse("x := number/parse-u64(\"1\")\n")
                .unwrap()
                .has_activated_ports()
        );
        assert!(
            !ParsedSource::parse("label := \"+> *\"\n")
                .unwrap()
                .has_activated_ports()
        );
        let malformed = ParsedSource::parse("+> @out :=\nx := 1\n");
        assert!(malformed.is_err() || malformed.unwrap().has_activated_ports());
    }

    #[test]
    fn canonical_ports_lower_to_exact_static_provider_requirements() {
        let program = compile(SOURCE).unwrap();
        assert_eq!(
            program.nodes.iter().map(|node| node.op).collect::<Vec<_>>(),
            [81, 82, 24, 83]
        );
        let requests = program.resource_requirements().unwrap();
        assert_eq!(
            requests,
            [
                NativeResourceRequirement {
                    base_uri: "cli://cli/args".into(),
                    context_name: "args".into(),
                    path: "1".into(),
                    intent: NativeResourceIntent::Read
                },
                NativeResourceRequirement {
                    base_uri: "cli://cli/stdout".into(),
                    context_name: "stdout".into(),
                    path: "line".into(),
                    intent: NativeResourceIntent::Send
                },
            ]
        );
        assert_eq!(
            program.resource_requirement_for_node(0).unwrap(),
            Some(requests[0].clone())
        );
        assert_eq!(program.resource_requirement_for_node(1).unwrap(), None);
        assert_eq!(
            program.resource_requirement_for_node(3).unwrap(),
            Some(requests[1].clone())
        );
        let scheduled = program.scheduled().unwrap();
        assert_eq!(scheduled.resource_requirements().unwrap(), requests);
        assert_eq!(
            scheduled
                .nodes
                .iter()
                .map(|node| node.op)
                .collect::<Vec<_>>(),
            [81, 82, 24, 83]
        );
    }

    #[test]
    fn imports_resolve_aliases_and_reject_unknown_or_conflicting_exports() {
        for import in [
            "+> parse := number/parse-u64",
            "+> number/parse-u64",
            "+> number/{parse-u64,format}",
            "+> number/*",
        ] {
            let call = if import.contains("parse :=") {
                "parse"
            } else {
                "parse-u64"
            };
            let source = format!(
                "{import}\n+> @a := cli/args\n+> @o := cli/stdout\nn := {call}(@a/1)\n@o/line <- number/format(n, 0)\n"
            );
            assert!(compile(&source).is_ok(), "{source}: {:?}", compile(&source));
        }
        for source in [
            "+> missing/*\nx := number/parse-u64(\"1\")\n",
            "+> number/missing\nx := number/parse-u64(\"1\")\n",
            "+> parse := number/parse-u64\n+> parse := math/sqrt\nx := parse(\"1\")\n",
            "+> *\nx := number/parse-u64(\"1\")\n",
        ] {
            assert!(compile(source).is_err(), "{source}");
        }
    }

    #[test]
    fn imported_math_aliases_are_resolved_before_matrix_purity_checks() {
        let source = "+> root := math/sqrt\nI := 0..2\nA := [[math/sqrt(root(1.0)) | j <- I]' | i <- I]'\nx := [1.0 | i <- I]'\ny := A ** x\n";
        let parsed = ParsedSource::parse(source).unwrap();
        let syntax = parsed.matrix_syntax(CATALOG).unwrap();
        let normalized = text(&syntax).unwrap();
        assert!(
            normalized.contains("math/sqrt(math/sqrt(1.0))"),
            "{normalized}"
        );
        assert!(normalized.contains("matrix/apply("), "{normalized}");
    }

    #[test]
    fn ports_fail_closed_on_unbound_dynamic_wrong_direction_or_legacy_routes() {
        for source in [
            "n := number/parse-u64(@missing/1)\n",
            "+> @a := cli/args\n+> @a := cli/args\nn := number/parse-u64(@a/1)\n",
            "+> @a := cli/stdout\nx := @a/line\n",
            "+> @a := cli/args\n@a/1 <- \"x\"\n",
            "+> @a := cli/args\nx := @a/zero\n",
            "+> @a := cli/args\nx := @a/0\n",
            "+> @a := cli/args\nx := @a/01\n",
            "+> @a := cli/args\nx := @a/18446744073709551616\n",
            "+> @o := cli/stdout\n@o/other <- \"x\"\n",
            "+> @a := cli/unknown\nx := @a/1\n",
            "+> @a := cli/args\nio/write(\"bypass\")\n",
            "+> write := io/write\n+> @a := cli/args\nwrite(\"bypass\")\n",
            "+> @a := cli/args\nf(x<u64>) => <u64> := number/parse-u64(@a/1)\ny := f(1)\n",
        ] {
            assert!(compile(source).is_err(), "{source}");
        }
    }

    #[test]
    fn stdout_text_and_line_remain_distinct_checked_ports() {
        let program =
            compile("+> @o := cli/stdout\n@o/text <- \"raw\"\n@o/line <- \"line\"\n").unwrap();
        let requests = program.resource_requirements().unwrap();
        assert_eq!(
            requests
                .iter()
                .map(|request| request.path.as_str())
                .collect::<Vec<_>>(),
            ["text", "line"]
        );
        let mut invalid = NativeResourceRequirement {
            base_uri: "cli://cli/args".into(),
            context_name: "args".into(),
            path: "01".into(),
            intent: NativeResourceIntent::Read,
        };
        assert!(invalid.validate().is_err());
        invalid.path = "1".into();
        assert!(invalid.validate().is_ok());
    }

    #[test]
    fn manifest_dedup_does_not_conflate_node_positions() {
        let program = compile("+> @a := cli/args\nx := @a/1\ny := @a/1\nz := @a/2\n").unwrap();
        assert_eq!(program.resource_requirements().unwrap().len(), 2);
        assert_eq!(
            program.resource_requirement_for_node(0).unwrap(),
            program.resource_requirement_for_node(1).unwrap()
        );
        assert_ne!(
            program.resource_requirement_for_node(1).unwrap(),
            program.resource_requirement_for_node(2).unwrap()
        );
        let mut malformed = program.clone();
        malformed.nodes[0].args[1] = program.constants.len();
        assert!(malformed.resource_requirements().is_err());
        let mut denied = program;
        let id = denied.nodes[0].args[1];
        denied.constants[id] = Constant::Text("stdout".into());
        assert!(denied.resource_requirements().is_err());
    }

    #[test]
    fn text_plus_uses_existing_concat_and_retains_left_to_right_reads() {
        let program =
            compile("+> @a := cli/args\n+> @o := cli/stdout\n@o/line <- @a/1 + \" : \" + @a/2\n")
                .unwrap();
        assert_eq!(
            program.nodes.iter().map(|node| node.op).collect::<Vec<_>>(),
            [81, 27, 81, 27, 83]
        );
        let scheduled = program.scheduled().unwrap();
        assert_eq!(
            scheduled
                .nodes
                .iter()
                .filter(|node| node.op == 81)
                .map(|node| match &scheduled.constants[node.args[2]] {
                    Constant::Text(path) => path.as_str(),
                    _ => panic!("static path"),
                })
                .collect::<Vec<_>>(),
            ["1", "2"]
        );
        for source in [
            "x := \"a\" - \"b\"\n",
            "x := \"a\" + 1\n",
            "x := \"a\" * \"b\"\n",
        ] {
            assert!(compile(source).is_err());
        }
    }

    #[test]
    fn pure_parse_is_fallible_ordered_and_has_no_host_requirements() {
        let program = compile("io/write(\"before\")\nn := number/parse-u64(\"18446744073709551616\")\nio/write(\"after\")\n").unwrap();
        assert!(program.resource_requirements().unwrap().is_empty());
        assert_eq!(
            program
                .scheduled()
                .unwrap()
                .nodes
                .iter()
                .map(|node| node.op)
                .collect::<Vec<_>>(),
            [8, 82, 8]
        );
    }
}

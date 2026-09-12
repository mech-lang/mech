use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use syn::parse::Parser;
use syn::spanned::Spanned;
use syn::visit::Visit;

const CONSUMER_HEADER: &str = "consumer-id\tpackage\tsource-path\tcaller-symbol\tcall-count\t\
feature-gate\tinput-policy\tfailure-policy\tcurrent-result-use\tcanonical-target\t\
cutover-action\tfixture-id\tnotes";
const FIXTURE_HEADER: &str = "fixture-id\tsource-path\tcanonical-root\tcanonical-outcome\t\
consumer-contracts\tnotes";
const EXPECTED_CONSUMERS: usize = 27;
const EXPECTED_PRODUCTION_CALLS: usize = 29;
const EXPECTED_FIXTURES: usize = 9;

#[derive(Debug)]
struct ConsumerRow {
    source_path: String,
    caller: String,
    calls: usize,
    feature_gate: String,
    input_policy: String,
    failure_policy: String,
    current_result_use: String,
    canonical_target: String,
    cutover_action: String,
    fixture: String,
    notes: String,
}

#[derive(Debug)]
struct FixtureRow {
    source_path: String,
    root: String,
    outcome: String,
    contracts: String,
    notes: String,
}

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root")
}

fn fields(line: &str) -> Vec<&str> {
    line.split('\t').collect()
}

fn consumers() -> BTreeMap<String, ConsumerRow> {
    let path = "docs/design/grammar-audit/source-parser-consumers.tsv";
    let source = fs::read_to_string(repository_root().join(path)).expect("read consumer inventory");
    let mut lines = source.lines();
    assert_eq!(lines.next(), Some(CONSUMER_HEADER));
    let mut previous = String::new();
    let mut rows = BTreeMap::new();
    for (index, line) in lines.enumerate() {
        let row = fields(line);
        assert_eq!(row.len(), 13, "invalid consumer row {}", index + 2);
        assert!(row[0] > previous.as_str(), "consumer rows are not ordered");
        previous = row[0].to_owned();
        let calls = row[4]
            .parse::<usize>()
            .unwrap_or_else(|_| panic!("invalid call count on row {}", index + 2));
        assert!(calls > 0, "empty call count on row {}", index + 2);
        let value = ConsumerRow {
            source_path: row[2].to_owned(),
            caller: row[3].to_owned(),
            calls,
            feature_gate: row[5].to_owned(),
            input_policy: row[6].to_owned(),
            failure_policy: row[7].to_owned(),
            current_result_use: row[8].to_owned(),
            canonical_target: row[9].to_owned(),
            cutover_action: row[10].to_owned(),
            fixture: row[11].to_owned(),
            notes: row[12].to_owned(),
        };
        assert!(rows.insert(row[0].to_owned(), value).is_none());
    }
    assert_eq!(rows.len(), EXPECTED_CONSUMERS);
    rows
}

fn fixtures() -> BTreeMap<String, FixtureRow> {
    let path = "docs/design/grammar-audit/source-parser-consumer-fixtures.tsv";
    let source = fs::read_to_string(repository_root().join(path)).expect("read fixture inventory");
    let mut lines = source.lines();
    assert_eq!(lines.next(), Some(FIXTURE_HEADER));
    let mut previous = String::new();
    let mut rows = BTreeMap::new();
    for (index, line) in lines.enumerate() {
        let row = fields(line);
        assert_eq!(row.len(), 6, "invalid fixture row {}", index + 2);
        assert!(row[0] > previous.as_str(), "fixture rows are not ordered");
        previous = row[0].to_owned();
        let value = FixtureRow {
            source_path: row[1].to_owned(),
            root: row[2].to_owned(),
            outcome: row[3].to_owned(),
            contracts: row[4].to_owned(),
            notes: row[5].to_owned(),
        };
        assert!(rows.insert(row[0].to_owned(), value).is_none());
    }
    assert_eq!(rows.len(), EXPECTED_FIXTURES);
    rows
}

fn visit_rust_files(path: &Path, files: &mut BTreeSet<PathBuf>) {
    for entry in
        fs::read_dir(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
    {
        let path = entry.expect("read source-tree entry").path();
        if path.is_dir() {
            // Each discovered package supplies its own source root. Test module
            // names have no meaning here; only Rust cfg can exclude their code.
            if !path.join("Cargo.toml").is_file() && path.file_name().unwrap() != "target" {
                visit_rust_files(&path, files);
            }
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.insert(path.canonicalize().expect("canonical source path"));
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct RustToken<'source> {
    text: &'source str,
    offset: usize,
    raw: bool,
}

#[derive(Debug, Default)]
struct SourceScan {
    call_sites: BTreeMap<String, BTreeSet<usize>>,
    prohibited_aliases: Vec<String>,
}

impl SourceScan {
    fn call_counts(&self) -> BTreeMap<String, usize> {
        self.call_sites
            .iter()
            .map(|(caller, sites)| (caller.clone(), sites.len()))
            .collect()
    }
}

fn quoted_literal_end(source: &str, start: usize, quote: u8) -> usize {
    let bytes = source.as_bytes();
    let mut cursor = start + 1;
    while cursor < bytes.len() {
        match bytes[cursor] {
            b'\\' => cursor = (cursor + 2).min(bytes.len()),
            found if found == quote => return cursor + 1,
            _ => cursor += 1,
        }
    }
    bytes.len()
}

fn character_literal_end(source: &str, start: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut cursor = start + 1;
    if bytes.get(cursor) == Some(&b'\\') {
        cursor += 1;
        match bytes.get(cursor).copied() {
            Some(b'u') if bytes.get(cursor + 1) == Some(&b'{') => {
                cursor += 2;
                while cursor < bytes.len() && bytes[cursor] != b'}' {
                    cursor += 1;
                }
                cursor = (cursor + 1).min(bytes.len());
            }
            Some(_) => cursor += 1,
            None => return None,
        }
    } else {
        let character = source.get(cursor..)?.chars().next()?;
        cursor += character.len_utf8();
    }
    (bytes.get(cursor) == Some(&b'\'')).then_some(cursor + 1)
}

fn raw_string_end(source: &str, start: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut delimiter = if bytes.get(start..start + 2) == Some(b"br")
        || bytes.get(start..start + 2) == Some(b"cr")
    {
        start + 2
    } else if bytes.get(start) == Some(&b'r') {
        start + 1
    } else {
        return None;
    };
    while bytes.get(delimiter) == Some(&b'#') {
        delimiter += 1;
    }
    if bytes.get(delimiter) != Some(&b'"') {
        return None;
    }
    let hashes = delimiter
        - if bytes.get(start) == Some(&b'r') {
            start + 1
        } else {
            start + 2
        };
    let mut cursor = delimiter + 1;
    while cursor < bytes.len() {
        if bytes[cursor] == b'"'
            && bytes
                .get(cursor + 1..cursor + 1 + hashes)
                .is_some_and(|suffix| suffix.iter().all(|byte| *byte == b'#'))
        {
            return Some(cursor + 1 + hashes);
        }
        cursor += 1;
    }
    Some(bytes.len())
}

fn rust_tokens(source: &str) -> Vec<RustToken<'_>> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut cursor = 0;
    while cursor < bytes.len() {
        if bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
            continue;
        }
        if bytes.get(cursor..cursor + 2) == Some(b"//") {
            cursor += 2;
            while cursor < bytes.len() && bytes[cursor] != b'\n' {
                cursor += 1;
            }
            continue;
        }
        if bytes.get(cursor..cursor + 2) == Some(b"/*") {
            cursor += 2;
            let mut depth = 1usize;
            while cursor < bytes.len() && depth > 0 {
                if bytes.get(cursor..cursor + 2) == Some(b"/*") {
                    depth += 1;
                    cursor += 2;
                } else if bytes.get(cursor..cursor + 2) == Some(b"*/") {
                    depth -= 1;
                    cursor += 2;
                } else {
                    cursor += 1;
                }
            }
            continue;
        }
        if let Some(end) = raw_string_end(source, cursor) {
            cursor = end;
            continue;
        }
        if bytes.get(cursor..cursor + 2) == Some(b"b\"")
            || bytes.get(cursor..cursor + 2) == Some(b"c\"")
        {
            cursor = quoted_literal_end(source, cursor + 1, b'"');
            continue;
        }
        if bytes.get(cursor) == Some(&b'"') {
            cursor = quoted_literal_end(source, cursor, b'"');
            continue;
        }
        if bytes.get(cursor..cursor + 2) == Some(b"b'")
            && let Some(end) = character_literal_end(source, cursor + 1)
        {
            cursor = end;
            continue;
        }
        if bytes.get(cursor) == Some(&b'\'')
            && let Some(end) = character_literal_end(source, cursor)
        {
            cursor = end;
            continue;
        }
        let mut raw_identifier = false;
        // Raw identifiers denote the same Rust name as their ordinary spelling.
        if bytes.get(cursor..cursor + 2) == Some(b"r#")
            && bytes
                .get(cursor + 2)
                .is_some_and(|byte| byte.is_ascii_alphabetic() || *byte == b'_')
        {
            cursor += 2;
            raw_identifier = true;
        }
        if bytes[cursor].is_ascii_alphabetic() || bytes[cursor] == b'_' {
            let start = cursor;
            cursor += 1;
            while cursor < bytes.len()
                && (bytes[cursor].is_ascii_alphanumeric() || bytes[cursor] == b'_')
            {
                cursor += 1;
            }
            tokens.push(RustToken {
                text: &source[start..cursor],
                offset: start,
                raw: raw_identifier,
            });
            continue;
        }
        let width = source[cursor..]
            .chars()
            .next()
            .expect("cursor is inside source")
            .len_utf8();
        tokens.push(RustToken {
            text: &source[cursor..cursor + width],
            offset: cursor,
            raw: false,
        });
        cursor += width;
    }
    tokens
}

fn matching_delimiter(
    tokens: &[RustToken<'_>],
    start: usize,
    open: &str,
    close: &str,
) -> Option<usize> {
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate().skip(start) {
        if token.text == open {
            depth += 1;
        } else if token.text == close {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

fn cfg_predicate(meta: &syn::Meta) -> Option<bool> {
    if matches!(meta, syn::Meta::Path(path) if path.is_ident("test")) {
        return Some(false);
    }
    let syn::Meta::List(list) = meta else {
        return None;
    };
    let arguments = syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated
        .parse2(list.tokens.clone())
        .expect("valid Rust cfg predicate");
    let values = arguments.iter().map(cfg_predicate).collect::<Vec<_>>();
    if list.path.is_ident("not") && values.len() == 1 {
        return values[0].map(|value| !value);
    }
    if list.path.is_ident("all") {
        if values.contains(&Some(false)) {
            return Some(false);
        }
        if values.iter().all(|value| *value == Some(true)) {
            return Some(true);
        }
    }
    if list.path.is_ident("any") {
        if values.contains(&Some(true)) {
            return Some(true);
        }
        if values.iter().all(|value| *value == Some(false)) {
            return Some(false);
        }
    }
    None
}

fn meta_excludes_production(meta: &syn::Meta) -> bool {
    let syn::Meta::List(list) = meta else {
        return false;
    };
    let arguments = || {
        syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated
            .parse2(list.tokens.clone())
            .expect("valid Rust conditional attribute")
    };
    if list.path.is_ident("cfg") {
        let arguments = arguments();
        return arguments.len() == 1 && cfg_predicate(&arguments[0]) == Some(false);
    }
    if list.path.is_ident("cfg_attr") {
        let arguments = arguments();
        // An unknown feature/target may leave the attribute unapplied. Only
        // attributes certainly applied without tests prove an exclusion.
        return arguments
            .first()
            .is_some_and(|condition| cfg_predicate(condition) == Some(true))
            && arguments.iter().skip(1).any(meta_excludes_production);
    }
    false
}

fn excludes_production(attrs: &[syn::Attribute]) -> bool {
    attrs
        .iter()
        .any(|attribute| meta_excludes_production(&attribute.meta))
}

#[derive(Default)]
struct ProductionExclusions {
    ranges: Vec<std::ops::Range<usize>>,
}

impl ProductionExclusions {
    fn exclude(&mut self, attrs: &[syn::Attribute], span: proc_macro2::Span) -> bool {
        if !excludes_production(attrs) {
            return false;
        }
        self.ranges.push(span.byte_range());
        true
    }
}

// Rust supplies the exact extent of each annotated construct. In particular,
// a cfg-disabled match arm ends at its own comma, never at the next arm's body.
macro_rules! visit_conditional_nodes {
    ($($method:ident: $node:ident),* $(,)?) => { $(
        fn $method(&mut self, node: &'ast syn::$node) {
            if !self.exclude(&node.attrs, node.span()) {
                syn::visit::$method(self, node);
            }
        }
    )* };
}

impl<'ast> Visit<'ast> for ProductionExclusions {
    visit_conditional_nodes! {
        visit_file: File,
        visit_item_const: ItemConst, visit_item_enum: ItemEnum,
        visit_item_extern_crate: ItemExternCrate, visit_item_fn: ItemFn,
        visit_item_foreign_mod: ItemForeignMod, visit_item_impl: ItemImpl,
        visit_item_macro: ItemMacro, visit_item_mod: ItemMod,
        visit_item_static: ItemStatic, visit_item_struct: ItemStruct,
        visit_item_trait: ItemTrait, visit_item_trait_alias: ItemTraitAlias,
        visit_item_type: ItemType, visit_item_union: ItemUnion, visit_item_use: ItemUse,
        visit_impl_item_const: ImplItemConst, visit_impl_item_fn: ImplItemFn,
        visit_impl_item_macro: ImplItemMacro, visit_impl_item_type: ImplItemType,
        visit_trait_item_const: TraitItemConst, visit_trait_item_fn: TraitItemFn,
        visit_trait_item_macro: TraitItemMacro, visit_trait_item_type: TraitItemType,
        visit_foreign_item_fn: ForeignItemFn, visit_foreign_item_macro: ForeignItemMacro,
        visit_foreign_item_static: ForeignItemStatic, visit_foreign_item_type: ForeignItemType,
        visit_arm: Arm, visit_local: Local, visit_field: Field,
        visit_field_value: FieldValue, visit_variant: Variant, visit_stmt_macro: StmtMacro,
        visit_expr_array: ExprArray,
        visit_expr_assign: ExprAssign,
        visit_expr_async: ExprAsync,
        visit_expr_await: ExprAwait,
        visit_expr_binary: ExprBinary,
        visit_expr_block: ExprBlock,
        visit_expr_break: ExprBreak,
        visit_expr_call: ExprCall,
        visit_expr_cast: ExprCast,
        visit_expr_closure: ExprClosure,
        visit_expr_const: ExprConst,
        visit_expr_continue: ExprContinue,
        visit_expr_field: ExprField,
        visit_expr_for_loop: ExprForLoop,
        visit_expr_group: ExprGroup,
        visit_expr_if: ExprIf,
        visit_expr_index: ExprIndex,
        visit_expr_infer: ExprInfer,
        visit_expr_let: ExprLet,
        visit_expr_lit: ExprLit,
        visit_expr_loop: ExprLoop,
        visit_expr_macro: ExprMacro,
        visit_expr_match: ExprMatch,
        visit_expr_method_call: ExprMethodCall,
        visit_expr_paren: ExprParen,
        visit_expr_path: ExprPath,
        visit_expr_range: ExprRange,
        visit_expr_raw_addr: ExprRawAddr,
        visit_expr_reference: ExprReference,
        visit_expr_repeat: ExprRepeat,
        visit_expr_return: ExprReturn,
        visit_expr_struct: ExprStruct,
        visit_expr_try: ExprTry,
        visit_expr_try_block: ExprTryBlock,
        visit_expr_tuple: ExprTuple,
        visit_expr_unary: ExprUnary,
        visit_expr_unsafe: ExprUnsafe,
        visit_expr_while: ExprWhile,
        visit_expr_yield: ExprYield,
    }
}

fn parsed_rust(source: &str) -> syn::File {
    syn::parse_file(source)
        .unwrap_or_else(|error| panic!("census cannot parse Rust source: {error}"))
}

fn production_exclusions(file: &syn::File) -> ProductionExclusions {
    let mut exclusions = ProductionExclusions::default();
    exclusions.visit_file(file);
    exclusions
}

fn production_tokens(source: &str) -> Vec<RustToken<'_>> {
    let exclusions = production_exclusions(&parsed_rust(source));
    rust_tokens(source)
        .into_iter()
        .filter(|token| {
            !exclusions
                .ranges
                .iter()
                .any(|range| range.contains(&token.offset))
        })
        .collect()
}

fn raw_function_scopes(tokens: &[RustToken<'_>]) -> Vec<(String, usize, usize)> {
    let mut scopes = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if token.text != "fn" || token.raw {
            continue;
        }
        let Some(name) = tokens.get(index + 1) else {
            continue;
        };
        if !name
            .text
            .as_bytes()
            .first()
            .is_some_and(|byte| byte.is_ascii_alphabetic() || *byte == b'_')
        {
            continue;
        }
        let Some(parameters) = tokens
            .iter()
            .enumerate()
            .skip(index + 2)
            .find_map(|(position, token)| match token.text {
                "(" => Some(Some(position)),
                ";" | "{" => Some(None),
                _ => None,
            })
            .flatten()
        else {
            continue;
        };
        let Some(parameters_end) = matching_delimiter(tokens, parameters, "(", ")") else {
            continue;
        };
        let Some(body) = tokens
            .iter()
            .enumerate()
            .skip(parameters_end + 1)
            .find_map(|(position, token)| match token.text {
                "{" => Some(Some(position)),
                ";" => Some(None),
                _ => None,
            })
            .flatten()
        else {
            continue;
        };
        if let Some(body_end) = matching_delimiter(tokens, body, "{", "}") {
            scopes.push((name.text.to_owned(), body, body_end));
        }
    }
    scopes
}

fn compact_rust_header(tokens: &[RustToken<'_>]) -> String {
    let mut header = String::new();
    let word_character = |character: char| character.is_ascii_alphanumeric() || character == '_';
    for token in tokens {
        if header.chars().next_back().is_some_and(word_character)
            && token.text.chars().next().is_some_and(word_character)
        {
            header.push(' ');
        }
        header.push_str(token.text);
    }
    header
}

/// Include lexical modules, impl headers, traits, and containing functions in
/// each caller key. Identically named methods are separate removal contracts.
fn function_scopes(tokens: &[RustToken<'_>]) -> Vec<(String, usize, usize)> {
    let functions = raw_function_scopes(tokens);
    let mut contexts = functions.clone();
    for (index, token) in tokens.iter().enumerate() {
        if !matches!(token.text, "mod" | "impl" | "trait") {
            continue;
        }
        if token.text == "impl"
            && index > 0
            && !matches!(
                tokens[index - 1].text,
                "{" | "}" | ";" | "]" | "unsafe" | "default"
            )
        {
            continue;
        }
        let Some(body) = tokens
            .iter()
            .enumerate()
            .skip(index + 2)
            .find_map(|(position, token)| match token.text {
                "{" => Some(Some(position)),
                ";" => Some(None),
                _ => None,
            })
            .flatten()
        else {
            continue;
        };
        let Some(end) = matching_delimiter(tokens, body, "{", "}") else {
            continue;
        };
        let label = match token.text {
            "impl" => format!("impl {}", compact_rust_header(&tokens[index + 1..body])),
            "trait" => format!("trait {}", tokens[index + 1].text),
            _ => tokens[index + 1].text.to_owned(),
        };
        contexts.push((label, body, end));
    }
    let mut qualified = functions
        .into_iter()
        .map(|(name, body, end)| {
            let mut parents = contexts
                .iter()
                .filter(|(_, outer_body, outer_end)| *outer_body < body && end < *outer_end)
                .collect::<Vec<_>>();
            parents.sort_by_key(|(_, body, _)| *body);
            let name = parents
                .into_iter()
                .map(|(name, _, _)| name.as_str())
                .chain([name.as_str()])
                .collect::<Vec<_>>()
                .join("::");
            (name, body, end)
        })
        .collect::<Vec<_>>();
    let mut counts = BTreeMap::new();
    for (name, _, _) in &qualified {
        *counts.entry(name.clone()).or_insert(0) += 1;
    }
    for (name, body, _) in &mut qualified {
        if counts[name] > 1 {
            // Mutually exclusive cfg implementations may share a full path.
            // A physical source position still prevents their counts merging.
            *name = format!("{name}@{}", tokens[*body].offset);
        }
    }
    qualified
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ParserReexports {
    None,
    RootCrate,
    SyntaxCrate,
}

fn file_module_path(source_path: &str, reexports: ParserReexports) -> Vec<String> {
    let path = if reexports == ParserReexports::SyntaxCrate {
        source_path
            .strip_prefix("src/syntax/src/")
            .unwrap_or(source_path)
    } else {
        source_path.strip_prefix("src/").unwrap_or(source_path)
    };
    let mut parts = path
        .trim_end_matches(".rs")
        .split('/')
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if parts
        .last()
        .is_some_and(|name| matches!(name.as_str(), "lib" | "main" | "mod"))
    {
        parts.pop();
    }
    parts
}

fn module_at(tokens: &[RustToken<'_>], index: usize, file_module: &[String]) -> Vec<String> {
    let mut module = file_module.to_vec();
    for cursor in 0..index {
        if tokens[cursor].text == "mod"
            && tokens
                .get(cursor + 2)
                .is_some_and(|token| token.text == "{")
            && matching_delimiter(tokens, cursor + 2, "{", "}").is_some_and(|end| index < end)
        {
            module.push(tokens[cursor + 1].text.to_owned());
        }
    }
    module
}

type CrateNames = BTreeMap<String, ParserReexports>;

fn default_crate_names() -> CrateNames {
    BTreeMap::from([
        ("mech_syntax".to_owned(), ParserReexports::SyntaxCrate),
        ("mech".to_owned(), ParserReexports::RootCrate),
    ])
}

// The syntax crate exports its parser module in full. The product exports a
// smaller root surface plus the complete syntax crate under `syntax`.
fn syntax_root_path(path: &[String], surface: ParserReexports) -> Option<Vec<String>> {
    match surface {
        ParserReexports::SyntaxCrate => Some(path.to_vec()),
        ParserReexports::RootCrate => match path.first().map(String::as_str) {
            Some("syntax") => Some(path[1..].to_vec()),
            None | Some("parse" | "parse_mech" | "parser" | "*") => Some(path.to_vec()),
            _ => None,
        },
        ParserReexports::None => None,
    }
}

// Resolve external Cargo names and self/super against the declaring Rust
// module. This preserves crate identity when a dependency is renamed.
fn syntax_path(
    path: &[String],
    reexports: ParserReexports,
    module: &[String],
    crate_names: &CrateNames,
) -> Option<Vec<String>> {
    if let Some(surface) = path.first().and_then(|name| crate_names.get(name)) {
        return syntax_root_path(&path[1..], *surface);
    }
    if reexports == ParserReexports::RootCrate && path.first().is_some_and(|name| name == "syntax")
    {
        return Some(path[1..].to_vec());
    }
    if reexports == ParserReexports::None {
        return None;
    }
    let mut resolved = Vec::new();
    let mut cursor = 0;
    match path.first()?.as_str() {
        "crate" => cursor = 1,
        "self" => {
            resolved = module.to_vec();
            cursor = 1;
        }
        "super" => {
            resolved = module.to_vec();
            while path.get(cursor).is_some_and(|name| name == "super") {
                resolved.pop()?;
                cursor += 1;
            }
        }
        _ => return None,
    }
    resolved.extend_from_slice(&path[cursor..]);
    syntax_root_path(&resolved, reexports)
}

fn source_entrypoint(name: &str) -> bool {
    matches!(
        name,
        "parse" | "parse_mech" | "program" | "mech_code" | "mech_code_alt"
    )
}

fn parser_reference_len(
    tokens: &[RustToken<'_>],
    start: usize,
    reexports: ParserReexports,
    module: &[String],
    imported_parser: bool,
    crate_names: &CrateNames,
) -> Option<usize> {
    // Never reinterpret a suffix of document::parser (or any other path) as
    // the retiring root parser module.
    if start > 0 && tokens[start - 1].text == ":" && start > 1 && tokens[start - 2].text != ":" {
        return None;
    }
    if start >= 3
        && tokens[start - 1].text == ":"
        && tokens[start - 2].text == ":"
        && tokens[start - 3]
            .text
            .chars()
            .next()
            .is_some_and(|ch| ch.is_alphanumeric() || ch == '_')
    {
        return None;
    }
    let mut path = vec![tokens[start].text.to_owned()];
    let mut len = 1;
    while tokens
        .get(start + len)
        .is_some_and(|token| token.text == ":")
        && tokens
            .get(start + len + 1)
            .is_some_and(|token| token.text == ":")
        && tokens.get(start + len + 2).is_some_and(|token| {
            token
                .text
                .chars()
                .next()
                .is_some_and(|ch| ch.is_alphabetic() || ch == '_')
        })
    {
        path.push(tokens[start + len + 2].text.to_owned());
        len += 3;
    }
    let resolved = if path.first().is_some_and(|name| name == "parser") && imported_parser {
        Some(path)
    } else {
        syntax_path(&path, reexports, module, crate_names)
    }?;
    let entry = match resolved.as_slice() {
        [name] => source_entrypoint(name),
        [parser, name] if parser == "parser" => source_entrypoint(name),
        _ => false,
    };
    entry.then_some(len)
}

fn source_line(source: &str, offset: usize) -> usize {
    source[..offset]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1
}

fn flattened_imports(
    tree: &syn::UseTree,
    prefix: &[String],
    result: &mut Vec<(Vec<String>, Option<String>)>,
) {
    let identifier = |name: &syn::Ident| name.to_string().trim_start_matches("r#").to_owned();
    let mut path = prefix.to_vec();
    match tree {
        syn::UseTree::Path(tree) => {
            path.push(identifier(&tree.ident));
            flattened_imports(&tree.tree, &path, result);
        }
        syn::UseTree::Name(tree) => {
            if tree.ident != "self" || path.is_empty() {
                path.push(identifier(&tree.ident));
            }
            result.push((path, None));
        }
        syn::UseTree::Rename(tree) => {
            if tree.ident != "self" || path.is_empty() {
                path.push(identifier(&tree.ident));
            }
            result.push((path, Some(identifier(&tree.rename))));
        }
        syn::UseTree::Glob(_) => {
            path.push("*".to_owned());
            result.push((path, None));
        }
        syn::UseTree::Group(tree) => {
            for tree in &tree.items {
                flattened_imports(tree, &path, result);
            }
        }
    }
}

fn imports_retiring_parser(
    import: &[RustToken<'_>],
    reexports: ParserReexports,
    module: &[String],
    crate_names: &CrateNames,
    reexport_item: bool,
    imported_parser: bool,
) -> bool {
    let source = compact_rust_header(import) + ";";
    let Ok(item) = syn::parse_str::<syn::ItemUse>(&source) else {
        // Opaque macro bodies can contain generated use trees (`paste!`,
        // metavariables). They cannot introduce an unreviewed syntax import.
        return import.iter().any(|token| {
            crate_names.contains_key(token.text)
                || imported_parser && token.text == "parser"
                || (reexports != ParserReexports::None
                    && matches!(token.text, "crate" | "self" | "super"))
        });
    };
    let mut imports = Vec::new();
    flattened_imports(&item.tree, &[], &mut imports);
    imports.iter().any(|(path, alias)| {
        let resolved = if imported_parser && path.first().is_some_and(|name| name == "parser") {
            Some(path.clone())
        } else {
            syntax_path(path, reexports, module, crate_names)
        };
        let Some(resolved) = resolved else {
            return false;
        };
        match resolved.as_slice() {
            [] => alias.is_some(),
            [name] if name == "*" => reexport_item || path != &["crate".to_owned(), "*".to_owned()],
            [name] if source_entrypoint(name) => true,
            [name] if name == "parser" => reexport_item || alias.is_some(),
            [name, ..] if name == "parser" => true,
            _ => false,
        }
    })
}

fn is_declared_root_parser_reexport(
    source_path: &str,
    import: &[RustToken<'_>],
    public_item: bool,
    root_declaration: bool,
) -> bool {
    if source_path != "src/lib.rs" || !public_item || !root_declaration {
        return false;
    }
    let Ok(item) = syn::parse_str::<syn::ItemUse>(&(compact_rust_header(import) + ";")) else {
        return false;
    };
    let mut imports = Vec::new();
    flattened_imports(&item.tree, &[], &mut imports);
    imports.iter().all(|(path, alias)| alias.is_none() && matches!(path.as_slice(), [root, name]
        if root == "mech_syntax" && matches!(name.as_str(),
            "ParseError" | "ParseErrorDetail" | "ParseResult" | "ParseString" | "ParserErrorContext" |
            "ParserErrorReport" | "SubmissionTerminal" | "TextFormatter" | "alt_best" | "graphemes" |
            "parse" | "parse_grammar" | "parse_mech" | "parser" | "submission_terminal")))
}

fn is_existing_syntax_parser_import(
    source_path: &str,
    import: &[RustToken<'_>],
    public_item: bool,
    file_top_level: bool,
    file_module: &[String],
) -> bool {
    file_top_level
        && match (source_path, public_item) {
            ("src/syntax/src/lib.rs", true) => file_module.is_empty(),
            ("src/syntax/src/base.rs", false) => file_module == ["base"],
            _ => false,
        }
        && import
            .iter()
            .map(|token| token.text)
            .eq(["use", "crate", ":", ":", "parser", ":", ":", "*"])
}

fn is_declared_syntax_crate_reexport(
    source_path: &str,
    tokens: &[RustToken<'_>],
    index: usize,
    root_declaration: bool,
) -> bool {
    root_declaration
        && source_path == "src/lib.rs"
        && index > 0
        && tokens[index - 1].text == "pub"
        && tokens
            .get(index)
            .is_some_and(|token| token.text == "extern")
        && tokens
            .get(index + 1)
            .is_some_and(|token| token.text == "crate")
        && tokens
            .get(index + 2)
            .is_some_and(|token| token.text == "mech_syntax")
        && tokens
            .get(index + 3)
            .is_some_and(|token| token.text == "as")
        && tokens
            .get(index + 4)
            .is_some_and(|token| token.text == "syntax")
        && tokens.get(index + 5).is_some_and(|token| token.text == ";")
}

fn bare_parser_available(
    tokens: &[RustToken<'_>],
    source_path: &str,
    reexports: ParserReexports,
) -> bool {
    if reexports == ParserReexports::None {
        return false;
    }
    matches!(
        source_path,
        "src/lib.rs" | "src/syntax/src/lib.rs" | "src/syntax/src/base.rs"
    ) || tokens.windows(6).any(|tokens| {
        tokens
            .iter()
            .map(|token| token.text)
            .eq(["use", "crate", ":", ":", "*", ";"])
    })
}

fn parser_import_bindings(
    tokens: &[RustToken<'_>],
    reexports: ParserReexports,
    file_module: &[String],
    crate_names: &CrateNames,
) -> Vec<(usize, usize, bool)> {
    let mut bindings = Vec::new();
    let mut blocks = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        match token.text {
            "{" => blocks.push(index),
            "}" => {
                blocks.pop();
            }
            "use" if !token.raw => {
                let end = tokens[index..]
                    .iter()
                    .position(|token| token.text == ";")
                    .map_or(tokens.len(), |length| index + length);
                let Ok(item) = syn::parse_str::<syn::ItemUse>(
                    &(compact_rust_header(&tokens[index..end]) + ";"),
                ) else {
                    continue;
                };
                let mut imports = Vec::new();
                flattened_imports(&item.tree, &[], &mut imports);
                for (path, alias) in imports {
                    if alias
                        .as_ref()
                        .or(path.last())
                        .is_some_and(|name| name == "parser")
                    {
                        let retiring = syntax_path(
                            &path,
                            reexports,
                            &module_at(tokens, index, file_module),
                            crate_names,
                        )
                        .is_some_and(|path| path == ["parser".to_owned()]);
                        let start = blocks.last().copied().unwrap_or(0);
                        let end = blocks
                            .last()
                            .and_then(|start| matching_delimiter(tokens, *start, "{", "}"))
                            .unwrap_or(tokens.len());
                        bindings.push((start, end, retiring));
                    }
                }
            }
            _ => {}
        }
    }
    bindings
}

fn scan_rust_source(source: &str, source_path: &str, reexports: ParserReexports) -> SourceScan {
    scan_rust_source_in_module(
        source,
        source_path,
        reexports,
        &file_module_path(source_path, reexports),
        &default_crate_names(),
    )
}

fn imported_parser_at(bindings: &[(usize, usize, bool)], index: usize, fallback: bool) -> bool {
    // Conditional imports in the same lexical scope are alternative bindings.
    // A later canonical alternative cannot hide a possible retiring binding.
    let scope = bindings
        .iter()
        .filter(|(start, end, _)| *start <= index && index < *end)
        .map(|(start, _, _)| *start)
        .max();
    scope.map_or(fallback, |scope| {
        bindings
            .iter()
            .any(|(start, end, retiring)| *start == scope && index < *end && *retiring)
    })
}

fn restricted_visibility_before(tokens: &[RustToken<'_>], index: usize) -> bool {
    if index == 0 || tokens[index - 1].text != ")" {
        return false;
    }
    let mut depth = 0usize;
    for cursor in (0..index).rev() {
        match tokens[cursor].text {
            ")" => depth += 1,
            "(" => {
                depth -= 1;
                if depth == 0 {
                    return cursor > 0
                        && tokens[cursor - 1].text == "pub"
                        && !tokens[cursor - 1].raw;
                }
            }
            _ => {}
        }
    }
    false
}

fn scan_rust_source_in_module(
    source: &str,
    source_path: &str,
    reexports: ParserReexports,
    file_module: &[String],
    crate_names: &CrateNames,
) -> SourceScan {
    let tokens = production_tokens(source);
    let scopes = function_scopes(&tokens);
    let bare_parser = bare_parser_available(&tokens, source_path, reexports);
    let parser_bindings = parser_import_bindings(&tokens, reexports, file_module, crate_names);
    let mut scan = SourceScan::default();
    let mut imports = Vec::new();

    let mut brace_depth = 0usize;
    for (index, token) in tokens.iter().enumerate() {
        let file_top_level = brace_depth == 0;
        let root_declaration = file_top_level && file_module.is_empty();
        match token.text {
            "{" => brace_depth += 1,
            "}" => brace_depth = brace_depth.saturating_sub(1),
            _ => {}
        }
        let public_item = index > 0 && tokens[index - 1].text == "pub";
        let restricted_item = token.text == "use" && restricted_visibility_before(&tokens, index);
        if token.text == "use"
            && !token.raw
            && !tokens.get(index + 1).is_some_and(|token| token.text == "<")
        {
            let end = tokens[index..]
                .iter()
                .position(|token| token.text == ";")
                .map_or(tokens.len(), |offset| index + offset);
            imports.push(index..end);
            let import = &tokens[index..end];
            if imports_retiring_parser(
                import,
                reexports,
                &module_at(&tokens, index, file_module),
                crate_names,
                public_item || restricted_item,
                imported_parser_at(&parser_bindings, index, bare_parser),
            ) && !is_declared_root_parser_reexport(
                source_path,
                import,
                public_item,
                root_declaration,
            ) && (restricted_item
                || !is_existing_syntax_parser_import(
                    source_path,
                    import,
                    public_item,
                    file_top_level,
                    file_module,
                ))
            {
                scan.prohibited_aliases.push(format!(
                    "line {} imports or aliases the retiring parser",
                    source_line(source, token.offset)
                ));
            }
        }
        if token.text == "extern"
            && !token.raw
            && tokens
                .get(index + 1)
                .is_some_and(|token| token.text == "crate")
            && tokens.get(index + 2).is_some_and(|token| {
                crate_names.contains_key(token.text)
                    || token.text == "self" && reexports != ParserReexports::None
            })
            && tokens
                .get(index + 3)
                .is_some_and(|token| token.text == "as")
            && !is_declared_syntax_crate_reexport(source_path, &tokens, index, root_declaration)
        {
            scan.prohibited_aliases.push(format!(
                "line {} aliases the mech_syntax crate",
                source_line(source, token.offset)
            ));
        }
    }

    let mut index = 0usize;
    while index < tokens.len() {
        let bare_reference = bare_parser
            && (tokens[index].text == "parse"
                || tokens[index].text == "parse_mech" && source_path != "src/syntax/src/parser.rs")
            && (index == 0 || !matches!(tokens[index - 1].text, "fn" | "." | ":"));
        let qualified = crate_names.contains_key(tokens[index].text)
            || matches!(
                tokens[index].text,
                "syntax" | "crate" | "self" | "super" | "parser"
            );
        let Some(path_len) = qualified
            .then(|| {
                parser_reference_len(
                    &tokens,
                    index,
                    reexports,
                    &module_at(&tokens, index, file_module),
                    imported_parser_at(&parser_bindings, index, bare_parser),
                    crate_names,
                )
            })
            .flatten()
            .or_else(|| bare_reference.then_some(1))
        else {
            index += 1;
            continue;
        };
        let reference = &tokens[index..index + path_len];
        let is_import = imports.iter().any(|range| range.contains(&index));
        if !is_import {
            if reference.last().is_some_and(|token| token.text != "parse") {
                scan.prohibited_aliases.push(format!(
                    "line {} uses an alternate retiring source entrypoint",
                    source_line(source, reference[0].offset)
                ));
            }
            if tokens
                .get(index + path_len)
                .is_some_and(|token| token.text == "(")
            {
                let caller = scopes
                    .iter()
                    .filter(|(_, body, end)| *body < index && index < *end)
                    .max_by_key(|(_, body, _)| *body)
                    .map_or("<module>", |(name, _, _)| name.as_str());
                scan.call_sites
                    .entry(caller.to_owned())
                    .or_default()
                    .insert(reference[0].offset);
            } else {
                scan.prohibited_aliases.push(format!(
                    "line {} takes or aliases a retiring parser function",
                    source_line(source, reference[0].offset)
                ));
            }
        }
        index += path_len;
    }
    scan
}

fn cargo_metadata(root: &Path, no_dependencies: bool) -> serde_json::Value {
    let mut command = std::process::Command::new(env!("CARGO"));
    command.args([
        "metadata",
        "--format-version",
        "1",
        "--all-features",
        "--offline",
        "--locked",
    ]);
    if no_dependencies {
        command.arg("--no-deps");
    }
    let output = command
        .current_dir(root)
        .output()
        .expect("run locked dependency metadata");
    assert!(
        output.status.success(),
        "dependency metadata for {} failed: {}",
        root.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("dependency metadata JSON")
}

fn manifest_declarations(manifest: &Path) -> serde_json::Value {
    // Cargo supplies the TOML parser only. Isolate config loading completely:
    // manifest-relative paths and patch-source matching are handled below,
    // never by Cargo config precedence or config-relative path rules.
    let scratch = CensusFixture::new();
    let output = std::process::Command::new(env!("CARGO"))
        .env_clear()
        .env("CARGO_HOME", scratch.0.join("cargo-home"))
        .current_dir(&scratch.0)
        .args(["-Z", "unstable-options", "--config"])
        .arg(manifest)
        .args(["config", "get", "--format", "json", "--locked", "--offline"])
        .output()
        .expect("read manifest declarations with Cargo");
    assert!(
        output.status.success(),
        "Cargo manifest extraction failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("Cargo manifest declaration JSON")
}

#[derive(Debug)]
struct LocalPatch {
    source: String,
    package: String,
    manifest: PathBuf,
}

fn declared_local_patches(root: &Path) -> Vec<LocalPatch> {
    let declarations = manifest_declarations(&root.join("Cargo.toml"));
    let mut patches = Vec::new();
    for (source, entries) in declarations["patch"].as_object().into_iter().flatten() {
        for (alias, patch) in entries.as_object().expect("patch entries") {
            let Some(path) = patch["path"].as_str() else {
                continue;
            };
            let manifest = root
                .join(path)
                .join("Cargo.toml")
                .canonicalize()
                .expect("local patch manifest");
            if manifest.starts_with(root) {
                patches.push(LocalPatch {
                    source: source.clone(),
                    package: patch["package"].as_str().unwrap_or(alias).to_owned(),
                    manifest,
                });
            }
        }
    }
    patches
}

fn patch_matches_dependency(patch: &LocalPatch, dependency: &serde_json::Value) -> bool {
    if dependency["name"].as_str() != Some(patch.package.as_str()) {
        return false;
    }
    let Some(source) = dependency["source"].as_str() else {
        return false;
    };
    if patch.source == "crates-io" {
        return source == "registry+https://github.com/rust-lang/crates.io-index";
    }
    source
        .strip_prefix("registry+")
        .is_some_and(|source| source == patch.source)
        || source
            .strip_prefix("git+")
            .is_some_and(|source| source.split(['?', '#']).next() == Some(patch.source.as_str()))
}

fn include_local_packages(
    root: &Path,
    metadata: &serde_json::Value,
    packages: &mut BTreeMap<PathBuf, serde_json::Value>,
    pending: &mut Vec<PathBuf>,
) {
    // Cargo lists development packages in metadata even when they cannot be
    // reached by any production build. Traverse its resolved non-dev edges
    // from workspace packages, retaining external intermediaries along the way.
    let nodes = metadata["resolve"]["nodes"]
        .as_array()
        .expect("Cargo resolve nodes")
        .iter()
        .map(|node| (node["id"].as_str().expect("resolved package id"), node))
        .collect::<BTreeMap<_, _>>();
    let mut reachable = BTreeSet::new();
    let mut work = metadata["workspace_members"]
        .as_array()
        .expect("workspace members")
        .iter()
        .map(|id| id.as_str().expect("workspace package id"))
        .collect::<Vec<_>>();
    while let Some(id) = work.pop() {
        if !reachable.insert(id) {
            continue;
        }
        for dependency in nodes.get(id).expect("workspace dependency resolve node")["deps"]
            .as_array()
            .expect("resolved dependencies")
        {
            if dependency["dep_kinds"]
                .as_array()
                .expect("dependency kinds")
                .iter()
                .any(|kind| kind["kind"].as_str() != Some("dev"))
            {
                work.push(dependency["pkg"].as_str().expect("resolved dependency id"));
            }
        }
    }
    for package in metadata["packages"].as_array().expect("Cargo packages") {
        let manifest = Path::new(package["manifest_path"].as_str().expect("manifest path"))
            .canonicalize()
            .expect("canonical package manifest");
        if reachable.contains(package["id"].as_str().expect("package id"))
            && manifest.starts_with(root)
            && !packages.contains_key(&manifest)
        {
            pending.push(manifest.clone());
            packages.insert(manifest, package.clone());
        }
    }
}

fn declared_package(manifest: &Path) -> serde_json::Value {
    // --no-deps reads inherited declarations without resolving this independent
    // workspace or creating its lockfile. Only the requested package is part of
    // this dependency edge; unrelated workspace members do not become callers.
    cargo_metadata(manifest.parent().expect("package directory"), true)["packages"]
        .as_array()
        .expect("Cargo packages")
        .iter()
        .find(|package| {
            Path::new(package["manifest_path"].as_str().expect("manifest path"))
                .canonicalize()
                .expect("canonical manifest")
                == manifest
        })
        .unwrap_or_else(|| panic!("Cargo metadata omitted {}", manifest.display()))
        .clone()
}

fn cargo_version_matches(name: &str, requirement: &str, version: &str) -> bool {
    // Cargo remains the version authority. This disposable, fully locked
    // two-package graph asks only whether the declared requirement admits the
    // actual candidate version; it cannot resolve or change repository inputs.
    let scratch = CensusFixture::new();
    let probe_name = format!("{name}-census-requirement-probe");
    scratch.write("Cargo.toml", &format!(
        "[package]\nname = {probe_name:?}\nversion = \"0.0.0\"\nedition = \"2024\"\n[workspace]\n[dependencies]\ncandidate = {{ package = {name:?}, version = {requirement:?}, path = \"candidate\" }}\n"
    ));
    scratch.write("src/lib.rs", "");
    scratch.write(
        "candidate/Cargo.toml",
        &format!("[package]\nname = {name:?}\nversion = {version:?}\nedition = \"2024\"\n"),
    );
    scratch.write("candidate/src/lib.rs", "");
    let lock = format!(
        "version = 4\n[[package]]\nname = {probe_name:?}\nversion = \"0.0.0\"\ndependencies = [{name:?}]\n[[package]]\nname = {name:?}\nversion = {version:?}\n"
    );
    scratch.write("Cargo.lock", &lock);
    let output = std::process::Command::new(env!("CARGO"))
        .env_clear()
        .env("CARGO_HOME", scratch.0.join("cargo-home"))
        .env("RUSTC", Path::new(env!("CARGO")).with_file_name("rustc"))
        .current_dir(&scratch.0)
        .args(["metadata", "--format-version", "1", "--locked", "--offline"])
        .output()
        .expect("Cargo version eligibility probe");
    assert_eq!(
        fs::read_to_string(scratch.0.join("Cargo.lock")).unwrap(),
        lock
    );
    if output.status.success() {
        let metadata: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("probe metadata");
        assert!(
            metadata["packages"]
                .as_array()
                .unwrap()
                .iter()
                .any(|package| package["name"] == name && package["version"] == version)
        );
        true
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains("failed to select a version for the requirement")
                && error.contains(name)
                && error.contains("candidate versions found which didn't match")
                && error.contains(version),
            "unexpected Cargo requirement failure: {error}"
        );
        false
    }
}

fn production_package_metadata(root: &Path) -> Vec<serde_json::Value> {
    let mut packages = BTreeMap::<PathBuf, serde_json::Value>::new();
    let mut pending = Vec::new();
    include_local_packages(
        root,
        &cargo_metadata(root, false),
        &mut packages,
        &mut pending,
    );
    let patches = declared_local_patches(root);
    let mut declarations = packages.clone();
    let mut version_matches = BTreeMap::new();
    while let Some(manifest) = pending.pop() {
        // Root --all-features does not activate every feature of dependencies
        // belonging to independent workspaces. Cargo's declaration metadata
        // includes their inactive optional and target-specific dependencies.
        let dependencies = packages[&manifest]["dependencies"]
            .as_array()
            .expect("package dependencies")
            .clone();
        for dependency in dependencies
            .iter()
            .filter(|dependency| dependency["kind"].as_str() != Some("dev"))
        {
            let mut candidates = Vec::new();
            if let Some(path) = dependency["path"].as_str() {
                candidates.push((
                    Path::new(path)
                        .join("Cargo.toml")
                        .canonicalize()
                        .expect("local dependency manifest"),
                    false,
                ));
            } else {
                candidates.extend(
                    patches
                        .iter()
                        .filter(|patch| patch_matches_dependency(patch, dependency))
                        .map(|patch| (patch.manifest.clone(), true)),
                );
            }
            for (manifest, patched) in candidates {
                if !manifest.starts_with(root) {
                    continue;
                }
                let package = declarations
                    .entry(manifest.clone())
                    .or_insert_with(|| declared_package(&manifest));
                let name = package["name"].as_str().expect("package name");
                assert_eq!(
                    Some(name),
                    dependency["name"].as_str(),
                    "local dependency package identity"
                );
                if patched {
                    let requirement = dependency["req"].as_str().expect("dependency requirement");
                    let version = package["version"].as_str().expect("package version");
                    let key = (name.to_owned(), requirement.to_owned(), version.to_owned());
                    if !*version_matches
                        .entry(key)
                        .or_insert_with(|| cargo_version_matches(name, requirement, version))
                    {
                        continue;
                    }
                }
                if !packages.contains_key(&manifest) {
                    pending.push(manifest.clone());
                    packages.insert(manifest, package.clone());
                }
            }
        }
    }
    packages.into_values().collect()
}

fn workspace_metadata(root: &Path) -> &'static [serde_json::Value] {
    static METADATA: std::sync::OnceLock<Vec<serde_json::Value>> = std::sync::OnceLock::new();
    METADATA.get_or_init(|| production_package_metadata(root))
}

fn local_packages<'a>(
    root: &Path,
    metadata: &'a [serde_json::Value],
) -> impl Iterator<Item = &'a serde_json::Value> {
    let root = root.to_owned();
    metadata.iter().filter(move |package| {
        Path::new(package["manifest_path"].as_str().expect("manifest path")).starts_with(&root)
    })
}

fn package_source_roots(package: &serde_json::Value) -> BTreeSet<PathBuf> {
    let source = Path::new(package["manifest_path"].as_str().expect("manifest path"))
        .parent()
        .expect("package directory")
        .join("src");
    source.is_dir().then_some(source).into_iter().collect()
}

fn package_target_roots(package: &serde_json::Value) -> BTreeSet<PathBuf> {
    package["targets"]
        .as_array()
        .expect("package targets")
        .iter()
        .filter(|target| {
            target["kind"]
                .as_array()
                .expect("target kinds")
                .iter()
                .any(|kind| {
                    matches!(
                        kind.as_str(),
                        Some(
                            "lib"
                                | "rlib"
                                | "dylib"
                                | "cdylib"
                                | "staticlib"
                                | "proc-macro"
                                | "bin"
                                | "custom-build"
                        )
                    )
                })
        })
        .map(|target| PathBuf::from(target["src_path"].as_str().expect("target source")))
        .collect()
}

fn package_crate_names(package: &serde_json::Value) -> CrateNames {
    // Metadata resolves workspace dependency inheritance, target conditions,
    // renamed package keys and optional dependencies without source spelling
    // guesses. Keep every production configuration in the census.
    package["dependencies"]
        .as_array()
        .expect("package dependencies")
        .iter()
        .filter(|dependency| dependency["kind"].as_str() != Some("dev"))
        .filter_map(|dependency| {
            let surface = match dependency["name"].as_str()? {
                "mech-syntax" => ParserReexports::SyntaxCrate,
                "mech" => ParserReexports::RootCrate,
                _ => return None,
            };
            let name = dependency["rename"]
                .as_str()
                .or(dependency["name"].as_str())
                .expect("dependency name");
            Some((name.replace('-', "_"), surface))
        })
        .collect()
}

fn workspace_source_roots(root: &Path) -> BTreeSet<PathBuf> {
    local_packages(root, workspace_metadata(root))
        .flat_map(package_source_roots)
        .collect()
}

fn production_target_roots(root: &Path) -> BTreeSet<PathBuf> {
    local_packages(root, workspace_metadata(root))
        .flat_map(package_target_roots)
        .collect()
}

fn conditional_module_paths(
    meta: &syn::Meta,
    definite: bool,
    possible: bool,
    paths: &mut Vec<(String, bool, bool)>,
) {
    if let syn::Meta::NameValue(value) = meta
        && value.path.is_ident("path")
        && let syn::Expr::Lit(value) = &value.value
        && let syn::Lit::Str(value) = &value.lit
    {
        paths.push((value.value(), definite, possible));
    } else if let syn::Meta::List(list) = meta
        && list.path.is_ident("cfg_attr")
    {
        let arguments = syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated
            .parse2(list.tokens.clone())
            .expect("valid module cfg_attr");
        if let Some(condition) = arguments.first() {
            let enabled = cfg_predicate(condition);
            for nested in arguments.iter().skip(1) {
                // Keep disabled alternatives as graph edges, so a named test
                // path cannot be mistaken for an unreferenced production file.
                conditional_module_paths(
                    nested,
                    definite && enabled == Some(true),
                    possible && enabled != Some(false),
                    paths,
                );
            }
        }
    }
}

struct ModuleDeclarations<'a> {
    directory: PathBuf,
    path_directory: PathBuf,
    exclusions: &'a ProductionExclusions,
    module: Vec<String>,
    enabled: bool,
    edges: Vec<(PathBuf, bool, Vec<String>, bool)>,
}

impl<'ast> Visit<'ast> for ModuleDeclarations<'_> {
    fn visit_item_mod(&mut self, module: &'ast syn::ItemMod) {
        let enabled = self.enabled
            && !self
                .exclusions
                .ranges
                .iter()
                .any(|range| range.contains(&module.span().byte_range().start));
        let mut paths = Vec::new();
        for attribute in &module.attrs {
            conditional_module_paths(&attribute.meta, true, true, &mut paths);
        }
        let definite_path = paths.iter().any(|(_, definite, _)| *definite);
        let name = module.ident.to_string();
        let name = name.strip_prefix("r#").unwrap_or(&name);
        self.module.push(name.to_owned());
        if module.content.is_some() {
            let saved_enabled = self.enabled;
            let saved_directory = self.directory.clone();
            let saved_path_directory = self.path_directory.clone();
            let mut directories = paths
                .iter()
                .map(|(path, _, possible)| (self.directory.join(path), *possible))
                .collect::<Vec<_>>();
            if !definite_path {
                directories.push((self.directory.join(name), true));
            }
            for (directory, possible) in directories {
                self.enabled = enabled && possible;
                self.directory = directory.clone();
                self.path_directory = directory;
                syn::visit::visit_item_mod(self, module);
            }
            self.enabled = saved_enabled;
            self.directory = saved_directory;
            self.path_directory = saved_path_directory;
        } else {
            let mut candidates = paths
                .iter()
                .map(|(path, _, possible)| (self.path_directory.join(path), true, *possible))
                .collect::<Vec<_>>();
            if !definite_path {
                candidates.extend([
                    (self.directory.join(format!("{name}.rs")), false, true),
                    (self.directory.join(name).join("mod.rs"), false, true),
                ]);
            }
            for (path, owns_parent, possible) in candidates {
                if path.is_file() {
                    self.edges.push((
                        path.canonicalize().expect("module path"),
                        enabled && possible,
                        self.module.clone(),
                        owns_parent,
                    ));
                }
            }
        }
        self.module.pop();
    }
}

fn module_edges(
    path: &Path,
    file: &syn::File,
    crate_root: bool,
) -> Vec<(PathBuf, bool, Vec<String>, bool)> {
    let exclusions = production_exclusions(file);
    let parent = path.parent().expect("source directory");
    let directory = if crate_root || path.file_name().is_some_and(|name| name == "mod.rs") {
        parent.to_owned()
    } else {
        parent.join(path.file_stem().expect("module stem"))
    };
    let mut declarations = ModuleDeclarations {
        directory,
        path_directory: parent.to_owned(),
        exclusions: &exclusions,
        module: Vec::new(),
        enabled: true,
        edges: Vec::new(),
    };
    declarations.visit_file(file);
    declarations.edges
}

fn production_source_modules(
    roots: &BTreeSet<PathBuf>,
    targets: &BTreeSet<PathBuf>,
) -> BTreeMap<PathBuf, BTreeSet<Vec<String>>> {
    let mut files = targets.clone();
    for root in roots {
        visit_rust_files(root, &mut files);
    }
    let mut graph = BTreeMap::new();
    let mut file_disabled = BTreeSet::new();
    let mut queue = files.iter().cloned().collect::<Vec<_>>();
    while let Some(path) = queue.pop() {
        let source = fs::read_to_string(&path).expect("read Rust module");
        let file = syn::parse_file(&source).unwrap_or_else(|error| {
            panic!(
                "census cannot parse {} at {:?}: {error}",
                path.display(),
                error.span().start()
            )
        });
        if excludes_production(&file.attrs) {
            file_disabled.insert(path.clone());
        }
        for owns_parent in [false, true] {
            let edges = module_edges(&path, &file, owns_parent);
            for (child, _, _, _) in &edges {
                if files.insert(child.clone()) {
                    queue.push(child.clone());
                }
            }
            graph.insert((path.clone(), owns_parent), edges);
        }
    }
    let referenced = graph
        .values()
        .flatten()
        .map(|(path, _, _, _)| path.clone())
        .collect::<BTreeSet<_>>();
    // Unreferenced source files are scanned conservatively. Exclusion requires
    // an actual cfg-disabled declaring path, never a tests.rs/tests basename.
    let mut queue = targets
        .iter()
        .chain(files.difference(&referenced))
        .map(|path| {
            (
                path.clone(),
                Vec::new(),
                BTreeSet::new(),
                targets.contains(path),
            )
        })
        .collect::<Vec<_>>();
    let mut production: BTreeMap<PathBuf, BTreeSet<Vec<String>>> = BTreeMap::new();
    let mut visited = BTreeSet::new();
    while let Some((path, module, mut ancestry, owns_parent)) = queue.pop() {
        if file_disabled.contains(&path) {
            continue;
        }
        assert!(
            ancestry.insert(path.clone()),
            "cyclic Rust modules at {}",
            path.display()
        );
        if !visited.insert((path.clone(), module.clone(), owns_parent)) {
            continue;
        }
        production
            .entry(path.clone())
            .or_default()
            .insert(module.clone());
        for (child, enabled, suffix, child_owns_parent) in &graph[&(path, owns_parent)] {
            if *enabled {
                let mut child_module = module.clone();
                child_module.extend(suffix.iter().cloned());
                queue.push((
                    child.clone(),
                    child_module,
                    ancestry.clone(),
                    *child_owns_parent,
                ));
            }
        }
    }
    production
}

fn parser_reexports(root: &Path, path: &Path) -> ParserReexports {
    let mut directory = path.parent();
    while let Some(candidate) = directory {
        if candidate.join("Cargo.toml").is_file() {
            return if candidate == root {
                ParserReexports::RootCrate
            } else if candidate == root.join("src/syntax") {
                ParserReexports::SyntaxCrate
            } else {
                ParserReexports::None
            };
        }
        if candidate == root {
            break;
        }
        directory = candidate.parent();
    }
    ParserReexports::None
}

fn discovered_calls_in_packages(
    root: &Path,
    metadata: &[serde_json::Value],
) -> (BTreeMap<(String, String), usize>, Vec<String>) {
    let mut calls: BTreeMap<(String, String), BTreeSet<usize>> = BTreeMap::new();
    let mut prohibited_aliases = BTreeSet::new();
    for package in local_packages(root, metadata) {
        let manifest = Path::new(package["manifest_path"].as_str().expect("manifest path"));
        let reexports = parser_reexports(root, manifest);
        let crate_names = package_crate_names(package);
        let files = production_source_modules(
            &package_source_roots(package),
            &package_target_roots(package),
        );
        for (path, modules) in files {
            let relative = path
                .strip_prefix(root)
                .expect("source path is below repository root")
                .to_string_lossy()
                .replace('\\', "/");
            let source = fs::read_to_string(&path).expect("read Rust source");
            // #[path] source inherits the crate that compiles it, not the
            // nearest manifest to the physical source file. A shared module is
            // checked in each compiling crate's namespace; physical calls count once.
            for module in modules {
                let scan = scan_rust_source_in_module(
                    &source,
                    &relative,
                    reexports,
                    &module,
                    &crate_names,
                );
                prohibited_aliases.extend(
                    scan.prohibited_aliases
                        .into_iter()
                        .map(|violation| format!("{relative}:{violation}")),
                );
                for (caller, sites) in scan.call_sites {
                    calls
                        .entry((relative.clone(), caller))
                        .or_default()
                        .extend(sites);
                }
            }
        }
    }
    (
        calls
            .into_iter()
            .map(|(caller, sites)| (caller, sites.len()))
            .collect(),
        prohibited_aliases.into_iter().collect(),
    )
}

fn discovered_production_calls() -> (BTreeMap<(String, String), usize>, Vec<String>) {
    let root = repository_root();
    discovered_calls_in_packages(&root, workspace_metadata(&root))
}

#[test]
fn production_parser_callers_are_exactly_inventoried() {
    let consumers = consumers();
    let mut expected = BTreeMap::new();
    let mut total = 0usize;
    for (id, row) in &consumers {
        let source = fs::read_to_string(repository_root().join(&row.source_path))
            .unwrap_or_else(|error| panic!("read consumer source for {id}: {error}"));
        assert!(
            function_scopes(&production_tokens(&source))
                .iter()
                .any(|(caller, _, _)| caller == &row.caller),
            "missing caller {} for {id}",
            row.caller
        );
        for (field, value) in [
            ("feature gate", &row.feature_gate),
            ("input policy", &row.input_policy),
            ("failure policy", &row.failure_policy),
            ("current result use", &row.current_result_use),
            ("canonical target", &row.canonical_target),
            ("cutover action", &row.cutover_action),
            ("notes", &row.notes),
        ] {
            assert!(!value.is_empty(), "empty {field} for {id}");
        }
        assert!(
            !row.canonical_target.contains("legacy")
                && !row.canonical_target.contains("compat")
                && !row.cutover_action.contains("legacy")
                && !row.cutover_action.contains("compat"),
            "noncanonical cutover target for {id}"
        );
        assert!(
            expected
                .insert((row.source_path.clone(), row.caller.clone()), row.calls)
                .is_none(),
            "duplicate source/caller contract for {id}"
        );
        total += row.calls;
    }
    assert_eq!(total, EXPECTED_PRODUCTION_CALLS);
    let (discovered, prohibited_aliases) = discovered_production_calls();
    assert!(
        prohibited_aliases.is_empty(),
        "production parser aliases are prohibited: {prohibited_aliases:#?}"
    );
    assert_eq!(discovered, expected);
}

#[test]
fn source_scanner_binds_each_call_to_its_enclosing_function() {
    let source = r##"
use mech_syntax::parser;
fn first() {
    let _ = "mech_syntax::parse(ignored)";
    let _ = r#"parser::parse(ignored)"#;
    // mech_syntax::parser::parse(ignored)
    mech_syntax::parse("one");
    parser::parse("two");
}

fn second() {
    /* parser::parse(ignored) */
    mech_syntax::parser::parse("three");
}
"##;
    let scan = scan_rust_source(source, "fixture.rs", ParserReexports::None);
    assert!(scan.prohibited_aliases.is_empty());
    assert_eq!(
        scan.call_counts(),
        BTreeMap::from([("first".to_owned(), 2), ("second".to_owned(), 1)])
    );
}

#[test]
fn source_scanner_rejects_every_parser_alias_shape() {
    for source in [
        "use mech_syntax::parse;\nfn run() { parse(\"source\"); }",
        "use mech_syntax::parser::parse as parse_source;\nfn run() { parse_source(\"source\"); }",
        "use mech_syntax::parser as old_parser;\nfn run() { old_parser::parse(\"source\"); }",
        "use mech_syntax::parser::*;\nfn run() { parse(\"source\"); }",
        "use mech_syntax as syntax;\nfn run() { syntax::parse(\"source\"); }",
        "extern crate mech_syntax as syntax;\nfn run() { syntax::parse(\"source\"); }",
        "fn run() { let parse_source = mech_syntax::parse; parse_source(\"source\"); }",
        "use mech_syntax::parser;\nfn run() { let parse_source = parser::parse; parse_source(\"source\"); }",
    ] {
        assert!(
            !scan_rust_source(source, "fixture.rs", ParserReexports::None)
                .prohibited_aliases
                .is_empty(),
            "alias escaped the parser census: {source}"
        );
    }
}

#[test]
fn source_scanner_allows_only_the_declared_root_reexports() {
    let source = "pub extern crate mech_syntax as syntax;\n\
                  pub use mech_syntax::{parse, parser};\n\
                  fn direct() { crate::parse(\"one\"); crate::parser::parse(\"two\"); }\n\
                  fn syntax() { crate::syntax::parse(\"three\"); crate::syntax::parser::parse(\"four\"); }";
    let scan = scan_rust_source(source, "src/lib.rs", ParserReexports::RootCrate);
    assert!(scan.prohibited_aliases.is_empty());
    assert_eq!(
        scan.call_counts(),
        BTreeMap::from([("direct".to_owned(), 2), ("syntax".to_owned(), 2)])
    );

    for path in ["fixture.rs", "src/lib.rs"] {
        let alias = scan_rust_source(
            "pub use mech_syntax::parse as old_parse;\nfn run() { old_parse(\"source\"); }",
            path,
            if path == "src/lib.rs" {
                ParserReexports::RootCrate
            } else {
                ParserReexports::None
            },
        );
        assert!(!alias.prohibited_aliases.is_empty(), "{path}");
    }
}

#[test]
fn source_scanner_allows_the_canonical_document_parser_module() {
    let source = "use mech_syntax::document::parser as canonical_parser;";
    let scan = scan_rust_source(source, "fixture.rs", ParserReexports::None);
    assert!(scan.prohibited_aliases.is_empty());
    assert!(scan.call_counts().is_empty());
}

#[test]
fn source_scanner_excludes_test_only_modules_and_items() {
    let external = scan_rust_source(
        "mod tests;\nfn after_tests() { mech_syntax::parse(\"source\"); }",
        "fixture.rs",
        ParserReexports::None,
    );
    assert_eq!(
        external.call_counts(),
        BTreeMap::from([("after_tests".to_owned(), 1)])
    );

    let inline = scan_rust_source(
        "#[cfg(test)] mod tests { fn ignored() { mech_syntax::parse(\"test\"); } }\n\
         #[cfg(all(test, target_arch = \"wasm32\"))]\n\
         mod browser_tests { fn ignored_too() { mech_syntax::parse(\"test\"); } }\n\
         #[cfg(test)]\n\
         fn ignored_helper() { mech_syntax::parse(\"test\"); }\n\
         #[cfg(test)]\n\
         impl Fixture { fn ignored_method() { mech_syntax::parse(\"test\"); } }\n\
         #[cfg(not(test))]\n\
         fn production_cfg() { mech_syntax::parse(\"source\"); }\n\
         fn after_tests() { mech_syntax::parse(\"source\"); }",
        "fixture.rs",
        ParserReexports::None,
    );
    assert_eq!(
        inline.call_counts(),
        BTreeMap::from([
            ("after_tests".to_owned(), 1),
            ("production_cfg".to_owned(), 1),
        ])
    );
}

#[test]
fn source_scanner_preserves_cfg_branches_available_without_tests() {
    for predicate in [
        "not(test)",
        "all(not(test), feature = \"source\")",
        "all(feature = \"source\", not(test))",
        "all(not(test), any(feature = \"source\", target_arch = \"wasm32\"))",
        "any(test, feature = \"source\")",
        "not(all(not(test), feature = \"source\"))",
        "not(not(not(test)))",
        "all()",
        "test = \"yes\"",
    ] {
        let source =
            format!("#[cfg({predicate})] fn production() {{ mech_syntax::parse(\"source\"); }}");
        let scan = scan_rust_source(&source, "fixture.rs", ParserReexports::None);
        assert_eq!(
            scan.call_counts(),
            BTreeMap::from([("production".to_owned(), 1)]),
            "{predicate}"
        );
    }
    for predicate in [
        "test",
        "all(test, feature = \"source\")",
        "not(not(test))",
        "any(test, all(test, feature = \"source\"))",
        "not(any(not(test), feature = \"source\"))",
    ] {
        for item in [
            "fn helper() { mech_syntax::parse(\"test\"); }",
            "impl Fixture { fn method() { mech_syntax::parse(\"test\"); } }",
            "mod nested { fn helper() { mech_syntax::parse(\"test\"); } }",
        ] {
            let source = format!("#[cfg({predicate})] {item}");
            assert!(
                scan_rust_source(&source, "fixture.rs", ParserReexports::None)
                    .call_counts()
                    .is_empty(),
                "{source}"
            );
        }
    }
}

#[test]
fn source_scanner_counts_syntax_crate_root_reexports_in_its_own_sources() {
    let root = repository_root();
    for path in ["src/syntax/src/lib.rs", "src/syntax/src/document/mod.rs"] {
        let reexports = parser_reexports(&root, &root.join(path));
        assert_eq!(reexports, ParserReexports::SyntaxCrate, "{path}");
        let scan = scan_rust_source(
            "fn caller() { crate::parse(\"one\"); crate::parser::parse(\"two\"); }",
            path,
            reexports,
        );
        assert_eq!(
            scan.call_counts(),
            BTreeMap::from([("caller".to_owned(), 2)])
        );
    }
    for (path, source) in [
        ("src/syntax/src/lib.rs", "pub use crate::parser::*;"),
        ("src/syntax/src/base.rs", "use crate::parser::*;"),
    ] {
        assert!(
            scan_rust_source(source, path, ParserReexports::SyntaxCrate)
                .prohibited_aliases
                .is_empty()
        );
        assert!(
            !scan_rust_source(
                source,
                "src/syntax/src/new_caller.rs",
                ParserReexports::SyntaxCrate
            )
            .prohibited_aliases
            .is_empty()
        );
    }
    for source in [
        "use crate::parse as old; fn caller() { old(\"source\"); }",
        "use crate::{parse as old}; fn caller() { old(\"source\"); }",
        "use crate::parser as old; fn caller() { old::parse(\"source\"); }",
        "fn caller() { let old = crate::parse; old(\"source\"); }",
    ] {
        assert!(
            !scan_rust_source(
                source,
                "src/syntax/src/lib.rs",
                ParserReexports::SyntaxCrate
            )
            .prohibited_aliases
            .is_empty(),
            "{source}"
        );
    }
    assert_eq!(
        parser_reexports(&root, &root.join("src/lib.rs")),
        ParserReexports::RootCrate
    );
    let reexports = parser_reexports(&root, &root.join("src/runtime/src/lib.rs"));
    assert_eq!(reexports, ParserReexports::None);
    assert!(
        scan_rust_source(
            "fn caller() { crate::parse(\"unrelated\"); }",
            "src/runtime/src/lib.rs",
            reexports
        )
        .call_counts()
        .is_empty()
    );
}

#[test]
fn source_scanner_distinguishes_method_module_and_nested_function_owners() {
    let source = r#"
impl Outer { fn compile_source() {} }
impl<'a> View<'a> { fn compile_source() { mech_syntax::parse("source"); } }
mod one { fn run() { mech_syntax::parse("one"); } }
mod two { fn run() { mech_syntax::parse("two"); } }
fn enclosing() { fn run() { mech_syntax::parse("nested"); } }
"#;
    let scan = scan_rust_source(source, "fixture.rs", ParserReexports::None);
    assert_eq!(
        scan.call_counts(),
        BTreeMap::from([
            ("impl <'a>View<'a>::compile_source".to_owned(), 1),
            ("one::run".to_owned(), 1),
            ("two::run".to_owned(), 1),
            ("enclosing::run".to_owned(), 1),
        ])
    );
    let moved = source
        .replace(
            "impl Outer { fn compile_source() {} }",
            "impl Outer { fn compile_source() { mech_syntax::parse(\"source\"); } }",
        )
        .replace(
            "impl<'a> View<'a> { fn compile_source() { mech_syntax::parse(\"source\"); } }",
            "impl<'a> View<'a> { fn compile_source() {} }",
        );
    let moved = scan_rust_source(&moved, "fixture.rs", ParserReexports::None);
    assert_eq!(
        scan.call_counts().values().sum::<usize>(),
        moved.call_counts().values().sum::<usize>()
    );
    assert_ne!(
        scan.call_counts(),
        moved.call_counts(),
        "moving a call between methods must invalidate the census"
    );
    assert_eq!(
        moved.call_counts().get("impl Outer::compile_source"),
        Some(&1)
    );

    let same_path = "#[cfg(feature = \"a\")] impl Same { fn run() { mech_syntax::parse(\"one\"); } } #[cfg(not(feature = \"a\"))] impl Same { fn run() { mech_syntax::parse(\"two\"); } }";
    let scan = scan_rust_source(same_path, "fixture.rs", ParserReexports::None);
    assert_eq!(
        scan.call_counts().len(),
        2,
        "cfg alternatives with identical names remain distinct source owners"
    );
    assert!(
        scan.call_counts()
            .keys()
            .all(|name| name.starts_with("impl Same::run@"))
    );
}

#[test]
fn source_scanner_closes_glob_and_root_reexport_call_bypasses() {
    for (source, reexports) in [
        (
            "use mech_syntax::*; fn run() { parse(\"source\"); }",
            ParserReexports::None,
        ),
        (
            "use mech_syntax::{*}; fn run() { parse(\"source\"); }",
            ParserReexports::None,
        ),
        (
            "use crate::{syntax as old}; fn run() { old::parse(\"source\"); }",
            ParserReexports::RootCrate,
        ),
        (
            "use crate::{syntax::{parse as old}}; fn run() { old(\"source\"); }",
            ParserReexports::RootCrate,
        ),
    ] {
        assert!(
            !scan_rust_source(source, "fixture.rs", reexports)
                .prohibited_aliases
                .is_empty(),
            "{source}"
        );
    }
    for (source, path, reexports) in [
        (
            "pub use mech_syntax::{parse, parser};",
            "src/lib.rs",
            ParserReexports::RootCrate,
        ),
        (
            "pub use crate::parser::*;",
            "src/syntax/src/lib.rs",
            ParserReexports::SyntaxCrate,
        ),
        (
            "use crate::parser::*;",
            "src/syntax/src/base.rs",
            ParserReexports::SyntaxCrate,
        ),
        (
            "use crate::*;",
            "src/syntax/src/example.rs",
            ParserReexports::SyntaxCrate,
        ),
    ] {
        let scan = scan_rust_source(
            &format!("{source} fn run() {{ parse(\"source\"); }}"),
            path,
            reexports,
        );
        assert!(scan.prohibited_aliases.is_empty(), "{source}");
        assert_eq!(
            scan.call_counts(),
            BTreeMap::from([("run".to_owned(), 1)]),
            "{source}"
        );
        let alias = scan_rust_source(
            &format!("{source} fn run() {{ let alias = parse; alias(\"source\"); }}"),
            path,
            reexports,
        );
        assert!(!alias.prohibited_aliases.is_empty(), "{source}");
    }
    let ordinary = scan_rust_source(
        "fn run() { parse(\"canonical helper\"); }",
        "src/syntax/src/document/parser/canonical.rs",
        ParserReexports::SyntaxCrate,
    );
    assert!(
        ordinary.call_counts().is_empty(),
        "unrelated local callbacks have no root parser binding"
    );
}

#[test]
fn source_scanner_requires_cfg_proof_before_excluding_a_tests_module() {
    for attribute in [
        "",
        "#[cfg(not(test))]",
        "#[cfg(all(not(test), feature = \"source\"))]",
    ] {
        let source =
            format!("{attribute} mod tests {{ fn run() {{ mech_syntax::parse(\"source\"); }} }}");
        assert_eq!(
            scan_rust_source(&source, "fixture.rs", ParserReexports::None).call_counts(),
            BTreeMap::from([("tests::run".to_owned(), 1)]),
            "{source}"
        );
    }
    let source = "#[cfg(test)] mod tests { fn run() { mech_syntax::parse(\"test\"); } }";
    assert!(
        scan_rust_source(source, "fixture.rs", ParserReexports::None)
            .call_counts()
            .is_empty()
    );
}

#[test]
fn production_scan_roots_follow_the_workspace_members() {
    let root = repository_root();
    let roots = workspace_source_roots(&root);
    assert!(roots.contains(&root.join("src")));
    assert!(roots.contains(&root.join("hosts/gpu/src")));
    assert!(roots.contains(&root.join("tests/fixtures/native-live-host/src")));
}

#[test]
fn every_consumer_names_a_frozen_fixture() {
    let fixtures = fixtures();
    let fixture_names = fixtures.keys().cloned().collect::<BTreeSet<_>>();
    for (id, row) in consumers() {
        assert!(
            fixture_names.contains(&row.fixture),
            "unknown fixture {} for {id}",
            row.fixture
        );
    }
}

#[test]
fn frozen_fixtures_define_canonical_document_expectations() {
    for (id, row) in fixtures() {
        assert_eq!(row.root, "document", "canonical root for {id}");
        assert!(
            matches!(row.outcome.as_str(), "accept" | "reject"),
            "canonical outcome for {id}"
        );
        assert!(
            !row.contracts.is_empty(),
            "empty consumer contracts for {id}"
        );
        assert!(!row.notes.is_empty(), "empty fixture note for {id}");
        assert!(
            repository_root().join(&row.source_path).is_file(),
            "missing canonical fixture {id}"
        );
    }
}

#[test]
fn source_scanner_resolves_relative_root_parser_paths() {
    for (path, reexports) in [
        ("src/new_consumer.rs", ParserReexports::RootCrate),
        (
            "src/syntax/src/new_consumer.rs",
            ParserReexports::SyntaxCrate,
        ),
    ] {
        for import in [
            "use super::parse;",
            "use super::parse as old;",
            "use super::{parse as old};",
            "use super::parser::parse_mech;",
            "use super::parser as old;",
            "use super::*;",
        ] {
            let scan = scan_rust_source(
                &format!("{import} fn run() {{ parse(source); }}"),
                path,
                reexports,
            );
            assert!(!scan.prohibited_aliases.is_empty(), "{path}: {import}");
        }
        let scan = scan_rust_source(
            "fn run() { super::parse(source); super::parser::parse_mech(input); }",
            path,
            reexports,
        );
        assert_eq!(scan.call_counts(), BTreeMap::from([("run".to_owned(), 2)]));
        let scan = scan_rust_source(
            "mod inner { use super::super::parse; fn run() { parse(source); } }",
            path,
            reexports,
        );
        assert!(!scan.prohibited_aliases.is_empty());
    }
    let scan = scan_rust_source(
        "mod inner { fn run() { super::parse(source); } } fn run() { self::parse(source); }",
        "src/lib.rs",
        ParserReexports::RootCrate,
    );
    assert_eq!(
        scan.call_counts(),
        BTreeMap::from([("inner::run".to_owned(), 1), ("run".to_owned(), 1)])
    );
    for source in [
        "use super::parse; fn run() { parse(source); }",
        "fn run() { self::parse(source); super::parse(source); }",
        "use mech_syntax::document::parser; fn run() { parser::parse(source); }",
        "fn run() { mech_syntax::document::parser::parse(source); }",
    ] {
        let scan = scan_rust_source(
            source,
            "src/syntax/src/document/helper.rs",
            ParserReexports::SyntaxCrate,
        );
        assert!(
            scan.call_counts().is_empty() && scan.prohibited_aliases.is_empty(),
            "{source}"
        );
    }
}

#[test]
fn source_scanner_normalizes_raw_parser_identifiers() {
    for reference in [
        "mech_syntax::r#parse",
        "r#mech_syntax::parse",
        "::r#mech_syntax::r#parser::r#parse",
        "mech_syntax::parser::r#parse_mech",
        "mech_syntax::r#parse_mech",
    ] {
        let scan = scan_rust_source(
            &format!("fn run() {{ {reference}(source); }}"),
            "fixture.rs",
            ParserReexports::None,
        );
        assert_eq!(
            scan.call_counts(),
            BTreeMap::from([("run".to_owned(), 1)]),
            "{reference}"
        );
        let scan = scan_rust_source(
            &format!("fn run() {{ let old = {reference}; old(source); }}"),
            "fixture.rs",
            ParserReexports::None,
        );
        assert!(!scan.prohibited_aliases.is_empty(), "{reference}");
    }
    for import in [
        "use mech_syntax::r#parse;",
        "use r#mech_syntax::{r#parse as old};",
        "use mech_syntax::r#parser::r#parse_mech;",
    ] {
        let scan = scan_rust_source(import, "fixture.rs", ParserReexports::None);
        assert!(!scan.prohibited_aliases.is_empty(), "{import}");
    }
    let scan = scan_rust_source(
        "use super::r#parse;",
        "src/new.rs",
        ParserReexports::RootCrate,
    );
    assert!(!scan.prohibited_aliases.is_empty());
    let scan = scan_rust_source(
        "struct Holder { r#use: u8 } fn run() { mech_syntax::r#parse(source); }",
        "fixture.rs",
        ParserReexports::None,
    );
    assert_eq!(scan.call_counts(), BTreeMap::from([("run".to_owned(), 1)]));
}

#[test]
fn source_scanner_covers_retiring_source_entrypoints_and_frozen_reexports() {
    for reference in [
        "mech_syntax::parse_mech",
        "mech_syntax::parser::parse_mech",
        "mech_syntax::parser::program",
        "mech_syntax::parser::mech_code",
        "mech_syntax::parser::mech_code_alt",
        "syntax::parse_mech",
        "syntax::parser::parse_mech",
        "mech::parse_mech",
        "mech::syntax::parse_mech",
        "mech::parser::parse_mech",
        "crate::parse_mech",
        "crate::syntax::parse_mech",
        "crate::syntax::parser::parse_mech",
    ] {
        let scan = scan_rust_source(
            &format!("fn run() {{ {reference}(source); }}"),
            "src/new.rs",
            ParserReexports::RootCrate,
        );
        assert_eq!(
            scan.call_counts(),
            BTreeMap::from([("run".to_owned(), 1)]),
            "{reference}"
        );
        let scan = scan_rust_source(
            &format!("use {reference} as old; fn run() {{ old(source); }}"),
            "src/new.rs",
            ParserReexports::RootCrate,
        );
        assert!(!scan.prohibited_aliases.is_empty(), "{reference}");
    }
    let scan = scan_rust_source(
        "use crate::*; fn run() { parse_mech(source); }",
        "src/new.rs",
        ParserReexports::RootCrate,
    );
    assert_eq!(scan.call_counts(), BTreeMap::from([("run".to_owned(), 1)]));
    assert!(
        !scan.prohibited_aliases.is_empty(),
        "swapping an inventoried parse call for parse_mech must fail even when the call count is unchanged"
    );
    let scan = scan_rust_source(
        "pub use mech_syntax::{parser::program}; fn run() { program(source); }",
        "src/lib.rs",
        ParserReexports::RootCrate,
    );
    assert!(
        !scan.prohibited_aliases.is_empty(),
        "a new nested export is not a frozen declaration"
    );
}

#[test]
fn source_scanner_cfg_arms_cannot_swallow_production_siblings() {
    for test_arm in [
        "E::A => test_only(),",
        "E::A => { test_only() }",
        "E::A { value } if predicate(value) => test_only(),",
        "E::A => (|x| { x })(test_only()),",
    ] {
        let source = format!(
            "fn run(e: E) {{ match e {{ #[cfg(test)] {test_arm} E::B => {{ mech_syntax::parse(source); }} }} }}"
        );
        assert_eq!(
            scan_rust_source(&source, "fixture.rs", ParserReexports::None).call_counts(),
            BTreeMap::from([("run".to_owned(), 1)]),
            "{source}"
        );
    }
    let source = "fn run(e: E) { match e { #[cfg(test)] E::A => mech_syntax::parse(test_source), E::B => mech_syntax::parse(source), } }";
    assert_eq!(
        scan_rust_source(source, "fixture.rs", ParserReexports::None).call_counts(),
        BTreeMap::from([("run".to_owned(), 1)])
    );
    let source = "fn run() { let label = \"◉\"; #[cfg(test)] let hidden = mech_syntax::parse(test_source); mech_syntax::parse(source); }";
    assert_eq!(
        scan_rust_source(source, "fixture.rs", ParserReexports::None).call_counts(),
        BTreeMap::from([("run".to_owned(), 1)])
    );
}

#[test]
fn source_scanner_evaluates_cfg_attr_without_assuming_features() {
    for (attribute, count) in [
        ("#[cfg_attr(not(test), cfg(test))]", 0),
        ("#[cfg_attr(not(test), cfg_attr(not(test), cfg(test)))]", 0),
        ("#[cfg_attr(test, cfg(test))]", 1),
        ("#[cfg_attr(feature = \"source\", cfg(test))]", 1),
        (
            "#[cfg_attr(not(test), cfg(any(test, feature = \"source\")))]",
            1,
        ),
    ] {
        let source = format!("{attribute} fn run() {{ mech_syntax::parse(source); }}");
        assert_eq!(
            scan_rust_source(&source, "fixture.rs", ParserReexports::None)
                .call_counts()
                .values()
                .sum::<usize>(),
            count,
            "{attribute}"
        );
        let source = format!(
            "fn run(e: E) {{ match e {{ {attribute} E::A => mech_syntax::parse(first), E::B => mech_syntax::parse(second), }} }}"
        );
        assert_eq!(
            scan_rust_source(&source, "fixture.rs", ParserReexports::None)
                .call_counts()
                .values()
                .sum::<usize>(),
            count + 1,
            "{attribute}"
        );
    }
    assert!(
        scan_rust_source(
            "#![cfg_attr(not(test), cfg(test))]\nfn run() { mech_syntax::parse(source); }",
            "fixture.rs",
            ParserReexports::None
        )
        .call_counts()
        .is_empty()
    );
}

struct CensusFixture(PathBuf);
impl CensusFixture {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "mech-census-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        Self(root.canonicalize().unwrap())
    }
    fn write(&self, path: &str, source: &str) -> PathBuf {
        let path = self.0.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, source).unwrap();
        path
    }
    fn metadata(&self) -> Vec<serde_json::Value> {
        let output = std::process::Command::new(env!("CARGO"))
            .args(["generate-lockfile", "--offline"])
            .current_dir(&self.0)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        fn manifest_bytes(path: &Path, bytes: &mut BTreeMap<PathBuf, Vec<u8>>) {
            for entry in fs::read_dir(path).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    manifest_bytes(&path, bytes);
                } else if matches!(
                    path.file_name().and_then(|name| name.to_str()),
                    Some("Cargo.toml" | "Cargo.lock")
                ) {
                    bytes.insert(path.clone(), fs::read(&path).unwrap());
                }
            }
        }
        let mut before = BTreeMap::new();
        manifest_bytes(&self.0, &mut before);
        let metadata = production_package_metadata(&self.0);
        let mut after = BTreeMap::new();
        manifest_bytes(&self.0, &mut after);
        assert_eq!(
            after, before,
            "discovery changed or created a manifest or lockfile"
        );
        metadata
    }
    fn modules(&self) -> BTreeMap<PathBuf, BTreeSet<Vec<String>>> {
        production_source_modules(
            &BTreeSet::from([self.0.clone()]),
            &BTreeSet::from([self.0.join("lib.rs")]),
        )
    }
}
impl Drop for CensusFixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn production_module_graph_requires_cfg_proof_for_tests_paths() {
    let fixture = CensusFixture::new();
    let tests = fixture.write(
        "tests.rs",
        "#[path = \"tests/nested.rs\"] mod nested; fn run() { mech_syntax::parse(source); }",
    );
    let nested = fixture.write(
        "tests/nested.rs",
        "fn run() { mech_syntax::parse(source); }",
    );
    for attribute in [
        "",
        "#[cfg(not(test))]",
        "#[cfg(any(test, feature = \"source\"))]",
    ] {
        fixture.write("lib.rs", &format!("{attribute} mod tests;"));
        let modules = fixture.modules();
        assert!(
            modules.contains_key(&tests) && modules.contains_key(&nested),
            "{attribute}"
        );
        for path in [&tests, &nested] {
            assert_eq!(
                scan_rust_source(
                    &fs::read_to_string(path).unwrap(),
                    "fixture.rs",
                    ParserReexports::None
                )
                .call_counts()
                .values()
                .sum::<usize>(),
                1
            );
        }
    }
    for attribute in ["#[cfg(test)]", "#[cfg_attr(not(test), cfg(test))]"] {
        fixture.write("lib.rs", &format!("{attribute} mod tests;"));
        let modules = fixture.modules();
        assert!(
            !modules.contains_key(&tests) && !modules.contains_key(&nested),
            "{attribute}"
        );
    }
    fixture.write(
        "lib.rs",
        "#[cfg(test)] mod tests; #[path = \"tests.rs\"] mod production;",
    );
    let modules = fixture.modules();
    assert!(
        modules.contains_key(&tests) && modules.contains_key(&nested),
        "a production declaration overrides another test-only declaration"
    );
}

#[test]
fn production_module_graph_follows_path_attributes_and_rust_module_names() {
    let fixture = CensusFixture::new();
    fixture.write(
        "lib.rs",
        "#[path = \"tests/renamed.rs\"] mod public_module;",
    );
    let renamed = fixture.write(
        "tests/renamed.rs",
        "use super::parse; fn run() { parse(source); }",
    );
    let modules = fixture.modules();
    assert_eq!(
        modules[&renamed],
        BTreeSet::from([vec!["public_module".to_owned()]])
    );
    for module in &modules[&renamed] {
        assert!(
            !scan_rust_source_in_module(
                &fs::read_to_string(&renamed).unwrap(),
                "tests/renamed.rs",
                ParserReexports::RootCrate,
                module,
                &default_crate_names(),
            )
            .prohibited_aliases
            .is_empty()
        );
    }
    fixture.write(
        "lib.rs",
        "#[cfg(test)] #[path = \"tests/renamed.rs\"] mod test_only;",
    );
    assert!(!fixture.modules().contains_key(&renamed));
    fixture.write(
        "lib.rs",
        "#[cfg_attr(not(test), path = \"tests/renamed.rs\")] mod public_module;",
    );
    assert!(fixture.modules().contains_key(&renamed));
    fixture.write(
        "lib.rs",
        "#[cfg_attr(feature = \"source\", path = \"tests/renamed.rs\")] mod public_module;",
    );
    let default = fixture.write(
        "public_module.rs",
        "fn run() { mech_syntax::parse(source); }",
    );
    let modules = fixture.modules();
    assert!(
        modules.contains_key(&renamed) && modules.contains_key(&default),
        "unknown feature paths remain production possibilities"
    );
    fixture.write(
        "lib.rs",
        "#[cfg_attr(test, path = \"tests/renamed.rs\")] mod public_module;",
    );
    let modules = fixture.modules();
    assert!(
        !modules.contains_key(&renamed) && modules.contains_key(&default),
        "a proven test-only path is not an orphan production seed"
    );
    fixture.write(
        "lib.rs",
        "mod nested { #[path = \"../tests/renamed.rs\"] mod redirected; }",
    );
    fs::create_dir_all(fixture.0.join("nested")).unwrap();
    assert_eq!(
        fixture.modules()[&renamed],
        BTreeSet::from([vec!["nested".to_owned(), "redirected".to_owned()]])
    );
}

#[test]
fn source_scanner_keeps_retiring_paths_visible_inside_macros() {
    for source in [
        "macro_rules! old { () => { fn run() { mech_syntax::parse(source); } }; }",
        "fn run() { invoke!({ mech_syntax::parse(source) }); }",
        "macro_rules! old { () => { use mech_syntax::parse as hidden; }; }",
        "macro_rules! old { ($name:ident) => { use mech_syntax::$name; }; }",
    ] {
        let scan = scan_rust_source(source, "fixture.rs", ParserReexports::None);
        assert!(
            !scan.call_counts().is_empty() || !scan.prohibited_aliases.is_empty(),
            "{source}"
        );
        let scan = scan_rust_source(
            &format!("#[cfg(test)] mod tests {{ {source} }}"),
            "fixture.rs",
            ParserReexports::None,
        );
        assert!(
            scan.call_counts().is_empty() && scan.prohibited_aliases.is_empty(),
            "{source}"
        );
    }
    let scan = scan_rust_source(
        "mod tests { #![cfg(test)] fn hidden() { mech_syntax::parse(source); } } fn run() { mech_syntax::parse(source); }",
        "fixture.rs",
        ParserReexports::None,
    );
    assert_eq!(scan.call_counts(), BTreeMap::from([("run".to_owned(), 1)]));
}

#[test]
fn source_scanner_tracks_group_and_relative_parser_module_bindings() {
    for source in [
        "use mech_syntax::{parser}; fn run() { parser::parse(source); }",
        "use super::parser; fn run() { parser::parse(source); }",
        "use super::{parser}; fn run() { parser::parse(source); }",
    ] {
        let scan = scan_rust_source(source, "src/new.rs", ParserReexports::RootCrate);
        assert_eq!(
            scan.call_counts(),
            BTreeMap::from([("run".to_owned(), 1)]),
            "{source}"
        );
    }
    let scan = scan_rust_source(
        "use mech_syntax::parser; fn retiring() { parser::parse(source); } mod canonical { use mech_syntax::document::parser; fn run() { parser::parse(source); } }",
        "src/new.rs",
        ParserReexports::RootCrate,
    );
    assert_eq!(
        scan.call_counts(),
        BTreeMap::from([("retiring".to_owned(), 1)])
    );
    assert!(scan.prohibited_aliases.is_empty());
}

#[test]
fn path_attributed_files_resolve_descendants_from_their_owned_directory() {
    let fixture = CensusFixture::new();
    fixture.write(
        "lib.rs",
        "#[path = \"tests/renamed.rs\"] mod public_module;",
    );
    fixture.write("tests/renamed.rs", "mod nested;");
    let child = fixture.write("tests/nested.rs", "use super::super::parse;");
    assert_eq!(
        fixture.modules()[&child],
        BTreeSet::from([vec!["public_module".to_owned(), "nested".to_owned()]])
    );
    fixture.write(
        "lib.rs",
        "#[cfg(test)] #[path = \"tests/renamed.rs\"] mod test_only;",
    );
    assert!(!fixture.modules().contains_key(&child));
    fixture.write("lib.rs", "mod tests;");
    fixture.write("tests.rs", "mod nested;");
    assert_eq!(
        fixture.modules()[&child],
        BTreeSet::from([vec!["tests".to_owned(), "nested".to_owned()]])
    );
    fixture.write(
        "tests/nested.rs",
        "#![cfg(test)]\nfn hidden() { mech_syntax::parse(source); }",
    );
    assert!(!fixture.modules().contains_key(&child));
}

#[test]
fn new_review_root_alternate_entrypoints_are_guarded() {
    for entry in ["program", "mech_code", "mech_code_alt"] {
        for source in [
            format!("fn run() {{ mech_syntax::{entry}(source); }}"),
            format!("use mech_syntax::{entry}; fn run() {{ {entry}(source); }}"),
        ] {
            let scan = scan_rust_source(&source, "src/consumer.rs", ParserReexports::None);
            assert!(!scan.prohibited_aliases.is_empty(), "{source}");
        }
    }
}

#[test]
fn new_review_cargo_dependency_aliases_are_guarded() {
    let fixture = CensusFixture::new();
    fixture.write(
        "Cargo.toml",
        r#"
[package]
name = "consumer"
version = "0.0.0"
edition = "2024"
[workspace]
members = ["syntax", "product"]
[dependencies]
legacy-syntax = { package = "mech-syntax", path = "syntax" }
product = { package = "mech", path = "product" }
"#,
    );
    fixture.write(
        "syntax/Cargo.toml",
        "[package]\nname = \"mech-syntax\"\nversion = \"0.0.0\"\nedition = \"2024\"\n",
    );
    fixture.write(
        "syntax/src/lib.rs",
        "pub fn parse(_: &str) {} pub mod document { pub mod parser {} }",
    );
    fixture.write(
        "product/Cargo.toml",
        "[package]\nname = \"mech\"\nversion = \"0.0.0\"\nedition = \"2024\"\n",
    );
    fixture.write("product/src/lib.rs", "");
    fixture.write(
        "src/lib.rs",
        "#[path = \"../syntax/shared.rs\"] mod shared;",
    );
    fixture.write(
        "syntax/shared.rs",
        "fn run() { legacy_syntax::parse(source); product::parse(source); }",
    );
    let metadata = fixture.metadata();
    let (calls, aliases) = discovered_calls_in_packages(&fixture.0, &metadata);
    assert_eq!(
        calls,
        BTreeMap::from([(("syntax/shared.rs".to_owned(), "run".to_owned()), 2)])
    );
    assert!(aliases.is_empty(), "{aliases:?}");
    let package = local_packages(&fixture.0, &metadata)
        .find(|package| package["name"] == "consumer")
        .unwrap();
    let names = package_crate_names(package);
    for source in [
        "use legacy_syntax::parse; fn run() { parse(source); }",
        "extern crate legacy_syntax as old; fn run() { old::parse(source); }",
        "fn run() { legacy_syntax::program(source); }",
        "fn run() { product::syntax::mech_code(source); }",
        "use product::parser as old; fn run() { old::parse(source); }",
    ] {
        let scan =
            scan_rust_source_in_module(source, "src/lib.rs", ParserReexports::None, &[], &names);
        assert!(!scan.prohibited_aliases.is_empty(), "{source}");
    }
    for source in [
        "use legacy_syntax::document::parser as canonical; fn run() { canonical::parse(source); }",
        "use product::syntax::document::parser as canonical; fn run() { canonical::parse(source); }",
        "fn run() { product::program(source); }",
    ] {
        let scan =
            scan_rust_source_in_module(source, "src/lib.rs", ParserReexports::None, &[], &names);
        assert!(scan.call_counts().is_empty(), "{source}");
        assert!(scan.prohibited_aliases.is_empty(), "{source}");
    }
}

#[test]
fn new_review_current_crate_aliases_are_guarded() {
    for (path, reexports) in [
        ("src/lib.rs", ParserReexports::RootCrate),
        ("src/syntax/src/lib.rs", ParserReexports::SyntaxCrate),
    ] {
        let scan = scan_rust_source(
            "extern crate self as legacy; fn run() { legacy::parse(source); }",
            path,
            reexports,
        );
        assert!(!scan.prohibited_aliases.is_empty(), "{path}");
    }
}

#[test]
fn new_review_local_production_dependencies_are_discovered() {
    let root = repository_root();
    let roots = workspace_source_roots(&root);
    let targets = production_target_roots(&root);
    for package in [
        "math",
        "compare",
        "logic",
        "range",
        "matrix",
        "set",
        "string",
        "stats",
        "combinatorics",
    ] {
        assert!(
            roots.contains(&root.join(format!("machines/{package}/src"))),
            "{package}"
        );
        assert!(
            targets.contains(&root.join(format!("machines/{package}/src/lib.rs"))),
            "{package}"
        );
    }
}

#[test]
fn new_review_reexport_exemptions_require_root_declarations() {
    for (path, reexports, declaration) in [
        (
            "src/lib.rs",
            ParserReexports::RootCrate,
            "pub use mech_syntax::parse;",
        ),
        (
            "src/lib.rs",
            ParserReexports::RootCrate,
            "pub extern crate mech_syntax as syntax;",
        ),
        (
            "src/syntax/src/lib.rs",
            ParserReexports::SyntaxCrate,
            "pub use crate::parser::*;",
        ),
    ] {
        let source = format!("mod nested {{ {declaration} fn run() {{ parse(source); }} }}");
        let scan = scan_rust_source(&source, path, reexports);
        assert!(!scan.prohibited_aliases.is_empty(), "{path}: {source}");
    }
}

#[test]
fn production_discovery_includes_inactive_optional_local_workspaces() {
    let fixture = CensusFixture::new();
    fixture.write(
        "Cargo.toml",
        r#"
[package]
name = "consumer"
version = "0.0.0"
edition = "2024"
[workspace]
exclude = ["machine", "syntax"]
[dependencies]
optional-machine = { package = "local-machine", path = "machine", optional = true }
"#,
    );
    fixture.write("src/lib.rs", "");
    fixture.write(
        "machine/Cargo.toml",
        r#"
[package]
name = "local-machine"
version = "0.0.0"
edition = "2024"
[workspace]
[dependencies]
renamed = { package = "mech-syntax", path = "../syntax" }
"#,
    );
    fixture.write(
        "machine/src/lib.rs",
        "#[cfg(feature = \"future\")] fn added() { renamed::parse(source); }",
    );
    fixture.write(
        "syntax/Cargo.toml",
        "[package]\nname = \"mech-syntax\"\nversion = \"0.0.0\"\nedition = \"2024\"\n[workspace]\n",
    );
    fixture.write("syntax/src/lib.rs", "pub fn parse(_: &str) {}");
    let metadata = fixture.metadata();
    assert_eq!(local_packages(&fixture.0, &metadata).count(), 3);
    let (calls, aliases) = discovered_calls_in_packages(&fixture.0, &metadata);
    assert_eq!(
        calls,
        BTreeMap::from([(("machine/src/lib.rs".to_owned(), "added".to_owned()), 1)])
    );
    assert!(aliases.is_empty(), "{aliases:?}");
}

#[test]
fn latest_review_standalone_self_aliases_are_guarded() {
    for (path, reexports) in [
        ("src/lib.rs", ParserReexports::RootCrate),
        ("src/syntax/src/lib.rs", ParserReexports::SyntaxCrate),
    ] {
        for import in ["use self as legacy;", "use {self as legacy};"] {
            let source = format!(
                "pub fn parse(_: &str) {{}} {import} fn run(source: &str) {{ legacy::parse(source); }}"
            );
            let scan = scan_rust_source(&source, path, reexports);
            assert!(!scan.prohibited_aliases.is_empty(), "{path}: {source}");
        }
    }
}

#[test]
fn latest_review_public_crate_globs_cannot_create_parser_namespaces() {
    for (path, reexports) in [
        ("src/lib.rs", ParserReexports::RootCrate),
        ("src/syntax/src/lib.rs", ParserReexports::SyntaxCrate),
    ] {
        for visibility in ["pub", "pub(crate)", "pub(super)", "pub(in crate)"] {
            for root in ["crate::*", "crate::{*}", "super::*", "super::{*}"] {
                let source = format!(
                    "pub fn parse(_: &str) {{}} mod legacy {{ {visibility} use {root}; }} fn run(source: &str) {{ legacy::parse(source); }}"
                );
                let scan = scan_rust_source(&source, path, reexports);
                assert!(!scan.prohibited_aliases.is_empty(), "{path}: {source}");
            }
        }
        let source = "pub fn parse(_: &str) {} mod local { use crate::*; fn run(source: &str) { parse(source); } }";
        let scan = scan_rust_source(source, path, reexports);
        assert!(
            scan.prohibited_aliases.is_empty(),
            "{path}: {:?}",
            scan.prohibited_aliases
        );
        assert_eq!(scan.call_counts().values().sum::<usize>(), 1, "{path}");
    }
}

#[test]
fn latest_review_discovery_follows_each_local_workspaces_optional_dependencies() {
    let fixture = CensusFixture::new();
    fixture.write(
        "Cargo.toml",
        r#"
[package]
name = "consumer"
version = "0.0.0"
edition = "2024"
[workspace]
exclude = ["first", "second", "leaf", "syntax", "dev-only"]
[dependencies]
first = { path = "first" }
"#,
    );
    fixture.write("src/lib.rs", "");
    for (name, next, feature) in [("first", "second", "future"), ("second", "leaf", "later")] {
        fixture.write(
            &format!("{name}/Cargo.toml"),
            &format!(
                r#"
[package]
name = "{name}"
version = "0.0.0"
edition = "2024"
[workspace]
[features]
{feature} = ["dep:{next}"]
[target.'cfg(unix)'.dependencies]
{next} = {{ path = "../{next}", optional = true }}
[dev-dependencies]
dev-only = {{ path = "../dev-only" }}
"#
            ),
        );
        fixture.write(&format!("{name}/src/lib.rs"), "");
    }
    fixture.write(
        "dev-only/Cargo.toml",
        "[package]\nname = \"dev-only\"\nversion = \"0.0.0\"\nedition = \"2024\"\n[workspace]\n",
    );
    fixture.write("dev-only/src/lib.rs", "");
    fixture.write(
        "leaf/Cargo.toml",
        r#"
[package]
name = "leaf"
version = "0.0.0"
edition = "2024"
[workspace]
[dependencies]
renamed = { package = "mech-syntax", path = "../syntax" }
"#,
    );
    fixture.write(
        "leaf/src/lib.rs",
        "fn hidden(source: &str) { renamed::parse(source); }",
    );
    fixture.write(
        "syntax/Cargo.toml",
        "[package]\nname = \"mech-syntax\"\nversion = \"0.0.0\"\nedition = \"2024\"\n[workspace]\n",
    );
    fixture.write("syntax/src/lib.rs", "pub fn parse(_: &str) {}");
    let metadata = fixture.metadata();
    let (calls, aliases) = discovered_calls_in_packages(&fixture.0, &metadata);
    assert_eq!(
        calls,
        BTreeMap::from([(("leaf/src/lib.rs".to_owned(), "hidden".to_owned()), 1)])
    );
    assert_eq!(local_packages(&fixture.0, &metadata).count(), 5);
    assert!(aliases.is_empty(), "{aliases:?}");
    for name in ["first", "second", "leaf", "syntax", "dev-only"] {
        assert!(!fixture.0.join(name).join("Cargo.lock").exists(), "{name}");
    }
}

#[test]
fn latest_review_discovery_follows_root_patched_optional_local_dependencies() {
    for patch_source in ["crates-io", "\"https://example.invalid/not-the-index\""] {
        let fixture = CensusFixture::new();
        fixture.write(
            "Cargo.toml",
            &r#"
[package]
name = "consumer"
version = "0.0.0"
edition = "2024"
[workspace]
exclude = ["first", "leaf", "syntax", "wrong-source", "unused", "incompatible", "prerelease"]
[dependencies]
first = { path = "first" }
[patch.PATCH_SOURCE]
renamed-patch-entry = { package = "census-patched-leaf", path = "leaf" }
incompatible = { package = "census-patched-leaf", path = "incompatible" }
prerelease = { package = "census-patched-leaf", path = "prerelease" }
unrelated = { package = "census-unrelated", path = "unused" }
"#
            .replace("PATCH_SOURCE", patch_source),
        );
        fixture.write("src/lib.rs", "");
        fixture.write(
            "first/Cargo.toml",
            r#"
[package]
name = "first"
version = "0.0.0"
edition = "2024"
[workspace]
[features]
future = ["dep:leaf"]
[dependencies]
leaf = { package = "census-patched-leaf", version = "^0.1", optional = true }
"#,
        );
        fixture.write("first/src/lib.rs", "");
        fixture.write(
            "leaf/Cargo.toml",
            r#"
[package]
name = "census-patched-leaf"
version = "0.1.0"
edition = "2024"
[workspace]
[dependencies]
renamed = { package = "mech-syntax", path = "../syntax" }
"#,
        );
        fixture.write(
            "leaf/src/lib.rs",
            "fn hidden(source: &str) { renamed::parse(source); }",
        );
        fixture.write(
        "syntax/Cargo.toml",
        "[package]\nname = \"mech-syntax\"\nversion = \"0.0.0\"\nedition = \"2024\"\n[workspace]\n",
    );
        fixture.write("syntax/src/lib.rs", "pub fn parse(_: &str) {}");
        for (directory, name, version) in [
            ("wrong-source", "census-patched-leaf", "0.1.1"),
            ("unused", "census-unrelated", "0.0.0"),
            ("incompatible", "census-patched-leaf", "9.0.0"),
            ("prerelease", "census-patched-leaf", "0.1.2-alpha.1"),
        ] {
            fixture.write(&format!("{directory}/Cargo.toml"), &format!(
            "[package]\nname = {name:?}\nversion = {version:?}\nedition = \"2024\"\n[workspace]\n[dependencies]\nrenamed = {{ package = \"mech-syntax\", path = \"../syntax\" }}\n"
        ));
            fixture.write(
                &format!("{directory}/src/lib.rs"),
                "fn unrelated(source: &str) { renamed::parse(source); }",
            );
        }
        let metadata = fixture.metadata();
        let (calls, aliases) = discovered_calls_in_packages(&fixture.0, &metadata);
        if patch_source == "crates-io" {
            assert_eq!(
                calls,
                BTreeMap::from([(("leaf/src/lib.rs".to_owned(), "hidden".to_owned()), 1)])
            );
            assert_eq!(local_packages(&fixture.0, &metadata).count(), 4);
        } else {
            assert!(
                calls.is_empty(),
                "a patch for another source must not supply this registry dependency"
            );
            assert_eq!(local_packages(&fixture.0, &metadata).count(), 2);
        }
        assert!(aliases.is_empty(), "{aliases:?}");
    }
}

#[test]
fn latest_review_local_patch_version_matching_uses_cargo_requirements() {
    for (requirement, version, expected) in [
        ("^1.2", "1.3.0", true),
        (">=1.2, <2", "1.9.7", true),
        ("~1.2", "1.3.0", false),
        ("^0.1", "0.2.0", false),
        ("*", "1.2.3-alpha.1", false),
        ("^1.2.3-alpha.1", "1.2.3-alpha.2", true),
        ("^1.2.3-alpha.1", "1.2.4-alpha.1", false),
    ] {
        assert_eq!(
            cargo_version_matches("census-candidate", requirement, version),
            expected,
            "{requirement} against {version}"
        );
    }
}

#[test]
fn latest_review2_unrenamed_parser_modules_cannot_create_exported_namespaces() {
    for visibility in ["pub", "pub(crate)", "pub(super)", "pub(in crate)"] {
        for (source_path, surface, origin) in [
            ("fixture.rs", ParserReexports::None, "mech_syntax::parser"),
            ("src/lib.rs", ParserReexports::RootCrate, "crate::parser"),
            ("src/lib.rs", ParserReexports::RootCrate, "super::parser"),
            (
                "src/syntax/src/lib.rs",
                ParserReexports::SyntaxCrate,
                "crate::parser",
            ),
            (
                "src/syntax/src/lib.rs",
                ParserReexports::SyntaxCrate,
                "super::parser",
            ),
        ] {
            let source = format!(
                "mod legacy {{ {visibility} use {origin}; }} fn run(source: &str) {{ legacy::parser::parse(source); }}"
            );
            let scan = scan_rust_source(&source, source_path, surface);
            assert!(
                !scan.prohibited_aliases.is_empty(),
                "{source_path}: {source}"
            );
        }
    }
    let scan = scan_rust_source(
        "use mech_syntax::parser; fn run() { parser::parse(source); }",
        "fixture.rs",
        ParserReexports::None,
    );
    assert!(scan.prohibited_aliases.is_empty());
    assert_eq!(scan.call_counts(), BTreeMap::from([("run".to_owned(), 1)]));
}

#[test]
fn latest_review2_macro_generated_relative_parser_imports_are_guarded() {
    for (source_path, surface) in [
        ("src/lib.rs", ParserReexports::RootCrate),
        ("src/syntax/src/lib.rs", ParserReexports::SyntaxCrate),
    ] {
        for import in [
            "use crate::parse as $alias;",
            "use self::parse as $alias;",
            "use super::parse as $alias;",
            "use $crate::parse as $alias;",
            "use crate::{parse as $alias};",
            "use super::parser::{parse as $alias};",
        ] {
            let source = format!(
                "macro_rules! old {{ ($alias:ident) => {{ {import} }} }} fn run() {{ old!(legacy); legacy(source); }}"
            );
            let scan = scan_rust_source(&source, source_path, surface);
            assert!(
                !scan.prohibited_aliases.is_empty(),
                "{source_path}: {source}"
            );
        }
    }
    let scan = scan_rust_source(
        "macro_rules! unrelated { ($alias:ident) => { use crate::ordinary as $alias; } }",
        "fixture.rs",
        ParserReexports::None,
    );
    assert!(scan.prohibited_aliases.is_empty());
}

#[test]
fn latest_review2_development_only_local_packages_do_not_enter_production_closure() {
    for dependency_kind in [
        "dev-dependencies",
        "dependencies",
        "build-dependencies",
        "target.'cfg(windows)'.dev-dependencies",
    ] {
        let fixture = CensusFixture::new();
        fixture.write(
            "Cargo.toml",
            &format!(
                r#"
[package]
name = "consumer"
version = "0.0.0"
edition = "2024"
[workspace]
exclude = ["helper", "leaf", "syntax"]
[{dependency_kind}]
helper = {{ path = "helper" }}
"#
            ),
        );
        fixture.write("src/lib.rs", "");
        for (directory, dependencies) in [
            (
                "helper",
                "leaf = { path = \"../leaf\" }\nrenamed = { package = \"mech-syntax\", path = \"../syntax\" }",
            ),
            (
                "leaf",
                "renamed = { package = \"mech-syntax\", path = \"../syntax\" }",
            ),
        ] {
            fixture.write(&format!("{directory}/Cargo.toml"), &format!("[package]\nname = {directory:?}\nversion = \"0.0.0\"\nedition = \"2024\"\n[workspace]\n[dependencies]\n{dependencies}\n"));
            fixture.write(
                &format!("{directory}/src/lib.rs"),
                "fn hidden(source: &str) { renamed::parse(source); }",
            );
        }
        fixture.write("syntax/Cargo.toml", "[package]\nname = \"mech-syntax\"\nversion = \"0.0.0\"\nedition = \"2024\"\n[workspace]\n");
        fixture.write("syntax/src/lib.rs", "pub fn parse(_: &str) {}");
        let metadata = fixture.metadata();
        let (calls, aliases) = discovered_calls_in_packages(&fixture.0, &metadata);
        if dependency_kind.contains("dev-dependencies") {
            assert!(calls.is_empty(), "{dependency_kind}: {calls:?}");
            assert_eq!(metadata.len(), 1, "{dependency_kind}");
        } else {
            assert_eq!(
                calls,
                BTreeMap::from([
                    (("helper/src/lib.rs".to_owned(), "hidden".to_owned()), 1),
                    (("leaf/src/lib.rs".to_owned(), "hidden".to_owned()), 1),
                ]),
                "{dependency_kind}"
            );
            assert_eq!(metadata.len(), 4, "{dependency_kind}");
        }
        assert!(aliases.is_empty(), "{dependency_kind}: {aliases:?}");
    }
}

#[test]
fn latest_review2_resolved_production_edges_cross_external_packages_without_scanning_them() {
    for dependency_kind in ["dependencies", "dev-dependencies"] {
        let fixture = CensusFixture::new();
        let external = CensusFixture::new();
        fixture.write("Cargo.toml", &format!("[package]\nname = \"consumer\"\nversion = \"0.0.0\"\nedition = \"2024\"\n[workspace]\nexclude = [\"leaf\", \"syntax\"]\n[{dependency_kind}]\nbridge = {{ path = {:?} }}\n", external.0.to_str().unwrap()));
        fixture.write("src/lib.rs", "");
        external.write("Cargo.toml", &format!("[package]\nname = \"bridge\"\nversion = \"0.0.0\"\nedition = \"2024\"\n[workspace]\n[dependencies]\nleaf = {{ path = {:?} }}\n", fixture.0.join("leaf").to_str().unwrap()));
        external.write("src/lib.rs", "");
        fixture.write("leaf/Cargo.toml", "[package]\nname = \"leaf\"\nversion = \"0.0.0\"\nedition = \"2024\"\n[workspace]\n[dependencies]\nrenamed = { package = \"mech-syntax\", path = \"../syntax\" }\n");
        fixture.write(
            "leaf/src/lib.rs",
            "fn hidden(source: &str) { renamed::parse(source); }",
        );
        fixture.write("syntax/Cargo.toml", "[package]\nname = \"mech-syntax\"\nversion = \"0.0.0\"\nedition = \"2024\"\n[workspace]\n");
        fixture.write("syntax/src/lib.rs", "pub fn parse(_: &str) {}");
        let external_manifest = fs::read(external.0.join("Cargo.toml")).unwrap();
        let metadata = fixture.metadata();
        let (calls, aliases) = discovered_calls_in_packages(&fixture.0, &metadata);
        if dependency_kind == "dependencies" {
            assert_eq!(
                calls,
                BTreeMap::from([(("leaf/src/lib.rs".to_owned(), "hidden".to_owned()), 1)])
            );
            assert_eq!(metadata.len(), 3);
        } else {
            assert!(calls.is_empty());
            assert_eq!(metadata.len(), 1);
        }
        assert!(aliases.is_empty());
        assert_eq!(
            fs::read(external.0.join("Cargo.toml")).unwrap(),
            external_manifest
        );
        assert!(!external.0.join("Cargo.lock").exists());
    }
}

#[test]
fn final_review_build_targets_are_production_sources() {
    for build in ["build.rs", "tools/generate.rs"] {
        let fixture = CensusFixture::new();
        fixture.write("Cargo.toml", &format!("[package]\nname = \"consumer\"\nversion = \"0.0.0\"\nedition = \"2024\"\nbuild = {build:?}\n[workspace]\nmembers = [\"syntax\"]\n[build-dependencies]\nmech-syntax = {{ path = \"syntax\" }}\n"));
        fixture.write("src/lib.rs", "");
        fixture.write(
            "syntax/Cargo.toml",
            "[package]\nname = \"mech-syntax\"\nversion = \"0.0.0\"\nedition = \"2024\"\n",
        );
        fixture.write("syntax/src/lib.rs", "pub fn parse(_: &str) {}");
        fixture.write(build, "fn main() { mech_syntax::parse(\"source\"); }");
        let (calls, aliases) = discovered_calls_in_packages(&fixture.0, &fixture.metadata());
        assert_eq!(
            calls,
            BTreeMap::from([((build.to_owned(), "main".to_owned()), 1)])
        );
        assert!(aliases.is_empty());
    }
}

#[test]
fn final_review_imported_parser_module_cannot_hide_function_aliases() {
    for import in [
        "use parser::parse as old_parse;",
        "use parser::{parse as old_parse};",
        "use parser::*;",
    ] {
        let source = format!("use mech_syntax::parser; {import} fn run() {{ old_parse(source); }}");
        let scan = scan_rust_source(&source, "fixture.rs", ParserReexports::None);
        assert_eq!(scan.prohibited_aliases.len(), 1, "{source}");
        let canonical = source.replace("mech_syntax::parser", "mech_syntax::document::parser");
        let scan = scan_rust_source(&canonical, "fixture.rs", ParserReexports::None);
        assert!(scan.prohibited_aliases.is_empty(), "{canonical}");
    }
}

#[test]
fn final_review_conditional_bindings_retain_every_possible_parser() {
    for imports in [
        "#[cfg(feature = \"old\")] use mech_syntax::parser; #[cfg(not(feature = \"old\"))] use mech_syntax::document::parser;",
        "#[cfg(not(feature = \"old\"))] use mech_syntax::document::parser; #[cfg(feature = \"old\")] use mech_syntax::parser;",
        "#[cfg(feature = \"old\")] use mech_syntax::{parser}; #[cfg(not(feature = \"old\"))] use mech_syntax::document::{parser};",
    ] {
        let source = format!("{imports} fn run() {{ parser::parse(source); }}");
        let scan = scan_rust_source(&source, "fixture.rs", ParserReexports::None);
        assert_eq!(
            scan.call_counts(),
            BTreeMap::from([("run".to_owned(), 1)]),
            "{source}"
        );
        let alias = format!("{imports} use parser::parse as old_parse;");
        let scan = scan_rust_source(&alias, "fixture.rs", ParserReexports::None);
        assert_eq!(scan.prohibited_aliases.len(), 1, "{alias}");
    }
}

#[test]
fn final_review_shared_file_unions_physical_calls_across_package_namespaces() {
    let fixture = CensusFixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"first\", \"second\", \"syntax\"]\nresolver = \"3\"\n",
    );
    fixture.write(
        "syntax/Cargo.toml",
        "[package]\nname = \"mech-syntax\"\nversion = \"0.0.0\"\nedition = \"2024\"\n",
    );
    fixture.write("syntax/src/lib.rs", "pub fn parse(_: &str) {}");
    for package in ["first", "second"] {
        fixture.write(&format!("{package}/Cargo.toml"), &format!("[package]\nname = {package:?}\nversion = \"0.0.0\"\nedition = \"2024\"\n[features]\n{package} = []\n[dependencies]\n{package}-syntax = {{ package = \"mech-syntax\", path = \"../syntax\" }}\n"));
        fixture.write(
            &format!("{package}/src/lib.rs"),
            "#[path = \"../../shared.rs\"] mod shared;",
        );
    }
    fixture.write("shared.rs", "fn run(source: &str) { #[cfg(feature = \"first\")] first_syntax::parse(source); #[cfg(feature = \"second\")] second_syntax::parse(source); }");
    let metadata = fixture.metadata();
    let (calls, aliases) = discovered_calls_in_packages(&fixture.0, &metadata);
    assert_eq!(
        calls,
        BTreeMap::from([(("shared.rs".to_owned(), "run".to_owned()), 2)])
    );
    assert!(aliases.is_empty());
    fixture.write(
        "shared.rs",
        "fn run(source: &str) { first_syntax::parse(source); }",
    );
    let (calls, aliases) = discovered_calls_in_packages(&fixture.0, &metadata);
    assert_eq!(
        calls,
        BTreeMap::from([(("shared.rs".to_owned(), "run".to_owned()), 1)])
    );
    assert!(aliases.is_empty());
}

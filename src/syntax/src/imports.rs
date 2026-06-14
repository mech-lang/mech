#[macro_use]
use crate::*;
use nom::{
    branch::alt,
    combinator::{cut, map},
    multi::many0,
    sequence::{delimited, preceded, tuple as nom_tuple},
};

const STDLIB_MODULE_ROOTS: &[&str] = &["math", "stats", "io", "string", "combinatorics"];

fn import_emoji(input: ParseString) -> ParseResult<Token> {
    let (input, _) = is_not(alt((
        slash,
        asterisk,
        left_brace,
        right_brace,
        comma,
        colon,
        equal,
        space,
        tab,
        new_line,
    )))(input)?;
    emoji(input)
}

fn import_part(input: ParseString) -> ParseResult<Identifier> {
    let (input, (first, mut rest)) = nom_tuple((
        alt((alpha_token, import_emoji, underscore)),
        many0(alt((alpha_token, digit_token, dash, underscore, import_emoji))),
    ))(input)?;

    let mut tokens = vec![first];
    tokens.append(&mut rest);

    let mut merged = Token::merge_tokens(&mut tokens).unwrap();
    merged.kind = TokenKind::Identifier;

    Ok((input, Identifier { name: merged }))
}

fn import_item_path(input: ParseString) -> ParseResult<Vec<Identifier>> {
    let (input, first) = import_part(input)?;
    let (input, mut rest) = many0(preceded(slash, import_part))(input)?;

    let mut items = vec![first];
    items.append(&mut rest);

    Ok((input, items))
}

fn stdlib_module(input: ParseString) -> ParseResult<Identifier> {
    let original = input.clone();
    let (input, module) = import_part(input)?;
    let module_name = module.to_string();

    if STDLIB_MODULE_ROOTS.iter().any(|root| root == &module_name.as_str()) {
        Ok((input, module))
    } else {
        Err(nom::Err::Error(ParseError::new(
            original,
            "not a known stdlib module import",
        )))
    }
}

fn import_alias_operator(input: ParseString) -> ParseResult<()> {
    let (input, _) = space_tab0(input)?;
    let (input, _) = colon(input)?;
    let (input, _) = equal(input)?;
    let (input, _) = space_tab0(input)?;
    Ok((input, ()))
}

fn import_group_separator(input: ParseString) -> ParseResult<()> {
    let (input, _) = alt((
        list_separator,
        map(whitespace1, |_| ()),
    ))(input)?;

    Ok((input, ()))
}

fn import_group_item(input: ParseString) -> ParseResult<ModuleImportGroupItem> {
    let (input, item) = import_item_path(input)?;
    Ok((input, ModuleImportGroupItem { item }))
}

fn import_group_items(input: ParseString) -> ParseResult<Vec<ModuleImportGroupItem>> {
    let (input, _) = whitespace0(input)?;
    let (input, first) = import_group_item(input)?;
    let (input, mut rest) = many0(preceded(import_group_separator, import_group_item))(input)?;
    let (input, _) = whitespace0(input)?;

    let mut items = vec![first];
    items.append(&mut rest);

    Ok((input, items))
}

fn module_import_end(input: ParseString) -> ParseResult<()> {
    let (input, _) = space_tab0(input)?;
    let (_, _) = new_line(input.clone())?;
    Ok((input, ()))
}

fn aliased_item_import(input: ParseString) -> ParseResult<ModuleImport> {
    let (input, alias) = import_part(input)?;
    let (input, _) = import_alias_operator(input)?;
    let (input, (module, _, item)) = cut(nom_tuple((
        stdlib_module,
        slash,
        import_item_path,
    )))(input)?;

    Ok((
        input,
        ModuleImport {
            module,
            item: Some(item),
            group_items: None,
            alias: Some(alias),
            kind: ModuleImportKind::Item,
        },
    ))
}

fn module_suffix_import(input: ParseString) -> ParseResult<ModuleImport> {
    let (input, module) = stdlib_module(input)?;
    let (input, _) = slash(input)?;
    let (input, (item, group_items, kind)) = cut(alt((
        map(asterisk, |_| (None, None, ModuleImportKind::Glob)),
        map(
            delimited(left_brace, cut(import_group_items), right_brace),
            |group_items| (None, Some(group_items), ModuleImportKind::Group),
        ),
        map(import_item_path, |item| (Some(item), None, ModuleImportKind::Item)),
    )))(input)?;

    Ok((
        input,
        ModuleImport {
            module,
            item,
            group_items,
            alias: None,
            kind,
        },
    ))
}

fn module_only_import(input: ParseString) -> ParseResult<ModuleImport> {
    let (input, module) = stdlib_module(input)?;

    Ok((
        input,
        ModuleImport {
            module,
            item: None,
            group_items: None,
            alias: None,
            kind: ModuleImportKind::Module,
        },
    ))
}

fn item_import(input: ParseString) -> ParseResult<ModuleImport> {
    let (input, module) = stdlib_module(input)?;
    let (input, _) = slash(input)?;
    let (input, item) = import_item_path(input)?;

    Ok((
        input,
        ModuleImport {
            module,
            item: Some(item),
            group_items: None,
            alias: None,
            kind: ModuleImportKind::Item,
        },
    ))
}

fn glob_import(input: ParseString) -> ParseResult<ModuleImport> {
    let (input, module) = stdlib_module(input)?;
    let (input, _) = slash(input)?;
    let (input, _) = asterisk(input)?;

    Ok((
        input,
        ModuleImport {
            module,
            item: None,
            group_items: None,
            alias: None,
            kind: ModuleImportKind::Glob,
        },
    ))
}

fn grouped_item_import(input: ParseString) -> ParseResult<ModuleImport> {
    let (input, module) = stdlib_module(input)?;
    let (input, _) = slash(input)?;
    let (input, group_items) = delimited(
        left_brace,
        cut(import_group_items),
        right_brace,
    )(input)?;

    Ok((
        input,
        ModuleImport {
            module,
            item: None,
            group_items: Some(group_items),
            alias: None,
            kind: ModuleImportKind::Group,
        },
    ))
}

pub fn module_import(input: ParseString) -> ParseResult<ModuleImport> {
    let (input, _) = whitespace0(input)?;
    let (input, _) = plus(input)?;
    let (input, _) = right_angle(input)?;
    let (input, _) = space_tab0(input)?;

    let (input, mut import) = alt((
        aliased_item_import,
        module_suffix_import,
        module_only_import,
    ))(input)?;

    import.module.name.src_range.end = input.loc();
    let (input, _) = module_import_end(input)?;

    Ok((input, import))
}

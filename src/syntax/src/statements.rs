#[macro_use]
use crate::*;
use nom::combinator::map as nom_map;
use nom::sequence::tuple as nom_tuple;

// #### Statements

// comment-sigil := "--" | "//" ;
pub fn comment_sigil(input: ParseString) -> ParseResult<()> {
  let (input, _) = alt((tag("--"),tag("//")))(input)?;
  Ok((input, ()))
}

// comment := comment_sigil, paragraph ;
pub fn comment(input: ParseString) -> ParseResult<Comment> {
  let (input, _) = many0(space_tab)(input)?;
  let (input, _) = comment_sigil(input)?;
  let (input, line_token) = skip_till_eol(input)?;
  let comment_text = line_token.to_string();

  if comment_text.is_empty() {
    return Ok((input, Comment { paragraph: Paragraph { elements: vec![], error_range: None } }));
  }

  let line_graphemes = graphemes::init_source(&comment_text);
  let line_input = ParseString::new(&line_graphemes);

  match inline_paragraph(line_input) {
    Ok((remaining, paragraph)) => {
      // A rich comment must parse the full line.
      if new_line(remaining.clone()).is_ok() {
        Ok((input, Comment { paragraph }))
      } else {
        let mut recovered_input = input;
        let start = line_token.src_range.start;
        let end = line_token.src_range.end;
        recovered_input.error_log.push((
          SourceRange { start, end },
          ParseErrorDetail {
            message: "Invalid rich comment syntax, preserving raw comment text",
            annotation_rngs: vec![],
          },
        ));
        Ok((recovered_input, Comment { paragraph: Paragraph::from_tokens(vec![line_token]) }))
      }
    }
    Err(_) => {
      let mut recovered_input = input;
      let start = line_token.src_range.start;
      let end = line_token.src_range.end;
      recovered_input.error_log.push((
        SourceRange { start, end },
        ParseErrorDetail {
          message: "Invalid rich comment syntax, preserving raw comment text",
          annotation_rngs: vec![],
        },
      ));
      Ok((recovered_input, Comment { paragraph: Paragraph::from_tokens(vec![line_token]) }))
    }
  }
}

// op-assign-operator := add-assign-operator | sub-assign-operator | mul-assign-operator | div-assign-operator | exp-assign-operator ;
pub fn op_assign_operator(input: ParseString) -> ParseResult<OpAssignOp> {
  alt((add_assign_operator, sub_assign_operator, mul_assign_operator, div_assign_operator, exp_assign_operator))(input)
}

// add-assign-operator := "+=" ;
pub fn add_assign_operator(input: ParseString) -> ParseResult<OpAssignOp> {
  let (input, _) = whitespace0(input)?;
  let (input, _) = tag("+=")(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input, OpAssignOp::Add))
}

// sub-assign-operator := "-=" ;
pub fn sub_assign_operator(input: ParseString) -> ParseResult<OpAssignOp> {
  let (input, _) = whitespace0(input)?;
  let (input, _) = tag("-=")(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input, OpAssignOp::Sub))
}

// mul-assign-operator := "*=" ;
pub fn mul_assign_operator(input: ParseString) -> ParseResult<OpAssignOp> {
  let (input, _) = whitespace0(input)?;
  let (input, _) = tag("*=")(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input, OpAssignOp::Mul))
}
// div-assign-operator := "/=" ;
pub fn div_assign_operator(input: ParseString) -> ParseResult<OpAssignOp> {
  let (input, _) = whitespace0(input)?;
  let (input, _) = tag("/=")(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input, OpAssignOp::Div))
}

// exp-assign-operator := "^=" ;
pub fn exp_assign_operator(input: ParseString) -> ParseResult<OpAssignOp> {
  let (input, _) = whitespace0(input)?;
  let (input, _) = tag("^=")(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input, OpAssignOp::Exp))
}

// split_data := (identifier | table), <!stmt_operator>, space*, split_operator, <space+>, <expression> ;
/*pub fn split_data(input: ParseString) -> ParseResult<ParserNode> {
  /*let msg1 = "Expects spaces around operator";
  let msg2 = "Expects expression";
  let (input, table) = alt((identifier, table))(input)?;
  let (input, _) = labelr!(null(is_not(stmt_operator)), skip_nil, msg1)(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, _) = split_operator(input)?;
  let (input, _) = labelr!(null(many1(space)), skip_nil, msg1)(input)?;
  let (input, expression) = label!(expression, msg2)(input)?;*/
  Ok((input, ParserNode::SplitData{children: vec![]}))
}*/

// flatten_data := identifier, <!stmt_operator>, space*, flatten_operator, <space+>, <expression> ;
/*pub fn flatten_data(input: ParseString) -> ParseResult<ParserNode> {
  /*let msg1 = "Expects spaces around operator";
  let msg2 = "Expects expression";
  let (input, table) = identifier(input)?;
  let (input, _) = labelr!(null(is_not(stmt_operator)), skip_nil, msg1)(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, _) = flatten_operator(input)?;
  let (input, _) = labelr!(null(many1(space)), skip_nil, msg1)(input)?;
  let (input, expression) = label!(expression, msg2)(input)?;*/
  Ok((input, ParserNode::FlattenData{children: vec![]}))
}*/


// send-operator := "<-" ;
pub fn send_operator(input: ParseString) -> ParseResult<()> {
  let (input, _) = whitespace0(input)?;
  let (input, _) = tag("<-")(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input, ()))
}

// context-send := prefixed-context-path, send-operator, expression ;
pub fn context_send(input: ParseString) -> ParseResult<ContextSend> {
  let msg2 = "Expects expression";
  let (input, target) = var(input)?;
  if target.context.is_none() {
    return Err(nom::Err::Error(ParseError::new(input, "send target must be an addressed context path")));
  }
  if target.kind.is_some() {
    return Err(nom::Err::Error(ParseError::new(input, "send targets cannot have kind annotations")));
  }
  let (input, _) = send_operator(input)?;
  let (input, expression) = label!(expression, msg2)(input)?;
  Ok((input, ContextSend { target, expression }))
}

// variable-define := tilde?, var, !assign-operator, define-operator, expression ;
pub fn variable_define(input: ParseString) -> ParseResult<VariableDefine> {
  let msg1 = "Expects spaces around operator";
  let msg2 = "Expects expression";
  let (input, mutable) = opt(tilde)(input)?;
  let (input, var) = var(input)?;
  if var.context.is_some() {
    return Err(nom::Err::Error(ParseError::new(input, "addressed context paths cannot be defined with :=")));
  }
  let (input, _) = labelr!(null(is_not(assign_operator)), skip_nil, msg1)(input)?;
  let (input, _) = define_operator(input)?;
  let (input, expression) = label!(expression, msg2)(input)?;
  let mutable = match mutable {
    Some(_) => true,
    None => false,
  };
  Ok((input, VariableDefine{mutable, var, expression}))
}

#[cfg(feature = "invariant_define")]
// invariant-define := identifier, "!", define-operator, expression ;
pub fn invariant_define(input: ParseString) -> ParseResult<InvariantDefine> {
  let msg1 = "Expects spaces around operator";
  let msg2 = "Expects expression";
  let (input, mut name) = identifier(input)?;
  let (input, exclam) = exclamation(input)?;
  name.name.chars.extend(exclam.chars.clone());
  let (input, _) = labelr!(null(is_not(assign_operator)), skip_nil, msg1)(input)?;
  let (input, _) = define_operator(input)?;
  let (input, expression) = label!(expression, msg2)(input)?;
  Ok((input, InvariantDefine{name, expression}))
}

// variable-assign := slice-ref, !define-operator, assign-operator, expression ;
pub fn variable_assign(input: ParseString) -> ParseResult<VariableAssign> {
  let msg1 = "Expects spaces around operator";
  let msg2 = "Expects expression";
  let (input, target) = slice_ref(input)?;
  let (input, _) = labelr!(null(is_not(define_operator)), skip_nil, msg1)(input)?;
  let (input, _) = assign_operator(input)?;
  let (input, expression) = label!(expression, msg2)(input)?;
  Ok((input, VariableAssign{target,expression}))
}

// op-assign := slice-ref, !define-operator, op-assign-operator, expression ;
pub fn op_assign(input: ParseString) -> ParseResult<OpAssign> {
  let msg1 = "Expects spaces around operator";
  let msg2 = "Expects expression";
  let (input, target) = slice_ref(input)?;
  let (input, _) = labelr!(null(is_not(define_operator)), skip_nil, msg1)(input)?;
  let (input, op) = op_assign_operator(input)?;
  let (input, expression) = label!(expression, msg2)(input)?;
  Ok((input, OpAssign{target,op,expression}))
}

// parser for the second line of the output table, generate the
// var name if there is one.

// split_operator := ">-" ;
/*pub fn split_operator(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag(">-")(input)?;
  Ok((input, ParserNode::Null))
}*/

// flatten_operator := "-<" ;
/*pub fn flatten_operator(input: ParseString) -> ParseResult<ParserNode> {
  let (input, _) = tag("-<")(input)?;
  Ok((input, ParserNode::Null))
}*/


// tuple-destructure := "(", list1(identifier, comma), ")", ":=", expression ;
fn tuple_destructure(input: ParseString) -> ParseResult<TupleDestructure> {
  let (input, _) = left_parenthesis(input)?;
  let (input, vars) = separated_list1(list_separator, identifier)(input)?;
  let (input, _) = right_parenthesis(input)?;
  let (input, _) = define_operator(input)?;
  let (input, expression) = expression(input)?;
  Ok((input, TupleDestructure{vars, expression}))
}

fn source_import_tail(input: ParseString) -> ParseResult<Token> {
  let start = input.loc();
  let (input, matched) = many1(nom_tuple((
    is_not(alt((new_line, semicolon))),
    any_token,
  )))(input)?;
  let mut tokens: Vec<Token> = matched.into_iter().map(|(_, token)| token).collect();
  let mut token = Token::merge_tokens(&mut tokens).unwrap_or(Token::default());
  while token.chars.last().is_some_and(|ch| ch.is_whitespace()) {
    token.chars.pop();
    token.src_range.end.col = token.src_range.end.col.saturating_sub(1);
  }
  token.kind = TokenKind::Any;
  token.src_range.start = start;
  Ok((input, token))
}

fn source_wildcard_specifier_is_valid(specifier: &str) -> bool {
  let wildcard_count = specifier.matches('*').count();
  wildcard_count == 0 || (wildcard_count == 1 && specifier.ends_with("/*"))
}

fn merge_source_tokens(start: SourceLocation, mut tokens: Vec<Token>) -> MechString {
  let mut token = Token::merge_tokens(&mut tokens).unwrap_or(Token::default());
  token.kind = TokenKind::Any;
  token.src_range.start = start;
  MechString { text: token }
}

fn source_path_component_token(input: ParseString) -> ParseResult<Token> {
  alt((alpha_token, digit_token, dash, underscore, period))(input)
}

fn source_path_component(input: ParseString) -> ParseResult<Vec<Token>> {
  many1(source_path_component_token)(input)
}

fn source_mec_path(input: ParseString) -> ParseResult<Vec<Token>> {
  let (input, first) = source_path_component(input)?;
  let (input, mut rest) = many0(nom_tuple((slash, source_path_component)))(input)?;
  let mut tokens = first;
  for (slash, mut component) in rest.drain(..) {
    tokens.push(slash);
    tokens.append(&mut component);
  }
  let text = Token::merge_tokens(&mut tokens.clone()).unwrap_or(Token::default()).to_string();
  if !text.ends_with(".mec") {
    return Err(nom::Err::Error(ParseError::new(input, "source import path must end in .mec")));
  }
  Ok((input, tokens))
}

fn source_mec_path_wildcard_suffix(input: ParseString) -> ParseResult<Vec<Token>> {
  let (input, suffix) = opt(nom_tuple((slash, asterisk)))(input)?;
  Ok((input, suffix.map(|(slash, asterisk)| vec![slash, asterisk]).unwrap_or_default()))
}

fn relative_source_import_specifier(input: ParseString) -> ParseResult<MechString> {
  let start = input.loc();
  let (input, prefix) = alt((
    nom_map(nom_tuple((period, period, slash)), |(a, b, c)| vec![a, b, c]),
    nom_map(nom_tuple((period, slash)), |(a, b)| vec![a, b]),
  ))(input)?;
  let (input, tail) = source_mec_path(input)?;
  let (input, wildcard) = source_mec_path_wildcard_suffix(input)?;
  let mut tokens = prefix;
  tokens.extend(tail);
  tokens.extend(wildcard);
  Ok((input, merge_source_tokens(start, tokens)))
}

fn absolute_source_import_specifier(input: ParseString) -> ParseResult<MechString> {
  let start = input.loc();
  let (input, leading) = slash(input)?;
  let (input, tail) = source_mec_path(input)?;
  let (input, wildcard) = source_mec_path_wildcard_suffix(input)?;
  let mut tokens = vec![leading];
  tokens.extend(tail);
  tokens.extend(wildcard);
  Ok((input, merge_source_tokens(start, tokens)))
}

fn bare_source_import_specifier(input: ParseString) -> ParseResult<MechString> {
  let start = input.loc();
  let (input, tokens) = source_mec_path(input)?;
  let (input, wildcard) = source_mec_path_wildcard_suffix(input)?;
  let mut tokens = tokens;
  tokens.extend(wildcard);
  Ok((input, merge_source_tokens(start, tokens)))
}

fn uri_scheme_part(input: ParseString) -> ParseResult<Token> {
  alt((alpha_token, digit_token, plus, dash, period))(input)
}

fn source_import_uri_scheme(input: ParseString) -> ParseResult<Vec<Token>> {
  let (input, first) = alpha_token(input)?;
  let (input, mut rest) = many0(uri_scheme_part)(input)?;
  let mut tokens = vec![first];
  tokens.append(&mut rest);
  Ok((input, tokens))
}

fn uri_source_import_specifier(input: ParseString) -> ParseResult<MechString> {
  let start = input.loc();
  let (input, mut tokens) = source_import_uri_scheme(input)?;
  let uri_marker_start = input.loc();
  let (input, uri_marker) = tag("://")(input)?;
  let uri_marker_end = input.loc();
  tokens.push(Token::new(TokenKind::Any, SourceRange { start: uri_marker_start, end: uri_marker_end }, uri_marker.chars().collect()));
  let (input, tail) = source_import_tail(input)?;
  tokens.push(tail);
  Ok((input, merge_source_tokens(start, tokens)))
}

fn source_import_specifier(input: ParseString) -> ParseResult<MechString> {
  alt((
    relative_source_import_specifier,
    absolute_source_import_specifier,
    uri_source_import_specifier,
    bare_source_import_specifier,
  ))(input)
}

// import-declaration := "+>", source-import-specifier ;
pub fn import_declaration(input: ParseString) -> ParseResult<ImportDeclaration> {
  let (input, _) = whitespace0(input)?;
  let (input, _) = module_import_sigil(input)?;
  let (input, _) = whitespace1(input)?;
  let (input, specifier) = source_import_specifier(input)?;
  if !source_wildcard_specifier_is_valid(&specifier.to_string()) {
    return Err(nom::Err::Failure(ParseError::new(input, "Invalid wildcard placement in import specifier")));
  }
  Ok((input, ImportDeclaration { specifier }))
}

// export-declaration := "<+", export-name ;
pub fn export_declaration(input: ParseString) -> ParseResult<ExportDeclaration> {
  let (input, _) = whitespace0(input)?;
  let (input, _) = module_export_sigil(input)?;
  let (input, _) = whitespace1(input)?;
  let (input, name) = identifier(input)?;
  Ok((input, ExportDeclaration { name }))
}

// context-declaration := "@", identifier, define-operator, context-base, ("{", list1(list-separator, context-capability-declaration), list-separator?, "}")? ;
pub fn context_declaration(input: ParseString) -> ParseResult<ContextDeclaration> {
  let (input, _) = whitespace0(input)?;
  let (input, _) = at(input)?;
  let (input, name) = identifier(input)?;
  let (input, _) = define_operator(input)?;
  let (input, base) = alt((context_base_resource_uri, context_base_context))(input)?;
  let (input, capabilities) = opt(|input| {
    let (input, _) = left_brace(input)?;
    let (input, capabilities) = separated_list1(list_separator, context_capability_declaration)(input)?;
    let (input, _) = opt(list_separator)(input)?;
    let (input, _) = right_brace(input)?;
    Ok((input, capabilities))
  })(input)?;
  Ok((input, ContextDeclaration { name, base, capabilities: capabilities.unwrap_or_default() }))
}

// context-base-context := "@", identifier ;
fn context_base_context(input: ParseString) -> ParseResult<ContextBase> {
  let (input, _) = at(input)?;
  let (input, name) = identifier(input)?;
  Ok((input, ContextBase::Context(name)))
}

// context-base-resource-uri := (alpha-token | digit-token | "-" | ".")+, "://", (alpha-token | digit-token | "-" | "." | "/" | "_")+ ;
fn context_base_resource_uri(input: ParseString) -> ParseResult<ContextBase> {
  let start = input.cursor;
  let src_start = input.loc();
  let (input, _) = many1(alt((alpha_token, digit_token, dash, period)))(input)?;
  let (input, _) = tag("://")(input)?;
  let (input, _) = many1(alt((alpha_token, digit_token, dash, period, slash, underscore)))(input)?;
  let uri = input.slice(start, input.cursor).trim().to_string();
  let src_end = input.loc();
  let src_range = SourceRange { start: src_start, end: src_end };
  let token = Token::new(TokenKind::Any, src_range, uri.chars().collect());
  Ok((input, ContextBase::ResourceUri(token)))
}

// context-capability-declaration := ":", identifier, "(", context-capability-scope, ")" ;
fn context_capability_declaration(input: ParseString) -> ParseResult<ContextCapabilityDeclaration> {
  let (input, _) = colon(input)?;
  let (input, operation) = identifier(input)?;
  let (input, _) = left_parenthesis(input)?;
  let (input, scope) = context_capability_scope(input)?;
  let (input, _) = right_parenthesis(input)?;
  Ok((input, ContextCapabilityDeclaration { operation, scope }))
}

fn context_capability_path_token(input: ParseString) -> ParseResult<Token> {
  alt((alpha_token, digit_token, dash, slash, underscore, period, asterisk))(input)
}

fn validate_context_capability_path<'a>(
  path: &str,
  input: ParseString<'a>,
) -> Result<(), nom::Err<ParseError<'a>>> {
  let star_count = path.chars().filter(|c| *c == '*').count();
  if star_count == 0 || (star_count == 1 && path.ends_with("/*") && path.len() > 2) {
    Ok(())
  } else {
    Err(nom::Err::Error(ParseError::new(
      input,
      "context capability wildcard must be either `*` or a final path segment like `foo/*`",
    )))
  }
}

fn context_capability_path(input: ParseString) -> ParseResult<Identifier> {
  let validation_input = input.clone();
  let (input, mut tokens) = many1(context_capability_path_token)(input)?;
  let mut merged = Token::merge_tokens(&mut tokens).unwrap();
  merged.kind = TokenKind::Identifier;
  validate_context_capability_path(&merged.to_string(), validation_input)?;
  Ok((input, Identifier { name: merged }))
}

// context-capability-scope := "*" | context-capability-path ;
fn context_capability_scope(input: ParseString) -> ParseResult<ContextCapabilityScope> {
  if let Ok((input, wildcard)) = asterisk(input.clone()) {
    Ok((input, ContextCapabilityScope::Wildcard(wildcard)))
  } else {
    let (input, path) = context_capability_path(input)?;
    Ok((input, ContextCapabilityScope::Path(path)))
  }
}

// statement := variable-define | variable-assign | op-assign | enum-define | tuple-destructure | kind-define ;
pub fn statement(input: ParseString) -> ParseResult<Statement> {
  let parsers: Vec<(&'static str,Box<dyn Fn(ParseString) -> ParseResult<Statement>>)> = vec![
    ("import_declaration", Box::new(|i| import_declaration(i).map(|(i, v)| (i, Statement::ImportDeclaration(v))))),
    ("export_declaration", Box::new(|i| export_declaration(i).map(|(i, v)| (i, Statement::ExportDeclaration(v))))),
    ("context_declaration", Box::new(|i| context_declaration(i).map(|(i, v)| (i, Statement::ContextDeclaration(v))))),
    ("fsm_declare", Box::new(|i| fsm_declare(i).map(|(i, v)| (i, Statement::FsmDeclare(v))))),
    #[cfg(feature = "invariant_define")]
    ("invariant_define", Box::new(|i| invariant_define(i).map(|(i, v)| (i, Statement::InvariantDefine(v))))),
    ("context_send", Box::new(|i| context_send(i).map(|(i, v)| (i, Statement::ContextSend(v))))),
    ("variable_define", Box::new(|i| variable_define(i).map(|(i, v)| (i, Statement::VariableDefine(v))))),
    ("variable_assign", Box::new(|i| variable_assign(i).map(|(i, v)| (i, Statement::VariableAssign(v))))),
    ("op_assign", Box::new(|i| op_assign(i).map(|(i, v)| (i, Statement::OpAssign(v))))),
    ("enum_define", Box::new(|i| enum_define(i).map(|(i, v)| (i, Statement::EnumDefine(v))))),
    ("tuple_destructure", Box::new(|i| tuple_destructure(i).map(|(i, v)| (i, Statement::TupleDestructure(v))))),
    ("kind_define", Box::new(|i| kind_define(i).map(|(i, v)| (i, Statement::KindDefine(v))))),
  ];
  alt_best(input, &parsers)
}

// enum-define := "<", identifier, ">", define-operator, list1(enum-separator, enum-variant);
pub fn enum_define(input: ParseString) -> ParseResult<EnumDefine> {
  let (input, (_, r)) = range(left_angle)(input)?;
  let (input, name) = identifier(input)?;
  let (input, _) = label!(right_angle, "Expects right angle", r)(input)?;
  let (input, _) = define_operator(input)?;
  let (input, variants) = separated_list1(enum_separator, enum_variant)(input)?;
  Ok((input, EnumDefine{name, variants}))
}

// enum-variant := grave?, colon?, identifier, enum-variant-kind? ;
pub fn enum_variant(input: ParseString) -> ParseResult<EnumVariant> {
  let (input, _) = opt(grave)(input)?;
  let (input, _) = opt(colon)(input)?;
  let (input, name) = identifier(input)?;
  let (input, value) = opt(alt((enum_variant_kind, enum_variant_inline_kind)))(input)?;
  Ok((input, EnumVariant{name, value}))
}

// enum-variant-kind := "(", kind-annotation, ")" ;
pub fn enum_variant_kind(input: ParseString) -> ParseResult<KindAnnotation> {
  let (input, _) = left_parenthesis(input)?;
  let (input, annotation) = kind_annotation(input)?;
  let (input, _) = right_parenthesis(input)?;
  Ok((input, annotation))
}

// enum-variant-inline-kind := kind-annotation ;
// Allows compact tagged-union syntax like `:ok<u64>`.
pub fn enum_variant_inline_kind(input: ParseString) -> ParseResult<KindAnnotation> {
  kind_annotation(input)
}

// kind-define := "<", identifier, ">", define-operator, kind-annotation ;
pub fn kind_define(input: ParseString) -> ParseResult<KindDefine> {
  let (input, (_, r)) = range(left_angle)(input)?;
  let (input, name) = identifier(input)?;
  let (input, _) = label!(right_angle, "Expects right angle", r)(input)?;
  let (input, _) = define_operator(input)?;
  let (input, knd) = kind_annotation(input)?;
  Ok((input, KindDefine{name,kind:knd}))
}

#[macro_use]
use crate::*;
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

// variable-define := tilde?, var, !assign-operator, define-operator, expression ;
pub fn variable_define(input: ParseString) -> ParseResult<VariableDefine> {
  let msg1 = "Expects spaces around operator";
  let msg2 = "Expects expression";
  let (input, mutable) = opt(tilde)(input)?; 
  let (input, var) = var(input)?;
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

// import-declaration := "+>", module-import-specifier ;
pub fn import_declaration(input: ParseString) -> ParseResult<ImportDeclaration> {
  let (input, _) = whitespace0(input)?;
  let (input, _) = module_import_sigil(input)?;
  let (input, _) = whitespace1(input)?;
  let start = input.loc();
  let spec_start = input.cursor;
  let (input, _) = many1(nom_tuple((is_not(alt((new_line, semicolon))), any_token)))(input)?;
  let specifier = input.slice(spec_start, input.cursor).trim().to_string();
  if specifier.starts_with('@')
    && (specifier.contains("/*")
      || specifier.contains('{')
      || specifier.starts_with("@ui/main")
      || !specifier.contains(":=")
      || !specifier.split_once(":=").map(|(_, rhs)| rhs.trim().contains('/')).unwrap_or(false))
  {
    return Err(nom::Err::Failure(ParseError::new(input, "Context import aliases must use module item import syntax")));
  }
  if specifier == "*" || specifier.contains("/*/") || specifier.contains('*') && !specifier.ends_with("/*") {
    return Err(nom::Err::Failure(ParseError::new(input, "Invalid wildcard placement in import specifier")));
  }
  let end = input.loc();
  let src_range = SourceRange { start, end };
  let token = Token {
    kind: TokenKind::Any,
    chars: specifier.chars().collect(),
    src_range,
  };
  Ok((input, ImportDeclaration { specifier: MechString { text: token } }))
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

// context-capability-scope := "*" | identifier ;
fn context_capability_scope(input: ParseString) -> ParseResult<ContextCapabilityScope> {
  if let Ok((input, wildcard)) = asterisk(input.clone()) {
    Ok((input, ContextCapabilityScope::Wildcard(wildcard)))
  } else {
    let (input, path) = identifier(input)?;
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

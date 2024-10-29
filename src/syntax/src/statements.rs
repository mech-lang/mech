#[macro_use]
use crate::*;
use nom::sequence::tuple as nom_tuple;

// #### Statements

// comment_sigil := "--" | "//" | "/*" ;
pub fn comment_sigil(input: ParseString) -> ParseResult<()> {
  let (input, _) = alt((tag("--"),tag("//")))(input)?;
  Ok((input, ()))
}

// comment := ws0, comment_sigil, text+ ;
pub fn comment(input: ParseString) -> ParseResult<Comment> {
  let (input, cmmnt) = alt((comment_singleline, comment_multiline))(input)?;
  Ok((input, cmmnt))
}

// comment := ws0, comment_sigil, text+ ;
pub fn comment_singleline(input: ParseString) -> ParseResult<Comment> {
  let (input, _) = whitespace0(input)?;
  let (input, _) = comment_sigil(input)?;
  let (input, mut text) = many1(text)(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input, Comment{text: merge_tokens(&mut text).unwrap()}))
}

// comment := ws0, "/*", text+, "*/" ;
pub fn comment_multiline(input: ParseString) -> ParseResult<Comment> {
  let (input, _) = whitespace0(input)?;
  let (input, _) = tag("/*")(input)?;
  let (input, text) = many1(nom_tuple((is_not(tag("*/")),alt((text,whitespace)))))(input)?;
  let mut text = text.iter().map(|(_,a)| a).cloned().collect::<Vec<Token>>();
  let (input, _) = tag("*/")(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input, Comment{text: merge_tokens(&mut text).unwrap()}))
}

// assign_operator := "=" ;
pub fn assign_operator(input: ParseString) -> ParseResult<()> {
  let (input, _) = whitespace0(input)?;
  let (input, _) = tag("=")(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input, ()))
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

// variable_define := identifier, define_operator, expression ;
pub fn variable_define(input: ParseString) -> ParseResult<VariableDefine> {
  let msg1 = "Expects spaces around operator";
  let msg2 = "Expects expression";
  let (input, var) = var(input)?;
  let (input, _) = labelr!(null(is_not(assign_operator)), skip_nil, msg1)(input)?;
  let (input, _) = define_operator(input)?;
  let (input, expression) = label!(expression, msg2)(input)?;
  Ok((input, VariableDefine{var,expression}))
}

// variable_define := identifier, assign_operator, expression ;
pub fn variable_assign(input: ParseString) -> ParseResult<VariableAssign> {
  let msg1 = "Expects spaces around operator";
  let msg2 = "Expects expression";
  let (input, target) = slice_ref(input)?;
  let (input, _) = labelr!(null(is_not(define_operator)), skip_nil, msg1)(input)?;
  let (input, _) = assign_operator(input)?;
  let (input, expression) = label!(expression, msg2)(input)?;
  Ok((input, VariableAssign{target,expression}))
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

// statement := variable_define | variable_assign | enum_define | fm_declare | kind_define ;
pub fn statement(input: ParseString) -> ParseResult<Statement> {
  match variable_define(input.clone()) {
    Ok((input, var_def)) => { return Ok((input, Statement::VariableDefine(var_def))); },
    //Err(Failure(err)) => {return Err(Failure(err))},
    _ => (),
  }
  match variable_assign(input.clone()) {
    Ok((input, var_asgn)) => { return Ok((input, Statement::VariableAssign(var_asgn))); },
    //Err(Failure(err)) => {return Err(Failure(err))},
    _ => (),
  }
  match enum_define(input.clone()) {
    Ok((input, enm_def)) => { return Ok((input, Statement::EnumDefine(enm_def))); },
    //Err(Failure(err)) => {return Err(Failure(err))},
    _ => (),
  }
  match fsm_declare(input.clone()) {
    Ok((input, var_def)) => { return Ok((input, Statement::FsmDeclare(var_def))); },
    //Err(Failure(err)) => {return Err(Failure(err))},
    _ => (),
  }
  match kind_define(input.clone()) {
    Ok((input, knd_def)) => { return Ok((input, Statement::KindDefine(knd_def))); },
    Err(err) => { return Err(err); },
  }
}

// enum_define := "<", identifier, ">", define_operator, list1(enum_separator, enum_variant);
pub fn enum_define(input: ParseString) -> ParseResult<EnumDefine> {
  let (input, _) = left_angle(input)?;
  let (input, name) = identifier(input)?;
  let (input, _) = right_angle(input)?;
  let (input, _) = define_operator(input)?;
  let (input, variants) = separated_list1(enum_separator, enum_variant)(input)?;
  Ok((input, EnumDefine{name, variants}))
}

// enum_variant := atom | identifier, enum_variant_kind? ;
pub fn enum_variant(input: ParseString) -> ParseResult<EnumVariant> {
  let (input, _) = opt(grave)(input)?;
  let (input, name) = identifier(input)?;
  let (input, value) = opt(enum_variant_kind)(input)?;
  Ok((input, EnumVariant{name, value}))
}

// enum_variant_kind := "(", kind_annotation, ")" ;
pub fn enum_variant_kind(input: ParseString) -> ParseResult<KindAnnotation> {
  let (input, _) = left_parenthesis(input)?;
  let (input, annotation) = kind_annotation(input)?;
  let (input, _) = right_parenthesis(input)?;
  Ok((input, annotation))
}

// kind_define := "<", identifier, ">", define_operator, kind_annotation ;
pub fn kind_define(input: ParseString) -> ParseResult<KindDefine> {
  let (input, _) = left_angle(input)?;
  let (input, name) = identifier(input)?;
  let (input, _) = right_angle(input)?;
  let (input, _) = define_operator(input)?;
  let (input, knd) = kind_annotation(input)?;
  Ok((input, KindDefine{name,kind:knd}))
}


  
  
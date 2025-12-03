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
  let (input, p) = paragraph(input)?;
  Ok((input, Comment{paragraph: p}))
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
  let (input, vars) = separated_list1(comma, identifier)(input)?;
  let (input, _) = right_parenthesis(input)?;
  let (input, _) = define_operator(input)?;
  let (input, expression) = expression(input)?;
  Ok((input, TupleDestructure{vars, expression}))
}

// statement := variable-define | variable-assign | op-assign | enum-define | tuple-destructure | kind-define ;
pub fn statement(input: ParseString) -> ParseResult<Statement> {
  let parsers: Vec<(&'static str,Box<dyn Fn(ParseString) -> ParseResult<Statement>>)> = vec![
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
  let (input, _) = left_angle(input)?;
  let (input, name) = identifier(input)?;
  let (input, _) = right_angle(input)?;
  let (input, _) = define_operator(input)?;
  let (input, variants) = separated_list1(enum_separator, enum_variant)(input)?;
  Ok((input, EnumDefine{name, variants}))
}

// enum-variant := grave?, identifier, enum-variant-kind? ;
pub fn enum_variant(input: ParseString) -> ParseResult<EnumVariant> {
  let (input, _) = opt(grave)(input)?;
  let (input, name) = identifier(input)?;
  let (input, value) = opt(enum_variant_kind)(input)?;
  Ok((input, EnumVariant{name, value}))
}

// enum-variant-kind := "(", kind-annotation, ")" ;
pub fn enum_variant_kind(input: ParseString) -> ParseResult<KindAnnotation> {
  let (input, _) = left_parenthesis(input)?;
  let (input, annotation) = kind_annotation(input)?;
  let (input, _) = right_parenthesis(input)?;
  Ok((input, annotation))
}

// kind-define := "<", identifier, ">", define-operator, kind-annotation ;
pub fn kind_define(input: ParseString) -> ParseResult<KindDefine> {
  let (input, _) = left_angle(input)?;
  let (input, name) = identifier(input)?;
  let (input, _) = right_angle(input)?;
  let (input, _) = define_operator(input)?;
  let (input, knd) = kind_annotation(input)?;
  Ok((input, KindDefine{name,kind:knd}))
}
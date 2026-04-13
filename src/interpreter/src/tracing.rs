use crate::*;

#[cfg(feature = "trace")]
#[derive(Debug, Clone)]
pub struct TraceEvent {
    pub index: usize,
    pub channel: Option<String>,
    pub label: Option<String>,
    pub message: String,
    pub rendered: String,
}

#[cfg(feature = "trace")]
pub(crate) fn parse_trace_line(rendered: &str) -> (Option<String>, Option<String>, String) {
    if !rendered.starts_with("[trace][") {
        return (None, None, rendered.to_string());
    }

    let mut rest = &rendered[8..];
    let Some(end_channel) = rest.find(']') else {
        return (None, None, rendered.to_string());
    };

    let channel = rest[..end_channel].to_string();
    rest = &rest[end_channel + 1..];

    if let Some(next) = rest.strip_prefix('[') {
        if let Some(end_label) = next.find(']') {
            let label = next[..end_label].to_string();
            let message = next[end_label + 1..].trim_start().to_string();
            return (Some(channel), Some(label), message);
        }
    }

    (Some(channel), None, rest.trim_start().to_string())
}

#[cfg(feature = "trace")]
pub fn trace_events_to_json(events: &[TraceEvent]) -> String {
    let mut json = String::from("[");
    for (idx, event) in events.iter().enumerate() {
        if idx > 0 {
            json.push(',');
        }
        json.push_str("{\"index\":");
        json.push_str(&event.index.to_string());
        json.push_str(",\"channel\":");
        push_json_opt_string(&mut json, event.channel.as_deref());
        json.push_str(",\"label\":");
        push_json_opt_string(&mut json, event.label.as_deref());
        json.push_str(",\"message\":");
        push_json_string(&mut json, &event.message);
        json.push_str(",\"rendered\":");
        push_json_string(&mut json, &event.rendered);
        json.push('}');
    }
    json.push(']');
    json
}

#[cfg(feature = "trace")]
fn push_json_opt_string(out: &mut String, value: Option<&str>) {
    if let Some(value) = value {
        push_json_string(out, value);
    } else {
        out.push_str("null");
    }
}

#[cfg(feature = "trace")]
fn push_json_string(out: &mut String, value: &str) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

pub(crate) fn format_trace(scope: &str, message: String) -> String {
  format!("[trace][{scope}] {message}")
}

pub(crate) fn format_trace_args(values: &Vec<Value>) -> String {
  values
    .iter()
    .map(summarize_function_value)
    .collect::<Vec<_>>()
    .join(", ")
}

pub(crate) fn summarize_function_value(value: &Value) -> String {
  const MAX_TRACE_CHARS: usize = 96;
  let rendered = trace_single_line_text(&summarize_function_value_compact(value, 0));
  trace_truncate(&rendered, MAX_TRACE_CHARS)
}

fn summarize_function_value_compact(value: &Value, depth: usize) -> String {
  if depth > 2 {
    return format!("{}(..)", value.kind().to_string());
  }
  match value {
    #[cfg(feature = "u64")]
    Value::U64(x) => format!("u64(@{:04x}:{})", trace_short_addr(x.addr()), *x.borrow()),
    #[cfg(feature = "i64")]
    Value::I64(x) => format!("i64(@{:04x}:{})", trace_short_addr(x.addr()), *x.borrow()),
    #[cfg(feature = "f64")]
    Value::F64(x) => format!("f64(@{:04x}:{})", trace_short_addr(x.addr()), *x.borrow()),
    #[cfg(feature = "bool")]
    Value::Bool(x) => format!("bool(@{:04x}:{})", trace_short_addr(x.addr()), *x.borrow()),
    #[cfg(feature = "string")]
    Value::String(x) => format!("str(@{:04x}:\"{}\")", trace_short_addr(x.addr()), x.borrow()),
    #[cfg(feature = "atom")]
    Value::Atom(x) => format!("{}(@{:04x})", x.borrow().to_string(), trace_short_addr(x.addr())),
    #[cfg(feature = "tuple")]
    Value::Tuple(tuple_ref) => summarize_function_tuple_value(tuple_ref, depth),
    _ => format!(
      "{}({})",
      value.kind().to_string(),
      trace_truncate(&trace_single_line_text(&format!("{:?}", value)), 48)
    ),
  }
}

#[cfg(feature = "tuple")]
fn summarize_function_tuple_value(tuple_ref: &Ref<MechTuple>, depth: usize) -> String {
  let tuple = tuple_ref.borrow();
  let mut parts = Vec::new();
  for element in tuple.elements.iter().take(3) {
    parts.push(summarize_function_value_compact(element, depth + 1));
  }
  if tuple.elements.len() > 3 {
    parts.push("…".to_string());
  }
  format!(
    "tuple(@{:04x}; len={}; [{}])",
    trace_short_addr(tuple_ref.addr()),
    tuple.elements.len(),
    parts.join(", ")
  )
}

fn trace_short_addr(addr: usize) -> u16 {
  (addr & 0xffff) as u16
}

pub(crate) fn summarize_values_with_kinds(values: &Vec<Value>) -> String {
  values
    .iter()
    .enumerate()
    .map(|(idx, value)| {
      format!(
        "#{idx}={} :{}",
        summarize_function_value(value),
        value.kind().to_string()
      )
    })
    .collect::<Vec<_>>()
    .join(", ")
}

pub(crate) fn summarize_function_pattern(pattern: &Pattern) -> String {
  match pattern {
    Pattern::Wildcard => "_".to_string(),
    Pattern::Expression(expr) => trace_truncate(&format!("{:?}", expr), 72),
    Pattern::Tuple(tuple) => format!("tuple(len={})", tuple.0.len()),
    Pattern::Array(array) => {
      let spread = if array.spread.is_some() { ",spread" } else { "" };
      format!(
        "array(prefix={}{} ,suffix={})",
        array.prefix.len(),
        spread,
        array.suffix.len()
      )
    }
    Pattern::TupleStruct(tuple_struct) => format!(
      "tuple-struct(name={},len={})",
      tuple_struct.name.to_string(),
      tuple_struct.patterns.len()
    ),
  }
}

fn trace_truncate(text: &str, max_chars: usize) -> String {
  let text = trace_single_line_text(text);
  if text.chars().count() <= max_chars {
    return text;
  }
  let mut truncated = text.chars().take(max_chars).collect::<String>();
  truncated.push('…');
  truncated
}

fn trace_single_line_text(text: &str) -> String {
  text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(feature = "state_machines")]
pub fn summarize_value(value: &Value) -> String {
    const MAX_TRACE_CHARS: usize = 1000;
    let rendered = trace_single_line_text(&summarize_value_compact(value, 0));
    truncate_for_trace(&rendered, MAX_TRACE_CHARS)
}

#[cfg(feature = "state_machines")]
fn summarize_value_compact(value: &Value, depth: usize) -> String {
    if depth > 2 {
        return format!("{}(..)", value.kind().to_string());
    }
    match value {
        #[cfg(feature = "u64")]
        Value::U64(x) => format!("u64(@{:04x}:{})", short_addr(x.addr()), *x.borrow()),
        #[cfg(feature = "i64")]
        Value::I64(x) => format!("i64(@{:04x}:{})", short_addr(x.addr()), *x.borrow()),
        #[cfg(feature = "f64")]
        Value::F64(x) => format!("f64(@{:04x}:{})", short_addr(x.addr()), *x.borrow()),
        #[cfg(feature = "bool")]
        Value::Bool(x) => format!("bool(@{:04x}:{})", short_addr(x.addr()), *x.borrow()),
        #[cfg(feature = "string")]
        Value::String(x) => format!("str(@{:04x}:\"{}\")", short_addr(x.addr()), x.borrow()),
        #[cfg(feature = "atom")]
        Value::Atom(x) => format!("{}(@{:04x})", x.borrow().to_string(), short_addr(x.addr())),
        #[cfg(feature = "tuple")]
        Value::Tuple(tuple_ref) => summarize_tuple_value(tuple_ref, depth),
        _ => format!(
            "{}({})",
            value.kind().to_string(),
            truncate_for_trace(&trace_single_line_text(&format!("{:?}", value)), 48)
        ),
    }
}

#[cfg(all(feature = "state_machines", feature = "tuple"))]
fn summarize_tuple_value(tuple_ref: &Ref<MechTuple>, depth: usize) -> String {
    let tuple = tuple_ref.borrow();
    if let Some(first) = tuple.elements.first() {
        if let Value::Atom(tag) = first.as_ref() {
            let mut parts = Vec::new();
            for element in tuple.elements.iter().skip(1).take(3) {
                parts.push(summarize_value_compact(element, depth + 1));
            }
            if tuple.elements.len() > 4 {
                parts.push("…".to_string());
            }
            let details = if parts.is_empty() {
                String::new()
            } else {
                format!("  {}", parts.join("  "))
            };
            return format!(
                "@{:04x}  {}(@{:04x}){}",
                short_addr(tuple_ref.addr()),
                tag.borrow().to_string(),
                short_addr(tag.addr()),
                details
            );
        }
    }

    let mut parts = Vec::new();
    for element in tuple.elements.iter().take(3) {
        parts.push(summarize_value_compact(element, depth + 1));
    }
    if tuple.elements.len() > 3 {
        parts.push("…".to_string());
    }
    format!(
        "(@{:04x}; len={}; [{}])",
        short_addr(tuple_ref.addr()),
        tuple.elements.len(),
        parts.join(", ")
    )
}

#[cfg(feature = "state_machines")]
fn short_addr(addr: usize) -> u16 {
    (addr & 0xffff) as u16
}

#[cfg(feature = "state_machines")]
pub fn summarize_pattern(pattern: &Pattern) -> String {
    match pattern {
        Pattern::Wildcard => "*".to_string(),
        Pattern::Expression(expr) => truncate_for_trace(&format!("{:?}", expr), 1000),
        Pattern::Tuple(tuple) => format!("tuple(len={})", tuple.0.len()),
        Pattern::Array(array) => {
            let spread = if array.spread.is_some() { ", spread" } else { "" };
            format!(
                "array(prefix={}, suffix={}{} )",
                array.prefix.len(),
                array.suffix.len(),
                spread
            )
        }
        Pattern::TupleStruct(tuple_struct) => {
            format!(
                ":{}(len={})",
                tuple_struct.name.to_string(),
                tuple_struct.patterns.len()
            )
        }
    }
}

#[cfg(feature = "state_machines")]
pub fn summarize_guard_condition(pattern: &Pattern) -> String {
    match pattern {
        Pattern::Wildcard => "*".to_string(),
        Pattern::Expression(expr) => summarize_expression(expr),
        _ => summarize_pattern(pattern),
    }
}

#[cfg(feature = "state_machines")]
fn summarize_expression(expr: &Expression) -> String {
    match expr {
        Expression::Formula(factor) => summarize_factor(factor),
        Expression::Var(var) => var.name.to_string(),
        Expression::Literal(lit) => summarize_literal(lit),
        _ => truncate_for_trace(&format!("{:?}", expr), 120),
    }
}

#[cfg(feature = "state_machines")]
fn summarize_factor(factor: &Factor) -> String {
    match factor {
        Factor::Expression(expr) => summarize_expression(expr),
        Factor::Parenthetical(inner) => format!("({})", summarize_factor(inner)),
        Factor::Negate(inner) => format!("-{}", summarize_factor(inner)),
        Factor::Not(inner) => format!("!{}", summarize_factor(inner)),
        Factor::Term(term) => summarize_term(term),
        _ => truncate_for_trace(&format!("{:?}", factor), 120),
    }
}

#[cfg(feature = "state_machines")]
fn summarize_term(term: &Term) -> String {
    let mut out = summarize_factor(&term.lhs);
    for (op, rhs) in &term.rhs {
        out.push(' ');
        out.push_str(formula_operator_symbol(op));
        out.push(' ');
        out.push_str(&summarize_factor(rhs));
    }
    out
}

#[cfg(feature = "state_machines")]
fn summarize_literal(lit: &Literal) -> String {
    match lit {
        Literal::Number(number) => number.to_string(),
        Literal::Boolean(token) => token.to_string(),
        Literal::String(text) => text.to_string(),
        Literal::Atom(atom) => format!(":{}", atom.name.to_string()),
        _ => truncate_for_trace(&format!("{:?}", lit), 64),
    }
}

#[cfg(feature = "state_machines")]
fn formula_operator_symbol(op: &FormulaOperator) -> &'static str {
    match op {
        FormulaOperator::Comparison(ComparisonOp::Equal) => "==",
        FormulaOperator::Comparison(ComparisonOp::NotEqual) => "!=",
        FormulaOperator::Comparison(ComparisonOp::StrictEqual) => "===",
        FormulaOperator::Comparison(ComparisonOp::StrictNotEqual) => "!==",
        FormulaOperator::Comparison(ComparisonOp::GreaterThan) => ">",
        FormulaOperator::Comparison(ComparisonOp::GreaterThanEqual) => ">=",
        FormulaOperator::Comparison(ComparisonOp::LessThan) => "<",
        FormulaOperator::Comparison(ComparisonOp::LessThanEqual) => "<=",
        FormulaOperator::AddSub(AddSubOp::Add) => "+",
        FormulaOperator::AddSub(AddSubOp::Sub) => "-",
        FormulaOperator::MulDiv(MulDivOp::Mul) => "*",
        FormulaOperator::MulDiv(MulDivOp::Div) => "/",
        FormulaOperator::MulDiv(MulDivOp::Mod) => "%",
        FormulaOperator::Power(PowerOp::Pow) => "^",
        FormulaOperator::Logic(LogicOp::And) => "&&",
        FormulaOperator::Logic(LogicOp::Or) => "||",
        FormulaOperator::Logic(LogicOp::Xor) => "xor",
        FormulaOperator::Logic(LogicOp::Not) => "!",
        _ => "?",
    }
}

#[cfg(feature = "state_machines")]
fn truncate_for_trace(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let mut truncated = text.chars().take(max_chars).collect::<String>();
    truncated.push('…');
    truncated
}

#[cfg(feature = "state_machines")]
pub fn format_fsm_trace(label: &str, message: String) -> String {
    format!("[trace][fsm][{label:>6}] {message}")
}

#[cfg(all(feature = "state_machines", feature = "trace"))]
pub fn format_fsm_trace_report(events: &[TraceEvent]) -> String {
    let mut name = "FSM".to_string();
    let mut lines: Vec<String> = Vec::new();
    let mut output = None::<String>;

    for event in events
        .iter()
        .filter(|evt| evt.channel.as_deref() == Some("fsm"))
    {
        match event.label.as_deref() {
            Some("start") => {
                if let Some((n, state)) = event.message.split_once(" state=") {
                    if let Some(name_value) = n.strip_prefix("name=") {
                        name = name_value.to_string();
                    }
                    lines.push(format!(" start  {state}"));
                }
            }
            Some("step") => {
                if let Some((step, state)) = event.message.split_once(" state=") {
                    lines.push(String::new());
                    lines.push(format!(" step {step}  {state}"));
                }
            }
            Some("arm") => {
                let arm = event
                    .message
                    .replace("check transition pattern=", "")
                    .replace("check guard pattern=", "");
                lines.push(format!("          arm{}", arm.replacen(']', "]  ", 1)));
            }
            Some("guard") => {
                let text = event
                    .message
                    .replace(" check ", " ")
                    .replace(" condition=", " ");
                lines.push(format!("          {}", text.replace("arm[0] ", "guard   ")));
            }
            Some("transition") => {
                if let Some((_, rhs)) = event.message.split_once(' ') {
                    if let Some((from, to)) = rhs.split_once(" -> ") {
                        lines.push(format!("          → {}  {}", from.trim(), to.trim()));
                    } else {
                        lines.push(format!("          → {}", rhs.trim()));
                    }
                }
            }
            Some("output") => {
                output = event.message.strip_prefix("value=").map(|x| x.to_string());
            }
            _ => {}
        }
    }

    let divider = "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━";
    let mut rendered = String::new();
    rendered.push_str(&name);
    rendered.push('\n');
    rendered.push_str(divider);
    rendered.push('\n');
    for line in lines {
        rendered.push_str(&line);
        rendered.push('\n');
    }
    rendered.push_str(divider);
    rendered.push('\n');
    if let Some(value) = output {
        rendered.push_str(&format!(" output  {}", value));
    } else {
        rendered.push_str(" output  <none>");
    }
    rendered
}

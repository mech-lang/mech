#[cfg(all(feature = "no_std", not(feature = "std"), feature = "trace"))]
use alloc::string::{String, ToString};
#[cfg(all(
    feature = "no_std",
    not(feature = "std"),
    feature = "trace",
    feature = "state_machines"
))]
use alloc::vec::Vec;

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

#[cfg(all(feature = "state_machines", feature = "trace"))]
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

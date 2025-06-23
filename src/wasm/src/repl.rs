use crate::*;
use gloo_net::http::Request;
use wasm_bindgen_futures::spawn_local;

use crate::CURRENT_MECH;

pub fn execute_repl_command(repl_cmd: ReplCommand) -> String {
  match repl_cmd {
    ReplCommand::Clear(_) => {
      CURRENT_MECH.with(|mech_ref| {
        if let Some(ptr) = *mech_ref.borrow() {
          unsafe {
            let mut mech = &mut *ptr;
            mech.interpreter.clear();
          }
        }
      });
      "".to_string()
    }
    ReplCommand::Clc => {
      let window = web_sys::window().expect("global window does not exists");    
      let document = window.document().expect("expecting a document on window");
      let output_element = document.get_element_by_id("mech-output").expect("REPL output element not found");
      // Remove all children
      while output_element.child_nodes().length() > 0 {
        let first_child = output_element
          .first_child()
          .expect("Expected a child node");
        output_element
          .remove_child(&first_child)
          .expect("Failed to remove child");
      }
      "".to_string()
    }
    ReplCommand::Code(code) => {
      CURRENT_MECH.with(|mech_ref| {
        if let Some(ptr) = *mech_ref.borrow() {
          unsafe {
            let mut mech = &mut *ptr;
            match run_mech_code(&mut mech.interpreter, &code)  {
              Ok(output) => { 
                return format!("<div class=\"mech-output-kind\">{:?}</div><div class=\"mech-output-value\">{}</div>", output.kind(), output.to_html());
              },
              Err(err) => { return format!("{:?}",err); }
            }
          }
        }
        "Error: No interpreter found.".to_string()
      })
    }
    ReplCommand::Step(count) => {
      CURRENT_MECH.with(|mech_ref| {
        if let Some(ptr) = *mech_ref.borrow() {
          unsafe {
            let mut mech = &mut *ptr;
            let n = match count {
              Some(n) => n,
              None => 1,
            };
            mech.interpreter.step(n as u64);
            return format!("<div class=\"mech-output-kind\">Step</div><div class=\"mech-output-value\">Executed {} step(s).</div>",n);
          }
        }
        "Error: No interpreter found.".to_string()
      })
    }
    ReplCommand::Whos(names) => {
      CURRENT_MECH.with(|mech_ref| {
        if let Some(ptr) = *mech_ref.borrow() {
          unsafe {
            let mut mech = &mut *ptr;
            return whos_html(&mech.interpreter, names)
          }
        }
        "Error: No interpreter found.".to_string()
      })
    }
    ReplCommand::Help => {
      help_html()
    }
    ReplCommand::Docs(doc) => {
      match doc {
        Some(d) => {
          load_doc(&d);
          format!("Fetching doc: {}...", d)
        },
        None => "Enter the name of a doc to load.".to_string(),
      } 
    }
    _ => todo!("Implement other REPL commands"),
  }
}

// Print out help information in HTML format
pub fn help_html() -> String {
  let text_logo = r#"
┌─────────┐ ┌──────┐ ┌─┐ ┌──┐ ┌─┐   ┌─┐
└───┐ ┌───┘ └──────┘ │ │ └┐ │ │ │   │ │
┌─┐ │ │ ┌─┐ ┌──────┐ │ │  └─┘ │ └─┐ │ │
│ │ │ │ │ │ │ ┌────┘ │ │  ┌─┐ │ ┌─┘ │ │
│ │ └─┘ │ │ │ └────┐ │ └──┘ │ │ │   │ │
└─┘     └─┘ └──────┘ └──────┘ └─┘   └─┘"#;
  let version = env!("CARGO_PKG_VERSION");
  let mut html = String::new();
  
  html.push_str("<div class=\"mech-help\">");
  html.push_str(&format!("<div class=\"mech-text-logo\">{}</div>", text_logo));  
  html.push_str(&format!("<div class=\"mech-version\">Version: <a href=\"https://github.com/mech-lang/mech/releases/tag/v{}-beta\">{}</a></div>", version, version));
  html.push_str("<p>Welcome to the Mech REPL!</p>");
  html.push_str("<p>Full documentation: <a href=\"https://docs.mech-lang.org\">docs.mech-lang.org</a>.</p>"); 
  html.push_str("<table class=\"mech-help-table\">");
    html.push_str("<thead><tr><th>Command</th><th>Short</th><th>Options</th><th>Description</th></tr></thead>");
    html.push_str("<tbody>");
    html.push_str("<tr><td><span class=\"mech-command\">:clc</span></td><td></span></td><td></td><td>Clear the REPL output.</td></tr>");
    html.push_str("<tr><td><span class=\"mech-command\">:clear</span><td></span></td></td><td></td><td>Clear the interpreter state.</td></tr>");
    html.push_str("<tr><td><span class=\"mech-command\">:help</span></td><td><span class=\"mech-command\">:h</span></td><td></td><td>Show this help message.</td></tr>");
    html.push_str("<tr><td><span class=\"mech-command\">:step</span></td><td></td><td><span class=\"mech-command\">[count]</span></td><td>Run the plan for a specified number of steps.</td></tr>");
    html.push_str("<tr><td><span class=\"mech-command\">:whos</span></td><td><span class=\"mech-command\">:w</span></td><td><span class=\"mech-command\">[names...]</span></td><td>Show the current symbol directory.</td></tr>");
    html.push_str("</tbody>");
  html.push_str("</table>");
  html.push_str("</div>");
  html
}

pub fn whos_html(intrp: &Interpreter, names: Vec<String>) -> String {
  let mut html = String::new();

  html.push_str("<table class=\"mech-table\">");
    html.push_str("<thead class=\"mech-table-header\"><tr>");
      html.push_str("<th class=\"mech-table-field\">Name</th>");
      html.push_str("<th class=\"mech-table-field\">Size</th>");
      html.push_str("<th class=\"mech-table-field\">Bytes</th>");
      html.push_str("<th class=\"mech-table-field\">Kind</th>");
    html.push_str("</tr></thead>");
  html.push_str("<tbody class=\"mech-table-body\">");

  let dictionary = intrp.dictionary();
  if !names.is_empty() {
    for target_name in names {
      for (id, var_name) in dictionary.borrow().iter() {
        if *var_name == target_name {
          if let Some(value_rc) = intrp.get_symbol(*id) {
            let value = value_rc.borrow();
            append_row(&mut html, var_name, &value);
          }
          break;
        }
      }
    }
  } else {
    for (id, var_name) in dictionary.borrow().iter() {
      if let Some(value_rc) = intrp.get_symbol(*id) {
        let value = value_rc.borrow();
        append_row(&mut html, var_name, &value);
      }
    }
  }
  html.push_str("</tbody></table>");
  html
}

fn append_row(html: &mut String, name: &str, value: &Value) {
  let name = html_escape(name);
  let size = html_escape(&format!("{:?}", value.shape()));
  let bytes = html_escape(&format!("{:?}", value.size_of()));
  let kind = html_escape(&format!("{:?}", value.kind()));

  html.push_str("<tr class=\"mech-table-row\">");

  let id = hash_str(&name);
  html.push_str(&format!("<td class=\"mech-table-column\"><span class=\"mech-var-name mech-clickable\" id=\"{}:0\">{}</span></td>",id, name));
  html.push_str(&format!("<td class=\"mech-table-column\">{}</td>", size));
  html.push_str(&format!("<td class=\"mech-table-column\">{}</td>", bytes));
  html.push_str(&format!("<td class=\"mech-table-column\">{}</td>", kind));
  html.push_str("</tr>");
}

fn html_escape(input: &str) -> String {
  input
    .replace('&', "&amp;")
    .replace('<', "&lt;")
    .replace('>', "&gt;")
}

use crate::*;
use gloo_net::http::Request;
use wasm_bindgen_futures::spawn_local;


pub fn execute_repl_command(intrp:  &mut Interpreter, repl_cmd: ReplCommand) -> String {
  match repl_cmd {
    ReplCommand::Clear(_) => {
      *intrp = Interpreter::new(intrp.id);
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
      match run_mech_code(intrp, &code)  {
        Ok(output) => { 
          return format!("<div class=\"mech-output-kind\">{:?}</div><div class=\"mech-output-value\">{}</div>", output.kind(), output.to_html());
        },
        Err(err) => { return format!("{:?}",err); }
      }
    }
    ReplCommand::Step(count) => {
      let n = match count {
        Some(n) => n,
        None => 1,
      };
      //let now = std::time::Instant::now();
      intrp.step(n as u64);
      //let elapsed_time = now.elapsed();
      //let cycle_duration = elapsed_time.as_nanos() as f64;
      //format!("{} cycles in {:0.2?} ns\n", n, cycle_duration)s
      "".to_string()
    }
    ReplCommand::Whos(names) => {
      whos_html(intrp, names)
    }
    ReplCommand::Help => {
      help_html()
    }
    ReplCommand::Docs(doc) => {
      log!("Loading doc: {:?}", doc);
      match doc {
        Some(d) => {
          load_doc(&d);
          "".to_string()
        },
        None => "".to_string(),
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
      log!("Processing variable: {} with id: {}", var_name, id);
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

pub fn load_doc(doc: &str) {
  let doc = doc.to_string();
  spawn_local(async move {
    log!("Loading doc: {}", doc);
    let doc_mec = fetch_docs(&doc).await;
    let window = web_sys::window().expect("global window does not exists");
    let document = window.document().expect("expecting a document on window");
    match parser::parse(&doc_mec) {
      Ok(tree) => {
        let mut formatter = Formatter::new();
        formatter.html = true;
        let doc_html = formatter.program(&tree);
        let output_element = document.get_element_by_id("mech-output").expect("REPL output element not found");
        // Make sure it's added to the end of the repl, and that we put the prompt at the end
        output_element.set_inner_html(&doc_html);
      },
      Err(err) => {
        web_sys::console::log_1(&format!("Error formatting doc: {:?}", err).into());
      }
    }

    let output_element = document.get_element_by_id("mech-output").expect("REPL output element not found");
  });
}

async fn fetch_docs(doc: &str) -> String {
  // the doc will be formatted as machine/doc
  let parts: Vec<&str> = doc.split('/').collect();
  if parts.len() >= 2 {
      let machine = parts[0];
      let doc = parts[1];
      let url = format!("https://raw.githubusercontent.com/mech-machines/{}/main/docs/{}.mec", machine, doc);
      match Request::get(&url).send().await {
        Ok(response) => match response.text().await {
          Ok(text) => {
            text
          }
          Err(e) => {
            web_sys::console::log_1(&format!("Error reading response text: {:?}", e).into());
            "".to_string()
          }
        },
        Err(err) => {
          web_sys::console::log_1(&format!("Fetch error: {:?}", err).into());
          "".to_string()
        }
      }
  } else {
    web_sys::console::log_1(&format!("Invalid doc format: {}", doc).into());
    "".to_string()
  }
}
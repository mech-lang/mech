use std::collections::HashSet;
use std::cell::RefCell;
use std::sync::Mutex;
use std::sync::Arc;

use tower_lsp::jsonrpc;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use mech_core::*;
use mech_core::nodes::*;

use crate::parser;
use crate::ast::Ast;
use crate::compiler::Compiler;

///
/// TODO
///
/// Current:
///
/// Hover  -- ok
/// Goto definition
///
/// Run mech instance
///
///
///   1. Integrate langserver into `mech` executable -- ok
///   2. Let parser track location information -- wait
///   3. (Include location information in parser nodes)? -- wait
///
///
/// Long run:
///   1. Implement syntax highlighting with lsp
///   2. Use delta to improve server's performance
/// 
/// Long term goals:
///   Running value
///   Debuger
///

fn collect_global_table_symbols(ast_node: &AstNode, set: &mut HashSet<String>) {
  match ast_node {
    AstNode::TableDefine{children} => {
      for node in children {
        match node {
          AstNode::Table{name, id: _, ..} => {
            let table_name = name.into_iter().collect();
            set.insert(table_name);
            break;
          },
          _ => (),
        }
      }
    },
    AstNode::Root{children} |
    AstNode::Block{children} |
    AstNode::Statement{children} |
    AstNode::Fragment{children} | 
    AstNode::Program{children, ..} |
    AstNode::Section{children, ..} |
    AstNode::Transformation{children} => {
      for node in children {
        collect_global_table_symbols(node, set);
      }
    },
    _ => (),
  }
}

unsafe impl Send for SharedState {}
unsafe impl Sync for SharedState {}

struct SharedState {
  global_table_symbols: Vec<String>,
  mech_core: Core,
}

impl SharedState {
  fn set_global_table_symbols(&mut self, symbols: &mut Vec<String>) {
    self.global_table_symbols.clear();
    self.global_table_symbols.append(symbols);
  }

  fn set_core_sections(&mut self, sections: Vec<Vec<SectionElement>>) {
    self.mech_core = Core::new();
    self.mech_core.load_sections(sections);
  }
}

pub struct Backend {
  client: Client,
  shared_state: Mutex<RefCell<SharedState>>,
}

impl Backend {
  pub fn new(client: Client) -> Self {
    Self {
      client,
      shared_state: Mutex::new(RefCell::new(SharedState {
        global_table_symbols: vec![],
        mech_core: Core::new(),
      })),
    }
  }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
  async fn initialize(&self, _: InitializeParams) -> jsonrpc::Result<InitializeResult> {
    println!("[INITIALIZE]");
    Ok(InitializeResult {
      capabilities: ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        completion_provider: Some(CompletionOptions::default()),
        ..Default::default()
      },
      ..Default::default()
    })
  }

  async fn did_change(&self, params: DidChangeTextDocumentParams) {
    let uri = params.text_document.uri;
    println!("[DID_CHANGE] {}", uri.path());
    let source_code_ref: &String = if params.content_changes.len() != 0 {
      &params.content_changes[0].text
    } else {
      return;
    };
    match parser::parse(source_code_ref) {
      Ok(tree) => {
        println!("source code ok!");
        self.client.publish_diagnostics(uri, vec![], None).await;
        let mut ast = Ast::new();
        ast.build_syntax_tree(&tree);
        let mut set = HashSet::new();
        collect_global_table_symbols(&ast.syntax_tree, &mut set);
        let mut symbols: Vec<String> = set.into_iter().collect();
        self.shared_state.lock().unwrap().borrow_mut().set_global_table_symbols(&mut symbols);
        let mut compiler = Compiler::new();
        let sections = compiler.compile_sections(&vec![ast.syntax_tree]).unwrap();
        self.shared_state.lock().unwrap().borrow_mut().set_core_sections(sections);
      },
      Err(err) => if let MechErrorKind::ParserError(node, report) = err.kind {
        println!("source code err!");
        println!("--------------------------------------------------------------");
        parser::print_err_report(source_code_ref, &report);
        println!("--------------------------------------------------------------");
        let mut diags = vec![];
        let err_locs = parser::get_err_locations(source_code_ref, &report);
        println!("{:?}", err_locs);
        for (i, err) in report.iter().enumerate() {
          let range = Range {
            start: Position {
              line: ((err_locs[i].0).0 - 1) as u32,
              character: ((err_locs[i].0).1 - 1) as u32,
            },
            end: Position {
              line: ((err_locs[i].1).0 - 1) as u32,
              character: ((err_locs[i].1).1 - 1) as u32,
            },
          };
          diags.push(Diagnostic {
            range,
            severity: None,
            code: None,
            code_description: None,
            source: None,
            message: err.err_message.clone(),
            related_information: None,
            tags: None,
            data: None,
          });
        }
        self.client.publish_diagnostics(uri, diags, None).await;
      } else {
        return;
      },
    }
  }

  async fn initialized(&self, _: InitializedParams) {
    println!("[INITIALIZED]");
    self.client
      .log_message(MessageType::INFO, "server initialized!")
      .await;
  }

  async fn shutdown(&self) -> jsonrpc::Result<()> {
    println!("[SHUTDOWN]");
    Ok(())
  }

  async fn completion(&self, _: CompletionParams) -> jsonrpc::Result<Option<CompletionResponse>> {
    println!("[COMPLETION]");
    let mut items = vec![];
    let symbols = self.shared_state.lock().unwrap().borrow().global_table_symbols.clone();
    for symbol in symbols {
      items.push(
        CompletionItem::new_simple(symbol, "Global table".to_string()),
      );
    }
    Ok(Some(CompletionResponse::Array(items)))
  }

  async fn hover(&self, params: HoverParams) -> jsonrpc::Result<Option<Hover>> {
    println!("[HOVER]");
    // read file
    let path = params.text_document_position_params.text_document.uri.path();
    let source: String = std::fs::read_to_string(path).unwrap();
    let position = params.text_document_position_params.position;
    let (mut row, mut col, mut index) = (0, 0, 0);
    for ch in source.chars() {
      if row == position.line && col == position.character {
          break;
      }
      if ch == '\n' {
        row += 1;
        col = 0;
      } else {
        col += 1;
      }
      index += 1;
    }
    println!("{}, {}, {}", row, col, index);
    let (mut start, mut end) = (0, source.len());
    for i in (0..=index).rev() {
      let c = source.chars().nth(i).unwrap();
      print!("?? {}", c);
      if !c.is_alphanumeric() || c == ' ' || c == '\n'  {
        println!(" accept");
        start = i;
        break;
      }
      println!(" cont");
    }
    for i in index..source.len() {
      let c = source.chars().nth(i).unwrap();
      println!("!! {}", c);
      if !c.is_alphanumeric() || c == ' ' || c == '\n'  {
        println!(" accept");
        end = i;
        break;
      }
      println!(" cont");
    }
    println!("{}", source[start..end].to_owned());
    Ok(Some(Hover {
      contents: HoverContents::Scalar(
        MarkedString::String("You're hovering!".to_string())
      ),
      range: None
    }))
  }
}

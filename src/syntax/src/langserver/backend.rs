use std::collections::HashSet;
use std::sync::Mutex;
use std::sync::Arc;
use std::rc::Rc;

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
///   2. Let parser track location information -- ok
///   3. (Include location information in parser nodes)? -- ok
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

fn location_in_range(loc: SourceLocation, rng: SourceRange) -> bool {
  loc >= rng.start && loc < rng.end
}

fn identifier_at_location(ast_node: &AstNode, loc: SourceLocation) -> Option<String> {
  match ast_node {
    AstNode::Root{children} |
    AstNode::Program{children, ..} |
    AstNode::Transformation{children} |
    AstNode::VariableDefine{children} |
    AstNode::Expression{children} |
    AstNode::MathExpression{children} |
    AstNode::AnonymousTableDefine{children} |
    AstNode::TableDefine{children} |
    AstNode::TableRow{children} |
    AstNode::TableColumn{children} |
    AstNode::Function{children, ..} |
    AstNode::Section{children, ..} => {
      for node in children {
        if let Some(name) = identifier_at_location(node, loc) {
          return Some(name);
        }
      }
      return None;
    },
    AstNode::Statement{children, src_range} |
    AstNode::Block{children, src_range} => {
      if location_in_range(loc, *src_range) {
        for node in children {
          if let Some(name) = identifier_at_location(node, loc) {
            return Some(name);
          }
        }
      }
      return None;
    },
    AstNode::Table{name, src_range, ..} |
    AstNode::SelectData{name, src_range, ..} |
    AstNode::Identifier{name, src_range, ..} => {
      if location_in_range(loc, *src_range) {
        return Some(name.iter().collect::<String>())
      }
      return None;
    },
    _ => return None,
  }
}

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
    AstNode::Block{children, ..} |
    AstNode::Statement{children, ..} |
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
  mech_ast: Ast,
}

pub struct Backend {
  client: Client,
  shared_state: Mutex<SharedState>,
}

impl Backend {
  pub fn new(client: Client) -> Self {
    Self {
      client,
      shared_state: Mutex::new(SharedState {
        global_table_symbols: vec![],
        mech_core: Core::new(),
        mech_ast: Ast::new(),
      }),
    }
  }

  fn with_shared_state<F>(&self, f: F)
  where
    F: FnOnce(&mut SharedState)
  {
    f(&mut self.shared_state.lock().unwrap())
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
    println!("[DID_CHANGE] @ {}", uri.path());
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
        let mut compiler = Compiler::new();
        let sections = compiler.compile_sections(&vec![ast.syntax_tree.clone()]).unwrap();
        self.with_shared_state(move |shared_state| {
          // refresh table symbols
          shared_state.global_table_symbols.clear();
          shared_state.global_table_symbols.append(&mut symbols);
          // refresh core
          shared_state.mech_core = Core::new();
          shared_state.mech_core.load_sections(sections);
          // refresh ast
          shared_state.mech_ast = ast;
        });
      },
      Err(err) => if let MechErrorKind::ParserError(node, report, msg) = err.kind {
        println!("source code err!");
        println!("--------------------------------------------------------------");
        parser::print_err_report(source_code_ref, &report);
        println!("--------------------------------------------------------------");
        let mut diags = vec![];
        for err in report {
          let range = Range {
            start: Position {
              line: (err.cause_rng.start.row - 1) as u32,
              character: (err.cause_rng.start.col - 1) as u32,
            },
            end: Position {
              line: (err.cause_rng.end.row - 1) as u32,
              character: (err.cause_rng.end.col - 1) as u32,
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
    let symbols = self.shared_state.lock().unwrap().global_table_symbols.clone();
    for symbol in symbols {
      items.push(
        CompletionItem::new_simple(symbol, "Global table".to_string()),
      );
    }
    Ok(Some(CompletionResponse::Array(items)))
  }

  async fn hover(&self, params: HoverParams) -> jsonrpc::Result<Option<Hover>> {
    println!("[HOVER]");
    let req_pos = params.text_document_position_params.position;
    let req_loc = SourceLocation {
      row: req_pos.line as usize + 1,
      col: req_pos.character as usize + 1,
    };

    let mut response = None;

    self.with_shared_state(|shared_state| {
      let name = match identifier_at_location(&shared_state.mech_ast.syntax_tree, req_loc) {
        Some(identifier) => identifier,
        None => return,
      };
      match shared_state.mech_core.get_table(&name) {
        Ok(table) => {
          let mut printer = BoxPrinter::new();
          printer.add_table(&table.borrow());
          response = Some(Hover {
            contents: HoverContents::Scalar(
              MarkedString::String(format!("```text{}```", printer.print()))
            ),
            range: None,
          })
        },
        Err(_) => return,
      };
    });

    Ok(response)
  }
}

use std::collections::HashSet;
use std::cell::RefCell;
use std::sync::Mutex;

use tower_lsp::jsonrpc;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use tokio::net::TcpListener;

use mech_syntax::parser;
use mech_syntax::ast::Ast;
use mech_core::*;
use mech_core::nodes::*;

///
/// TODO
///
/// Current:
///
/// Hover
/// Goto definition
///
///
/// Running value
/// Debuger
///
/// Run mech instance
///
///
///   1. Integrate langserver into `mech` executable
///   2. Let parser track location information
///   3. (Include location information in parser nodes)?
///
///
/// Long run:
///   1. Implement syntax highlighting with lsp
///   2. Use delta to improve server's performance
///

fn collect_global_table_symbols(ast_node: &AstNode, set: &mut HashSet<String>) {
  match ast_node {
    AstNode::TableDefine{children} => {
      for node in children {
        match node {
          AstNode::Table{name, id} => {
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
    AstNode::Transformation{children}=> {
      for node in children {
        collect_global_table_symbols(node, set);
      }
    },
    AstNode::Program{title, children} |
    AstNode::Section{title, children} => {
      for node in children {
        collect_global_table_symbols(node, set);
      }
    }
    _ => (),
  }
}

struct MechLangBackend {
  client: Client,
  shared_state: Mutex<RefCell<SharedState>>,
}

struct SharedState {
  global_table_symbols: Vec<String>,
}

impl SharedState {
  fn set_global_table_symbols(&mut self, symbols: &mut Vec<String>) {
    self.global_table_symbols.clear();
    self.global_table_symbols.append(symbols);
  }
}

impl MechLangBackend {
  fn new(client: Client) -> Self {
    Self {
      client,
      shared_state: Mutex::new(RefCell::new(SharedState {
        global_table_symbols: vec![],
      })),
    }
  }
}

#[tower_lsp::async_trait]
impl LanguageServer for MechLangBackend {
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
            start: Position client_addr{
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

  async fn hover(&self, _: HoverParams) -> jsonrpc::Result<Option<Hover>> {
    println!("[HOVER]");
    Ok(Some(Hover {
      contents: HoverContents::Scalar(
        MarkedString::String("You're hovering!".to_string())
      ),
      range: None
    }))
  }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
  let listener = TcpListener::bind("127.0.0.1:4041").await?;

  loop {
    println!("Waiting for client...");
    match listener.accept().await {
      Ok((conn_sk, client_addr)) => {
        println!("Incomming client: {:?}", client_addr);
        let (service, client_sk) = LspService::new(|client| MechLangBackend::new(client));
        let (conn_sk_in, conn_sk_out) = conn_sk.into_split();
        Server::new(conn_sk_in, conn_sk_out, client_sk).serve(service).await;
      },
      Err(e) => println!("couldn't get client: {:?}", e),
    }
  }

  Ok(())
}

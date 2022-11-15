use tower_lsp::jsonrpc;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use tokio::net::TcpListener;

use mech_syntax::parser;
use mech_core::*;

///
/// TODO
///
/// Current:
///   1. Integrate langserver into `mech` executable
///   2. Let parser track location information
///   3. (Include location information in parser nodes)?
///
///
/// Long run:
///   1. Implement syntax highlighting with lsp
///   2. Use delta to improve server's performance
///

#[derive(Debug)]
struct MechLangBackend {
  client: Client,
}

impl MechLangBackend {
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
    let _syntax_tree = match parser::parse(source_code_ref) {
      Ok(tree) => {
        println!("source code ok!");
        self.client.publish_diagnostics(uri, vec![], None).await;
        tree
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
        node
      } else {
        return;
      },
    };
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
    Ok(Some(CompletionResponse::Array(vec![
      CompletionItem::new_simple("Hello".to_string(), "Some detail".to_string()),
      CompletionItem::new_simple("Bye".to_string(), "More detail".to_string())
    ])))
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
        let (service, client_sk) = LspService::new(|client| MechLangBackend { client });
        let (conn_sk_in, conn_sk_out) = conn_sk.into_split();
        Server::new(conn_sk_in, conn_sk_out, client_sk).serve(service).await;
      },
      Err(e) => println!("couldn't get client: {:?}", e),
    }
  }

  Ok(())
}

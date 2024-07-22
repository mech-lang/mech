mod backend;

use tokio::net::TcpListener;

use mech_core::{MechError, MechErrorKind};
use tower_lsp::{LspService, Server};

use backend::Backend;

pub async fn run_langserver(addr: &str, port: &str) -> Result<(), MechError> {
  let addr_port = format!("{}:{}", addr, port);

  // bind to address and port
  let listener = match TcpListener::bind(&addr_port).await {
    Ok(listener) => listener,
    Err(_) => return Err(MechError{tokens: vec![], msg: "".to_string(), id: 8923, kind: MechErrorKind::GenericError(format!("Unable to bind to {}", addr_port)),}),
  };

  // accept one client, then quit
  match listener.accept().await {
    Ok((conn_sk, client_addr)) => {
      let (service, client_sk) = LspService::new(|client| Backend::new(client));
      let (conn_sk_in, conn_sk_out) = conn_sk.into_split();
      Server::new(conn_sk_in, conn_sk_out, client_sk).serve(service).await;
      Ok(())
    },
    Err(e) => Err(MechError{tokens: vec![], msg: "".to_string(), id: 8924, kind: MechErrorKind::GenericError(format!("Unable to get client: {}", e))}),
  }
}

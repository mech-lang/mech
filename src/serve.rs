use warp::http::header::{HeaderMap, HeaderValue};
use warp::{Filter, Future, Reply};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use base64::{encode, decode};
use chrono::Local;
use mech_core::*;
use crate::*;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
    
const VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct MechServer {
  badge: ColoredString,
  init: bool,
  stylesheet: String,
  html_shim: String,
  full_address: String,
  sources: Arc<RwLock<HashMap<String, MechSourceCode>>>,
  js: Vec<u8>,
  wasm: Vec<u8>,
}

impl MechServer {

  pub fn new(name: String, full_address: String, stylesheet: String, html_shim: String, wasm: Vec<u8>, js: Vec<u8>) -> Self {
    Self {
      badge: format!("[{}] Server", name).truecolor(34, 204, 187),
      init: false,
      stylesheet,
      html_shim,
      js,
      wasm,
      full_address: full_address,
      sources: Arc::new(RwLock::new(HashMap::new())),
    }
  }

  pub async fn init(&mut self) -> MResult<()> {
    let mut sources = self.sources.write().unwrap();
    if !self.html_shim.is_empty() {
      sources.insert("index.html".to_string(), MechSourceCode::Html(self.html_shim.clone()));
    }
    if !self.stylesheet.is_empty() {
      sources.insert("style.css".to_string(), MechSourceCode::String(self.stylesheet.clone()));
    }
    self.init = true;
    Ok(())
  }

  pub fn load_sources(&mut self, paths: &Vec<String>) -> MResult<()> {
    for path in paths {
      let source = std::fs::read_to_string(path)?;
      let key = std::path::Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.clone());
      let code = match std::path::Path::new(path).extension().and_then(|ext| ext.to_str()) {
        Some("html") => MechSourceCode::Html(source),
        _ => MechSourceCode::String(source),
      };
      self.sources.write().unwrap().insert(key, code);
    }
    Ok(())
  }

  fn choose_bytes_or_path<'a>( &'a self, user_path: &'a str, embedded: &'a [u8], backup_url: &'a str) -> Source<'a> {
    if !user_path.is_empty() {
      Source::UserFile(user_path)
    } else if !embedded.is_empty() {
      Source::Embedded(embedded)
    } else {
      Source::Url(backup_url)
    }
  }

  pub async fn serve(&self) -> MResult<()> {
    if !self.init {
      return Err(MechError::new(ServerNotInitializedError, None).with_compiler_loc());
    }
      
    let server_badge = || {"[Mech Server]".truecolor(34, 204, 187)};
    ctrlc::set_handler(move || {
      println!("{} Server received shutdown signal. Process terminating.", server_badge());
      std::process::exit(0);
    }).expect("Error setting Ctrl-C handler");


    let code_source = self.sources.clone();

    let index = warp::get()
      .and(warp::filters::addr::remote())
      .and(warp::path::full())
      .map(move |remote: Option<SocketAddr>, path: warp::path::FullPath| {
        let date = Local::now();
        let url = path.as_str().strip_prefix("/").unwrap_or("");
        let content_type = match std::path::Path::new(path.as_str()).extension().and_then(|e| e.to_str()) {
          Some("html") | Some("mec") => "text/html",
          Some("css") => "text/css",
          Some("js") => "application/javascript",
          _ => "text/html",
        };

        match code_source.read() {
          Ok(sources) => {
            if url.starts_with("code/") {
              let url = url.strip_prefix("code/").unwrap();

              // If it's code, serve it
              match sources.get(url) {
                Some(MechSourceCode::Tree(tree)) => {
                  let tree: Program = tree.clone();
                  #[cfg(feature = "serde")]
                  match compress_and_encode(&tree) {
                    Ok(encoded) => {
                      return warp::reply::with_header(encoded, "content-type", "text/plain").into_response();
                    }
                    Err(e) => {
                      todo!("{} Error compressing and encoding tree: {}", server_badge(), e);
                    }
                  }
                  #[cfg(not(feature = "serde"))]
                  {
                    // return an error if serde feature is not enabled
                    return warp::reply::with_status(
                      warp::reply::with_header("Serde feature is not enabled", "content-type", "text/plain"),
                      warp::http::StatusCode::NOT_IMPLEMENTED,
                    ).into_response();
                  }
                }
                None => {
                  let mech_html = format!(
                    "<html><head><title>404 Not Found</title></head>\
                    <body><h1>404 Not Found</h1>\
                    <p>The requested URL {} was not found on this server.</p></body></html>",
                    url
                  );
                  return warp::reply::with_status(
                    warp::reply::with_header(mech_html, "content-type", "text/html"),
                    warp::http::StatusCode::NOT_FOUND,
                  )
                  .into_response();
                }
                _ => {
                  return warp::reply::with_status(
                    warp::reply::with_header("Invalid code source".to_string(), "content-type", "text/plain"),
                    warp::http::StatusCode::BAD_REQUEST,
                  )
                  .into_response();
                }
              }
            // serve images from images folder
            } else if url.starts_with("images/") {
              match sources.get(url) {
                Some(MechSourceCode::Image(extension, img_data)) => {
                  let content_type = match extension.as_str() {
                    "png" => "image/png",
                    "jpg" | "jpeg" => "image/jpeg",
                    "gif" => "image/gif",
                    "svg" => "image/svg+xml",
                    _ => "application/octet-stream",
                  };
                  let response = warp::reply::with_header(img_data.clone(), "content-type", content_type).into_response();
                  println!(
                    "{} Response generated with status: {} and content-type: image/png",
                    server_badge(),
                    response.status()
                  );
                  return response;
                }
                _ => {
                  let mech_html = format!(
                    "<html><head><title>404 Not Found</title></head>\
                    <body><h1>404 Not Found</h1>\
                    <p>The requested URL {} was not found on this server.</p></body></html>",
                    url
                  );
                  return warp::reply::with_status(
                    warp::reply::with_header(mech_html, "content-type", "text/html"),
                    warp::http::StatusCode::NOT_FOUND,
                  )
                  .into_response();
                }
              }
            }

            if let Some(addr) = remote {
              println!(
                "{} {} -- New request: {} -- /{}",
                server_badge(),
                date.format("%Y-%m-%d %H:%M:%S"),
                addr,
                url
              );
            } else {
              println!(
                "{} {} -- New request from unknown address -- /{}",
                server_badge(),
                date.format("%Y-%m-%d %H:%M:%S"),
                url
              );
            }

            let mech_html = match sources.get(url) {
              Some(MechSourceCode::Html(source)) => source.clone(),
              _ => {
              let mech_html = format!(
                "<html><head><title>404 Not Found</title></head>\
                <body><h1>404 Not Found</h1>\
                <p>The requested URL {} was not found on this server.</p></body></html>",
                url
              );
              let response = warp::reply::with_status(
                warp::reply::with_header(mech_html.clone(), "content-type", "text/html"),
                warp::http::StatusCode::NOT_FOUND,
              )
              .into_response();
              println!(
                "{} Response generated with status: {} and content-type: text/html",
                server_badge(),
                response.status()
              );
              return response;
              }
            };

            let response = warp::reply::with_header(mech_html, "content-type", content_type).into_response();
            println!(
              "{} Response generated with status: {} and content-type: {}",
              server_badge(),
              response.status(),
              content_type
            );
            return response;
          }
          Err(e) => {
            println!("{} Error writing sources: {}", server_badge(), e);
            todo!();
          }
        }
      });


    // Serve the JS file which includes the wasm
    let mech_js: Vec<u8> = self.js.clone();
    let mut headers = HeaderMap::new();
    headers.insert("accept-ranges", HeaderValue::from_static("bytes"));
    headers.insert("content-type", HeaderValue::from_static("application/javascript"));
    let nb = warp::path!("pkg" / "mech_wasm.js")
              .map(move || {
                mech_js.clone()
              }).with(warp::reply::with::headers(headers));

    // Serve the wasm. This file is large so it's brotli compressed.
    let mech_wasm = self.wasm.clone();
    let mut headers = HeaderMap::new();
    headers.insert("accept-ranges", HeaderValue::from_static("bytes"));
    headers.insert("content-type", HeaderValue::from_static("application/wasm"));
    headers.insert("content-encoding", HeaderValue::from_static("br"));
    let pkg = warp::path!("pkg" / "mech_wasm_bg.wasm")
              .map(move || {
                mech_wasm.clone()
              })
              .with(warp::reply::with::headers(headers)); 

    let routes = nb.or(pkg).or(index);

    println!("{} Awaiting connections at {}", server_badge(), self.full_address);
    let socket_address: SocketAddr = self.full_address.parse().unwrap();
    warp::serve(routes).run(socket_address).await;
    
    println!("{} Closing server.", server_badge());
    std::process::exit(0);
  }

}

#[derive(Debug, Clone)]
pub struct ServerNotInitializedError;
impl MechErrorKind for ServerNotInitializedError {
  fn name(&self) -> &str { "ServerNotInitializedError" }

  fn message(&self) -> String {
    format!("The server is not initialized.")
  }
}

#[derive(Debug, Clone)]
pub struct Utf8ConversionError {
  pub source_error: String
}
impl MechErrorKind for Utf8ConversionError {
  fn name(&self) -> &str {
    "Utf8ConversionError"
  }
  fn message(&self) -> String {
    format!("Failed to convert bytes into UTF-8 string: {}", self.source_error)
  }
}

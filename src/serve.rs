use warp::http::header::{HeaderMap, HeaderValue};
use warp::{Filter, Future, Reply};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use base64::{encode, decode};
use chrono::Local;
use mech_core::*;
use crate::*;
    
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(has_file_wasm)]
static MECHWASM: &[u8] = include_bytes!("../src/wasm/pkg/mech_wasm_bg.wasm.br");
#[cfg(not(has_file_wasm))]
static MECHWASM: &[u8] = b"";

#[cfg(has_file_js)]
static MECHJS: &[u8] = include_bytes!("../src/wasm/pkg/mech_wasm.js");
#[cfg(not(has_file_js))]
static MECHJS: &[u8] = b"";

#[cfg(has_file_shim)]
static SHIMHTML: &str = include_str!("../include/index.html");
#[cfg(not(has_file_shim))]
static SHIMHTML: &str = "123";

#[cfg(has_file_stylesheet)]
static STYLESHEET: &str = include_str!("../include/style.css");
#[cfg(not(has_file_stylesheet))]
static STYLESHEET: &str = "";

pub enum Source<'a> {
  UserFile(&'a str),
  Embedded(&'a [u8]),
  Url(&'a str),
}

pub struct MechServer {
  badge: ColoredString,
  init: bool,
  stylesheet_path: (String, String),
  shim_path: (String, String),
  wasm_path: (String, String),
  js_path: (String, String),
  full_address: String,
  mechfs: MechFileSystem,
  js: Vec<u8>,
  wasm: Vec<u8>,
}

impl MechServer {

  pub fn new(full_address: String, stylesheet_path: String, shim_path: String, wasm_pkg: String) -> Self {
    let shim_backup_url = "https://raw.githubusercontent.com/mech-lang/mech/refs/heads/main/include/shim.html".to_string();
    let stylesheet_backup_url = "https://raw.githubusercontent.com/mech-lang/mech/refs/heads/main/include/style.css".to_string();
    let wasm_backup_url = format!("https://github.com/mech-lang/mech/releases/download/v{}-beta/mech_wasm_bg.wasm.br", VERSION);
    let js_backup_url = format!("https://github.com/mech-lang/mech/releases/download/v{}-beta/mech_wasm.js", VERSION);

    let wasm_path = format!("{}/mech_wasm_bg.wasm.br", wasm_pkg);
    let js_path = format!("{}/mech_wasm.js", wasm_pkg);

    let mut mechfs = MechFileSystem::new();

    Self {
      badge: "[Mech Server]".truecolor(34, 204, 187),
      init: false,
      stylesheet_path: (stylesheet_path, stylesheet_backup_url),
      shim_path: (shim_path, shim_backup_url),
      wasm_path: (wasm_path, wasm_backup_url),
      js_path: (js_path, js_backup_url),
      full_address: full_address,
      mechfs,
      js: vec![],
      wasm: vec![],
    }
  }

  pub async fn init(&mut self) -> MResult<()> {
    let (stylesheet_path, stylesheet_backup_url) = &self.stylesheet_path;
    let (wasm_path, wasm_backup_url) = &self.wasm_path;
    let (js_path, js_backup_url) = &self.js_path;
    let (shim_path, shim_backup_url) = &self.shim_path;

    // Load stylesheet
    println!("{} Loading resources...", self.badge);
    print!("{} Loading stylesheet...", self.badge);
    let stylesheet = self
        .read_or_download(stylesheet_path, stylesheet_backup_url, Some(STYLESHEET.as_bytes()))
        .await?;
    let stylesheet_str = String::from_utf8(stylesheet)
        .map_err(|e| MechError2::new(Utf8ConversionError { source_error: e.to_string() }, None).with_compiler_loc())?;
    self.mechfs.set_stylesheet(&stylesheet_str);

    // Load shim HTML
    print!("{} Loading HTML shim...", self.badge);
    let shim = self
        .read_or_download(shim_path, shim_backup_url, Some(SHIMHTML.as_bytes()))
        .await?;
    let shim_str = String::from_utf8(shim)
        .map_err(|e| MechError2::new(Utf8ConversionError { source_error: e.to_string() }, None).with_compiler_loc())?;
    self.mechfs.set_shim(&shim_str);

    // WASM (supports user -> embedded -> download)
    print!("{} Loading WASM...", self.badge);
    let wasm = self
        .read_or_download(wasm_path, wasm_backup_url, Some(MECHWASM))
        .await?;
    self.wasm = wasm;

    // JS shim (supports user -> embedded -> download)
    print!("{} Loading JS...", self.badge);
    let js = self
        .read_or_download(js_path, js_backup_url, Some(MECHJS))
        .await?;
    self.js = js;

    self.init = true;
    Ok(())
  }

  pub fn load_sources(&mut self, paths: &Vec<String>) -> MResult<()> {
    for path in paths {
      self.mechfs.watch_source(&path)?;
    }
    Ok(())
  }

  fn choose_bytes_or_path<'a>(
      &'a self,
      user_path: &'a str,
      embedded: &'a [u8],
      backup_url: &'a str,
  ) -> Source<'a> {
      if !user_path.is_empty() {
          Source::UserFile(user_path)
      } else if !embedded.is_empty() {
          Source::Embedded(embedded)
      } else {
          Source::Url(backup_url)
      }
  }

  pub async fn read_or_download(&self,path: &str,backup_url: &str, embedded: Option<&[u8]>) -> MResult<Vec<u8>> {

    // 1. User-supplied path always wins
    match std::fs::read(path) {
      Ok(content) => {
        println!("Using user-supplied resource: {}", path);
        return Ok(content);
      }
      Err(_) => { /* continue to embedded / download */ }
    }

    // 2. Embedded bytes (included via include_bytes!)
    if let Some(bytes) = embedded {
      if !bytes.is_empty() {
        println!("Using embedded resource");
        return Ok(bytes.to_vec());
      }
    }

    // 3. Fallback: Download from remote URL
    println!("Downloading from {}", backup_url);

    let response = reqwest::get(backup_url).await.map_err(|e| {
      MechError2::new(
        HttpRequestFailed {
          url: backup_url.to_string(),
          source: e.to_string(),
        },
        None,
      )
      .with_compiler_loc()
    })?;

    if !response.status().is_success() {
      return Err(MechError2::new(
        HttpRequestStatusFailed {
          url: backup_url.to_string(),
          status_code: response.status().as_u16(),
        },
        None,
      )
      .with_compiler_loc());
    }

    let bytes = response.bytes().await.map_err(|e| {
      MechError2::new(
        HttpRequestFailed {
          url: backup_url.to_string(),
          source: e.to_string(),
        },
        None,
      )
      .with_compiler_loc()
    })?;

    Ok(bytes.to_vec())
  }

  pub async fn serve(&self) -> MResult<()> {
    if !self.init {
      return Err(MechError2::new(ServerNotInitializedError, None).with_compiler_loc());
    }
      
    let server_badge = || {"[Mech Server]".truecolor(34, 204, 187)};
    ctrlc::set_handler(move || {
      println!("{} Server received shutdown signal. Process terminating.", server_badge());
      std::process::exit(0);
    }).expect("Error setting Ctrl-C handler");


    let code_source = self.mechfs.sources();

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
              match sources.get_tree(url) {
                Some(tree) => {
                  let tree: Program = if let MechSourceCode::Tree(tree) = tree {
                    tree
                  } else {
                    todo!("{} Error getting tree from sources", server_badge());
                  };
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
              }
            // serve images from images folder
            } else if url.starts_with("images/") {
              match sources.get_image(url) {
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

            let mech_html = match sources.get_html(url) {
              Some(MechSourceCode::Html(source)) => source,
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
impl MechErrorKind2 for ServerNotInitializedError {
  fn name(&self) -> &str { "ServerNotInitializedError" }

  fn message(&self) -> String {
    format!("The server is not initialized.")
  }
}

#[derive(Debug, Clone)]
pub struct Utf8ConversionError {
  pub source_error: String
}
impl MechErrorKind2 for Utf8ConversionError {
  fn name(&self) -> &str {
    "Utf8ConversionError"
  }
  fn message(&self) -> String {
    format!("Failed to convert bytes into UTF-8 string: {}", self.source_error)
  }
}
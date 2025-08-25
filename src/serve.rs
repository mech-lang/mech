use warp::http::header::{HeaderMap, HeaderValue};
use warp::{Filter, Future, Reply};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use base64::{encode, decode};
use chrono::Local;
use mech_core::*;
use crate::*;
    
const VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct MechServer {
  badge: ColoredString,
  init: bool,
  stylesheet_path: (String, String),
  wasm_path: (String, String),
  js_path: (String, String),
  full_address: String,
  mechfs: MechFileSystem,
  js: Vec<u8>,
  wasm: Vec<u8>,
}

impl MechServer {

  pub fn new(full_address: String, stylesheet_path: String, wasm_pkg: String) -> Self {
    let stylesheet_backup_url = "https://raw.githubusercontent.com/mech-lang/mech/refs/heads/main/include/style.css".to_string();
    let wasm_backup_url = format!("https://github.com/mech-lang/mech/releases/download/v{}-beta/mech_wasm_bg.wasm", VERSION);
    let js_backup_url = format!("https://github.com/mech-lang/mech/releases/download/v{}-beta/mech_wasm.js", VERSION);

    let wasm_path = format!("{}/mech_wasm_bg.wasm", wasm_pkg);
    let js_path = format!("{}/mech_wasm.js", wasm_pkg);

    let mut mechfs = MechFileSystem::new();

    Self {
      badge: "[Mech Server]".truecolor(34, 204, 187),
      init: false,
      stylesheet_path: (stylesheet_path, stylesheet_backup_url),
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

    let stylesheet = self.read_or_download(stylesheet_path, stylesheet_backup_url).await?;
    match String::from_utf8(stylesheet) {
      Ok(s) => {
        println!("{} Loaded stylesheet", self.badge);
        self.mechfs.set_stylesheet(&s);
      },
      Err(e) => {
        let msg = format!("Failed to convert stylesheet to string: {}", e);
        return Err(MechError{file: file!().to_string(), tokens: vec![], msg, id: line!(), kind: MechErrorKind::None});
      }
    }

    let wasm = self.read_or_download(wasm_path, wasm_backup_url).await?;
    let js = self.read_or_download(js_path, js_backup_url).await?;

    self.wasm = wasm;
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

  pub async fn read_or_download(&self, path: &str, backup_url: &str) -> MResult<Vec<u8>> {
    match fs::read(path) {
      Ok(content) => Ok(content),
      Err(_) => {
        match reqwest::get(backup_url).await {
          Ok(response) => {
            if response.status().is_success() {
              match response.bytes().await {
                Ok(bytes) => Ok(bytes.to_vec()),
                Err(e) => {
                  let msg = format!("Failed to download file: {}", e);
                  Err(MechError{file: file!().to_string(), tokens: vec![], msg, id: line!(), kind: MechErrorKind::None})
                }
              }
            } else {
              let msg = format!("Failed to download file from URL: {} (status: {}).", backup_url, response.status());
              Err(MechError{file: file!().to_string(), tokens: vec![], msg, id: line!(), kind: MechErrorKind::None})
            }
          },
          Err(e) => {
            let msg = format!("Failed to download file: {}", e);
            Err(MechError{file: file!().to_string(), tokens: vec![], msg, id: line!(), kind: MechErrorKind::None})
          }
        }
      }
    }
  }

  pub async fn serve(&self) -> MResult<()> {
    if !self.init {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "Server not initialized".to_string(), id: line!(), kind: MechErrorKind::None});
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
                return warp::reply::with_status(
                  warp::reply::with_header(mech_html, "content-type", "text/html"),
                  warp::http::StatusCode::NOT_FOUND,
                )
                .into_response();
              }
            };
            return warp::reply::with_header(mech_html, "content-type", content_type).into_response();
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

    // Serve the wasm. This file is large so it's gzipped
    let mech_wasm = self.wasm.clone();
    let mut headers = HeaderMap::new();
    headers.insert("accept-ranges", HeaderValue::from_static("bytes"));
    headers.insert("content-type", HeaderValue::from_static("application/wasm"));
    let pkg = warp::path!("pkg" / "mech_wasm_bg.wasm")
              .map(move || {
                mech_wasm.clone()
              })
              //.with(warp::compression::gzip())
              .with(warp::reply::with::headers(headers)); 

    let routes = nb.or(pkg).or(index);

    println!("{} Awaiting connections at {}", server_badge(), self.full_address);
    let socket_address: SocketAddr = self.full_address.parse().unwrap();
    warp::serve(routes).run(socket_address).await;
    
    println!("{} Closing server.", server_badge());
    std::process::exit(0);
  }

}
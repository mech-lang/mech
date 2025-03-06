use warp::http::header::{HeaderMap, HeaderValue};
use warp::Filter;
use warp::Future;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use base64::{encode, decode};
use chrono::Local;
use mech_core::*;
use crate::*;
    
pub async fn serve_mech(full_address: &str, mech_paths: &Vec<String>) {
    
  let server_badge = || {"[Mech Server]".truecolor(34, 204, 187)};
  ctrlc::set_handler(move || {
    println!("{} Server received shutdown signal. Process terminating.", server_badge());
    std::process::exit(0);
  }).expect("Error setting Ctrl-C handler");

  let stylesheet: String = fs::read_to_string("include/style.css").unwrap();
  let ssclone = stylesheet.clone();
  let mech_wasm: Vec<u8> = fs::read("src/wasm/pkg/mech_wasm_bg.wasm").unwrap();
  let mech_js: Vec<u8> = fs::read("src/wasm/pkg/mech_wasm.js").unwrap();

  let mut mechfs = MechFileSystem::new();
  for path in mech_paths {
    mechfs.watch_source(path);
  }

  let index_source = mechfs.sources();
  let any_source = mechfs.sources();

  // Serve the HTML file which includes the JS
  let mut headers = HeaderMap::new();
  headers.insert("content-type", HeaderValue::from_static("text/html"));
  let index = warp::get()
    .and(warp::filters::addr::remote()) // Capture remote address
    .and(warp::path::full())            // Capture full path
    .map(move |remote: Option<SocketAddr>, path: warp::path::FullPath| {  
      let date = Local::now();
      // strip leading "/" from path
      let url = path.as_str().strip_prefix("/").unwrap_or("");
      
      if let Some(addr) = remote {
        println!("{} {} -- New connection from: {} -- {}", server_badge(), date.format("%Y-%m-%d %H:%M:%S"), addr, url);
      } else {
        println!("{} {} -- New connection from unknown address -- {}", server_badge(), date.format("%Y-%m-%d %H:%M:%S"), url);
      }
      match index_source.read() {
        Ok(sources) => {
          // search for a document named index.mec, index.html. If not found return a default page.
          let source = match sources.get_source(url) {
            Some(source) => source,
            None => {
              let response_html = format!(
                "Not Found: {}", url);
              MechSourceCode::String(response_html)
            }
          };
            let tree = match source {
              MechSourceCode::String(source) => match parser::parse(&source) {
                Ok(tree) => tree,
                Err(_) => {
                let response_html = format!(
                  "<html>
                    <head><title>Parsing Error</title></head>
                    <body>
                      <p>Failed to parse source</p>
                    </body>
                  </html>");
                return warp::reply::with_header(response_html, "content-type", "text/html");
                }
              },
              _ => {
                let response_html = format!(
                "<html>
                  <head><title>Unsupported Format</title></head>
                  <body>
                    <p>Unsupported source format</p>
                  </body>
                </html>");
                return warp::reply::with_header(response_html, "content-type", "text/html");
              }
            };

          let mut formatter = Formatter::new();
          let formatted_mech = formatter.format_html(&tree,stylesheet.clone());
          let mech_html = Formatter::humanize_html(formatted_mech);
          return warp::reply::with_header(mech_html, "content-type", "text/html");
        },
        Err(e) => {
          println!("{} Error writing sources: {}", server_badge(), e);
          todo!();
        }
      }
    })
    .with(warp::reply::with::headers(headers));

  // Serve the JS file which includes the wasm
  let mut headers = HeaderMap::new();
  headers.insert("accept-ranges", HeaderValue::from_static("bytes"));
  headers.insert("content-type", HeaderValue::from_static("application/javascript"));
  let nb = warp::path!("pkg" / "mech_wasm.js")
            .map(move || {
              mech_js.clone()
            })
            .with(warp::reply::with::headers(headers));

  // Serve the wasm. This file is large so it's gzipped
  let mut headers = HeaderMap::new();
  headers.insert("accept-ranges", HeaderValue::from_static("bytes"));
  headers.insert("content-type", HeaderValue::from_static("application/wasm"));
  let pkg = warp::path!("pkg" / "mech_wasm_bg.wasm")
            .map(move || {
              mech_wasm.to_vec()
            })
            //.with(warp::compression::gzip())
            .with(warp::reply::with::headers(headers));
  
  // Serve the code. It's compressed and encoded to Base64.
  // We'll decode and decompress on the other side to a source tree.
  let code = warp::path("code")
              .and(warp::addr::remote())
              .map(move |addr: Option<SocketAddr>| {
                //let encoded = compress_and_encode(&tree2);
                //encoded
                "hello"
              });    

  let routes = pkg.or(nb).or(code).or(index);

  println!("{} Awaiting connections at {}", server_badge(), full_address);
  let socket_address: SocketAddr = full_address.parse().unwrap();
  warp::serve(routes).run(socket_address).await;
  
  println!("{} Closing server.", server_badge());
  std::process::exit(0);
  
}
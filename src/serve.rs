use warp::http::header::{HeaderMap, HeaderValue};
use warp::Filter;
use warp::Future;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use base64::{encode, decode};
use chrono::Local;
use mech_core::*;
use crate::*;
    
pub async fn serve_mech(full_address: &str, mech_paths: Vec<String>) {
    
    let server_badge = || {"[Mech Server]".truecolor(34, 204, 187)};
    ctrlc::set_handler(move || {
      println!("{} Server received shutdown signal. Process terminating.", server_badge());
      std::process::exit(0);
    }).expect("Error setting Ctrl-C handler");

    // read index.html from disc
    let mech_html: String = fs::read_to_string("src/wasm/index.html").unwrap();
    let mech_wasm: Vec<u8> = fs::read("src/wasm/pkg/mech_wasm_bg.wasm").unwrap();
    let mech_js: Vec<u8> = fs::read("src/wasm/pkg/mech_wasm.js").unwrap();

    let code = match read_mech_files(&mech_paths) {
      Ok(code) => code,
      Err(err) => {
        println!("{:?}", err);
        vec![]
      }
    };

    // Serve the HTML file which includes the JS
    let mut headers = HeaderMap::new();
    headers.insert("content-type", HeaderValue::from_static("text/html"));
    let index = warp::get()
        .and(warp::path::end())
        .and(warp::filters::addr::remote()) // Capture remote address
        .map(move |remote: Option<SocketAddr>| {
            let date = Local::now();
            if let Some(addr) = remote {
              println!("{} {} - New connection from: {}", server_badge(), date.format("%Y-%m-%d %H:%M:%S"), addr);
            } else {
              println!("{} {} - New connection from unknown address", server_badge(), date.format("%Y-%m-%d %H:%M:%S"));
            }
            mech_html.clone()
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
              .with(warp::reply::with::headers(headers));
    
    let code = warp::path("code")
                .and(warp::addr::remote())
                .map(move |addr: Option<SocketAddr>| {
                  let (file,source) = &code[0];
                  let resp = if let MechSourceCode::String(s) = source {
                    s.clone()
                  } else {
                    "".to_string()
                  };
                  resp
                });    

    let routes = index.or(pkg).or(nb).or(code);

    println!("{} Awaiting connections at {}", server_badge(), full_address);
    let socket_address: SocketAddr = full_address.parse().unwrap();
    warp::serve(routes).run(socket_address).await;
    
    println!("{} Closing server.", server_badge());
    std::process::exit(0);
}
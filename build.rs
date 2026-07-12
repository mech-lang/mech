use std::{error::Error, path::Path};
extern crate winres;

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo::rustc-check-cfg=cfg(has_file_wasm)");
    println!("cargo::rustc-check-cfg=cfg(has_file_js)");
    println!("cargo::rustc-check-cfg=cfg(has_file_shim)");
    println!("cargo::rustc-check-cfg=cfg(has_file_stylesheet)");

    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        res.set_icon("mech.ico");
        res.compile().unwrap();
    }

    println!("cargo::rerun-if-changed=src/wasm/pkg/mech_wasm_bg.wasm.br");
    if Path::new("src/wasm/pkg/mech_wasm_bg.wasm.br").exists() {
        println!("cargo:rustc-cfg=has_file_wasm");
    }

    println!("cargo::rerun-if-changed=src/wasm/pkg/mech_wasm.js");
    if Path::new("src/wasm/pkg/mech_wasm.js").exists() {
        println!("cargo:rustc-cfg=has_file_js");
    }

    println!("cargo::rerun-if-changed=include/index.html");
    if Path::new("include/index.html").exists() {
        println!("cargo:rustc-cfg=has_file_shim");
    }

    println!("cargo::rerun-if-changed=include/style.css");
    if Path::new("include/style.css").exists() {
        println!("cargo:rustc-cfg=has_file_stylesheet");
    }

    Ok(())
}

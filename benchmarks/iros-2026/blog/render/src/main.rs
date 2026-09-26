use std::{env, fs};
use mech_syntax::{parser, Formatter};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let paths: Vec<_> = env::args().skip(1).collect();
    if paths.len() != 3 {
        return Err("usage: mech-iros-blog-render SOURCE.mec SHIM.html OUTPUT.html".into());
    }
    let source = fs::read_to_string(&paths[0])?;
    let tree = parser::parse(source.trim()).map_err(|e| format!("{e:?}"))?;
    let shim = fs::read_to_string(&paths[1])?;
    let html = Formatter::new().format_html(&tree, String::new(), shim);
    fs::write(&paths[2], html)?;
    Ok(())
}

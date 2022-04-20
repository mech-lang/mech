use std::{
  env,
  error::Error,
  fs::{self, File},
  io::Write,
  path::Path,
};
extern crate winres;

const SOURCE_DIR: &str = r"project";

fn main() -> Result<(), Box<dyn Error>> {
  
  if cfg!(target_os = "windows") {
    let mut res = winres::WindowsResource::new();
    res.set_icon("mech.ico");
    res.compile().unwrap();
  }

  let out_dir = env::var("OUT_DIR")?;
  let dest_path = Path::new(&out_dir).join("mech_app_files.rs");
  let mut all_the_files = File::create(&dest_path)?;

  writeln!(&mut all_the_files, r##"["##,)?;

  for f in fs::read_dir(SOURCE_DIR)? {
      let f = f?;

      if !f.file_type()?.is_file() {
          continue;
      }

      writeln!(
          &mut all_the_files,
          r##"(r"{name}", include_bytes!(r"../../../../../{name}")),"##,
          name = f.path().display(),
      )?;
  }

  writeln!(&mut all_the_files, r##"]"##,)?;

  Ok(())
}
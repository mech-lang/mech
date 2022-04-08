use std::{
  env,
  error::Error,
  fs::{self, File},
  io::Write,
  path::Path,
};
extern crate winres;

const SOURCE_DIR: &str = r"demo";

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



/*extern crate winres;

use std::env;
use std::fs;
use std::path::Path;

fn main() {
  
  for arg in std::env::args() {
    println!("{:?}", arg);
  }

  if cfg!(target_os = "windows") {
    let mut res = winres::WindowsResource::new();
    res.set_icon("mech.ico");
    res.compile().unwrap();
  }
  let out_dir = env::var_os("OUT_DIR").unwrap();
  let dest_path = Path::new(&out_dir).join("hello.rs");
  fs::write(
      &dest_path,
r#"# Mech Demo

block
  i = 1:2000
  x = i / 2000 * 400
  y = i / 2000 * 400
  vx = x / 5
  vy = y / 5 
  #balls = [|x y vx vy radius|
              x y vx vy 2]
  #gravity = 0.1

Add a game timer
  #time/timer = [period: 16<ms> ticks: 0]      

## Motion Model

Move the ball with every tick of the timer
  ~ #time/timer.ticks
  #balls.x := #balls.x + #balls.vx
  #balls.y := #balls.y + #balls.vy
  #balls.vy := #balls.vy + #gravity

Keep the balls within the boundary height
  ~ #time/timer.ticks
  iy = #balls.y > 500
  iyy = #balls.y < 0
  #balls.y{iy} := 500
  #balls.y{iyy} := 0
  #balls.vy{iy | iyy} := #balls.vy * -0.80

Keep the balls within the boundary width
  ~ #time/timer.ticks
  ix = #balls.x > 500
  ixx = #balls.x < 0
  #balls.x{ix} := 500
  #balls.x{ixx} := 0
  #balls.vx{ix | ixx} := #balls.vx * -0.80"#
  ).unwrap();
  println!("cargo:rerun-if-changed=build.rs");
}*/
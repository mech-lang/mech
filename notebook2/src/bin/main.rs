#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use eframe::{epi, egui};
use eframe::egui::{containers::*, *};
extern crate mech;

use mech::program::*;
use mech::utilities::*;
use mech::core::*;
use mech::core as mech_core;
use mech::Compiler;
use std::thread::JoinHandle;

struct MechApp {
  //mech_client: RunLoop,
  ticks: f32,
  mech: mech_core::Core,
  maestro_thread: Option<JoinHandle<()>>,
}

impl Default for MechApp {
    fn default() -> Self {
      /*let runner = ProgramRunner::new("Mech Run");
      let mech_client = runner.run().unwrap();
      let mcc = mech_client.outgoing.clone();
      
      let maestro_thread = match mech_client.socket_address {
        Some(ref mech_socket_address) => {
          Some(mech::start_maestro(mech_socket_address.to_string(), "127.0.0.1:0".to_string(), "127.0.0.1:3235".to_string(), "127.0.0.1:3235".to_string(), mcc).unwrap())
        }
        None => None,
      };
      mech_client.send(RunLoopMessage::Code(MechCode::String(r#"#x = #z + 1"#.to_string())));
      let thread_receiver = mech_client.incoming.clone();
      // Empty recv queue
      loop {
        match thread_receiver.recv().unwrap() {
          ClientMessage::Done => break,
          _ => (),
        }
      }*/
      let code = r#"
# Mech Demo

block
  i = 1:20000
  x = i / 20000 * 400
  y = i / 20000 * 400
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
  #balls.vx{ix | ixx} := #balls.vx * -0.80"#;

      let mut mech_core = mech_core::Core::new();
      let mut compiler = Compiler::new(); 
      match compiler.compile_str(&code) {
        Ok(blocks) => {
          mech_core.load_blocks(blocks);
        }
        Err(x) => {
          
        }
      }
      
      Self {
        ticks: 0.0,
        //mech_client,
        mech: mech_core,
        maestro_thread: None,
      }
    }
  }

impl epi::App for MechApp {

  fn name(&self) -> &str {
    "Mech Notebook"
  }

  fn update(&mut self, ctx: &egui::Context, frame: &epi::Frame) {
    let Self { ticks, mech, .. } = self;
    /*mech_client.send(RunLoopMessage::GetValue((hash_str("x"),TableIndex::Index(1),TableIndex::Index(1))));
    let thread_receiver = mech_client.incoming.clone();
    let mut values = vec![];
    let mut log = "".to_string();
    loop {
      match thread_receiver.recv().unwrap() {
        ClientMessage::Value(v) => values.push(v),
        ClientMessage::Done => break,
        x => {log=format!("{:?}\n{:?}",log,x)},
      }
    }
    if values.len() > 0 {
      match &values[0] {
        Value::U64(s) => {
          *ticks = (*s).into();
        }
        Value::F32(s) => {
          *ticks = (*s).into();
        }
        _ => (),
      } 
    }*/

    egui::CentralPanel::default().show(ctx, |ui| {

      let color = Color32::from_additive_luminance(196);

      Frame::none().show(ui, |ui| {
        ui.ctx().request_repaint();
        let time = ui.input().time;
        let table = self.mech.get_table("balls").unwrap();
        let table_brrw = table.borrow();
        let x = table_brrw.get_column_unchecked(0);
        let y = table_brrw.get_column_unchecked(1);
        let r = table_brrw.get_column_unchecked(4);


        let desired_size = ui.available_width() * vec2(1.0, 0.35);
        let (_id, rect) = ui.allocate_space(desired_size);

        //let to_screen = emath::RectTransform::from_to(Rect::from_x_y_ranges(0.0..=500.0, 0.0..=500.0), rect);

        let mut shapes = vec![];

        match (x,y,r) {
          (Column::F32(x), Column::F32(y), Column::F32(r)) => {
            let x_brrw = x.borrow();
            let y_brrw = y.borrow();
            let r_brrw = r.borrow();
            for i in 0..table_brrw.rows {
              shapes.push(epaint::Shape::Circle(epaint::CircleShape{
                center: Pos2{x: x_brrw[i].into(), y: y_brrw[i].into()},
                radius: r_brrw[i].into(),
                fill: color,
                stroke: epaint::Stroke::new(1.0,color),
              }));
            }
          }
          _ => (),
        }



        ui.painter().extend(shapes);
        let change = Change::Set((hash_str("time/timer"),vec![(TableIndex::Index(1),TableIndex::Index(2),Value::U64(U64::new(time as u64)))]));
        self.mech.process_transaction(&vec![change]);
      });
      let table = self.mech.get_table("balls");
      //ui.heading(format!("{:?}", table));
    });
  }
}

fn main() {
    let app = MechApp::default();
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(app), native_options);
}


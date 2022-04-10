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

static LONG_STRING: &'static str = include_str!(concat!(env!("OUT_DIR"), "/hello.rs"));

impl MechApp {
  pub fn new(input: String) -> Self {
    let code = LONG_STRING;

    let mut mech_core = mech_core::Core::new();
    let mut compiler = Compiler::new(); 
    match compiler.compile_str("test2.mec") {
      Ok(blocks) => {
        mech_core.load_blocks(blocks);
      }
      Err(x) => {
        
      }
    }
    
    Self {
      ticks: 0.0,
      //mech_client,
      core: mech_core,
      maestro_thread: None,
    }
  }

  pub fn add_apps(&mut self) -> Result<(), JsValue> {
    match self.core.get_table("mech/app") {
      Ok(app_table) => {        
        let app_table_brrw = app_table.borrow();
        for row in 1..=app_table_brrw.rows as usize {
          match (app_table_brrw.get(&TableIndex::Index(row), &TableIndex::Alias(*ROOT)), 
                 app_table_brrw.get(&TableIndex::Index(row), &TableIndex::Alias(*CONTAINS))) {
            (Ok(Value::String(root)), Ok(contents)) => {
              let root_id = root.hash();

            }
            x => log!("4846 {:?}",x),
          }  
        }
      }
      x => {
        log!("4847 {:?}",x);
      },
    }
    Ok(())
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
        let table = self.core.get_table("balls").unwrap();
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
        self.core.process_transaction(&vec![change]);
      });
      let table = self.core.get_table("balls");
      //ui.heading(format!("{:?}", table));
    });
  }
}


fn main() {
    let input = std::env::args().nth(1).unwrap();
    let app = MechApp::new(input);
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(app), native_options);
}


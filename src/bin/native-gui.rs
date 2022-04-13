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

#[macro_use]
extern crate lazy_static;

lazy_static! {
  static ref CONTAINS: u64 = hash_str("contains");
  static ref PARAMETERS: u64 = hash_str("parameters");
  static ref HEIGHT: u64 = hash_str("height");
  static ref WIDTH: u64 = hash_str("width");
  static ref ROOT: u64 = hash_str("root");
  static ref KIND: u64 = hash_str("kind");
  static ref TEXT: u64 = hash_str("text");
  static ref URL: u64 = hash_str("url");
  static ref LINK: u64 = hash_str("link");
  static ref CANVAS: u64 = hash_str("link");
}

struct MechApp {
  //mech_client: RunLoop,
  ticks: f32,
  core: mech_core::Core,
  maestro_thread: Option<JoinHandle<()>>,
}

//static LONG_STRING: &'static str = include_str!(concat!(env!("OUT_DIR"), "/hello.rs"));

impl MechApp {
  pub fn new() -> Self {
    //let code = LONG_STRING;
    let code = include_str!("test2.mec");
    let mut mech_core = mech_core::Core::new();
    let mut compiler = Compiler::new(); 
    match compiler.compile_str(code) {
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
  
  pub fn render_app(&mut self, ui: &mut egui::Ui) -> Result<(), MechError> {
    match self.core.get_table("mech/app") {
      Ok(app_table) => {        
        let app_table_brrw = app_table.borrow();
        ui.columns(app_table_brrw.cols, |cols| {
          for (col, col_ui) in cols.iter_mut().enumerate() {
            for row in 1..=app_table_brrw.rows as usize {
              match app_table_brrw.get(&TableIndex::Index(row), &TableIndex::Index(col+1)) {
                Ok(contents) => {
                  self.render_value(contents, col_ui);
                }
                x => {return Err(MechError{id: 6486, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
              }
            }
          }
          Ok(())
        });
      }
      x => {return Err(MechError{id: 6487, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
    }
    Ok(())
  }

  fn render_value(&mut self, value: Value, ui: &mut egui::Ui) -> Result<(), MechError> {
    match value {
      Value::String(chars) => {
        let contents_string = chars.to_string();
        ui.label(&contents_string);
      },
      Value::F32(x) => {ui.label(&format!("{:.2?}", x));},
      Value::F64(x) => {ui.label(&format!("{:?}", x));},
      Value::U128(x) => {ui.label(&format!("{:?}", x));},
      Value::U64(x) => {ui.label(&format!("{:?}", x));},
      Value::U32(x) => {ui.label(&format!("{:?}", x));},
      Value::U16(x) => {ui.label(&format!("{:?}", x));},
      Value::U8(x) => {ui.label(&format!("{:?}", x));},
      Value::I128(x) => {ui.label(&format!("{:?}", x));},
      Value::I64(x) => {ui.label(&format!("{:?}", x));},
      Value::I32(x) => {ui.label(&format!("{:?}", x));},
      Value::I16(x) => {ui.label(&format!("{:?}", x));},
      Value::I8(x) => {ui.label(&format!("{:?}", x));},
      Value::Reference(TableId::Global(table_id)) => {
        let table = self.core.get_table_by_id(table_id).unwrap();
        ui.group(|ui| {
          self.make_element(&table.borrow(), ui);  
        });
        //div.append_child(&rendered_ref)?;
      }
      x => {return Err(MechError{id: 6488, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
    }
    Ok(())
  }

  fn make_element(&mut self, table: &Table, ui: &mut egui::Ui) -> Result<(), MechError> {
    match table.col_map.get_index(&*KIND) {
      Ok(_) => {
        for row in 1..=table.rows {
          match table.get(&TableIndex::Index(row), &TableIndex::Alias(*KIND))  {
            Ok(Value::String(kind)) => {
              let raw_kind = kind.hash();
              // Render an element
              if raw_kind == *LINK { self.render_link(table,ui)?; }
              /*else if raw_kind == *A { render_link(table,&mut container,wasm_core)?; }
              else if raw_kind == *IMG { render_img(table,&mut container,wasm_core)?; }
              else if raw_kind == *BUTTON { render_button(table,&mut container,wasm_core)?; }
              else if raw_kind == *SLIDER { render_slider(table,&mut container,wasm_core)?; }
              */else if raw_kind == *CANVAS { self.render_canvas(table,ui)?; }
              else {
                return Err(MechError{id: 6489, kind: MechErrorKind::GenericError(format!("{:?}", raw_kind))});
              }
            }
            x => {return Err(MechError{id: 6488, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
            Err(x) => {return Err(MechError{id: 6488, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
          }
        }
      }
      // There's no Type column, so we are going to treat the table as a generic thing and just turn it into divs
      Err(_) => {
        ui.columns(table.cols, |cols| {
          for (col, col_ui) in cols.iter_mut().enumerate() {
            for row in 1..=table.rows as usize {
              match table.get(&TableIndex::Index(row), &TableIndex::Index(col+1)) {
                Ok(contents) => {
                  self.render_value(contents, col_ui);
                }
                x => {return Err(MechError{id: 6496, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
              }
            }
          }
          Ok(())
        });
      }
    }
    Ok(())
  }

  pub fn render_link(&mut self, table: &Table, container: &mut egui::Ui) -> Result<(),MechError> {
    for row in 1..=table.rows {
      match (table.get(&TableIndex::Index(row), &TableIndex::Alias(*TEXT)),
             table.get(&TableIndex::Index(row), &TableIndex::Alias(*URL))) {
        (Ok(Value::String(text)), Ok(Value::String(url))) => {
          container.hyperlink_to(text.to_string(),url.to_string());
        }
        x => {return Err(MechError{id: 6496, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
      }
    }
    Ok(())
  }

  pub fn render_canvas(&mut self, table: &Table, container: &mut egui::Ui) -> Result<(),MechError> {
    for row in 1..=table.rows {
      match table.get(&TableIndex::Index(row), &TableIndex::Alias(*CONTAINS)) {
        Ok(contents) => {
          // Is there a parameters field?
          match table.get(&TableIndex::Index(row), &TableIndex::Alias(*PARAMETERS)) {
            Ok(Value::Reference(parameters_table_id)) => {
              let parameters_table = self.core.get_table_by_id(*parameters_table_id.unwrap()).unwrap();
              let parameters_table_brrw = parameters_table.borrow();
              match (parameters_table_brrw.get(&TableIndex::Index(1), &TableIndex::Alias(*HEIGHT)),
                     parameters_table_brrw.get(&TableIndex::Index(1), &TableIndex::Alias(*WIDTH))) {
                (Ok(Value::F32(height)),Ok(Value::F32(width))) => {
                  //canvas.set_attribute("height", &format!("{:?}",height));
                  //canvas.set_attribute("width", &format!("{:?}",width));
                }
                x => {return Err(MechError{id: 6496, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
              }
            }
            x => {return Err(MechError{id: 6496, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
          }
          // Add the contents
          match table.get(&TableIndex::Index(row), &TableIndex::Alias(*CONTAINS)) {
            Ok(Value::Reference(contains_table_id)) => {
              //canvas.set_attribute("elements", &format!("{}",contains_table_id.unwrap()));
            }
            x => {return Err(MechError{id: 6496, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
          }
          //container.append_child(&canvas)?;
        }
        x => {return Err(MechError{id: 6496, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
      }
    }
    Ok(())
  }


}

impl epi::App for MechApp {

  fn name(&self) -> &str {
    "Mech Notebook"
  }

  fn update(&mut self, ctx: &egui::Context, frame: &epi::Frame) {
    let Self { ticks, core, .. } = self;

    egui::CentralPanel::default().show(ctx, |ui| {
      self.render_app(ui);
      /*let color = Color32::from_additive_luminance(196);

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
      });*/
      //let table = self.core.get_table("balls");
    });
  }
}


fn main() {
    //let input = std::env::args().nth(1).unwrap();
    let app = MechApp::new();
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(app), native_options);
}


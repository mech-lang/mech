#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![recursion_limit="256"]

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
  static ref LINK: u64 = hash_str("link");
  static ref IMG: u64 = hash_str("img");
  static ref SRC: u64 = hash_str("src");
  static ref CONTAINS: u64 = hash_str("contains");
  static ref KIND: u64 = hash_str("kind");
  static ref BUTTON: u64 = hash_str("button");
  static ref SLIDER: u64 = hash_str("slider");
  static ref MIN: u64 = hash_str("min");
  static ref MAX: u64 = hash_str("max");
  static ref VALUE: u64 = hash_str("value");
  static ref CANVAS: u64 = hash_str("canvas");
  static ref PARAMETERS: u64 = hash_str("parameters");
  static ref HEIGHT: u64 = hash_str("height");
  static ref WIDTH: u64 = hash_str("width");
  static ref SHAPE: u64 = hash_str("shape");
  static ref CIRCLE: u64 = hash_str("circle");
  static ref RECTANGLE: u64 = hash_str("rectangle");
  static ref LINE: u64 = hash_str("line");
  static ref PATH: u64 = hash_str("path");
  static ref START__POINT: u64 = hash_str("start-point");
  static ref LINE__WIDTH: u64 = hash_str("line-width");
  static ref START__ANGLE: u64 = hash_str("start-angle");
  static ref END__ANGLE: u64 = hash_str("end-angle");
  static ref QUADRATIC: u64 = hash_str("quadratic");
  static ref CONTROL__POINT: u64 = hash_str("control-point");
  static ref CONTROL__POINTS: u64 = hash_str("control-points");
  static ref END__POINT: u64 = hash_str("end-point");
  static ref X1: u64 = hash_str("x1");
  static ref X2: u64 = hash_str("x2");
  static ref Y1: u64 = hash_str("y1");
  static ref Y2: u64 = hash_str("y2");
  static ref RADIUS: u64 = hash_str("radius");
  static ref STROKE: u64 = hash_str("stroke");
  static ref FILL: u64 = hash_str("fill");
  static ref CENTER__X: u64 = hash_str("center-x");
  static ref CENTER__Y: u64 = hash_str("center-y");
  static ref IMAGE: u64 = hash_str("image");
  static ref X: u64 = hash_str("x");
  static ref Y: u64 = hash_str("y");
  static ref ROTATE: u64 = hash_str("rotate");
  static ref TRANSLATE: u64 = hash_str("translate");
  static ref SOURCE: u64 = hash_str("source");
  static ref TIME_TIMER: u64 = hash_str("time/timer");
  static ref PERIOD: u64 = hash_str("period");
  static ref TICKS: u64 = hash_str("ticks");
  static ref TARGET: u64 = hash_str("target");
  static ref KEY: u64 = hash_str("key");
  static ref EVENT__ID: u64 = hash_str("event-id");
  static ref ARC: u64 = hash_str("arc");
  static ref ELLIPSE: u64 = hash_str("ellipse");
  static ref MAJOR__AXIS: u64 = hash_str("major-axis");
  static ref MINOR__AXIS: u64 = hash_str("minor-axis");
  static ref STARTING__ANGLE: u64 = hash_str("starting-angle");
  static ref ENDING__ANGLE: u64 = hash_str("ending-angle");
  static ref TEXT: u64 = hash_str("text");
  static ref FONT: u64 = hash_str("font");
  static ref SIZE: u64 = hash_str("size");
  static ref FACE: u64 = hash_str("face");
  static ref STYLE: u64 = hash_str("style");
  static ref WEIGHT: u64 = hash_str("weight");
  static ref BOLD: u64 = hash_str("bold");
  static ref NORMAL: u64 = hash_str("normal");
  static ref ITALIC: u64 = hash_str("italic");
  static ref FAMILY: u64 = hash_str("family");
  static ref DIRECTION: u64 = hash_str("direction");
  static ref ALIGNMENT: u64 = hash_str("alignment");
  static ref START: u64 = hash_str("start");
  static ref END: u64 = hash_str("end");
  static ref LEFT: u64 = hash_str("left");
  static ref RIGHT: u64 = hash_str("right");
  static ref CENTER: u64 = hash_str("center");
  static ref BEZIER: u64 = hash_str("bezier");
  static ref URL: u64 = hash_str("url");
}

struct MechApp {
  //mech_client: RunLoop,
  ticks: f32,
  core: mech_core::Core,
  maestro_thread: Option<JoinHandle<()>>,
  shapes: Vec<epaint::Shape>,
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
    
    let code = r#"
timer  
  #time/timer = [|period<s> ticks<u64>|]"#;

    let mut compiler = Compiler::new();
    let blocks = compiler.compile_str(&code).unwrap();
    mech_core.load_blocks(blocks);

    let mut shapes = vec![epaint::Shape::Noop; 100000];

    Self {
      ticks: 0.0,
      //mech_client,
      core: mech_core,
      maestro_thread: None,
      shapes,
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
              else if raw_kind == *SLIDER { self.render_slider(table,ui)?; }
              /*else if raw_kind == *IMG { render_img(table,&mut container,wasm_core)?; }
              else if raw_kind == *BUTTON { render_button(table,&mut container,wasm_core)?; }
              */
              else if raw_kind == *CANVAS { 
                self.render_canvas(table,ui)?; 
              }
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
        Ok(Value::Reference(contains_table_id)) => {
          Frame::none().show(container, |ui| {
            let table = self.core.get_table_by_id(*contains_table_id.unwrap()).unwrap();
            let table_brrw = table.borrow();
            match (table_brrw.get(&TableIndex::Index(1), &TableIndex::Alias(*SHAPE)),
                    table_brrw.get(&TableIndex::Index(1), &TableIndex::Alias(*PARAMETERS)))  {
              (Ok(Value::String(kind)),Ok(Value::Reference(contains_table_id))) => {
                let table = self.core.get_table_by_id(*contains_table_id.unwrap()).unwrap();
                let table_brrw = table.borrow();
                let raw_kind = kind.hash();
                // Render an element
                if raw_kind == *CIRCLE { 
                  let shapes = self.render_circle(&table_brrw,ui)?;
                  ui.painter().extend(shapes);
                } else {
                  return Err(MechError{id: 6489, kind: MechErrorKind::GenericError(format!("{:?}", raw_kind))});
                }
              }
              x => {
                return Err(MechError{id: 6496, kind: MechErrorKind::GenericError(format!("{:?}", x))});
              },
            }
            Ok(())
          });
        }
        x => {return Err(MechError{id: 6496, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
      }
    }
    Ok(())
  }

  pub fn render_circle(&mut self, table: &Table, container: &mut egui::Ui) -> Result<Vec<epaint::Shape>,MechError> {
    let x = table.get_column_unchecked(0);
    let y = table.get_column_unchecked(1);
    let r = table.get_column_unchecked(2);

    let desired_size = container.available_width() * vec2(1.0, 1.00);
    let (_id, rect) = container.allocate_space(desired_size);

    //let to_screen = emath::RectTransform::from_to(Rect::from_x_y_ranges(0.0..=500.0, 0.0..=500.0), rect);
    let mut shapes = vec![];
    let color = Color32::from_additive_luminance(196);

    match (x,y,r) {
      (Column::F32(x), Column::F32(y), Column::F32(r)) => {
        let x_brrw = x.borrow();
        let y_brrw = y.borrow();
        let r_brrw = r.borrow();
        for i in 0..table.rows {
          shapes.push(epaint::Shape::Circle(epaint::CircleShape{
            center: Pos2{x: x_brrw[i].into(), y: y_brrw[i].into()},
            radius: r_brrw[i].into(),
            fill: color,
            stroke: epaint::Stroke::new(1.0,color),
          }));
        }
      }
      x => {return Err(MechError{id: 6496, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
    }
    Ok(shapes)
  }

  pub fn render_slider(&mut self, table: &Table, container: &mut egui::Ui) -> Result<(),MechError> {
    for row in 1..=table.rows {
      match (table.get(&TableIndex::Index(row), &TableIndex::Alias(*MIN)),
             table.get(&TableIndex::Index(row), &TableIndex::Alias(*MAX)),
             table.get(&TableIndex::Index(row), &TableIndex::Alias(*VALUE))) {
          (Ok(Value::F32(min)), Ok(Value::F32(max)), Ok(Value::Reference(value_table_id))) => {
          let value_table = self.core.get_table_by_id(*value_table_id.unwrap())?;
          let value_table_brrw = value_table.borrow();
          match value_table_brrw.get(&TableIndex::Index(1), &TableIndex::Index(1)) {
            Ok(Value::F32(value)) => {
              self.ticks = value.into();
              let response = container.add(egui::Slider::new(&mut self.ticks, min.into()..=max.into()));
              if response.changed() {
                let change = Change::Set((value_table_brrw.id,vec![(TableIndex::Index(1),TableIndex::Index(1),Value::F32(F32::new(self.ticks)))]));
                self.core.process_transaction(&vec![change]);
              }
            }
            x => {return Err(MechError{id: 6497, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
          }
        }
        x => {return Err(MechError{id: 6497, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
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
      ui.ctx().request_repaint();
      let desired_size = ui.available_width() * vec2(1.0, 0.35);
      let (_id, rect) = ui.allocate_space(desired_size);

      self.render_app(ui);

      let time = ui.input().time;
      let change = Change::Set((hash_str("time/timer"),vec![(TableIndex::Index(1),TableIndex::Index(2),Value::U64(U64::new(time as u64)))]));
      self.core.process_transaction(&vec![change]);
    });
  }
}


fn main() {
    //let input = std::env::args().nth(1).unwrap();
    let app = MechApp::new();
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(app), native_options);
}



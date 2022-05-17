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
extern crate image;
use std::path::Path;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;

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
  static ref MIN__WIDTH: u64 = hash_str("min-width");
  static ref MIN__HEIGHT: u64 = hash_str("min-height");
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
  static ref TEXT: u64 = hash_str("text");
  static ref URL: u64 = hash_str("url");
  static ref CODE: u64 = hash_str("code");
  static ref PANEL__TOP: u64 = hash_str("panel-top");
  static ref PANEL__BOTTOM: u64 = hash_str("panel-bottom");
  static ref PANEL__LEFT: u64 = hash_str("panel-left");
  static ref PANEL__CENTER: u64 = hash_str("panel-center");
  static ref PANEL__RIGHT: u64 = hash_str("panel-right");
  static ref DEBUG: u64 = hash_str("debug");
  static ref CLICKED: u64 = hash_str("clicked");
  static ref TABLE__WINDOW: u64 = hash_str("table-window");
}

fn load_icon(path: &Path) -> epi::IconData {
  let (icon_rgba, icon_width, icon_height) = {
      let image = image::open(path)
          .expect("Failed to open icon path")
          .into_rgba8();
      let (width, height) = image.dimensions();
      let rgba = image.into_raw();
      (rgba, width, height)
  };
  epi::IconData{rgba: icon_rgba, width: icon_width, height: icon_height}
}

struct MechApp {
  //mech_client: RunLoop,
  ticks: f32,
  frame: usize,
  code: String,
  core: mech_core::Core,
  maestro_thread: Option<JoinHandle<()>>,
  shapes: Vec<epaint::Shape>,
  value_store: HashMap<usize,Value>,
  changes: Vec<Change>,
  windows: HashSet<String>,
}

//static LONG_STRING: &'static str = include_str!(concat!(env!("OUT_DIR"), "/hello.rs"));

fn load_mech() -> mech_core::Core {
  let code = fs::read_to_string(r#"C:\Users\cmont\mech\mech\notebook2\src\bin\notebook.mec"#).unwrap();
  let mut mech_core = mech_core::Core::new();
  let mut compiler = Compiler::new(); 
  match compiler.compile_str(&code) {
    Ok(blocks) => {
      mech_core.load_blocks(blocks);
    }
    Err(x) => {
      
    }
  }
  
  let mut code = r#"
#time/timer = [|period<s> ticks<u64>|]
#mech/compiler = [|code<string>| "hi"]
#io/pointer = [|x<f32> y<f32>| 0 0]"#.to_string();

code += r#"
#mech/tables = [|name<string>|
               "time/timer"
               "io/pointer"
               "mech/tables"
               "mech/compiler""#;
for name in mech_core.table_names() {
code += &format!("\n{:?}",name);     
}
code += "]";

  let mut compiler = Compiler::new();
  let blocks = compiler.compile_str(&code).unwrap();
  mech_core.load_blocks(blocks);
  mech_core.schedule_blocks();
  mech_core
}


impl MechApp {
  pub fn new() -> Self {
    //let code = LONG_STRING;
    //let code = include_str!("notebook.mec");

    let mech_core = load_mech();
    let mut shapes = vec![epaint::Shape::Noop; 100000];

    Self {

      frame: 0,
      ticks: 0.0,
      code: "".to_string(),
      //mech_client,
      core: mech_core,
      maestro_thread: None,
      shapes,
      windows: HashSet::new(),
      value_store: HashMap::new(),
      changes: vec![],
    }
  }
  
  pub fn render_app(&mut self, ui: &mut egui::Ui) -> Result<(), MechError> {
    match self.core.get_table("app") {
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
      Value::Bool(x) => {ui.label(&format!("{}", x));},
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
        self.make_element(&table.borrow(), ui);  
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
              if raw_kind == *LINK { self.render_link(table,row,ui)?; }
              else if raw_kind == *SLIDER { self.render_slider(table,row,ui)?; }
              else if raw_kind == *CODE { self.render_code(table,row,ui)?; }
              else if raw_kind == *PANEL__TOP { self.render_panel_top(table,row,ui)?; }
              else if raw_kind == *PANEL__BOTTOM { self.render_panel_bottom(table,row,ui)?; }
              else if raw_kind == *PANEL__RIGHT { self.render_panel_right(table,row,ui)?; }
              else if raw_kind == *PANEL__LEFT { self.render_panel_left(table,row,ui)?; }
              else if raw_kind == *PANEL__CENTER { self.render_panel_center(table,row,ui)?; }
              else if raw_kind == *BUTTON { self.render_button(table,row,ui)?; }
              else if raw_kind == *TABLE__WINDOW { self.render_table__window(table,row,ui)?; }
              else if raw_kind == *CANVAS { self.render_canvas(table,row,ui)?; }
              else if raw_kind == *DEBUG { self.render_debug(table,row,ui)?; }
              //else if raw_kind == *IMAGE { render_iamge(table,ui)?; }
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

  pub fn render_code(&mut self, table: &Table, row: usize, container: &mut egui::Ui) -> Result<(),MechError> {
    match (table.get(&TableIndex::Index(row), &TableIndex::Alias(*TEXT))) {
      Ok(Value::Reference(code_table_id)) => {
        let code_table = self.core.get_table_by_id(*code_table_id.unwrap()).unwrap();
        let code_table_brrw = code_table.borrow();
        match code_table_brrw.get(&TableIndex::Index(1), &TableIndex::Index(1)) {
          Ok(Value::String(code)) => {
            self.code = code.to_string();
            let response = container.add_sized(container.available_size(), egui::TextEdit::multiline(&mut self.code)
              .code_editor()
              .frame(false)
            );
            if response.changed() {
              self.changes.push(Change::Set((code_table_brrw.id,vec![(TableIndex::Index(1),TableIndex::Index(1),Value::String(MechString::from_string(self.code.clone())))])));
            }
          }
          x => {return Err(MechError{id: 6496, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
        }
      }
      x => {return Err(MechError{id: 6496, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
    }
    Ok(())
  }

  pub fn render_panel_bottom(&mut self, table: &Table, row: usize, container: &mut egui::Ui) -> Result<(),MechError> {
    match (table.get(&TableIndex::Index(row), &TableIndex::Alias(*CONTAINS)),
            table.get(&TableIndex::Index(row), &TableIndex::Alias(*PARAMETERS))) {
      (contained,parameters_table) => {
        let mut frame = Frame::default();
        let mut min_height = 100.0;
        if let Ok(Value::Reference(parameters_table_id)) = parameters_table {
          match self.core.get_table_by_id(*parameters_table_id.unwrap()) {
            Ok(parameters_table) => {
              let parameters_table_brrow = parameters_table.borrow();
              if let Ok(Value::U128(value)) = parameters_table_brrow.get(&TableIndex::Index(1), &TableIndex::Alias(*FILL)) {
                let color: u32 = value.into();
                let r = (color >> 16) as u8;
                let g = (color >> 8) as u8;
                let b = color as u8;
                frame.fill = Color32::from_rgb(r,g,b);
              }
              if let Ok(Value::F32(value)) = parameters_table_brrow.get(&TableIndex::Index(1), &TableIndex::Alias(*MIN__HEIGHT)) {
                min_height = value.into();
              }
            }
            _ => (),
          }
        }
        frame.margin = egui::style::Margin::same(0.0);
        egui::TopBottomPanel::bottom(humanize(&table.id))
          .resizable(false)
          .min_height(min_height)
          .frame(frame)
        .show_inside(container, |ui| {
          if let Ok(contained) = contained {
            self.render_value(contained, ui);
          }
        });
      }
    }
    Ok(())
  }

  pub fn render_panel_top(&mut self, table: &Table, row: usize, container: &mut egui::Ui) -> Result<(),MechError> {
    match (table.get(&TableIndex::Index(row), &TableIndex::Alias(*CONTAINS)),
            table.get(&TableIndex::Index(row), &TableIndex::Alias(*PARAMETERS))) {
      (contained,parameters_table) => {
        let mut frame = Frame::default();
        let mut min_height = 100.0;
        if let Ok(Value::Reference(parameters_table_id)) = parameters_table {
          match self.core.get_table_by_id(*parameters_table_id.unwrap()) {
            Ok(parameters_table) => {
              let parameters_table_brrow = parameters_table.borrow();
              if let Ok(Value::U128(value)) = parameters_table_brrow.get(&TableIndex::Index(1), &TableIndex::Alias(*FILL)) {
                let color: u32 = value.into();
                let r = (color >> 16) as u8;
                let g = (color >> 8) as u8;
                let b = color as u8;
                frame.fill = Color32::from_rgb(r,g,b);
              }
              if let Ok(Value::F32(value)) = parameters_table_brrow.get(&TableIndex::Index(1), &TableIndex::Alias(*MIN__HEIGHT)) {
                min_height = value.into();
              }
            }
            _ => (),
          }
        }
        frame.margin = egui::style::Margin::same(0.0);
        egui::TopBottomPanel::top(humanize(&table.id))
          .resizable(false)
          .min_height(min_height)
          .frame(frame)
        .show_inside(container, |ui| {
          if let Ok(contained) = contained {
            self.render_value(contained, ui);
          }
        });
      }
    }
    Ok(())
  }

  pub fn render_panel_left(&mut self, table: &Table, row: usize, container: &mut egui::Ui) -> Result<(),MechError> {
    match (table.get(&TableIndex::Index(row), &TableIndex::Alias(*CONTAINS)),
            table.get(&TableIndex::Index(row), &TableIndex::Alias(*PARAMETERS))) {
      (contained,parameters_table) => {
        let mut frame = Frame::default();
        let mut min_width = 100.0;
        if let Ok(Value::Reference(parameters_table_id)) = parameters_table {
          match self.core.get_table_by_id(*parameters_table_id.unwrap()) {
            Ok(parameters_table) => {
              let parameters_table_brrow = parameters_table.borrow();
              if let Ok(Value::U128(value)) = parameters_table_brrow.get(&TableIndex::Index(1), &TableIndex::Alias(*FILL)) {
                let color: u32 = value.into();
                let r = (color >> 16) as u8;
                let g = (color >> 8) as u8;
                let b = color as u8;
                frame.fill = Color32::from_rgb(r,g,b);
              }
              if let Ok(Value::F32(value)) = parameters_table_brrow.get(&TableIndex::Index(1), &TableIndex::Alias(*MIN__WIDTH)) {
                min_width = value.into();
              }
            }
            _ => (),
          }
        }
        frame.margin = egui::style::Margin::same(10.0);
        egui::SidePanel::left(humanize(&table.id))
          .resizable(false)
          .min_width(min_width)
          .frame(frame)
        .show_inside(container, |ui| {
          if let Ok(contained) = contained {
            self.render_value(contained, ui);
          }
        });
      }
    }
    Ok(())
  }

  pub fn render_panel_right(&mut self, table: &Table, row: usize, container: &mut egui::Ui) -> Result<(),MechError> {
    match (table.get(&TableIndex::Index(row), &TableIndex::Alias(*CONTAINS)),
            table.get(&TableIndex::Index(row), &TableIndex::Alias(*PARAMETERS))) {
      (contained,parameters_table) => {
        let mut frame = Frame::default();
        let mut min_width = 100.0;
        if let Ok(Value::Reference(parameters_table_id)) = parameters_table {
          match self.core.get_table_by_id(*parameters_table_id.unwrap()) {
            Ok(parameters_table) => {
              let parameters_table_brrow = parameters_table.borrow();
              if let Ok(Value::U128(value)) = parameters_table_brrow.get(&TableIndex::Index(1), &TableIndex::Alias(*FILL)) {
                let color: u32 = value.into();
                let r = (color >> 16) as u8;
                let g = (color >> 8) as u8;
                let b = color as u8;
                frame.fill = Color32::from_rgb(r,g,b);
              }
              if let Ok(Value::F32(value)) = parameters_table_brrow.get(&TableIndex::Index(1), &TableIndex::Alias(*MIN__WIDTH)) {
                min_width = value.into();
              }
            }
            _ => (),
          }
        }
        frame.margin = egui::style::Margin::same(10.0);
        egui::SidePanel::right(humanize(&table.id))
          .resizable(false)
          .min_width(min_width)
          .frame(frame)
        .show_inside(container, |ui| {
          if let Ok(contained) = contained {
            self.render_value(contained, ui);
          }
        });
      }
    }
    Ok(())
  }

  pub fn render_panel_center(&mut self, table: &Table, row: usize, container: &mut egui::Ui) -> Result<(),MechError> {
    match (table.get(&TableIndex::Index(row), &TableIndex::Alias(*CONTAINS)),
            table.get(&TableIndex::Index(row), &TableIndex::Alias(*PARAMETERS))) {
      (contained,parameters_table) => {
        let mut frame = Frame::default();
        if let Ok(Value::Reference(parameters_table_id)) = parameters_table {
          match self.core.get_table_by_id(*parameters_table_id.unwrap()) {
            Ok(parameters_table) => {
              let parameters_table_brrw = parameters_table.borrow();
              let fill = match parameters_table_brrw.get(&TableIndex::Index(1), &TableIndex::Alias(*FILL)) {
                Ok(Value::U128(fill_value)) => get_color(fill_value),
                _ => Color32::from_rgb(0xFF,0x00,0x00),
              };
              frame.fill = fill;
            }
            _ => (),
          }
        }
        frame.margin = egui::style::Margin::same(10.0);
        egui::CentralPanel::default()
          .frame(frame)
        .show_inside(container, |ui| {
          if let Ok(contained) = contained {
            self.render_value(contained, ui);
          }
        });
      }
      x => {return Err(MechError{id: 6496, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
    }
    Ok(())
  }

  pub fn render_link(&mut self, table: &Table, row: usize, container: &mut egui::Ui) -> Result<(),MechError> {
    match (table.get(&TableIndex::Index(row), &TableIndex::Alias(*TEXT)),
            table.get(&TableIndex::Index(row), &TableIndex::Alias(*URL))) {
      (Ok(Value::String(text)), Ok(Value::String(url))) => {
        container.hyperlink_to(text.to_string(),url.to_string());
      }
      x => {return Err(MechError{id: 6496, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
    }
    Ok(())
  }
  
  pub fn render_table__window(&mut self, table: &Table, row: usize, container: &mut egui::Ui) -> Result<(),MechError> {
    match (table.get(&TableIndex::Index(row), &TableIndex::Alias(*TEXT)),
            table.get(&TableIndex::Index(row), &TableIndex::Alias(*CLICKED))) {
        (Ok(Value::String(text)), Ok(Value::Reference(value_table_id))) => {
        let value_table = self.core.get_table_by_id(*value_table_id.unwrap())?;
        let value_table_brrw = value_table.borrow();
        match value_table_brrw.get(&TableIndex::Index(1), &TableIndex::Index(1)) {
          Ok(Value::Bool(value)) => {
            if container.add(egui::Button::new(text.to_string())).clicked() {
              let new_value = !value;
              if new_value {
                self.windows.insert(text.to_string());
              } else {
                self.windows.remove(&text.to_string());
              }
              self.changes.push(Change::Set((value_table_brrw.id,vec![(TableIndex::Index(1),TableIndex::Index(1),Value::Bool(!value))])));
            }
          }
          x => {return Err(MechError{id: 6497, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
        }
      }
      x => {return Err(MechError{id: 6497, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
    }
    Ok(())
  }


  pub fn render_button(&mut self, table: &Table, row: usize, container: &mut egui::Ui) -> Result<(),MechError> {
    match (table.get(&TableIndex::Index(row), &TableIndex::Alias(*TEXT)),
            table.get(&TableIndex::Index(row), &TableIndex::Alias(*CLICKED))) {
        (Ok(Value::String(text)), Ok(Value::Reference(value_table_id))) => {
        let value_table = self.core.get_table_by_id(*value_table_id.unwrap())?;
        let value_table_brrw = value_table.borrow();
        match value_table_brrw.get(&TableIndex::Index(1), &TableIndex::Index(1)) {
          Ok(Value::Bool(value)) => {
            if container.add(egui::Button::new(text.to_string())).clicked() {
              self.changes.push(Change::Set((value_table_brrw.id,vec![(TableIndex::Index(1),TableIndex::Index(1),Value::Bool(!value))])));
            }
          }
          x => {return Err(MechError{id: 6497, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
        }
      }
      x => {return Err(MechError{id: 6497, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
    }
    Ok(())
  }

  pub fn render_slider(&mut self, table: &Table, row: usize, container: &mut egui::Ui) -> Result<(),MechError> {
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
              self.changes.push(Change::Set((value_table_brrw.id,vec![(TableIndex::Index(1),TableIndex::Index(1),Value::F32(F32::new(self.ticks)))])));
            }
          }
          x => {return Err(MechError{id: 6497, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
        }
      }
      x => {return Err(MechError{id: 6497, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
    }
    Ok(())
  }  

  pub fn render_debug(&mut self, table: &Table, row: usize, container: &mut egui::Ui) -> Result<(),MechError> {
    egui::ScrollArea::vertical().show(container, |ui| {
      ui.label(format!("{:?}", self.core));
    });
    Ok(())
  }  

  pub fn render_canvas(&mut self, table: &Table, row: usize, container: &mut egui::Ui) -> Result<(),MechError> {
    match table.get(&TableIndex::Index(row), &TableIndex::Alias(*CONTAINS)) {
      Ok(Value::Reference(contains_table_id)) => {
        Frame::none().show(container, |ui| {
          let table = self.core.get_table_by_id(*contains_table_id.unwrap()).unwrap();
          let table_brrw = table.borrow();
          match (table_brrw.get(&TableIndex::Index(row), &TableIndex::Alias(*SHAPE)),
                  table_brrw.get(&TableIndex::Index(row), &TableIndex::Alias(*PARAMETERS)))  {
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
    Ok(())
  }



  pub fn render_circle(&mut self, table: &Table, container: &mut egui::Ui) -> Result<Vec<epaint::Shape>,MechError> {
    let mut shapes = vec![];
    match (table.get_column(&TableIndex::Alias(*CENTER__X)),
           table.get_column(&TableIndex::Alias(*CENTER__Y))) {
      (Ok(Column::F32(x)), Ok(Column::F32(y))) => {

        let x_brrw = x.borrow();
        let y_brrw = y.borrow();
   
        let radius = if let Ok(Column::F32(radius)) = table.get_column(&TableIndex::Alias(*RADIUS)) { radius }
        else { ColumnV::new(vec![F32::new(1.0); table.rows]) };

        let line_width = if let Ok(Column::F32(line_width)) = table.get_column(&TableIndex::Alias(*LINE__WIDTH)) { line_width }
        else { ColumnV::new(vec![F32::new(0.0); table.rows]) };

        let fill = if let Ok(Column::U128(fill)) = table.get_column(&TableIndex::Alias(*FILL)) { fill }
        else { ColumnV::new(vec![U128::new(0); table.rows]) };

        let stroke = if let Ok(Column::U128(color)) = table.get_column(&TableIndex::Alias(*STROKE)) { color }
        else { ColumnV::new(vec![U128::new(0); table.rows]) };

        let radius_brrw = radius.borrow();
        let line_width_brrw = line_width.borrow();
        let stroke_brrw = stroke.borrow();
        let fill_brrw = fill.borrow();

        for i in 0..table.rows {
          let line_width: f32 = line_width_brrw[i].into();
          shapes.push(epaint::Shape::Circle(epaint::CircleShape{
            center: Pos2{x: x_brrw[i].into(), y: y_brrw[i].into()},
            radius: radius_brrw[i].into(),
            fill: get_color(fill_brrw[i]),
            stroke: epaint::Stroke::new(line_width, get_color(stroke_brrw[i])),
          }));
        }
      }
      x => {return Err(MechError{id: 6497, kind: MechErrorKind::GenericError(format!("{:?}", x))});},
    }
    Ok(shapes)
  }

}

pub fn get_color(color_value: U128) -> Color32 {
  let color: u32 = color_value.into();
  let r = (color >> 16) as u8;
  let g = (color >> 8) as u8;
  let b = color as u8;
  Color32::from_rgb(r,g,b)
}

impl epi::App for MechApp {

  fn name(&self) -> &str {
    " Mech Notebook"
  }

  fn update(&mut self, ctx: &egui::Context, frame: &epi::Frame) {
    let Self { ticks, core, .. } = self;

    let windows = self.windows.clone();
    
    for table_id in windows {
      egui::Window::new(table_id.clone()).show(ctx, |ui| {
        let table = self.core.get_table(&table_id).unwrap();
        let table_brrw = table.borrow();
        self.make_element(&table_brrw,ui);
      });
    }

    // Set font
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert("FiraCode-Regular".to_owned(),FontData::from_static(include_bytes!("../../../assets/fonts/FiraCode-Regular.ttf")));
    fonts.families.get_mut(&FontFamily::Proportional).unwrap().insert(0, "FiraCode-Regular".to_owned());
    ctx.set_fonts(fonts);

    // Draw frame
    let mut frame = Frame::default();
    frame.margin = egui::style::Margin::same(0.0);
    frame.fill = Color32::from_rgb(0x23,0x22,0x2A);
    egui::CentralPanel::default()
      .frame(frame)
    .show(ctx, |ui| {
      // Compile new code...
      {
        let code_table = self.core.get_table("mech/compiler").unwrap();
        let code_table_brrw = code_table.borrow();
        if let Value::String(code_string) = code_table_brrw.get(&TableIndex::Index(1),&TableIndex::Index(1)).unwrap() {
          if code_string.to_string() != "" {
            let mut compiler = Compiler::new();
            let blocks = compiler.compile_str(&code_string.to_string()).unwrap();
            self.core.load_blocks(blocks);
            self.core.schedule_blocks();    
            self.changes.push(Change::Set((hash_str("mech/compiler"),vec![
              (TableIndex::Index(1),TableIndex::Index(1),Value::String(MechString::from_string("".to_string())))
            ])));
          }
        }
      }


      //ui.ctx().request_repaint();
      self.render_app(ui);

      // Update IO
      let time = ui.input().time;
      self.frame += 1;
      self.changes.push(Change::Set((hash_str("time/timer"),vec![(TableIndex::Index(1),TableIndex::Index(2),Value::U64(U64::new(self.frame as u64)))])));
      match ui.input().pointer.hover_pos() {
        Some(pos) => {
          self.changes.push(Change::Set((hash_str("io/pointer"),vec![
            (TableIndex::Index(1),TableIndex::Index(1),Value::F32(F32::new(pos.x))),
            (TableIndex::Index(1),TableIndex::Index(2),Value::F32(F32::new(pos.y)))
          ])));
        }
        _ => (),
      }
      if ui.input().keys_down.contains(&egui::Key::Escape) {
        let core = load_mech();
        self.core = core;
      }
      self.core.process_transaction(&self.changes);
      self.changes.clear();
    });
  }

  fn clear_color(&self) -> egui::Rgba {
    egui::Rgba::from_rgb(35.0,34.0,42.0)
  }

  fn warm_up_enabled(&self) -> bool {
    true
  }

}


fn main() {
    //let input = std::env::args().nth(1).unwrap();
    let app = MechApp::new();
    let mut native_options = eframe::NativeOptions::default();
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/mech.ico");
    let icon = load_icon(Path::new(path));
    native_options.icon_data = Some(icon);
    native_options.min_window_size = Some(Vec2{x: 1480.0, y: 800.0});
    eframe::run_native(Box::new(app), native_options);
}



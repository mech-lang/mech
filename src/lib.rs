#![feature(alloc)]
#![feature(drain_filter)]
extern crate wasm_bindgen;
extern crate hashbrown;
//extern crate web_sys;
#[macro_use]
extern crate alloc;
#[macro_use]
extern crate serde_derive;
extern crate core;
extern crate web_sys;
extern crate mech_core;
extern crate mech_syntax;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use hashbrown::hash_set::HashSet;
use alloc::vec::Vec;
use core::fmt;
use mech_syntax::compiler::Compiler;
use mech_core::{Transaction, Hasher, Change, Index, Value, Table, Quantity, ToQuantity, QuantityMath};

macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}

#[wasm_bindgen]
pub struct Core {
  core: mech_core::Core,
  changes: Vec<Change>,
}

#[wasm_bindgen]
impl Core {

  pub fn new() -> Core {
    Core {
      core: mech_core::Core::new(100_000,100),
      changes: Vec::new(),
    }
  }

  pub fn compile_code(&mut self, code: String) {
    let mut compiler = Compiler::new();
    compiler.compile_string(code);
    self.core.register_blocks(compiler.blocks.clone());
    self.core.step();
    log!("Compiled {} blocks.", compiler.blocks.len());
  }


  pub fn clear(&mut self) {
    self.core.clear();
    log!("Core Cleared");
  }

  pub fn pause(&mut self) {
    self.core.pause();
    log!("Core Paused");
  }

  pub fn resume(&mut self) {
    self.core.resume();
    log!("Core Resumed");
  }

  pub fn step_back_one(&mut self) {
    self.core.step_back_one();
    log!("Core Time -{}", self.core.offset);
  }

  pub fn step_forward_one(&mut self) {
    self.core.step_forward_one();
    log!("Core Time -{}", self.core.offset);
  }

  pub fn display_core(&mut self) {
    log!("{:?}", self.core);
  }

  pub fn display_runtime(&mut self) {
    log!("{:?}", self.core.runtime);
  }

  pub fn queue_change(&mut self, table: String, row: u32, column: u32, value: u32) {
    let table_id = Hasher::hash_string(table);
    let change = Change::Set{table: table_id, 
                             row: Index::Index(row as u64), 
                             column: Index::Index(column as u64),
                             value: Value::from_u64(value as u64),
                            };
    self.changes.push(change);
  }

  pub fn process_transaction(&mut self) {
    if !self.core.paused {
        let txn = Transaction::from_changeset(self.changes.clone());
        //log!("{:?}", txn);
        self.core.process_transaction(&txn);
    }
    self.changes.clear();
  }

  pub fn get_mantissas(&mut self, table: u64, column: u64) -> Vec<i64> {
      let mut output: Vec<i64> = vec![];
      match self.core.store.get_column(table, Index::Index(column)) {
          Some(column) => {
              for row in column {
                  output.push(row.as_quantity().unwrap().mantissa());
              }
          }
          _ => log!("{} not found", table),
      }
      output
  }

  pub fn get_ranges(&mut self, table: u64, column: u64) -> Vec<i64> {
      let mut output: Vec<i64> = vec![];
      match self.core.store.get_column(table, Index::Index(column)) {
          Some(column) => {
              for row in column {
                  output.push(row.as_quantity().unwrap().range());
              }
          }
          _ => log!("{} not found", table),
      }
      output
  }

  pub fn get_column(&mut self, table: u64, column: u64) -> Vec<f64> {
      let mut output: Vec<f64> = vec![];
      match self.core.store.get_column(table, Index::Index(column)) {
          Some(column) => {
              for row in column {
                  output.push(row.as_quantity().unwrap().to_float());
              }
          }
          _ => log!("{} not found", table),
      }
      output
  }

  pub fn add_application(&self) -> Result<(), JsValue> {
    let table_id = Hasher::hash_str("app/main");
    match self.core.store.get_table(table_id) {
      Some(app_table) => {

        let window = web_sys::window().expect("no global `window` exists");
        let document = window.document().expect("should have a document on window");
        let body = document.body().expect("document should have a body");
        let drawing_area = document.get_element_by_id("drawing").unwrap();
        let mut app = document.create_element("div")?;
        let contents_id = app_table.data[1][0].as_u64().unwrap();
        let contents_table = self.core.store.get_table(contents_id).unwrap();
        self.draw_contents(&contents_table, &mut app);
        drawing_area.append_child(&app)?;
      }
      _ => (),
    }
    Ok(())
  }

  fn draw_contents(&self, table: &Table, container: &mut web_sys::Element) -> Result<(), JsValue> {
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    for row in 0..table.rows as usize {
      let tag = &table.data[0][row].as_string().unwrap();
      match tag.as_ref() {
        "div" => {
          let mut div = document.create_element("div")?;
          let class = &table.data[1][row].as_string().unwrap();
          div.set_attribute("class",class);
          match &table.data[2][row] {
            Value::String(value) => div.set_inner_html(&value),
            Value::Reference(reference) => {
              let referenced_table = self.core.store.get_table(*reference).unwrap();
              self.draw_contents(&referenced_table, &mut div);
            }
            _ => (),
          };
          container.append_child(&div)?;
        },
        "img" => {
          let class = &table.data[1][row].as_string().unwrap();
          let value = &table.data[2][row].as_string().unwrap();
          let mut img = web_sys::HtmlImageElement::new().unwrap();
          img.set_attribute("class", class);
          img.set_src(value);
          container.append_child(&img)?;
        },
        "canvas" => { 
          let canvas = document.create_element("canvas")?;
          let elements_id_str = &table.data[4][row].as_string().unwrap();
          let elements_id = &table.data[4][row].as_u64().unwrap();
          canvas.set_attribute("id","drawing canvas");
          canvas.set_attribute("elements",elements_id_str);
          canvas.set_attribute("width", &format!("{}", table.data[2][row].as_float().unwrap()));
          canvas.set_attribute("height", &format!("{}", table.data[3][row].as_float().unwrap()));
          canvas.set_attribute("style", "background-color: rgb(255, 255, 255)");
          let canvas: web_sys::HtmlCanvasElement = canvas
                .dyn_into::<web_sys::HtmlCanvasElement>()
                .map_err(|_| ())
                .unwrap();
          self.render_canvas(&canvas);
          container.append_child(&canvas)?;
        },
        _ => (),
      }
    }
    Ok(())
  }

  pub fn render_canvas(&self, canvas: &web_sys::HtmlCanvasElement) -> Result<(), JsValue> {

    let context = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();
    let radius = 10.0;
    let table_id = Hasher::hash_str("html/canvas");
    let table = self.core.store.get_table(table_id).unwrap();

    // Get the elements table for this canvas
    let elements = canvas.get_attribute("elements").unwrap();
    let elements_table_id: u64 = elements.parse::<u64>().unwrap();
    let elements_table = self.core.store.get_table(elements_table_id).unwrap();

    context.clear_rect(0.0, 0.0, canvas.width().into(), canvas.height().into());
    for i in 0..elements_table.rows as usize {
      match elements_table.data[0][i as usize] {
        Value::String(ref shape) => {
          match shape.as_ref() {
            "circle" => {
              let x = elements_table.data[1][i].as_float().unwrap();
              let y = elements_table.data[2][i].as_float().unwrap();
              let radius = elements_table.data[3][i].as_float().unwrap();
              context.begin_path();
              context.arc(x, y, radius, 0.0, 2.0 * 3.14);
              context.set_fill_style(&JsValue::from_str("#0B79CE"));
              context.fill();  
            },
            "line" => {
              let x1 = elements_table.data[1][i].as_float().unwrap();
              let y1 = elements_table.data[2][i].as_float().unwrap();
              let x2 = elements_table.data[4][i].as_float().unwrap();
              let y2 = elements_table.data[5][i].as_float().unwrap();
              context.begin_path();
              context.move_to(x1, y1);
              context.line_to(x2, y2);
              context.close_path();
              context.stroke();
            },
            "image" => {
              let mut img = web_sys::HtmlImageElement::new().unwrap();
              let image_source = elements_table.data[7][i].as_string().unwrap();
              let rotation = elements_table.data[3][i].as_float().unwrap();
              let x = elements_table.data[1][i].as_float().unwrap();
              let y = elements_table.data[2][i].as_float().unwrap();
              img.set_src(&image_source.to_owned());
              context.save();
              context.translate(x, y);
              context.rotate(rotation * 3.141592654 / 180.0);
              context.draw_image_with_html_image_element(&img, 0.0, 0.0);
              context.restore();
            },
            _ => (),
          }
        },    
        _ => (),    
      }


    }
    Ok(())
  } 

  pub fn list_global_tables(&self) -> Result<(), JsValue> {
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    let body = document.body().expect("document should have a body");
    let table_list_div = document.create_element("div")?;
    let table_list = document.create_element("ul")?;
    for (table_id, table) in self.core.store.tables.map.iter() {
      let table_list_item = document.create_element("li")?;
      let table_name = match self.core.store.names.get(table_id) {
        Some(name) => name,
        None => "",
      };
      table_list_item.set_inner_html(table_name);
      table_list.append_child(&table_list_item)?;
    }
    table_list_div.append_child(&table_list)?;
    body.append_child(&table_list_div)?;
    Ok(())
  }

}

#[wasm_bindgen]
pub fn hash_string(input: String) -> u64 {
    Hasher::hash_string(input)
} 
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

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::cell::Cell;
use std::rc::Rc;
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
  images: HashMap<u64, web_sys::HtmlImageElement>,
  nodes: HashMap<u64, Vec<u64>>,
}

#[wasm_bindgen]
impl Core {

  pub fn new() -> Core {
    Core {
      core: mech_core::Core::new(100_000,100),
      changes: Vec::new(),
      images: HashMap::new(),
      nodes: HashMap::new(),
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

  pub fn render(&mut self) {
    let window = web_sys::window().expect("no global `window` exists");
    let wasm_core = self as *mut Core;
    let closure = Closure::wrap(Box::new(move || {
      let window = web_sys::window().expect("no global `window` exists");
      let document = window.document().expect("should have a document on window");
      let canvases = document.get_elements_by_tag_name("canvas");
      for i in 0..canvases.length() {
        let canvas = canvases.get_with_index(i);
        let canvas: web_sys::HtmlCanvasElement = canvas
                  .unwrap()
                  .dyn_into::<web_sys::HtmlCanvasElement>()
                  .map_err(|_| ())
                  .unwrap();
        unsafe {
          (*wasm_core).render_canvas(&canvas);
        }
      }
    }) as Box<FnMut()>);
    window.request_animation_frame(closure.as_ref().unchecked_ref());
    closure.forget();
    /*
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    for (table_id, index) in self.core.runtime.changed_this_round.drain() {
      match self.nodes.get(&table_id) {
        Some(nodes) => {
          for node in nodes {
            let element = document.get_element_by_id(&format!("{}",node));
            match element {
              Some(html_element) => {
                let table = self.core.store.get_table(table_id).unwrap();
                html_element.set_inner_html(&table.data[2][1].as_string().unwrap());
              },
              _ => (),
            }
          }
        },
        _ => (),
      }
    }*/
  }

  pub fn queue_change(&mut self, table: String, row: u32, column: u32, value: i32) {
    let table_id = Hasher::hash_string(table);
    let change = Change::Set{table: table_id, 
                             row: Index::Index(row as u64), 
                             column: Index::Index(column as u64),
                             value: Value::from_i64(value as i64),
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

  pub fn add_application(&mut self) -> Result<(), JsValue> {
    let table_id = Hasher::hash_str("app/main");
    let core = &mut self.core as *mut mech_core::Core;
    let table;
    // TODO Make this safe
    unsafe {
      table = (*core).store.get_table(table_id);
    }
    match table {
      Some(app_table) => {
        let window = web_sys::window().expect("no global `window` exists");
        let document = window.document().expect("should have a document on window");
        let body = document.body().expect("document should have a body");
        let drawing_area = document.get_element_by_id("drawing").unwrap();
        let mut app = document.create_element("div")?;
        let contents_id = app_table.data[1][0].as_u64().unwrap();
        let contents_table;
        // TODO Make this safe
        unsafe {
          contents_table = (*core).store.get_table(contents_id).unwrap();       
        }
        self.draw_contents(&contents_table, &mut app);
        drawing_area.append_child(&app)?;
      }
      _ => (),
    }
    Ok(())
  }

  fn draw_contents(&mut self, table: &Table, container: &mut web_sys::Element) -> Result<(), JsValue> {
    let core = &mut self.core as *mut mech_core::Core;
    let wasm_core = self as *mut Core;
    let changes = &mut self.changes as *mut Vec<Change>;
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    for row in 0..table.rows as usize {
      let tag = &table.data[0][row].as_string().unwrap();
      match tag.as_ref() {
        "div" => {
          let element_id = Hasher::hash_string(format!("div-{:?}-{:?}", table.id, row));
          let mut div = document.create_element("div")?;
          unsafe {
            let nodes = (*wasm_core).nodes.entry(table.id).or_insert(vec![]);
            nodes.push(element_id);
          }
          div.set_id(&format!("{:?}",element_id));
          match &table.data[1][row].as_string() {
            Some(class) => {
              div.set_attribute("class",class);
            },
            _ => (),
          }
          match &table.data[2][row] {
            Value::String(value) => div.set_inner_html(&value),
            Value::Number(value) => div.set_inner_html(&format!("{:?}", value.to_float())),
            Value::Reference(reference) => {
              let referenced_table;
              // TODO Make this safe
              unsafe {
                referenced_table = (*core).store.get_table(*reference).unwrap();
              }
              self.draw_contents(&referenced_table, &mut div);
            }
            _ => (),
          };
          container.append_child(&div)?;
        },
        "img" => {
          let element_id = Hasher::hash_string(format!("img-{:?}-{:?}", table.id, row));
          let class = &table.data[1][row].as_string().unwrap();
          let value = &table.data[2][row].as_string().unwrap();
          let mut img = web_sys::HtmlImageElement::new().unwrap();
          img.set_attribute("class", class);
          img.set_id(&format!("{:?}",element_id));
          img.set_src(value);
          container.append_child(&img)?;
        },
        "slider" => {
          let element_id = Hasher::hash_string(format!("slider-{:?}-{:?}", table.id, row));
          let mut slider = document.create_element("input")?;
          let mut slider: web_sys::HtmlInputElement = slider
                .dyn_into::<web_sys::HtmlInputElement>()
                .map_err(|_| ())
                .unwrap();
          let parameters_id_str = &table.data[3][row].as_string().unwrap();
          let parameters_id = &table.data[3][row].as_u64().unwrap();
          let parameters_table = self.core.store.get_table(*parameters_id).unwrap();
          let min = &parameters_table.data[0][0].as_string().unwrap();
          let max = &parameters_table.data[1][0].as_string().unwrap();
          let value = &parameters_table.data[2][0].as_string().unwrap();
          slider.set_id(&format!("{:?}", element_id));
          slider.set_type("range");
          slider.set_min(min);
          slider.set_max(max);
          slider.set_value(value);
          slider.set_attribute("parameters", parameters_id_str);
          {
            let closure = Closure::wrap(Box::new(move |event: web_sys::InputEvent| {
              match event.target() {
                Some(target) => {
                  let slider = target.dyn_ref::<web_sys::HtmlInputElement>().unwrap();
                  let table_id = Hasher::hash_str("angle1");
                  let slider_value = slider.value().parse::<i64>().unwrap();
                  let parameters_id = slider.get_attribute("parameters").unwrap().parse::<u64>().unwrap();
                  let change = Change::Set{
                    table: parameters_id, 
                    row: Index::Index(1), 
                    column: Index::Index(3),
                    value: Value::from_i64(slider_value),
                  };
                  let txn = Transaction::from_change(change);
                  // TODO Make this safe
                  unsafe {
                    (*core).process_transaction(&txn);
                    (*wasm_core).render();
                  }
                },
                _ => (),
              }
            }) as Box<dyn FnMut(_)>);
            slider.set_oninput(Some(closure.as_ref().unchecked_ref()));
            closure.forget();
          }
          container.append_child(&slider)?;
        },
        "canvas" => { 
          let element_id = Hasher::hash_string(format!("canvas-{:?}-{:?}", table.id, row));
          let canvas = document.create_element("canvas")?;
          let elements_id_str = &table.data[2][row].as_string().unwrap();
          let elements_id = &table.data[2][row].as_u64().unwrap();
          let parameters_id = &table.data[3][row].as_u64().unwrap();
          let parameters_table;
          unsafe {
            parameters_table = (*core).store.get_table(*parameters_id).unwrap();
          }
          canvas.set_id(&format!("{:?}",element_id));
          canvas.set_attribute("elements",elements_id_str);
          canvas.set_attribute("width", &parameters_table.data[0][0].as_string().unwrap());
          canvas.set_attribute("height",&parameters_table.data[1][0].as_string().unwrap());
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

  pub fn render_canvas(&mut self, canvas: &web_sys::HtmlCanvasElement) -> Result<(), JsValue> {
    let wasm_core = self as *mut Core;
    let context = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();

    // Get the elements table for this canvas
    let elements = canvas.get_attribute("elements").unwrap();
    let elements_table_id: u64 = elements.parse::<u64>().unwrap();
    let elements_table = self.core.store.get_table(elements_table_id).unwrap();
    let context = Rc::new(context);

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
              let image_source = elements_table.data[7][i].as_string().unwrap();
              let source_hash = Hasher::hash_string(image_source.clone());
              match self.images.entry(source_hash) {
                Entry::Occupied(img_entry) => {
                  let img = img_entry.get();
                  let rotation = elements_table.data[3][i].as_float().unwrap();
                  let x = elements_table.data[1][i].as_float().unwrap();
                  let y = elements_table.data[2][i].as_float().unwrap();
                  let ix = img.width() as f64 / 2.0;
                  let iy = img.height() as f64 / 2.0;
                  // Draw it
                  context.save();
                  context.translate(x, y);
                  context.rotate(rotation * 3.141592654 / 180.0);
                  context.draw_image_with_html_image_element(&img, -ix, -iy);
                  context.restore();
                },
                Entry::Vacant(v) => {
                  let mut img = web_sys::HtmlImageElement::new().unwrap();
                  img.set_src(&image_source.to_owned());
                  {
                    let closure = Closure::wrap(Box::new(move || {
                      unsafe {
                        (*wasm_core).render();
                      }
                    }) as Box<FnMut()>);
                    img.set_onload(Some(closure.as_ref().unchecked_ref()));
                    v.insert(img);
                    closure.forget();
                  }
                },
              }
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
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
use mech_syntax::formatter::Formatter;
use mech_syntax::compiler::{Compiler, Node, Program, Section, Element};
use mech_core::{Transaction, BlockState, Hasher, Change, Index, Value, Table, Quantity, ToQuantity, QuantityMath};

macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}

#[wasm_bindgen]
pub struct Core {
  core: mech_core::Core,
  programs: Vec<Program>,
  changes: Vec<Change>,
  images: HashMap<u64, web_sys::HtmlImageElement>,
  nodes: HashMap<u64, Vec<u64>>,
  views: HashSet<u64>,
  roots: HashSet<String>,
}

#[wasm_bindgen]
impl Core {
  pub fn new(changes: usize, tables: usize) -> Core {
    Core {
      core: mech_core::Core::new(changes,tables),
      programs: Vec::new(),
      changes: Vec::new(),
      images: HashMap::new(),
      nodes: HashMap::new(),
      views: HashSet::new(),
      roots: HashSet::new(),
    }
  }

  pub fn compile_code(&mut self, code: String) {
    let mech_code = Hasher::hash_str("mech/code");
    let changes = vec![
      Change::NewTable{id: mech_code, rows: 1, columns: 1},
      Change::Set{table: mech_code, row: mech_core::Index::Index(1), column: mech_core::Index::Index(1), value: Value::from_str(&code)},
    ];
    let mut compiler = Compiler::new();
    compiler.compile_string(code);
    self.core.register_blocks(compiler.blocks.clone());
    self.core.step();
    self.core.process_transaction(&Transaction::from_changeset(changes));
    self.programs = compiler.programs.clone();
    self.render_program();
    log!("Compiled {} blocks.", compiler.blocks.len());
  }

  pub fn render_program(&mut self) -> Result<(), JsValue>  {
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    let body = document.body().expect("document should have a body");

    let mut documentation = document.create_element("div")?;
    documentation.set_attribute("class", "mech-docs");
    let mut contents = document.create_element("div")?;
    contents.set_attribute("class", "mech-contents");


    let wasm_core = self as *mut Core;
    for program in &self.programs {

      // Make controls area
      let mut controls = document.create_element("div")?;
      controls.set_attribute("class", "mech-controls");
      controls.set_id("mech-controls");

      // Code
      let mut code_button = document.create_element("a")?;
      code_button.set_attribute("class", "mech-control ion-pound");
      code_button.set_attribute("href", "/#/docs/index.mec");
      controls.append_child(&code_button);

      // Tables
      let mut tables_button = document.create_element("a")?;
      tables_button.set_attribute("class", "mech-control ion-grid");
      tables_button.set_attribute("href", "/#/tables/index.mec");
      controls.append_child(&tables_button);

      // Documentation
      let mut docs_button = document.create_element("a")?;
      docs_button.set_attribute("class", "mech-control ion-ios-bookmarks");
      docs_button.set_attribute("href", "/#/docs/index.mec");
      controls.append_child(&docs_button);
      
      // Make contents entry
      let mut contents_heading = document.create_element("div")?;
      contents_heading.set_attribute("class", "mech-contents-heading");
      contents_heading.set_inner_html(&format!("{}", &program.title.clone().unwrap()));
      contents.append_child(&contents_heading);

      // Make application area
      let mut drawing = document.create_element("div")?;
      drawing.set_attribute("class", "mech-application");
      drawing.set_id("drawing");

      // Render the program
      let mut rendered_program = document.create_element("div")?;
      rendered_program.set_attribute("class", "mech-program");
      let mut title = document.create_element("h1")?;
      title.set_inner_html(&format!("# {}", &program.title.clone().unwrap()));
      rendered_program.append_child(&title)?;
      for section in &program.sections {
        let mut rendered_section = document.create_element("div")?;
        match &section.title {
          Some(title_text) => {
            // Make contents entry
            let mut contents_heading = document.create_element("div")?;
            contents_heading.set_attribute("class", "mech-contents-sub-heading");
            contents_heading.set_inner_html(&format!("{}", &title_text.clone()));
            {
              let closure = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
                let window = web_sys::window().expect("no global `window` exists");
                let document = window.document().expect("should have a document on window");
                //let root_node = document.get_element_by_id(root).unwrap();
                let target: web_sys::HtmlElement = event.target()
                                        .unwrap()
                                        .dyn_into::<web_sys::HtmlElement>()
                                        .map_err(|_| ())
                                        .unwrap();
                let heading = document.get_element_by_id(&target.inner_text()).unwrap();
                heading.scroll_into_view();
              }) as Box<dyn FnMut(_)>);
              contents_heading.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())?;
              closure.forget();
            }
            contents.append_child(&contents_heading);
            
            let mut title = document.create_element("h2")?;
            title.set_attribute("id", &title_text.clone());
            title.set_inner_html(&title_text.clone());
            rendered_section.append_child(&title);
          },
          _ => (),
        }
        for element in &section.elements {
          match element {
            Element::Paragraph(node) => {
              let mut paragraph = render_paragraph(node)?;
              rendered_section.append_child(&paragraph);             
            },
            Element::List(node) => {
              let mut unordered_list = render_unordered_list(node)?;
              rendered_section.append_child(&unordered_list);
            },
            Element::CodeBlock(node) => {
              let mut code_block = render_code_block(node)?;
              rendered_section.append_child(&code_block);
            },
            Element::Block((block_id, block_ast)) => {
              let mut formatter = Formatter::new();
              let mut code = document.create_element("div")?;
              let mut code_text = document.create_element("pre")?;
              code_text.set_attribute("contenteditable","true");
              code_text.set_attribute("class","mech-code");
              code_text.set_attribute("block-id",&format!("{}",block_id));
              code_text.set_attribute("spellcheck", "false");
              {
                let closure = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
                  /*if pressed.get() {

                    

                  }*/

                  /*

                  log!("{} {}", event.offset_x() as f64, event.offset_y() as f64);
                  log!("{:?}", event.target().unwrap());
                  let target: web_sys::HtmlElement = event.target()
                                                          .unwrap()
                                                          .dyn_into::<web_sys::HtmlElement>()
                                                          .map_err(|_| ())
                                                          .unwrap();
                  log!("{:?}", target.get_attribute("id"));
                  let window = web_sys::window().expect("no global `window` exists");
                  let selection = window.get_selection();*/

                }) as Box<dyn FnMut(_)>);
                code_text.add_event_listener_with_callback("mousedown", closure.as_ref().unchecked_ref())?;
                closure.forget();
              }
              {
                let core = &mut self.core as *mut mech_core::Core;
                let closure = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {

                  let mut target: web_sys::HtmlElement = event.target()
                                                              .unwrap()
                                                              .dyn_into::<web_sys::HtmlElement>()
                                                              .map_err(|_| ())
                                                              .unwrap();
                  let block_id = target.get_attribute("block-id").unwrap().parse::<usize>().unwrap();;


                  if event.key_code() == 13 && event.ctrl_key() {
                    let mut compiler = Compiler::new();
                    let block_ast = compiler.compile_block_string(target.inner_text());
                    compiler.compile_block(block_ast);
                    let mut new_block = &mut compiler.blocks[0];
                    new_block.id = block_id;
                    unsafe {
                      (*core).remove_block(&block_id);
                      (*core).register_blocks(vec![new_block.clone()]);
                      (*core).step();
                      (*wasm_core).render();
                    }
                    //let mut formatter = Formatter::new();
                    //let html = formatter.format(&block_ast, true);
                    //target.set_inner_html(&html);
                  }


                  //log!("{}", target.inner_text());
                  /*
                  var el = document.getElementsByTagName('div')[0];
                  var range = document.createRange();
                  var sel = window.getSelection();
                  range.setStart(el.childNodes[0], 2);
                  range.collapse(true);
                  sel.removeAllRanges();
                  sel.addRange(range);
                  el.focus();*/
/*
                  let window = web_sys::window().expect("no global `window` exists");
                  let mut selection = window.get_selection().unwrap().unwrap();
                 
                  let focus_node = selection.focus_node().unwrap();
                  let focus_offset = selection.focus_offset();

                  log!("{:?}", focus_node);
                  


                  let mut range = web_sys::Range::new().unwrap();
                  range.set_start(&focus_node, focus_offset);
                  range.collapse();                                                    
                  selection.remove_all_ranges();
                  selection.add_range(&range);*/

                }) as Box<dyn FnMut(_)>);
                code_text.add_event_listener_with_callback("keyup", closure.as_ref().unchecked_ref())?;
                closure.forget();
              }
              let block = &self.core.runtime.blocks.get(block_id).unwrap();
              let html = formatter.format(block_ast, true);
              code_text.set_inner_html(&html);
              code.append_child(&code_text);
              // Add output to the block if we have it
              let view_id = Hasher::hash_str("block/view");
              match block.get_table(view_id) {
                Some(table) => {
                  let mut view = document.create_element("div")?;
                  view.set_attribute("class", "mech-view");
                  view.set_id(&format!("{}",block_id));
                  self.views.insert(*block_id as u64);
                  let mut output = "".to_string();
                  let view_type = table.get_column(&Index::Alias(Hasher::hash_str("type")));
                  let x_pts = table.get_column(&Index::Alias(Hasher::hash_str("x")));
                  let y_pts = table.get_column(&Index::Alias(Hasher::hash_str("y")));
                  match (view_type, x_pts, y_pts) {
                    (Some(view_type), Some(x_pt_table), Some(y_pt_table)) => { 
                      match view_type[0].as_string().unwrap().as_ref() {
                        // Draw a scatter plot
                        "scatter" => {
                          let mut canvas = document.create_element("canvas")?;
                          
                          let canvas: web_sys::HtmlCanvasElement = canvas.dyn_into::<web_sys::HtmlCanvasElement>()
                                                                         .map_err(|_| ())
                                                                         .unwrap();
                          canvas.set_attribute("width", "700");

                          let context = canvas
                              .get_context("2d")
                              .unwrap()
                              .unwrap()
                              .dyn_into::<web_sys::CanvasRenderingContext2d>()
                              .unwrap();
                          let x_data = block.get_table(x_pt_table[0].as_u64().unwrap()).unwrap();
                          let y_data = block.get_table(y_pt_table[0].as_u64().unwrap()).unwrap();

                          for i in 0..x_data.rows as usize {
                            let y = &y_data.data[0][i].as_float().unwrap();
                            let x = &x_data.data[0][i].as_float().unwrap(); {
                              context.save();
                              context.begin_path();
                              context.arc(*x, *y, 2.0, 0.0, 1.0 * 3.14);
                              context.set_fill_style(&JsValue::from_str("#000000"));
                              context.fill();  
                              context.restore();
                            }
                          }

                          view.append_child(&canvas);
                        },
                        _ => (),
                      }
                    } 
                    _ => {
                      for i in 0..table.rows {
                        for j in 0..table.columns {
                          output = format!("{} {}", output, &table.data[j as usize][i as usize].as_string().unwrap());
                        }
                        output = format!("{}</br>",output);
                      }
                      view.set_inner_html(&output);
                    },
                  }

                  code.append_child(&view);
                }
                _ => (),
              }
              // Add a chart if we have it
              rendered_section.append_child(&code);
            },
            _ => (),
          }
        }
        rendered_program.append_child(&rendered_section)?;
      }
      documentation.append_child(&controls);
      documentation.append_child(&contents);
      documentation.append_child(&rendered_program)?;
      documentation.append_child(&drawing)?;
      body.append_child(&documentation)?;
      // Register inline views
      let inline_view_elements = document.get_elements_by_class_name("mech-inline-mech-view");
      for ix in 0..inline_view_elements.length() {
        let view = inline_view_elements.item(ix).unwrap();
        let id = view.id().parse::<u64>().unwrap();
        let view_table = self.core.store.get_table(id).unwrap();
        let data = &view_table.data[0][0];
        view.set_inner_html(&data.as_string().unwrap());
      }
      // Set pending blocks
      let mech_code_blocks = document.get_elements_by_class_name("mech-code");
      for ix in 0..mech_code_blocks.length() {
        let code_block = mech_code_blocks.item(ix).unwrap();
        let block_id = code_block.get_attribute("block-id").unwrap().parse::<usize>().unwrap();
        let block = self.core.runtime.blocks.get(&block_id).unwrap();
        match block.state {
          BlockState::Pending => {
            let class = code_block.get_attribute("class").unwrap();
            let class = format!("{} pending", class);
            code_block.set_attribute("class", &class);
          }
          _ =>()
        }
      }
    }
    Ok(())
  }

  pub fn clear(&mut self) {
    self.core.clear();
    for root in self.roots.iter() {
      let window = web_sys::window().expect("no global `window` exists");
      let document = window.document().expect("should have a document on window");
      match document.get_element_by_id(root) {
        Some(root_node) => {
          'remove_nodes: loop {
            match root_node.first_child() {
              Some(mech_node) => root_node.remove_child(&mech_node),
              None => break 'remove_nodes,
            };
          }
        }
        _ =>(),
      }
    }
    self.programs.clear();
    self.changes.clear();
    self.images.clear();
    self.nodes.clear();
    self.views.clear();
    self.roots.clear();
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

  pub fn set_time(&mut self, time: usize) {
    self.core.set_time(time);
    log!("Core Time -{}", self.core.offset);
  }

  pub fn display_core(&self) {
    log!("{:?}", self.core);
  }

  pub fn display_runtime(&self) {
    log!("{:?}", self.core.runtime);
  }

  pub fn display_changes(&self) {
    for change in &self.core.store.changes {
      log!("{:?}", change);
    }
  }

  pub fn render(&mut self) {
    let window = web_sys::window().expect("no global `window` exists");
    let wasm_core = self as *mut Core;
    let closure = Closure::wrap(Box::new(move || {
      let window = web_sys::window().expect("no global `window` exists");
      let document = window.document().expect("should have a document on window");

      // render canvases
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

    // render views
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    let view_id = Hasher::hash_str("block/view");
    for view in self.views.iter() {
      let view_node = document.get_element_by_id(&format!("{}",view)).unwrap();
      let block = &self.core.runtime.blocks.get(&(*view as usize)).unwrap();
      let table = block.get_table(view_id).unwrap();
      let mut output = "".to_string();
      for i in 0..table.rows {
        for j in 0..table.columns {
          output = format!("{} {}", output, &table.data[j as usize][i as usize].as_string().unwrap());
        }
        output = format!("{}</br>",output);
      }
      view_node.set_inner_html(&output);
    }

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

  pub fn get_mantissas(&mut self, table: String, column: u32) -> Vec<i32> {
      let table_id = Hasher::hash_string(table);
      let mut output: Vec<i32> = vec![];
      match self.core.store.get_column(table_id, Index::Index(column as u64)) {
          Some(column) => {
              for row in column {
                  output.push(row.as_quantity().unwrap().mantissa() as i32);
              }
          }
          _ => log!("{} not found", table_id),
      }
      output
  }

  pub fn get_ranges(&mut self, table: String, column: u32) -> Vec<i32> {
      let table_id = Hasher::hash_string(table);    
      let mut output: Vec<i32> = vec![];
      match self.core.store.get_column(table_id, Index::Index(column as u64)) {
          Some(column) => {
              for row in column {
                  output.push(row.as_quantity().unwrap().range() as i32);
              }
          }
          _ => log!("{} not found", table_id),
      }
      output
  }

  pub fn get_column(&mut self, table: String, column: u32) -> Vec<f32> {
      let table_id = Hasher::hash_string(table);    
      let mut output: Vec<f32> = vec![];
      match self.core.store.get_column(table_id, Index::Index(column as u64)) {
          Some(column) => {
              for row in column {
                  output.push(row.as_quantity().unwrap().to_float() as f32);
              }
          }
          _ => log!("{} not found", table_id),
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
        for row in 0..app_table.rows as usize {
          let root_id = app_table.data[0][row].as_string().unwrap();
          self.roots.insert(root_id.clone());
          let contents_id = app_table.data[2][row].as_u64().unwrap();
          let contents_table;
          let mut app = document.create_element("div")?;
          let drawing_area = document.get_element_by_id(&root_id).unwrap();
          // TODO Make this safe
          unsafe {
            contents_table = (*core).store.get_table(contents_id).unwrap();       
          }
          self.draw_contents(&contents_table, &mut app);
          drawing_area.append_child(&app)?;
        }
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
    for row in 0..elements_table.rows as usize {
      match elements_table.data[0][row] {
        Value::String(ref shape) => {
          match shape.as_ref() {
            "circle" => {
              let parameters_id = &elements_table.data[1][row].as_u64().unwrap();
              let parameters_table = self.core.store.get_table(*parameters_id).unwrap();              
              let x = parameters_table.data[0][0].as_float().unwrap();
              let y = parameters_table.data[1][0].as_float().unwrap();
              let radius = parameters_table.data[2][0].as_float().unwrap();
              let fill = parameters_table.data[3][0].as_string().unwrap();
              context.save();
              context.begin_path();
              context.arc(x, y, radius, 0.0, 2.0 * 3.14);
              context.set_fill_style(&JsValue::from_str(&fill));
              context.fill();  
              context.restore();
            },
            "line" => {
              let parameters_id = &elements_table.data[1][row].as_u64().unwrap();
              let parameters_table = self.core.store.get_table(*parameters_id).unwrap();
              let x1 = parameters_table.data[0][0].as_float().unwrap();
              let y1 = parameters_table.data[1][0].as_float().unwrap();
              let x2 = parameters_table.data[2][0].as_float().unwrap();
              let y2 = parameters_table.data[3][0].as_float().unwrap();
              let stroke = parameters_table.data[4][0].as_string().unwrap();
              context.save();
              context.begin_path();
              context.move_to(x1, y1);
              context.line_to(x2, y2);
              context.close_path();
              context.set_stroke_style(&JsValue::from_str(&stroke));
              context.stroke();
              context.restore();
            },
            "image" => {
              let parameters_id = &elements_table.data[1][row].as_u64().unwrap();
              let parameters_table = self.core.store.get_table(*parameters_id).unwrap();
              let image_source = parameters_table.data[3][0].as_string().unwrap();
              let source_hash = Hasher::hash_string(image_source.clone());
              match self.images.entry(source_hash) {
                Entry::Occupied(img_entry) => {
                  let img = img_entry.get();
                  let rotation = parameters_table.data[2][0].as_float().unwrap();
                  let x = parameters_table.data[0][0].as_float().unwrap();
                  let y = parameters_table.data[1][0].as_float().unwrap();
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
      match self.core.store.names.get(table_id) {
        Some(name) => {
          table_list_item.set_inner_html(name);
          table_list.append_child(&table_list_item)?;
        },
        None => (),
      };
      
    }
    table_list_div.append_child(&table_list)?;
    //body.append_child(&table_list_div)?;
    Ok(())
  }

}

fn render_inline_mech(inline_mech_node: &Node) -> Result<web_sys::Element, JsValue> {
  match inline_mech_node {
    Node::InlineMechCode{children} => {
      let window = web_sys::window().expect("no global `window` exists");
      let document = window.document().expect("should have a document on window");
      let mut inline_mech = document.create_element("span")?;
      inline_mech.set_attribute("class", "mech-inline-mech");
      // define the rest of the block
      for child in children {
        match child {
          _ => {
            let mut formatter = Formatter::new();
            let formatted_inline_block = formatter.format(child, true);
            let block_id = formatter.format(child, false);
            let id = Hasher::hash_str(&block_id);
            let mut inline_code = document.create_element("span")?;
            inline_code.set_attribute("class", "mech-inline-mech-code");
            inline_code.set_inner_html(&formatted_inline_block);
            inline_mech.append_child(&inline_code);

            let mut inline_view = document.create_element("span")?;
            inline_view.set_attribute("class", "mech-inline-mech-view");
            inline_view.set_attribute("id", &format!("{}",id));
            inline_mech.append_child(&inline_view);
          },
        }
      }
      Ok(inline_mech)
    }
    _ => Err(wasm_bindgen::JsValue::from_str("Expected Paragraph")),
  }  
}

fn render_inline_code(inline_code_node: &Node) -> Result<web_sys::Element, JsValue> {
  match inline_code_node {
    Node::InlineCode{children} => {
      let window = web_sys::window().expect("no global `window` exists");
      let document = window.document().expect("should have a document on window");
      let mut inline_code = document.create_element("span")?;
      inline_code.set_attribute("class", "mech-inline-code");
      for child in children {
        match child {
          Node::String{text} => {
            inline_code.set_inner_html(&text);
          },
          _ => (),
        }
      }
      Ok(inline_code)
    }
    _ => Err(wasm_bindgen::JsValue::from_str("Expected Paragraph")),
  }
}

fn render_paragraph(paragraph_node: &Node) -> Result<web_sys::Element, JsValue> {
  match paragraph_node {
    Node::Paragraph{children} => {
      let window = web_sys::window().expect("no global `window` exists");
      let document = window.document().expect("should have a document on window");
      let mut paragraph = document.create_element("p")?;
      for child in children {
        match child {
          Node::ParagraphText{text} => {
            let mut paragraph_text = document.create_element("span")?;
            paragraph_text.set_inner_html(&text);
            paragraph.append_child(&paragraph_text);
          },
          Node::InlineCode{..} => {
            let mut inline_code = render_inline_code(&child)?;
            paragraph.append_child(&inline_code);         
          },
          Node::InlineMechCode{..} => {
            let inline_mech = render_inline_mech(&child)?;
            paragraph.append_child(&inline_mech);         
          }
          _ => (),
        }
      }
      Ok(paragraph)
    }
    _ => Err(wasm_bindgen::JsValue::from_str("Expected Paragraph")),
  }
}

fn render_code_block(code_block_node: &Node) -> Result<web_sys::Element, JsValue> {
  match code_block_node {
    Node::CodeBlock{children} => {
      let window = web_sys::window().expect("no global `window` exists");
      let document = window.document().expect("should have a document on window");
      let mut code_block = document.create_element("pre")?;
      code_block.set_attribute("class", "mech-code-block");
      for child in children {
        let mut code = document.create_element("span")?;
        match child {
          Node::String{text} => {
            code.set_inner_html(&text);
          },
          _ => (),
        }
        code_block.append_child(&code);
      }
      Ok(code_block)
    },
    _ => Err(wasm_bindgen::JsValue::from_str("Expected Code Block")),
  }
}

fn render_unordered_list(unordered_list_node: &Node) -> Result<web_sys::Element, JsValue> {
  match unordered_list_node {
    Node::UnorderedList{children} => {
      let window = web_sys::window().expect("no global `window` exists");
      let document = window.document().expect("should have a document on window");
      let mut unordered_list = document.create_element("ul")?;
      for child in children {
        let mut list_item = document.create_element("li")?;
        match child {
          Node::ListItem{children} => {
            match &children[0] {
              Node::Paragraph{..} => {
                let mut paragraph = render_paragraph(&children[0])?;
                list_item.append_child(&paragraph);
              },
              _ => (),
            }
          },
          _ => (),
        }
        unordered_list.append_child(&list_item);
      }
      Ok(unordered_list)
    },
    _ => Err(wasm_bindgen::JsValue::from_str("Expected Unordered List")),
  }
}

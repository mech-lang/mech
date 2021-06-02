#![recursion_limit="256"]
#![feature(alloc)]
#![feature(drain_filter)]
#![feature(get_mut_unchecked)]
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
extern crate mech_utilities;
extern crate mech_math;
extern crate serde_json;
extern crate bincode;
#[macro_use]
extern crate lazy_static;

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use hashbrown::hash_set::HashSet;
use alloc::vec::Vec;
use core::fmt;
use mech_syntax::formatter::Formatter;
use mech_syntax::compiler::{Compiler, Node, Program, Section, Element};
use mech_core::{hash_string, ValueType, humanize, Block, ValueMethods, TableId, ErrorType, Transaction, BlockState, Change, TableIndex, Value, Table, Quantity, ToQuantity, NumberLiteralKind, QuantityMath};
use mech_utilities::{SocketMessage, MiniBlock};
use mech_math::{math_cos, math_sin, math_floor, math_round};
use web_sys::{ErrorEvent, MessageEvent, WebSocket, FileReader};
use std::sync::Arc;

#[macro_export]
macro_rules! log {
  ( $( $t:tt )* ) => {
    web_sys::console::log_1(&format!( $( $t )* ).into());
  }
}

lazy_static! {
  static ref HTML_APP: u64 = hash_string("html/app");
  static ref DIV: u64 = hash_string("div");
  static ref A: u64 = hash_string("a");
  static ref IMG: u64 = hash_string("img");
  static ref SRC: u64 = hash_string("src");
  static ref CONTAINS: u64 = hash_string("contains");
  static ref ROOT: u64 = hash_string("root");
  static ref TYPE: u64 = hash_string("type");
  static ref HREF: u64 = hash_string("href");
  static ref BUTTON: u64 = hash_string("button");
  static ref SLIDER: u64 = hash_string("slider");
  static ref MIN: u64 = hash_string("min");
  static ref MAX: u64 = hash_string("max");
  static ref VALUE: u64 = hash_string("value");
  static ref CANVAS: u64 = hash_string("canvas");
  static ref PARAMETERS: u64 = hash_string("parameters");
  static ref HEIGHT: u64 = hash_string("height");
  static ref WIDTH: u64 = hash_string("width");
  static ref SHAPE: u64 = hash_string("shape");
  static ref CIRCLE: u64 = hash_string("circle");
  static ref RECTANGLE: u64 = hash_string("rectangle");
  static ref LINE: u64 = hash_string("line");
  static ref PATH: u64 = hash_string("path");
  static ref START__POINT: u64 = hash_string("start-point");
  static ref LINE__WIDTH: u64 = hash_string("line-width");
  static ref START__ANGLE: u64 = hash_string("start-angle");
  static ref END__ANGLE: u64 = hash_string("end-angle");
  static ref QUADRATIC: u64 = hash_string("quadratic");
  static ref CONTROL__POINT: u64 = hash_string("control-point");
  static ref CONTROL__POINTS: u64 = hash_string("control-points");
  static ref END__POINT: u64 = hash_string("end-point");
  static ref X1: u64 = hash_string("x1");
  static ref X2: u64 = hash_string("x2");
  static ref Y1: u64 = hash_string("y1");
  static ref Y2: u64 = hash_string("y2");
  static ref RADIUS: u64 = hash_string("radius");
  static ref STROKE: u64 = hash_string("stroke");
  static ref FILL: u64 = hash_string("fill");
  static ref CENTER__X: u64 = hash_string("center-x");
  static ref CENTER__Y: u64 = hash_string("center-y");
  static ref IMAGE: u64 = hash_string("image");
  static ref X: u64 = hash_string("x");
  static ref Y: u64 = hash_string("y");
  static ref ROTATE: u64 = hash_string("rotate");
  static ref TRANSLATE: u64 = hash_string("translate");
  static ref SOURCE: u64 = hash_string("source");
  static ref TIME_TIMER: u64 = hash_string("time/timer");
  static ref PERIOD: u64 = hash_string("period");
  static ref TICKS: u64 = hash_string("ticks");
  static ref HTML_EVENT_POINTER__MOVE: u64 = hash_string("html/event/pointer-move");
  static ref HTML_EVENT_POINTER__DOWN: u64 = hash_string("html/event/pointer-down");
  static ref HTML_EVENT_POINTER__UP: u64 = hash_string("html/event/pointer-up");
  static ref HTML_EVENT_KEY__DOWN: u64 = hash_string("html/event/key-down");
  static ref HTML_EVENT_KEY__UP: u64 = hash_string("html/event/key-up");
  static ref TARGET: u64 = hash_string("target");
  static ref KEY: u64 = hash_string("key");
  static ref EVENT__ID: u64 = hash_string("event-id");
  static ref ARC: u64 = hash_string("arc");
  static ref ELLIPSE: u64 = hash_string("ellipse");
  static ref MAJOR__AXIS: u64 = hash_string("major-axis");
  static ref MINOR__AXIS: u64 = hash_string("minor-axis");
  static ref STARTING__ANGLE: u64 = hash_string("starting-angle");
  static ref ENDING__ANGLE: u64 = hash_string("ending-angle");
  static ref TEXT: u64 = hash_string("text");
  static ref FONT: u64 = hash_string("font");
  static ref SIZE: u64 = hash_string("size");
  static ref FACE: u64 = hash_string("face");
  static ref STYLE: u64 = hash_string("style");
  static ref WEIGHT: u64 = hash_string("weight");
  static ref BOLD: u64 = hash_string("bold");
  static ref NORMAL: u64 = hash_string("normal");
  static ref ITALIC: u64 = hash_string("italic");
  static ref FAMILY: u64 = hash_string("family");
  static ref DIRECTION: u64 = hash_string("direction");
  static ref ALIGNMENT: u64 = hash_string("alignment");
  static ref START: u64 = hash_string("start");
  static ref END: u64 = hash_string("end");
  static ref LEFT: u64 = hash_string("left");
  static ref RIGHT: u64 = hash_string("right");
  static ref CENTER: u64 = hash_string("center");
  static ref BEZIER: u64 = hash_string("bezier");
}

#[wasm_bindgen]
pub struct WasmCore {
  core: mech_core::Core,
  programs: Vec<Program>,
  changes: Vec<Change>,
  images: HashMap<u64, web_sys::HtmlImageElement>,
  canvases: HashSet<u64>,
  nodes: HashMap<u64, Vec<u64>>,
  views: HashSet<u64>,
  inline_views: HashSet<u64>,
  websocket: Option<web_sys::WebSocket>,
  remote_tables: HashMap<u64, (web_sys::WebSocket, HashSet<u64>)>,
  event_id: u32,
  timers: HashMap<usize,Closure<dyn FnMut()>>,
  applications: HashSet<u64>,
}

#[wasm_bindgen]
impl WasmCore {

  pub fn new(capacity: usize, recursion_limit: u64) -> WasmCore {
    let mut mech = mech_core::Core::new(capacity, recursion_limit);
    mech.load_standard_library();
    mech.runtime.load_library_function("math/sin",Some(math_sin));

    mech.insert_string("time/timer");
    mech.insert_string("period");
    mech.insert_string("ticks");
    mech.insert_string("html/event/click");
    mech.insert_string("html/event/pointer-move");
    mech.insert_string("html/event/pointer-down");
    mech.insert_string("html/event/pointer-up");
    mech.insert_string("html/event/key-down");
    mech.insert_string("html/event/key-up");
    mech.insert_string("x");
    mech.insert_string("y");
    mech.insert_string("target");
    mech.insert_string("event-id");

    let new_table = |table_id: u64, rows: usize, a: Vec<u64>, | {
      let mut changes = Vec::new();
      changes.push(Change::NewTable{
        table_id: table_id, 
        rows: rows,
        columns: a.len(),
      });
      for (ix, alias) in a.iter().enumerate() {
        changes.push(Change::SetColumnAlias{
          table_id: table_id,
          column_ix: (ix + 1) as usize,
          column_alias: *alias
        });
      }
      changes
    };

    let mut changes = vec![];
    changes.append(&mut new_table(*TIME_TIMER, 0, vec![*PERIOD, *TICKS]));
    changes.append(&mut new_table(*HTML_EVENT_POINTER__MOVE, 1, vec![*X, *Y, *TARGET, *EVENT__ID]));
    changes.append(&mut new_table(*HTML_EVENT_POINTER__DOWN, 1, vec![*X, *Y, *TARGET, *EVENT__ID]));
    changes.append(&mut new_table(*HTML_EVENT_POINTER__UP, 1, vec![*X, *Y, *TARGET, *EVENT__ID]));
    changes.append(&mut new_table(*HTML_EVENT_KEY__DOWN, 1, vec![*KEY, *EVENT__ID]));
    changes.append(&mut new_table(*HTML_EVENT_KEY__UP, 1, vec![*KEY, *EVENT__ID]));

    let txn = Transaction{changes};
    mech.process_transaction(&txn);

    WasmCore {
      core: mech,
      programs: Vec::new(),
      changes: Vec::new(),
      images: HashMap::new(),
      canvases: HashSet::new(),
      nodes: HashMap::new(),
      views: HashSet::new(),
      inline_views: HashSet::new(),
      websocket: None,
      remote_tables: HashMap::new(),
      event_id: 0,
      timers: HashMap::new(),
      applications: HashSet::new(),
    }
  }
  
  pub fn start_websocket(&mut self, address: String) -> Result<(), JsValue> {
    let ws = WebSocket::new(&address)?;
    ws.set_binary_type(web_sys::BinaryType::Arraybuffer);
    let wasm_core = self as *mut WasmCore;
    // create callback
    let cloned_ws = ws.clone();
   
    // OnMessage
    {
      let wasm_core = self as *mut WasmCore;
      let onmessage_callback = Closure::wrap(Box::new(move |e: MessageEvent| {
        if let Ok(abuf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
          let array = js_sys::Uint8Array::new(&abuf);
          let len = array.byte_length() as usize;
          let msg: Result<SocketMessage, bincode::Error> = bincode::deserialize(&array.to_vec());
          match msg {
            Ok(SocketMessage::Transaction(txn)) => {
              unsafe {
                (*wasm_core).core.process_transaction(&txn);
                (*wasm_core).add_application();
                (*wasm_core).render();
              }
            }
            msg => log!("{:?}", msg),
          }
          /*
          // here you can for example use Serde Deserialize decode the message
          // for demo purposes we switch back to Blob-type and send off another binary message
          cloned_ws.set_binary_type(web_sys::BinaryType::Blob);
          match cloned_ws.send_with_u8_array(&vec![5, 6, 7, 8]) {
            Ok(_) => log!("binary message successfully sent"),
            Err(err) => log!("error sending message: {:?}", err),
          }*/
        } else if let Ok(blob) = e.data().dyn_into::<web_sys::Blob>() {
          log!("message event, received blob: {:?}", blob);
          // better alternative to juggling with FileReader is to use https://crates.io/crates/gloo-file
          let fr = web_sys::FileReader::new().unwrap();
          let fr_c = fr.clone();
          // create onLoadEnd callback
          let onloadend_cb = Closure::wrap(Box::new(move |_e: web_sys::ProgressEvent| {
            let array = js_sys::Uint8Array::new(&fr_c.result().unwrap());
            let len = array.byte_length() as usize;
            log!("Blob received {}bytes: {:?}", len, array.to_vec());
            // here you can for example use the received image/png data
          }) as Box<dyn FnMut(web_sys::ProgressEvent)>);
          fr.set_onloadend(Some(onloadend_cb.as_ref().unchecked_ref()));
          fr.read_as_array_buffer(&blob).expect("blob not readable");
          onloadend_cb.forget();
        } else if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
          log!("message event, received Text: {:?}", txt);
        } else {
          log!("message event, received Unknown: {:?}", e.data());
        }
      }) as Box<dyn FnMut(MessageEvent)>);
      ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
      onmessage_callback.forget();
    }

    // OnError
    let onerror_callback = Closure::wrap(Box::new(move |e: ErrorEvent| {
      log!("error event: {:?}", e);
    }) as Box<dyn FnMut(ErrorEvent)>);
    ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
    onerror_callback.forget();

    // OnOpen
    {
      let wasm_core = self as *mut WasmCore;
      let cloned_ws = ws.clone();
      let onopen_callback = Closure::wrap(Box::new(move |_| {
        // Upon an open connection, send the server a list of tables about which we want updates
        unsafe {
          for input_table_id in (*wasm_core).core.runtime.needed_registers.iter() {
            let result = bincode::serialize(&SocketMessage::Listening(input_table_id.clone())).unwrap();
            // send off binary message
            match cloned_ws.send_with_u8_array(&result) {
              Ok(_) => log!("binary message successfully sent"),
              Err(err) => log!("error sending message: {:?}", err),
            }
          }
        }
      }) as Box<dyn FnMut(JsValue)>);
      ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
      onopen_callback.forget();
    }

    // On Close
    {
      let onclose_callback = Closure::wrap(Box::new(move |event: web_sys::Event| {
        log!("Closing");
      }) as Box<dyn FnMut(_)>);
      ws.set_onclose(Some(&onclose_callback.as_ref().unchecked_ref()));
      onclose_callback.forget();
    }

    // Todo, make sef.websocket int oa vector of websockets.
    self.websocket = Some(ws);
    Ok(())
  }
/*
  pub fn connect_remote_cores(&mut self) {
    let wasm_core = self as *mut Core;
    match self.core.get_table("mech/remote-cores".to_string()) {
      Some(table_ref_rc) => {
        let table_ref = table_ref_rc.borrow();
        for i in 0..table_ref.rows as usize {
          let address = table_ref.data[1][i].as_string().unwrap().clone();
          unsafe {
            (*wasm_core).start_websocket(address);
          }
        }
      }
      _ => (),
    }
  }

  pub fn compile_code(&mut self, code: String) {
    let mech_code = Hasher::hash_str("mech/code");
    let changes = vec![
      Change::NewTable{id: mech_code, rows: 1, columns: 1},
      Change::Set{table_id: mech_code, value: vec![(mech_core::TableIndex::Index(1), mech_core::TableIndex::Index(1), Value::from_str(&code))]},
    ];
    let mut compiler = Compiler::new();
    compiler.compile_string(code);
    self.core.register_blocks(compiler.blocks.clone());
    self.core.step();
    self.core.process_transaction(&Transaction{changes});
    self.programs = compiler.programs.clone();
    //self.render_program();
    log!("Compiled {} blocks.", compiler.blocks.len());
  }*/
  
  pub fn add_timers(&mut self) {
    let window = web_sys::window().expect("no global `window` exists");
   
    match self.core.get_table(hash_string("time/timer")) {
      Some(timers_table) => {
        for row in 1..=timers_table.rows {
          match self.timers.entry(row) {
            Entry::Occupied(timer) => {
       
            },
            Entry::Vacant(v) => {
              self.changes.push(Change::Set{
                table_id: *TIME_TIMER, values: vec![
                (TableIndex::Index(1), 
                TableIndex::Alias(*TICKS),
                Value::from_u64(0))],
              });             
              self.process_transaction();
              let (period, _) = timers_table.get_u64(&TableIndex::Index(row), &TableIndex::Alias(*PERIOD)).unwrap();

              let wasm_core = self as *mut WasmCore;
              let closure = || { 
                Closure::wrap(Box::new(move || {
                  unsafe{
                    let table = (*wasm_core).core.get_table(hash_string("time/timer")).unwrap();
                    let (ticks, _) = table.get_u64(&TableIndex::Index(1), &TableIndex::Alias(*TICKS)).unwrap();
                    (*wasm_core).changes.push(Change::Set{
                      table_id: *TIME_TIMER, values: vec![
                      (TableIndex::Index(row), 
                      TableIndex::Alias(*TICKS),
                      Value::from_u64(ticks+1))],
                    });           
                    (*wasm_core).process_transaction();
                    (*wasm_core).render();
                    //let table = (*wasm_core).core.get_table_by_name("mouth").unwrap();
                    //log!("{:?}", table);
                    //log!("{:?}", table.get_f32(&TableIndex::Index(1),&TableIndex::Index(4)).unwrap());
                  }
                }) as Box<dyn FnMut()>)
              };
              let timer_callback = closure();
              let id = window.set_interval_with_callback_and_timeout_and_arguments_0(
                timer_callback.as_ref().unchecked_ref(),
                period as i32
              ).unwrap();
              self.timers.insert(row,timer_callback);
            }
          }
        }
      }
      _ => (),
    }    
  }


  pub fn load_blocks(&mut self, serialized_miniblocks: Vec<u8>) {
    let miniblocks: Vec<MiniBlock> = bincode::deserialize(&serialized_miniblocks).unwrap();
    let mut blocks: Vec<Block> = Vec::new() ;
    for miniblock in miniblocks {
      let mut block = Block::new(100);
      let store = unsafe{&mut *Arc::get_mut_unchecked(&mut block.store)};
      for (key, value) in miniblock.strings {
        store.strings.insert(key, value.to_string());
      }
      for (key, value) in miniblock.number_literals {
        store.number_literals.insert(key, value);
      }
      for tfms in miniblock.transformations {
        block.register_transformations(tfms);
      }
      
      block.plan = miniblock.plan.clone();
      block.gen_id();
      blocks.push(block);
    }
    let len = blocks.len();
    self.core.register_blocks(blocks);
    self.core.step();
    self.add_timers();
      
    log!("Loaded {} blocks.", len);
  }

  pub fn render_program(&mut self) -> Result<(), JsValue>  {
    /*
    let new_table = |s: String, a: Vec<String>| {
      let mut changes = Vec::new();
      let table_id = Hasher::hash_string(s.clone());
      log!("{:?} -> {:0x}",&s, table_id);
      changes.push(Change::NewTable{
        id: table_id, 
        rows: 1, 
        columns: a.len() as u64,
      });
      for (ix, alias) in a.iter().enumerate() {
        let alias_id = Hasher::hash_str(alias);
        changes.push(Change::RenameColumn{
          table: table_id,
          column_ix: (ix + 1) as u64,
          column_alias: alias_id
        });
      }
      changes
    };
    let mut changes = vec![];
    changes.append(&mut new_table("html/event/click".to_string(), vec!["x".to_string(),"y".to_string()]));
    changes.append(&mut new_table("html/event/pointermove".to_string(), vec!["x".to_string(),"y".to_string()]));
    changes.append(&mut new_table("html/event/pointerdown".to_string(), vec!["x".to_string(),"y".to_string()]));
    changes.append(&mut new_table("html/event/keydown".to_string(), vec!["key".to_string(),"id".to_string()]));
    changes.append(&mut new_table("html/event/keyup".to_string(), vec!["key".to_string(),"id".to_string()]));

    let txn = Transaction{changes};
    self.core.process_transaction(&txn);

    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    let body = document.body().expect("document should have a body");
    let wasm_core = self as *mut Core;

    {
      let closure = |i| { Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
        let window = web_sys::window().expect("no global `window` exists");
        let document = window.document().expect("should have a document on window");
        let key = event.key();
        let table_id = Hasher::hash_str(i);
        // TODO Make this safe
        unsafe {
          (*wasm_core).changes.push(Change::Set{
            table_id: table_id, 
            values: vec![(TableIndex::Index(1), 
            TableIndex::Index(1),
            Value::from_string(key.to_string()))],
          });    
          (*wasm_core).changes.push(Change::Set{
            table_id: table_id, 
            values: vec![(TableIndex::Index(1), 
            TableIndex::Index(2),
            Value::from_f64(event.time_stamp()))],
          });               
          (*wasm_core).process_transaction();
          (*wasm_core).render();
        }
      }) as Box<dyn FnMut(_)>)
      };
      let keydown_callback = closure("html/event/keydown");
      document.add_event_listener_with_callback("keydown", keydown_callback.as_ref().unchecked_ref())?;
      keydown_callback.forget();
    }

    {
      let closure = |i| { Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
        let window = web_sys::window().expect("no global `window` exists");
        let document = window.document().expect("should have a document on window");
        let key = event.key();
        let table_id = Hasher::hash_str(i);
        // TODO Make this safe
        unsafe {
          (*wasm_core).changes.push(Change::Set{
            table_id: table_id, 
            values: vec![(TableIndex::Index(1), 
            TableIndex::Index(1),
            Value::from_string(key.to_string()))],
          });    
          (*wasm_core).changes.push(Change::Set{
            table_id: table_id, 
            values: vec![(TableIndex::Index(1), 
            TableIndex::Index(2),
            Value::from_f64(event.time_stamp()))],
          });               
          (*wasm_core).process_transaction();
          (*wasm_core).render();
        }
      }) as Box<dyn FnMut(_)>)
      };
      let keydown_callback = closure("html/event/keyup");
      document.add_event_listener_with_callback("keyup", keydown_callback.as_ref().unchecked_ref())?;
      keydown_callback.forget();
    }

    // Add an event listener to mech-app that removes modals on pointer click
    {
      let closure = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
        let window = web_sys::window().expect("no global `window` exists");
        let document = window.document().expect("should have a document on window");
        match document.get_element_by_id("mech-modal") {
          Some(modal) => {
            let parent = modal.parent_node().unwrap();
            parent.remove_child(&modal);
          },
          _ => (),
        };
      }) as Box<dyn FnMut(_)>);
      let app = document.get_element_by_id("mech-app").unwrap();
      app.add_event_listener_with_callback("pointerdown", closure.as_ref().unchecked_ref())?;
      closure.forget();
    }

    let mut documentation = document.create_element("div")?;
    documentation.set_attribute("class", "mech-docs");
    documentation.set_attribute("id", "mech-docs");
    let mut contents = document.create_element("div")?;
    contents.set_attribute("class", "mech-contents");


    let wasm_core = self as *mut Core;
    for program in &self.programs {
      
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
      rendered_program.set_attribute("id", "mech-program");
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
                  event.stop_propagation();
                  let target: web_sys::HtmlElement = event.target()
                                                          .unwrap()
                                                          .dyn_into::<web_sys::HtmlElement>()
                                                          .map_err(|_| ())
                                                          .unwrap();
                  let block_id = target.parent_node().unwrap()
                                                     .dyn_into::<web_sys::HtmlElement>()
                                                     .map_err(|_| ())
                                                     .unwrap()
                                                     .get_attribute("block-id")
                                                     .unwrap_or("0".to_string())
                                                     .parse::<usize>().unwrap();

                  let window = web_sys::window().expect("no global `window` exists");
                  let document = window.document().expect("should have a document on window");
                    
                  // Remove previous modal
                  match document.get_element_by_id("mech-modal") {
                    Some(modal) => {
                      let parent = modal.parent_node().unwrap();
                      parent.remove_child(&modal);
                    },
                    _ => (),
                  };

                  // Get the table data
                  let table_id = Hasher::hash_str(&target.inner_text());
                  let table_name = target.inner_text();
                  let (data, scope) = 
                  // Format local variable
                  if target.get_attribute("class").unwrap_or("".to_string()) == "highlight-local-variable"{ 
                    let mut output = format!("<h3>{}</h3>", table_name);
                    unsafe {
                      let table_string = (*wasm_core).draw_table(block_id, table_id);
                      output = format!("{}{}",output, table_string);
                    }
                    (output, "local")
                  // Format global variable
                  } else if target.get_attribute("class").unwrap_or("".to_string()) == "highlight-global-variable" {
                    let mut output = format!("<h3><span class=\"highlight-bracket\">#</span>{}</h3><table>", table_name);
                    unsafe {
                      let table_string = (*wasm_core).draw_table(0, table_id);
                      output = format!("{}{}",output, table_string);
                    }
                    (output, "global")
                  } else {
                    ("".to_string(), "")
                  };

                  if data != "" {
                    // Set new modal
                    let mut modal = document.create_element("div").unwrap()
                                                                  .dyn_into::<web_sys::HtmlElement>()
                                                                  .map_err(|_| ())
                                                                  .unwrap();
                    modal.set_attribute("id", "mech-modal");
                    let mut table_inspector = document.create_element("div").unwrap();
                    table_inspector.set_attribute("class", "mech-table-inspector");
                    table_inspector.set_attribute("id", "mech-table-inspector");
                    table_inspector.set_attribute("block-id", &format!("{:?}", block_id));
                    table_inspector.set_attribute("table-id", &format!("{:?}", table_id));
                    table_inspector.set_attribute("table-name", &table_name);
                    table_inspector.set_attribute("table-scope", &scope);
                    table_inspector.set_inner_html(&data);
                    modal.append_child(&table_inspector);
                    let mut app = document.get_element_by_id("mech-app").unwrap();
                    app.append_child(&modal);
                    log!("{:?} {:?}", modal.offset_width(), modal.offset_height());
                    let x = if event.client_x() - modal.offset_width() / 2 > 0 {
                      event.client_x() - modal.offset_width() / 2
                    } else {
                      0
                    };
                    let y = if event.client_y() - modal.offset_height() - 10 > 0 {
                      event.client_y() - modal.offset_height() - 10
                    } else {
                      0
                    };
                    modal.set_attribute("style", &format!("left: {}px; top: {}px;", x, y));
                  }
                  
                }) as Box<dyn FnMut(_)>);
                code_text.add_event_listener_with_callback("pointerdown", closure.as_ref().unchecked_ref())?;
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
                  let block_id = target.get_attribute("block-id").unwrap().parse::<usize>().unwrap();

                  if event.key_code() == 13 && event.ctrl_key() {
                    let mut compiler = Compiler::new();
                    let block_ast = compiler.compile_block_string(target.inner_text());
                    compiler.compile_block(block_ast);
                    let mut new_block = &mut compiler.blocks[0];
                    new_block.id = block_id;
                    unsafe {
                      (*core).remove_block(&block_id);
                      (*core).register_blocks(vec![new_block.clone()]);
                      (*core).runtime.ready_blocks.insert(block_id);
                      (*core).step();
                      (*wasm_core).render();
                    }
                    if new_block.errors.is_empty() && target.get_attribute("state").unwrap() == "error".to_string() {
                      target.set_attribute("state","ready");
                      target.set_attribute("class","mech-code");
                      let parent = target.parent_node().unwrap();
                      let error_box = parent.last_child().unwrap();
                      parent.remove_child(&error_box);
                    }
                    // Remove error table.
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
                  let table = table.borrow();
                  let mut view = document.create_element("div")?;
                  view.set_attribute("class", "mech-view");
                  view.set_id(&format!("{}",block_id));
                  self.views.insert(*block_id as u64);
                  let mut output = "".to_string();
                  let view_type = table.get_column(&TableIndex::Alias(Hasher::hash_str("type")));
                  let x_pts = table.get_column(&TableIndex::Alias(Hasher::hash_str("x")));
                  let y_pts = table.get_column(&TableIndex::Alias(Hasher::hash_str("y")));
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
                          let x_data = block.get_table(x_pt_table[0].as_u64().unwrap()).unwrap().borrow();
                          let y_data = block.get_table(y_pt_table[0].as_u64().unwrap()).unwrap().borrow();

                          for i in 0..x_data.rows as usize {
                            let y = &y_data.data[0][i].as_f64().unwrap();
                            let x = &x_data.data[0][i].as_f64().unwrap(); {
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
              // Add an error indication if there is one
              if block.state == BlockState::Error {
                let mut error_view = document.create_element("div")?;
                error_view.set_attribute("class", "mech-error-view");
                log!("{:?}", block.errors);
                // Write error text
                for error in &block.errors {
                  let error_text = match error.error_id {
                    ErrorType::DuplicateAlias(alias_id) => {
                      let alias = &self.core.store.names.get(&alias_id).unwrap();
                      format!("Local table {:?} defined more than once.", alias)
                    },
                    ErrorType::DomainMismatch(d1,d2) => {
                      let get_unit_label = |i| match i {
                        1 => "weight",
                        2 => "length",
                        _ => "",
                      };
                      format!("Tried to add units of {} with {}", get_unit_label(d1), get_unit_label(d2))
                    },
                    _ => "".to_string(),
                  };
                  error_view.set_inner_html(&error_text);
                }
                code.append_child(&error_view);
              }
              rendered_section.append_child(&code);
            },
            _ => (),
          }
        }
        rendered_program.append_child(&rendered_section)?;
      }
      //documentation.append_child(&controls);
      documentation.append_child(&contents);
      documentation.append_child(&rendered_program)?;
      //documentation.append_child(&drawing)?;
     
      let editor_container = document.get_element_by_id("mech-editor-container").unwrap();
      editor_container.append_child(&documentation)?;

      // Create drawing area
      let mut drawing = document.create_element("div")?;
      drawing.set_attribute("id", "drawing");
      editor_container.append_child(&drawing);

      // Register inline views
      let inline_view_elements = document.get_elements_by_class_name("mech-inline-mech-view");
      for ix in 0..inline_view_elements.length() {
        let view = inline_view_elements.item(ix).unwrap();
        let id = view.id().parse::<u64>().unwrap();
        self.inline_views.insert(id);
        let view_table = self.core.get_table(id).unwrap();
        let data = &view_table.data[0][0];
        view.set_inner_html(&data.as_string().unwrap());
      }
      // Set block status
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
            code_block.set_attribute("state","pending");
          }
          BlockState::Error => {
            let class = code_block.get_attribute("class").unwrap();
            let class = format!("{} error", class);
            code_block.set_attribute("class", &class);
            code_block.set_attribute("state","error");
          }
          _ =>()
        }
      }
    }*/
    Ok(())
  }
/*
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
    self.inline_views.clear();
    self.roots.clear();
    log!("Core Cleared");
  }

  /*
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
  }*/

  pub fn display_core(&self) {
    log!("{:?}", self.core);
  }

  pub fn display_runtime(&self) {
    log!("{:?}", self.core.runtime);
  }

  /*pub fn display_changes(&self) {
    for change in &self.core.store.changes {
      log!("{:?}", change);
    }
  }*/

  fn render_view(&mut self, view: u64) -> Result<(), JsValue> {

    let mut output = "".to_string();

    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    let view_id = Hasher::hash_str("block/view");
    let view_node = document.get_element_by_id(&format!("{}",view)).unwrap();
    let block = &self.core.runtime.blocks.get(&(view as usize)).unwrap();
    let table = block.get_table(view_id).unwrap().borrow();

    let view_type = table.get_column(&TableIndex::Alias(Hasher::hash_str("type")));
    let x_pts = table.get_column(&TableIndex::Alias(Hasher::hash_str("x")));
    let y_pts = table.get_column(&TableIndex::Alias(Hasher::hash_str("y")));

    match (view_type, x_pts, y_pts) {
      (Some(view_type), Some(x_pt_table), Some(y_pt_table)) => { 
        match view_type[0].as_string().unwrap().as_ref() {
          // Draw a scatter plot
          "scatter" => {
            
            let mut canvas = view_node.first_child().unwrap();
            
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
            let x_data = block.get_table(x_pt_table[0].as_u64().unwrap()).unwrap().borrow();
            let y_data = block.get_table(y_pt_table[0].as_u64().unwrap()).unwrap().borrow();

            for i in 0..x_data.rows as usize {
              let y = &y_data.data[0][i].as_f64().unwrap();
              let x = &x_data.data[0][i].as_f64().unwrap(); {
                context.save();
                context.begin_path();
                context.arc(*x, *y, 2.0, 0.0, 1.0 * 3.14);
                context.set_fill_style(&JsValue::from_str("#000000"));
                context.fill();  
                context.restore();
              }
            }
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
        view_node.set_inner_html(&output);
      },
    }
    Ok(())
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
        let elements = match canvas.get_attribute("elements") {
          Some(elements) => elements,
          _ => continue,
        };
        let elements = canvas.get_attribute("elements").unwrap();
        let elements_table_id: u64 = elements.parse::<u64>().unwrap();
        unsafe {
          if (*wasm_core).core.runtime.changed_this_round.contains(&(elements_table_id, TableIndex::Index(0))) {
            (*wasm_core).render_canvas(&canvas);
          }
        }
      }
    }) as Box<FnMut()>);
    window.request_animation_frame(closure.as_ref().unchecked_ref());
    closure.forget();

    // render nodes
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    for (k,v) in self.nodes.iter() {
      for node in v {
        match document.get_element_by_id(&format!("{:?}",node)) {
          Some(mut div) => {
            let row = div.get_attribute("row").unwrap().parse::<usize>().unwrap();
            let table = &self.core.store.tables.get(*k).unwrap().borrow();
            match &table.data[2][row - 1] {
              Value::String(value) => div.set_inner_html(&value),
              Value::Number(value) => div.set_inner_html(&format!("{:?}", value.to_float())),
              Value::Reference(TableId::Local(table_ref)) => {
                let child_nodes = div.children();
                let child = child_nodes.get_with_index(0).unwrap();
                if *table_ref != child.id().parse::<u64>().unwrap() {
                  //div.remove_child(&child);
                  unsafe {
                    let referenced_table: &Table = &(*wasm_core).core.store.get_table(*table_ref).unwrap().borrow();
                    //(*wasm_core).draw_contents(&referenced_table, &mut div);
                  }
                }
              },
              _ => (),
            };
                            
          }
          // TODO Remove old nodes from the table if they aren't in the DOM tree anymore.
          _ => (),
        }
      }
      //log!("{:?}--{:?}",k,v);
    }

    // render views
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    let view_id = Hasher::hash_str("block/view");
    for view in self.views.iter() {
      unsafe {
        (*wasm_core).render_view(*view);
      }
    }

    // render inline views
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    let view_id = Hasher::hash_str("block/view");
    for view in self.inline_views.iter() {
      let view_node = document.get_element_by_id(&format!("{}",view)).unwrap();
      let table = &self.core.store.tables.get(*view).unwrap().borrow();
      let mut output = format!("{}", &table.data[0][0].as_string().unwrap());
      view_node.set_inner_html(&output);
    }

    // render inspector
    match document.get_element_by_id("mech-table-inspector") {
      Some(table_inspector) => {
        //log!("{:?}", self.core.runtime.changed_this_round);
        match table_inspector.get_attribute("table-scope").unwrap_or("".to_string()).as_ref() {
          "local" => {
            let table_name = table_inspector.get_attribute("table-name").unwrap();
            let table_id = table_inspector.get_attribute("table-id").unwrap().parse::<u64>().unwrap();
            let block_id = table_inspector.get_attribute("block-id").unwrap().parse::<usize>().unwrap();
            let mut output = format!("<h3>{}</h3>{}", table_name, self.draw_table(block_id, table_id));           
            table_inspector.set_inner_html(&output);
          },
          "global" => {
            let table_name = table_inspector.get_attribute("table-name").unwrap();
            let table_id = table_inspector.get_attribute("table-id").unwrap().parse::<u64>().unwrap();
            let mut output = format!("<h3><span class=\"highlight-bracket\">#</span>{}</h3>{}", table_name, self.draw_table(0, table_id));           
            table_inspector.set_inner_html(&output);
          },
          _ => log!("Unknown"),
        };
      }
      _ => (),
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

  fn draw_table(&self, block_id: usize, table_id: u64) -> String { 
    let table = match block_id {
      0 => self.core.store.get_table(table_id).unwrap().borrow(),
      _ => self.core.runtime.blocks.get(&block_id).unwrap().get_table(table_id).unwrap().borrow(),
    };
    let mut output = String::from("<table>");
    if table.column_aliases.len() > 0 {
      output = format!("{}<tr>",output);
      for alias in table.column_index_to_alias.iter() {
        match alias {
          Some(alias_id) => {
            output = format!("{}<th>",output);
            output = format!("{} {}",output, self.core.store.names.get(&alias_id).unwrap());
            output = format!("{}</th>",output);
          },
          _ => (),
        }
      }
      output = format!("{}</tr>",output);
    }
    for i in 0..table.rows {
      output = format!("{}<tr>",output);
      for j in 0..table.columns {
        output = format!("{}<td>",output);
        let cell_content: String = match &table.data[j as usize][i as usize] {
          Value::Reference(TableId::Local(r)) => self.draw_table(0, *r),
          x => x.as_string().unwrap(),
        };
        output = format!("{} {}", output, cell_content);
        output = format!("{}</td>",output);
      }
      output = format!("{}</tr>",output);
    }
    output = format!("{}</table>",output); 
    output
  }

  /*pub fn queue_change(&mut self, table: String, row: u32, column: u32, value: i32) {
    let table_id = Hasher::hash_string(table);
    let change = Change::Set{table_id: table_id, 
                             values: vec![(TableIndex::Index(row as usize), 
                             TableIndex::Index(column as usize),
                             Value::from_i64(value as i64))],
                            };
    self.changes.push(change);
  }*/
  */
  pub fn process_transaction(&mut self) {
    //if !self.core.paused {
      let txn = Transaction{changes: self.changes.clone()};
      //let pre_changes = self.core.store.len();
      self.core.process_transaction(&txn);
      //self.render();
      /*
      for (id, (ws, remote_tables)) in self.remote_tables.iter() {
        let mut changes: Vec<Change> = Vec::new();
        for i in pre_changes..self.core.store.len() {
          let change = &self.core.store.changes[i-1];
          match change {
            Change::Set{table_id, ..} => {
              match remote_tables.contains(&table_id) {
                true => changes.push(change.clone()),
                _ => (),
              }
            }
            _ => ()
          } 
        }
        let txn = Transaction{changes};
        let txn_msg = serde_json::to_string(&WebsocketMessage::Transaction(txn.clone())).unwrap();
        ws.send_with_str(&txn_msg);
      }*/
    //}
    self.changes.clear();
  }
  /*
  /*pub fn get_mantissas(&mut self, table: String, column: u32) -> Vec<i32> {
      let table_id = Hasher::hash_string(table);
      let mut output: Vec<i32> = vec![];
      match self.core.store.get_column(TableId::Global(table_id), TableIndex::Index(column as u64)) {
          Some(column) => {
              for row in column {
                  output.push(row.as_quantity().unwrap().mantissa() as i32);
              }
          }
          _ => log!("{} not found", table_id),
      }
      output
  }*/

  /*pub fn get_ranges(&mut self, table: String, column: u32) -> Vec<i32> {
      let table_id = Hasher::hash_string(table);    
      let mut output: Vec<i32> = vec![];
      match self.core.store.get_column(TableId::Global(table_id), TableIndex::Index(column as usize)) {
          Some(column) => {
              for row in column {
                  output.push(row.as_quantity().unwrap().range() as i32);
              }
          }
          _ => log!("{} not found", table_id),
      }
      output
  }*/

  /*pub fn get_column(&mut self, table: String, column: u32) -> Vec<f32> {
      let table_id = Hasher::hash_string(table);    
      let mut output: Vec<f32> = vec![];
      match self.core.get_column(TableId::Global(table_id), TableIndex::Index(column as usize)) {
          Some(column) => {
              for row in column {
                  output.push(row.as_quantity().unwrap().to_float() as f32);
              }
          }
          _ => log!("{} not found", table_id),
      }
      output
  }*/

  */

  pub fn init(&mut self) -> Result<(), JsValue> {
    let wasm_core = self as *mut WasmCore;
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");

    {
      let key_closure = |table_id| { 
        Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
          let window = web_sys::window().expect("no global `window` exists");
          let document = window.document().expect("should have a document on window");
          let key = event.key();
          // TODO Make this safe
          unsafe {
            (*wasm_core).changes.push(Change::Set{
              table_id: table_id, 
              values: vec![(TableIndex::Index(1), 
              TableIndex::Alias(*KEY),
              Value::from_string(&key.to_string()))],
            });    
            (*wasm_core).event_id += 1;
            let eid = (*wasm_core).event_id;
            (*wasm_core).changes.push(Change::Set{
              table_id: table_id, values: vec![
              (TableIndex::Index(1), 
              TableIndex::Alias(*EVENT__ID),
              Value::from_u32(eid))],
            });           
            (*wasm_core).process_transaction();
            (*wasm_core).render();
            //let table = (*wasm_core).core.get_table(hash_string("balls"));
            //log!("{:?}", table);
          }
        }) as Box<dyn FnMut(_)>)
      };
      let keydown_callback = key_closure(*HTML_EVENT_KEY__DOWN);
      document.add_event_listener_with_callback("keydown", keydown_callback.as_ref().unchecked_ref())?;
      let keyup_callback = key_closure(*HTML_EVENT_KEY__UP);
      document.add_event_listener_with_callback("keyup", keyup_callback.as_ref().unchecked_ref())?;
      keydown_callback.forget();
      keyup_callback.forget();
    }
    {
      let pointer_closure = |table_id| { 
        Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
          let window = web_sys::window().expect("no global `window` exists");
          let document = window.document().expect("should have a document on window");
          let target = event.target().unwrap();
          let target_element = target.dyn_ref::<web_sys::HtmlElement>().unwrap();
          let target_table_id = target_element.id().parse::<u64>().unwrap();
          //log!("{:?}", target_element.id().parse::<u64>().unwrap());

          let x = event.offset_x();
          let y = event.offset_y();
          //log!("event: {:?} {:?}", x, y);
          // TODO Make this safe
          unsafe {

            (*wasm_core).changes.push(Change::Set{
              table_id: table_id, values: vec![
              (TableIndex::Index(1), 
              TableIndex::Alias(*X),
              Value::from_i32(x as i32))],
            });
            (*wasm_core).changes.push(Change::Set{
              table_id: table_id, values: vec![
              (TableIndex::Index(1), 
              TableIndex::Alias(*Y),
              Value::from_i32(y as i32))],
            });              
            (*wasm_core).changes.push(Change::Set{
              table_id: table_id, values: vec![
              (TableIndex::Index(1), 
              TableIndex::Alias(*TARGET),
              Value::from_id(target_table_id))],
            });            
            (*wasm_core).event_id += 1;
            let eid = (*wasm_core).event_id;
            (*wasm_core).changes.push(Change::Set{
              table_id: table_id, values: vec![
              (TableIndex::Index(1), 
              TableIndex::Alias(*EVENT__ID),
              Value::from_u32(eid))],
            });           
            (*wasm_core).process_transaction();
            (*wasm_core).render();
            //let table = (*wasm_core).core.get_table(hash_string("clicked"));
            //log!("{:?}", table);
          }
        }) as Box<dyn FnMut(_)>)
      };
      let move_callback = pointer_closure(*HTML_EVENT_POINTER__MOVE);
      document.add_event_listener_with_callback("pointermove", move_callback.as_ref().unchecked_ref())?;
      let down_callback = pointer_closure(*HTML_EVENT_POINTER__DOWN);
      document.add_event_listener_with_callback("pointerdown", down_callback.as_ref().unchecked_ref())?;
      let up_callback = pointer_closure(*HTML_EVENT_POINTER__UP);
      document.add_event_listener_with_callback("pointerup", up_callback.as_ref().unchecked_ref())?;

      move_callback.forget();
      down_callback.forget();
      up_callback.forget();
    }
    Ok(())
  }

  pub fn add_application(&mut self) -> Result<(), JsValue> {
    let wasm_core = self as *mut WasmCore;
    let table = self.core.get_table(*HTML_APP);
    match table {
      Some(app_table) => {
        let window = web_sys::window().expect("no global `window` exists");
        let document = window.document().expect("should have a document on window");
        for row in 1..=app_table.rows as usize {
          match (app_table.get(&TableIndex::Index(row), &TableIndex::Alias(*ROOT)), 
                 app_table.get(&TableIndex::Index(row), &TableIndex::Alias(*CONTAINS))) {
            (Some((root_id,_)), Some((contents,_))) => {
              match self.applications.contains(&root_id) {
                true => continue,
                false => {
                  self.applications.insert(root_id.clone());
                  let root_string_id = &self.core.get_string(&root_id).unwrap();
                  match document.get_element_by_id(&root_string_id) {
                    Some(drawing_area) => {
                      let app = self.render_value(contents)?;
                      drawing_area.append_child(&app)?;
                    }
                    _ => {log!("No drawing area found.");},
                  }
                }
              }
            }
            _ => {log!("No root or contents column in #html/app");}, // TODO Alert user there is no root and or contents column in app_table
          }        
        }
      }
      _ => {log!("No #html/app in the core");}, // TODO Alert the user no app was found
    }
    Ok(())
  }

  pub fn render(&mut self) -> Result<(), JsValue> {
    let wasm_core = self as *mut WasmCore;
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    self.render_canvases();
    Ok(())
  }

  fn render_value(&mut self, value: Value) -> Result<web_sys::Element, JsValue> {
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    let mut div = document.create_element("div")?;
    match value.value_type() {
      ValueType::String => {
        let str_hash = value.as_string().unwrap();
        let contents_string = self.core.get_string(&str_hash).unwrap();
        div.set_inner_html(&contents_string);
      },
      ValueType::Quantity => {
        let quantity = value.as_f64().unwrap();
        div.set_inner_html(&format!("{:?}", quantity));
      }
      ValueType::Reference => {
        let reference = value.as_reference().unwrap();
        let table = self.core.get_table(reference).unwrap();
        let rendered_ref = self.make_element(&table)?;
        div.append_child(&rendered_ref)?;
      }
      _ => (), // TODO Unhandled Boolean and Empty
    }
    Ok(div)
  }

  fn make_element(&mut self, table: &Table) -> Result<web_sys::Element, JsValue> {
    let wasm_core = self as *mut WasmCore;
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    let mut container: web_sys::Element = document.create_element("div")?;
    let element_id = hash_string(&format!("div-{:?}", table.id));
    container.set_id(&format!("{:?}",element_id));
    container.set_attribute("table-id", &format!("{}", table.id))?;
    // First check to see if the table has a "type" column. If it doesn't, just render the table
    if table.has_column_alias(*TYPE) == true {
      for row in 1..=table.rows {
        match table.get(&TableIndex::Index(row), &TableIndex::Alias(*TYPE))  {
          Some((kind,_)) => {
            // ---------------------
            // RENDER A DIV
            // ---------------------
            let raw_kind = kind.as_raw();
            if raw_kind == *DIV {
              // Get contents
              match table.get(&TableIndex::Index(row), &TableIndex::Alias(*CONTAINS)) {
                Some((contents,_)) => {
                  let element_id = hash_string(&format!("div-{:?}-{:?}", table.id, row));
                  let rendered = self.render_value(contents)?;
                  rendered.set_id(&format!("{:?}",element_id));
                  container.append_child(&rendered)?;
                }
                _ => {log!("No \"contains\" on type 'div'");}, // TODO Alert there are no contents
              }
            // ---------------------
            // RENDER A LINK
            // ---------------------
            } else if raw_kind == *A {
              // Get contents
              match (table.get(&TableIndex::Index(row), &TableIndex::Alias(*HREF)),
                     table.get(&TableIndex::Index(row), &TableIndex::Alias(*CONTAINS))) {
                (Some((href,_)), Some((contents,_))) => {
                  let element_id = hash_string(&format!("div-{:?}-{:?}", table.id, row));
                  let rendered = self.render_value(contents)?;
                  rendered.set_id(&format!("{:?}",element_id));
                  let mut link: web_sys::Element = document.create_element("a")?;
                  let href_string = &self.core.get_string(&href).unwrap();
                  let element_id = hash_string(&format!("a-{:?}-{:?}", table.id, row));
                  link.set_attribute("href",href_string)?;
                  link.set_id(&format!("{:?}",element_id));
                  link.append_child(&rendered)?;
                  container.append_child(&link)?;
                }
                (None, Some(_)) => {log!("No \"href\" on type 'a'");}, // TODO Alert there are no href
                (Some(_), None) => {log!("No \"contains\" on type 'a'");}, // TODO Alert there are no contents
                _ => {log!("No \"contains\" or \"href\" on type 'a'");}, // TODO Alert both
              }
            // ---------------------
            // RENDER AN IMG
            // ---------------------
            } else if raw_kind == *IMG {
              // Get contents
              match table.get(&TableIndex::Index(row), &TableIndex::Alias(*SRC)) {
                Some((src,_)) => {
                  let mut img: web_sys::Element = document.create_element("img")?;
                  let src_string = &self.core.get_string(&src).unwrap();
                  let element_id = hash_string(&format!("img-{:?}-{:?}", table.id, row));
                  img.set_attribute("src",src_string)?;
                  img.set_id(&format!("{:?}",element_id));
                  container.append_child(&img)?;
                }
                _ => {log!("No \"src\" on type 'img'");}, // TODO Alert there are no contents
              }
            // ---------------------
            // RENDER A BUTTON
            // ---------------------
            } else if raw_kind == *BUTTON {
              // Get contents
              match table.get(&TableIndex::Index(row), &TableIndex::Alias(*CONTAINS)) {
                Some((contents,_)) => {
                  let element_id = hash_string(&format!("div-{:?}-{:?}", table.id, row));
                  let rendered = self.render_value(contents)?;
                  rendered.set_id(&format!("{:?}",element_id));
                  let mut button: web_sys::Element = document.create_element("button")?;
                  let element_id = hash_string(&format!("button-{:?}-{:?}", table.id, row));
                  button.set_id(&format!("{:?}",element_id));
                  button.append_child(&rendered)?;
                  container.append_child(&button)?;
                }
                _ => {log!("No \"contains\" on type 'button'");}, // TODO Alert there are no contents
              }
            // ---------------------
            // RENDER A CANVAS
            // ---------------------
            } else if raw_kind == *CANVAS {
              // Get contents
              match table.get(&TableIndex::Index(row), &TableIndex::Alias(*CONTAINS)) {
                Some(contents) => {
                  let mut canvas: web_sys::Element = document.create_element("canvas")?;
                  let element_id = hash_string(&format!("canvas-{:?}-{:?}", table.id, row));
                  canvas.set_id(&format!("{:?}",element_id));
                  self.canvases.insert(element_id);
                  // Is there a parameters field?
                  match table.get(&TableIndex::Index(row), &TableIndex::Alias(*PARAMETERS)) {
                    Some((parameters_table_id,_)) => {
                      match parameters_table_id.as_reference() {
                        Some(parameters_table_id) => {
                          let parameters_table = self.core.get_table(parameters_table_id).unwrap();
                          match parameters_table.get_f64(&TableIndex::Index(1), &TableIndex::Alias(*HEIGHT)) {
                            Some(height) => {
                              canvas.set_attribute("height", &format!("{}",height));
                            }
                            _ => (),
                          }
                          match parameters_table.get_f64(&TableIndex::Index(1), &TableIndex::Alias(*WIDTH)) {
                            Some(width) => {
                              canvas.set_attribute("width", &format!("{}",width));
                            }
                            _ => (),
                          }
                        }
                        _ => {log!("Parameter field on canvas must be a table reference");}, // TODO Alert user the parameters field needs to be a table
                      }
                      let table = self.core.get_table(*HTML_APP);
                    }
                    _ => (), // Do nothing, the parameters field is optional
                  }
                  // Add the contents
                  match table.get(&TableIndex::Index(row), &TableIndex::Alias(*CONTAINS)) {
                    Some((contains_table_id,_)) => {
                      match contains_table_id.as_reference() {
                        Some(contains_table_id) => {
                          canvas.set_attribute("elements", &format!("{}",contains_table_id));
                        },
                        _ => {log!("Contains must be a table");},
                      }
                    }
                    _ => (),
                  }
                  container.append_child(&canvas)?;
                }
                _ => {log!("No \"contains\" on type 'canvas'");}, // TODO Alert there are no contents
              }
            // ---------------------
            // RENDER A SLIDER
            // ---------------------
            } else if raw_kind == *SLIDER {
              // Get contents
              match (table.get(&TableIndex::Index(row), &TableIndex::Alias(*MIN)),
                     table.get(&TableIndex::Index(row), &TableIndex::Alias(*MAX)),
                     table.get(&TableIndex::Index(row), &TableIndex::Alias(*VALUE))) {
                (Some((min,_)), Some((max,_)), Some((value,_))) => {
                  match (min.as_f64(), max.as_f64(), value.as_f64()) {
                    (Some(min_value), Some(max_value), Some(value_value)) => {
                      let mut slider: web_sys::Element = document.create_element("input")?;
                      let mut slider: web_sys::HtmlInputElement = slider
                        .dyn_into::<web_sys::HtmlInputElement>()
                        .map_err(|_| ())
                        .unwrap();
                      let element_id = hash_string(&format!("slider-{:?}-{:?}", table.id, row));
                      slider.set_attribute("type","range");
                      slider.set_attribute("min", &format!("{}", min_value));
                      slider.set_attribute("max", &format!("{}", max_value));
                      slider.set_attribute("value", &format!("{}", value_value));
                      slider.set_attribute("row", &format!("{}", row));
                      slider.set_attribute("table", &format!("{}", table.id));
                      slider.set_id(&format!("{:?}",element_id));
                      // Changes to the slider update its own table
                      {
                        let closure = Closure::wrap(Box::new(move |event: web_sys::InputEvent| {
                          match event.target() {
                            Some(target) => {
                              let slider = target.dyn_ref::<web_sys::HtmlInputElement>().unwrap();
                              let slider_value = slider.value().parse::<i32>().unwrap();
                              let table_id = slider.get_attribute("table").unwrap().parse::<u64>().unwrap();

                              let row = slider.get_attribute("row").unwrap().parse::<usize>().unwrap();
                              let change = Change::Set{
                                table_id: table_id, values: vec![ 
                                  (TableIndex::Index(row),
                                   TableIndex::Alias(*VALUE),
                                   Value::from_i32(slider_value)),
                                ]
                              };
                              // TODO Make this safe
                              unsafe {
                                let table = (*wasm_core).core.get_table(table_id).unwrap();
                                (*wasm_core).changes.push(change);
                                (*wasm_core).process_transaction();
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
                    _ => {log!("Slider values are not quantities");}, // TODO fields aren't the right type
                  }
                }
                _ => {log!("No \"min\" \"max\" \"value\" on type 'slider'");}, // TODO Alert there are no min max value
              }
            }
          }
          None => {log!("No type on table");}, // TODO Alert there is no type
        }
      }
    // There's no Type column, so we are going to treat the table as a generic thing and just turn it into divs
    } else {
      // Make a div for each row
      for row in 1..=table.rows {
        let mut row_div = document.create_element("div")?;
        let element_id = hash_string(&format!("div-{:?}-{:?}", table.id, row));
        row_div.set_id(&format!("{:?}",element_id));
        // Make an internal div for each cell 
        for column in 1..=table.columns {
          // Get contents
          match table.get(&TableIndex::Index(row), &TableIndex::Index(column)) {
            Some((contents,_)) => {
              let mut cell_div = document.create_element("div")?;
              let element_id = hash_string(&format!("div-{:?}-{:?}-{:?}", table.id, row, column));
              let rendered = self.render_value(contents)?;
              rendered.set_id(&format!("{:?}",element_id));
              row_div.append_child(&rendered)?;
            }
            _ => {log!("Cell not found");} // TODO Alert there are no contents
          }          
        }
        container.append_child(&row_div)?;
      }
    }
    Ok(container)
  }


/*
  fn draw_contents(&mut self, table: &Table, container: &mut web_sys::Element) -> Result<(), JsValue> {
    let core = &mut self.core as *mut mech_core::Core;
    let wasm_core = self as *mut Core;
    let changes = &mut self.changes as *mut Vec<Change>;
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    for row in 1..=table.rows as usize {
      'column_loop: for j in 1..=table.columns as usize {
        match &table.get_unchecked(row,j).as_string() {
          Some(tag) => {
            match tag {
              DIV | UL | LI | A => {     
                let tag_string = self.core.database.borrow().store.strings.get(tag).unwrap();

                let tag_string = get_string(tag);
                let element_id = Hasher::hash_string(format!("div-{:?}-{:?}", table.id, row));
                let mut div = document.create_element(tag_string)?;
                unsafe {
                  let nodes = (*wasm_core).nodes.entry(table.id).or_insert(vec![]);
                  nodes.push(element_id);
                }
                div.set_id(&format!("{:?}",element_id));
                div.set_attribute("row",&format!("{:?}",row + 1));
                /*match &table.data[1][row].as_string() {
                  Some(class) => {
                    div.set_attribute("class",class);
                  },
                  _ => (),
                }*/
                /*match &table.data[2][row] {
                  Value::String(value) => div.set_inner_html(&value),
                  Value::Number(value) => div.set_inner_html(&format!("{:?}", value.to_float())),
                  Value::Reference(TableId::Local(reference)) => {
                    let referenced_table;
                    // TODO Make this safe
                    unsafe {
                      referenced_table = (*core).get_table(reference).unwrap();
                    }
                    self.draw_contents(&referenced_table, &mut div);
                  }
                  _ => (),
                };*/
                container.append_child(&div)?;
              },
              IMG => {
                let element_id = Hasher::hash_string(format!("img-{:?}-{:?}", table.id, row));
                let class = &table.get_string(&table.get_unchecked(row,2).as_string().unwrap()).unwrap().to_string();
                let value = &table.get_string(&table.get_unchecked(row,3).as_string().unwrap()).unwrap().to_string();
                let mut img = web_sys::HtmlImageElement::new().unwrap();
                img.set_attribute("class", class);
                img.set_id(&format!("{:?}",element_id));
                img.set_src(value);
                container.append_child(&img)?;
              },
              SLIDER => {
                let element_id = Hasher::hash_string(format!("slider-{:?}-{:?}", table.id, row));
                let mut slider = document.create_element("input")?;
                let mut slider: web_sys::HtmlInputElement = slider
                      .dyn_into::<web_sys::HtmlInputElement>()
                      .map_err(|_| ())
                      .unwrap();
                let parameters_id = &table.get_unchecked(row,4).as_reference().unwrap();
                let parameters_table = self.core.get_table(*parameters_id).unwrap();
                let min = &parameters_table.get_string(&parameters_table.get_unchecked(1,1).as_string().unwrap()).unwrap().to_string();
                let max = &parameters_table.get_string(&parameters_table.get_unchecked(1,2).as_string().unwrap()).unwrap().to_string();
                let value = &parameters_table.get_string(&parameters_table.get_unchecked(1,3).as_string().unwrap()).unwrap().to_string();
                slider.set_id(&format!("{:?}", element_id));
                slider.set_type("range");
                slider.set_min(min);
                slider.set_max(max);
                slider.set_value(value);
                slider.set_attribute("parameters", &format!("{:?}",parameters_id));

                container.append_child(&slider)?;
              },
              CANVAS => { 
                let element_id = Hasher::hash_string(format!("canvas-{:?}-{:?}", table.id, row));
                let canvas = document.create_element("canvas")?;
                let elements_id = &table.get_unchecked(row,3).as_u64().unwrap();
                let parameters_id = &table.get_unchecked(row,4).as_u64().unwrap();
                let parameters_table;
                unsafe {
                  parameters_table = (*core).database.borrow().tables.get(parameters_id).unwrap();
                }
                canvas.set_id(&format!("{:?}",element_id));
                canvas.set_attribute("elements",&format!("{:?}",elements_id));
                let height = &parameters_table.get_string(&parameters_table.get_unchecked(1,1).as_string().unwrap()).unwrap().to_string();
                let width = &parameters_table.get_string(&parameters_table.get_unchecked(1,1).as_string().unwrap()).unwrap().to_string();
                canvas.set_attribute("width", height);
                canvas.set_attribute("height", width);
                canvas.set_attribute("style", "background-color: rgb(255, 255, 255)");
                let canvas: web_sys::HtmlCanvasElement = canvas
                      .dyn_into::<web_sys::HtmlCanvasElement>()
                      .map_err(|_| ())
                      .unwrap();
                  {
                    let closure = |i| { Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
                      let window = web_sys::window().expect("no global `window` exists");
                      let document = window.document().expect("should have a document on window");
                      let x = event.offset_x();
                      let y = event.offset_y();
                      let table_id = Hasher::hash_str(i);
                      // TODO Make this safe
                      unsafe {
                        (*wasm_core).changes.push(Change::Set{
                          table_id: table_id, values: vec![
                          (TableIndex::Index(1), 
                          TableIndex::Index(1),
                          Value::from_i64(x as i64))],
                        });
                        (*wasm_core).changes.push(Change::Set{
                          table_id: table_id, values: vec![
                          (TableIndex::Index(1), 
                          TableIndex::Index(2),
                          Value::from_i64(y as i64))],
                        });                  
                        (*wasm_core).process_transaction();
                        (*wasm_core).render();
                      }
                    }) as Box<dyn FnMut(_)>)
                    };
                    let click_callback = closure("html/event/click");
                    canvas.add_event_listener_with_callback("click", click_callback.as_ref().unchecked_ref())?;
                    let move_callback = closure("html/event/pointermove");
                    canvas.add_event_listener_with_callback("pointermove", move_callback.as_ref().unchecked_ref())?;
                    let down_callback = closure("html/event/pointerdown");
                    canvas.add_event_listener_with_callback("pointerdown", down_callback.as_ref().unchecked_ref())?;
                  
                    click_callback.forget();
                    move_callback.forget();
                    down_callback.forget();
                  }
                self.render_canvas(&canvas);
                container.append_child(&canvas)?;
              },
              _ => (),
            }
            break 'column_loop;
          },
          /*
          Value::Reference(TableId::Local(reference)) => {
            let element_id = Hasher::hash_string(format!("div-{:?}-{:?}", table.id, row));
            let mut div = document.create_element("div")?;
            let referenced_table;
            // TODO Make this safe
            unsafe {
              referenced_table = (*core).database.borrow().tables.get(&reference).unwrap();
            }
            self.draw_contents(&referenced_table, &mut div);
            container.append_child(&div)?;
          }*/
          _ => (),
        };
      }      
    }
    Ok(())
  }*/

  pub fn render_canvases(&mut self) -> Result<(), JsValue> {
    let wasm_core = self as *mut WasmCore;
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    
    for canvas_id in &self.canvases {
      match document.get_element_by_id(&format!("{}",canvas_id)) {
        Some(canvas) => {
          let canvas: web_sys::HtmlCanvasElement = canvas
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .map_err(|_| ())
            .unwrap();
          unsafe {
            (*wasm_core).render_canvas(&canvas);
          }
        }
        _ => (),
      }
    }
    Ok(())
  }


  pub fn render_canvas(&mut self, canvas: &web_sys::HtmlCanvasElement) -> Result<(), JsValue> {

    let wasm_core = self as *mut WasmCore;
    let context = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();

    // Define a function to make this a lot easier
    let get_stroke_string = |parameters_table: &Table, row: usize, alias: u64| { 
      match parameters_table.get(&TableIndex::Index(row), &TableIndex::Alias(alias))  {
        Some((stroke_id,_)) => {
          match stroke_id.as_number_literal() {
            Some(stroke_number_literal_id) => {
              match unsafe{ (*wasm_core).core.get_number_literal(stroke_number_literal_id) } {
                Some(number_literal) => {
                  let mut color_string: String = "#".to_string();
                  for byte in number_literal {
                    color_string = format!("{}{:02x}", color_string, byte);
                  }
                  color_string
                }
                None => {
                  log!("NONE");
                  "#000000".to_string()
                }
              }
            },
            _ => {
              log!("Color must be a three byte hexadecimal number literal. Defaulting to 0x000000");
              "#000000".to_string()
            },
          }
        }
        _ => "#000000".to_string(),
      }
    };
    
    let get_line_width = |parameters_table: &Table, row: usize| {
      match parameters_table.get(&TableIndex::Index(row), &TableIndex::Alias(*LINE__WIDTH))  {
        Some((line_width,_)) => {
          match line_width.as_f64() {
            Some(line_width) => line_width,
            _ => {
              log!("Line width must be a quantity. Defaulting to 1");
              1.0
            }
          }
        }
        _ => 1.0
      }
    };

    let get_property = |parameters_table: &Table, row: usize, alias: u64| {
      match parameters_table.get(&TableIndex::Index(row), &TableIndex::Alias(alias))  {
        Some((property,_)) => {
          match property.value_type() {
            ValueType::Quantity => format!("{:?}", property.as_f64().unwrap()),
            ValueType::String => {
              parameters_table.get_string_from_hash(property).unwrap().clone()
            }
            _ => "".to_string(),
          }
        }
        _ => "".to_string()
      }
    };

    // Get the elements table for this canvas
    let elements_table_id_string = canvas.get_attribute("elements").unwrap();
    let elements_table_id: u64 = elements_table_id_string.parse::<u64>().unwrap();
    let elements_table = self.core.get_table(elements_table_id).unwrap();
    let context = Rc::new(context);
    context.clear_rect(0.0, 0.0, canvas.width().into(), canvas.height().into());
    for row in 1..=elements_table.rows as usize {
      match (elements_table.get(&TableIndex::Index(row), &TableIndex::Alias(*SHAPE)),
             elements_table.get_reference(&TableIndex::Index(row), &TableIndex::Alias(*PARAMETERS))) {
        (Some((shape,_)), Some(parameters_table_id)) => {
          let shape = shape.as_raw();
          let parameters_table = self.core.get_table(parameters_table_id).unwrap();
          // ---------------------
          // RENDER A CIRCLE
          // ---------------------
          if shape == *CIRCLE {
            for row in 1..=parameters_table.rows {
              match (parameters_table.get_f64(&TableIndex::Index(row), &TableIndex::Alias(*CENTER__X)),
                     parameters_table.get_f64(&TableIndex::Index(row), &TableIndex::Alias(*CENTER__Y)),
                     parameters_table.get_f64(&TableIndex::Index(row), &TableIndex::Alias(*RADIUS))) {
                (Some(cx), Some(cy), Some(radius)) => {
                  let stroke = get_stroke_string(&parameters_table,row, *STROKE);
                  let fill = get_stroke_string(&parameters_table,row, *FILL);
                  let line_width = get_line_width(&parameters_table,row);
                  context.save();
                  context.begin_path();
                  context.arc(cx, cy, radius, 0.0, 2.0 * 3.141592654);
                  context.set_fill_style(&JsValue::from_str(&fill));
                  context.fill();
                  context.set_stroke_style(&JsValue::from_str(&stroke));
                  context.set_line_width(line_width);    
                  context.stroke();                
                  context.restore();
                }
                _ => {
                  log!("Missing center-x, center-y, or radius");
                },
              }        
            }
          // ---------------------
          // RENDER AN ELLIPSE
          // --------------------- 
          } else if shape == *ELLIPSE {
            for row in 1..=parameters_table.rows {
              match (parameters_table.get_f64(&TableIndex::Index(row), &TableIndex::Alias(*CENTER__X)),
                     parameters_table.get_f64(&TableIndex::Index(row), &TableIndex::Alias(*CENTER__Y)),
                     parameters_table.get_f64(&TableIndex::Index(row), &TableIndex::Alias(*MAJOR__AXIS)),
                     parameters_table.get_f64(&TableIndex::Index(row), &TableIndex::Alias(*MINOR__AXIS))) {
                (Some(cx), Some(cy), Some(maja), Some(mina)) => {
                  let stroke = get_stroke_string(&parameters_table,row, *STROKE);
                  let fill = get_stroke_string(&parameters_table,row, *FILL);
                  let line_width = get_line_width(&parameters_table,row);
                  let pi = 3.141592654;
                  context.save();
                  context.begin_path();
                  context.ellipse(cx, cy, maja, mina, 0.0, 0.0, 2.0 * pi);
                  context.set_fill_style(&JsValue::from_str(&fill));
                  context.fill();
                  context.set_stroke_style(&JsValue::from_str(&stroke));
                  context.set_line_width(line_width);    
                  context.stroke();                
                  context.restore();
                }
                _ => {
                  log!("Missing center-x, center-y, or radius");
                },
              }   
            }     
          // ---------------------
          // RENDER AN ARC
          // --------------------- 
          } else if shape == *ARC {
            for row in 1..=parameters_table.rows {
              match (parameters_table.get_f64(&TableIndex::Index(row), &TableIndex::Alias(*CENTER__X)),
                     parameters_table.get_f64(&TableIndex::Index(row), &TableIndex::Alias(*CENTER__Y)),
                     parameters_table.get_f64(&TableIndex::Index(row), &TableIndex::Alias(*STARTING__ANGLE)),
                     parameters_table.get_f64(&TableIndex::Index(row), &TableIndex::Alias(*ENDING__ANGLE)),
                     parameters_table.get_f64(&TableIndex::Index(row), &TableIndex::Alias(*RADIUS))) {
                (Some(cx), Some(cy), Some(sa), Some(ea), Some(radius)) => {
                  let stroke = get_stroke_string(&parameters_table,row, *STROKE);
                  let fill = get_stroke_string(&parameters_table,row, *FILL);
                  let line_width = get_line_width(&parameters_table,row);
                  let pi = 3.141592654;
                  context.save();
                  context.begin_path();
                  context.arc(cx, cy, radius, sa * pi / 180.0, ea * pi / 180.0);
                  context.set_fill_style(&JsValue::from_str(&fill));
                  context.fill();
                  context.set_stroke_style(&JsValue::from_str(&stroke));
                  context.set_line_width(line_width);    
                  context.stroke();                
                  context.restore();
                }
                _ => {
                  log!("Missing center-x, center-y, or radius");
                },
              }        
            }
          // ---------------------
          // RENDER A RECTANGLE
          // ---------------------    
          } else if shape == *RECTANGLE {
            match (parameters_table.get_f64(&TableIndex::Index(1), &TableIndex::Alias(*X)),
                   parameters_table.get_f64(&TableIndex::Index(1), &TableIndex::Alias(*Y)),
                   parameters_table.get_f64(&TableIndex::Index(1), &TableIndex::Alias(*WIDTH)),
                   parameters_table.get_f64(&TableIndex::Index(1), &TableIndex::Alias(*HEIGHT))) {
              (Some(x), Some(y), Some(width), Some(height)) => {
                let stroke = get_stroke_string(&parameters_table,1, *STROKE);
                let fill = get_stroke_string(&parameters_table,1, *FILL);
                let line_width = get_line_width(&parameters_table,1);
                context.save();
                context.set_fill_style(&JsValue::from_str(&fill));
                context.fill_rect(x,y,width,height);
                context.set_stroke_style(&JsValue::from_str(&stroke));
                context.set_line_width(line_width);
                context.stroke_rect(x,y,width,height);
                context.restore();
              }
              _ => {
                log!("Missing x, y, width, height");
              },
            }
          // ---------------------
          // RENDER TEXT
          // ---------------------    
          } else if shape == *TEXT {
            match (parameters_table.get(&TableIndex::Index(1), &TableIndex::Alias(*TEXT)),
                   parameters_table.get_f64(&TableIndex::Index(1), &TableIndex::Alias(*X)),
                   parameters_table.get_f64(&TableIndex::Index(1), &TableIndex::Alias(*Y))) {
              (Some((text_value,_)), Some(x), Some(y)) => {
                let stroke = get_stroke_string(&parameters_table,1, *STROKE);
                let fill = get_stroke_string(&parameters_table,1, *FILL);
                let line_width = get_line_width(&parameters_table,1);
                let text = get_property(&parameters_table, 1, *TEXT);

                context.save();
                context.set_fill_style(&JsValue::from_str(&fill));
                context.set_line_width(line_width);
                match parameters_table.get_reference(&TableIndex::Index(1), &TableIndex::Alias(*FONT)) {
                  Some(font_table_id) => {
                    let font_table = self.core.get_table(font_table_id).unwrap();
                    let size = get_property(&font_table, 1, *SIZE);
                    let face = match &*get_property(&font_table, 1, *FACE) {
                      "" => "sans-serif".to_string(),
                      x => x.to_string(),
                    };
                    let font_string = format!("{}px {}", size, face);
                    context.set_font(&*font_string);
                  }
                  _ => (),
                }
                context.fill_text(&text,x,y);
                context.restore();
              }
              _ => {
                log!("Missing x, y, text");
              },
            }
          // ---------------------
          // RENDER A PATH
          // ---------------------    
          } else if shape == *PATH {
            context.save();
            let rotate = match parameters_table.get_f64(&TableIndex::Index(1), &TableIndex::Alias(*ROTATE)) {
              Some(rotate) => rotate,
              None => 0.0,
            };
            let (tx,ty) = match parameters_table.get_reference(&TableIndex::Index(1), &TableIndex::Alias(*TRANSLATE)) {
              Some(translate_table_id) => {
                let translate_table = self.core.get_table(translate_table_id).unwrap();
                match (translate_table.get_f64(&TableIndex::Index(1), &TableIndex::Alias(*X)),
                       translate_table.get_f64(&TableIndex::Index(1), &TableIndex::Alias(*Y))) {
                  (Some(tx),Some(ty)) => (tx,ty),
                  _ => (0.0,0.0),
                }
              },
              None => (0.0,0.0),
            };
            context.translate(tx,ty);
            context.rotate(rotate * 3.141592654 / 180.0);
            context.begin_path();
            match (parameters_table.get_reference(&TableIndex::Index(1), &TableIndex::Alias(*START__POINT)),
                   parameters_table.get_reference(&TableIndex::Index(1), &TableIndex::Alias(*CONTAINS))) {
              (Some(start_point_id), Some(contains_table_id)) => {
                let start_point_table = self.core.get_table(start_point_id).unwrap();
                match (start_point_table.get_f64(&TableIndex::Index(1), &TableIndex::Alias(*X)),
                        start_point_table.get_f64(&TableIndex::Index(1), &TableIndex::Alias(*Y))) {
                  (Some(x), Some(y)) => {
                    context.move_to(x, y);
                    // Get the contained shapes
                    let contains_table = self.core.get_table(contains_table_id).unwrap();
                    for i in 1..=contains_table.rows {
                      match (contains_table.get(&TableIndex::Index(i), &TableIndex::Alias(*SHAPE)),
                              contains_table.get_reference(&TableIndex::Index(i), &TableIndex::Alias(*PARAMETERS))) {
                        (Some((shape,_)),Some(parameters_table_id)) => {
                          let shape = shape.as_raw();
                          // -------------------
                          // PATH LINE
                          // -------------------
                          if shape == *LINE {
                            let parameters_table = self.core.get_table(parameters_table_id).unwrap();
                            match (parameters_table.get_f64(&TableIndex::Index(1), &TableIndex::Alias(*X)),
                                    parameters_table.get_f64(&TableIndex::Index(1), &TableIndex::Alias(*Y))) {
                              (Some(x), Some(y)) => {
                                context.line_to(x, y);
                              }
                              _ => (), // Expected x and y fields
                            }
                          // -------------------
                          // PATH QUADRATIC
                          // -------------------
                          } else if shape == *QUADRATIC {
                            let parameters_table = self.core.get_table(parameters_table_id).unwrap();
                            match (parameters_table.get_reference(&TableIndex::Index(1), &TableIndex::Alias(*CONTROL__POINT)),
                                    parameters_table.get_reference(&TableIndex::Index(1), &TableIndex::Alias(*END__POINT))) {
                              (Some(control__point_table_id), Some(end__point_table_id)) => {
                                let control__point_table = self.core.get_table(control__point_table_id).unwrap();
                                let end__point_table = self.core.get_table(end__point_table_id).unwrap();
                                match (control__point_table.get_f64(&TableIndex::Index(1), &TableIndex::Alias(*X)),
                                        control__point_table.get_f64(&TableIndex::Index(1), &TableIndex::Alias(*Y)),
                                        end__point_table.get_f64(&TableIndex::Index(1), &TableIndex::Alias(*X)),
                                        end__point_table.get_f64(&TableIndex::Index(1), &TableIndex::Alias(*Y))) {
                                  (Some(cx), Some(cy), Some(ex), Some(ey)) => {
                                    context.quadratic_curve_to(cx, cy, ex, ey);
                                  }
                                  _ => (), // Expected x and y fields
                                }
                              }
                              _ => (), // Expected control-point and end-point fields
                            }
                        // -------------------
                        // PATH BEZIER
                        // -------------------
                        } else if shape == *BEZIER {
                          let parameters_table = self.core.get_table(parameters_table_id).unwrap();
                          match (parameters_table.get_reference(&TableIndex::Index(1), &TableIndex::Alias(*CONTROL__POINTS)),
                                  parameters_table.get_reference(&TableIndex::Index(1), &TableIndex::Alias(*END__POINT))) {
                            (Some(control__point_table_id), Some(end__point_table_id)) => {
                              let control__point_table = self.core.get_table(control__point_table_id).unwrap();
                              let end__point_table = self.core.get_table(end__point_table_id).unwrap();
                              match (control__point_table.get_f64(&TableIndex::Index(1), &TableIndex::Alias(*X)),
                                      control__point_table.get_f64(&TableIndex::Index(1), &TableIndex::Alias(*Y)),
                                      control__point_table.get_f64(&TableIndex::Index(2), &TableIndex::Alias(*X)),
                                      control__point_table.get_f64(&TableIndex::Index(2), &TableIndex::Alias(*Y)),
                                      end__point_table.get_f64(&TableIndex::Index(1), &TableIndex::Alias(*X)),
                                      end__point_table.get_f64(&TableIndex::Index(1), &TableIndex::Alias(*Y))) {
                                (Some(cx1), Some(cy1), Some(cx2), Some(cy2), Some(ex), Some(ey)) => {
                                  context.bezier_curve_to(cx1, cy1, cx2, cy2, ex, ey);
                                }
                                _ => (), // Expected x and y fields
                              }
                            }
                            _ => (), // Expected control-point and end-point fields
                          }
                          // -------------------
                          // PATH ARC
                          // -------------------
                          } else if shape == *ARC {
                            let parameters_table = self.core.get_table(parameters_table_id).unwrap();
                            match (parameters_table.get_f64(&TableIndex::Index(1), &TableIndex::Alias(*CENTER__X)),
                                   parameters_table.get_f64(&TableIndex::Index(1), &TableIndex::Alias(*CENTER__Y)),
                                   parameters_table.get_f64(&TableIndex::Index(1), &TableIndex::Alias(*STARTING__ANGLE)),
                                   parameters_table.get_f64(&TableIndex::Index(1), &TableIndex::Alias(*ENDING__ANGLE)),
                                   parameters_table.get_f64(&TableIndex::Index(1), &TableIndex::Alias(*RADIUS))) {
                              (Some(cx), Some(cy), Some(sa), Some(ea), Some(radius)) => {
                                let pi = 3.141592654;
                                context.arc(cx, cy, radius, sa * pi / 180.0, ea * pi / 180.0);
                              }
                              _ => (), // Expected control-point and end-point fields
                            }
                          }
                        }
                        _ => log!("Expected shape and parameters"), // TODO Expected shape and parameters fields
                      }
                    }
                  }
                  _ => (), // TODO Expected x and y not fields
                }
                let stroke = get_stroke_string(&parameters_table,1, *STROKE);
                let line_width = get_line_width(&parameters_table,1);
                match parameters_table.get(&TableIndex::Index(1), &TableIndex::Alias(*FILL))  {
                  Some(_) => {
                    let fill = get_stroke_string(&parameters_table,1, *FILL);
                    context.set_fill_style(&JsValue::from_str(&fill));
                    context.fill();
                  }
                  _ => (),
                }
                context.set_stroke_style(&JsValue::from_str(&stroke));
                context.set_line_width(line_width);
                context.stroke();
              }
              (Some(_), None) => log!("Contains is not a reference"),
              (None, Some(_)) => log!("Start-point is not a reference"),
              (None, None) => log!("Start-point and Contains are not references"),
            }
            //context.close_path();
            context.restore();
          // ---------------------
          // RENDER A IMAGE
          // --------------------- 
          } else if shape == *IMAGE {
            match (parameters_table.get_string(&TableIndex::Index(1), &TableIndex::Alias(*SOURCE)),
                    parameters_table.get_f64(&TableIndex::Index(1), &TableIndex::Alias(*X)),
                    parameters_table.get_f64(&TableIndex::Index(1), &TableIndex::Alias(*Y)),
                    parameters_table.get_f64(&TableIndex::Index(1), &TableIndex::Alias(*ROTATE))) {
              (Some((source_string,_)), Some(x), Some(y), Some(rotation)) => {
                let source_hash = hash_string(&source_string);
                match self.images.entry(source_hash) {
                  Entry::Occupied(img_entry) => {
                    let img = img_entry.get();
                    let ix = img.width() as f64 / 2.0;
                    let iy = img.height() as f64 / 2.0;
                    context.save();
                    context.translate(x, y);
                    context.rotate(rotation * 3.141592654 / 180.0);
                    context.draw_image_with_html_image_element(&img, -ix, -iy);
                    context.restore();
                  },
                  Entry::Vacant(v) => {
                    let mut img = web_sys::HtmlImageElement::new().unwrap();
                    img.set_src(&source_string.to_owned());
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
                  }
                }
              }
              _ => {log!("Missing source, x, y, or rotation");},
            }
          } else {
            log!("Unknown canvas element");
          }
        },
        _ => {log!("Missing shape or parameters table");}
      }
    }
    Ok(())
  } 

  /*
  pub fn list_global_tables(&self) -> Result<(), JsValue> {
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    let body = document.body().expect("document should have a body");
    let table_list_div = document.create_element("div")?;
    let table_list = document.create_element("ul")?;
    for (table_id, table) in self.core.database.borrow().tables.iter() {
      let table_list_item = document.create_element("li")?;
      match self.core.database.borrow().store.strings.get(table_id) {
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
            let name = formatter.format(child, false);
            let name = format!("mech/inline/{}", Hasher::hash_string(name.clone()));
            let id = Hasher::hash_string(name.clone());
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
  */
}
/*

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
}*/
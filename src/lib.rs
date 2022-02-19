#![recursion_limit="256"]
#![feature(alloc)]
#![feature(drain_filter)]
#![feature(get_mut_unchecked)]
extern crate wasm_bindgen;
extern crate hashbrown;
#[macro_use]
extern crate alloc;
extern crate core;
extern crate web_sys;
extern crate mech_core;
extern crate mech_syntax;
extern crate mech_utilities;
extern crate mech_math;
extern crate bincode;
#[macro_use]
extern crate lazy_static;
extern crate miniz_oxide;
extern crate base64;

use base64::{encode, decode};
use miniz_oxide::inflate::decompress_to_vec;
use miniz_oxide::deflate::compress_to_vec;
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
//use mech_syntax::formatter::Formatter;
use mech_syntax::compiler::{Compiler};
use mech_core::*;
use mech_utilities::{SocketMessage, MiniBlock};
use mech_math::{
  math_sin, 
  //math_cos, 
  //math_floor,
  //math_round,
};
use web_sys::{ErrorEvent, MessageEvent, WebSocket, FileReader};
use std::sync::Arc;

#[macro_export]
macro_rules! log {
  ( $( $t:tt )* ) => {
    web_sys::console::log_1(&format!( $( $t )* ).into());
  }
}

lazy_static! {
  static ref HTML_APP: u64 = hash_str("html/app");
  static ref DIV: u64 = hash_str("div");
  static ref A: u64 = hash_str("a");
  static ref IMG: u64 = hash_str("img");
  static ref SRC: u64 = hash_str("src");
  static ref CONTAINS: u64 = hash_str("contains");
  static ref ROOT: u64 = hash_str("root");
  static ref TYPE: u64 = hash_str("type");
  static ref HREF: u64 = hash_str("href");
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
  static ref HTML_EVENT_POINTER__MOVE: u64 = hash_str("html/event/pointer-move");
  static ref HTML_EVENT_POINTER__DOWN: u64 = hash_str("html/event/pointer-down");
  static ref HTML_EVENT_POINTER__UP: u64 = hash_str("html/event/pointer-up");
  static ref HTML_EVENT_KEY__DOWN: u64 = hash_str("html/event/key-down");
  static ref HTML_EVENT_KEY__UP: u64 = hash_str("html/event/key-up");
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
  static ref HTML_LOCATION: u64 = hash_str("html/location");
  static ref HASH: u64 = hash_str("hash");
  static ref HOST: u64 = hash_str("host");
  static ref HOST__NAME: u64 = hash_str("host-name");
  static ref ORIGIN: u64 = hash_str("origin");
  static ref PATH__NAME: u64 = hash_str("path-name");
  static ref PORT: u64 = hash_str("port");
  static ref PROTOCOL: u64 = hash_str("protocol");
  static ref SEARCH: u64 = hash_str("search");
}

#[wasm_bindgen]
pub struct WasmCore {
  core: mech_core::Core,
  //programs: Vec<Program>,
  changes: Vec<Change>,
  //images: HashMap<u64, web_sys::HtmlImageElement>,*/
  canvases: HashSet<u64>,
  /*nodes: HashMap<u64, Vec<u64>>,
  views: HashSet<u64>,
  inline_views: HashSet<u64>,
  websocket: Option<web_sys::WebSocket>,
  remote_tables: HashSet<Register>,*/
  event_id: u32,
  //timers: HashMap<usize,Closure<dyn FnMut()>>,
  apps: HashSet<u64>,
  window: web_sys::Window,
  document: web_sys::Document,
}

#[wasm_bindgen]
impl WasmCore {

  
  pub fn new(capacity: usize, recursion_limit: u64) -> WasmCore {
    let mut mech = mech_core::Core::new();
    /*
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
    mech.insert_string("html/location");
    mech.insert_string("hash");
    mech.insert_string("host");
    mech.insert_string("host-name");
    mech.insert_string("href");
    mech.insert_string("origin");
    mech.insert_string("path");
    mech.insert_string("port");
    mech.insert_string("protocol");
    mech.insert_string("search");

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
    changes.append(&mut new_table(*HTML_LOCATION, 1, vec![*HASH, *HOST, *HOST__NAME, *HREF, *ORIGIN, *PATH__NAME, *PORT, *PROTOCOL, *SEARCH]));

    let txn = Transaction{changes};
    mech.process_transaction(&txn);
*/
    WasmCore {
      core: mech,
      //programs: Vec::new(),
      changes: Vec::new(),
      //images: HashMap::new(),*/
      canvases: HashSet::new(),
      /*nodes: HashMap::new(),
      views: HashSet::new(),
      inline_views: HashSet::new(),
      websocket: None,
      remote_tables: HashSet::new(),*/
      event_id: 0,
      //timers: HashMap::new(),
      apps: HashSet::new(),
      window: web_sys::window().unwrap(),
      document: web_sys::window().unwrap().document().unwrap(),
    }
  }
  
  pub fn connect_remote_core(&mut self, address: String) -> Result<(), JsValue> {
    /*
    let ws = WebSocket::new(&address)?;
    ws.set_binary_type(web_sys::BinaryType::Arraybuffer);
   
    // OnMessage
    {
      let wasm_core = self as *mut WasmCore;
      let cloned_ws = ws.clone();
      let onmessage_callback = Closure::wrap(Box::new(move |e: MessageEvent| {
        if let Ok(abuf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
          let compressed_message = js_sys::Uint8Array::new(&abuf).to_vec();
          let serialized_message = decompress_to_vec(&compressed_message).expect("Failed to decompress!");
          match serialized_message[0] {
            0x42 => {
              let mut table_id: u64 = 0;
              for i in 1..8 {
                let b = serialized_message[i];
                table_id = table_id | (b as u64) << ((i - 1) * 8);
              }
              let mut value: u64 = 0;
              let mut data = vec![];
              for i in 9..serialized_message.len() {
                let b = serialized_message[i];
                value = value | (b as u64) << (((i - 9) % 8) * 8);
                if (i - 9) % 8 == 7 {
                  data.push(value.clone());
                  value = 0;
                }
              }
              unsafe {
                let txn = Transaction{changes: vec![Change::Table{table_id,data}]};
                (*wasm_core).core.process_transaction(&txn);
                (*wasm_core).add_apps();
                (*wasm_core).render();
              }
            }
            _ => {
              let msg: Result<SocketMessage, bincode::Error> = bincode::deserialize(&serialized_message.to_vec());
              match msg {
                Ok(SocketMessage::Transaction(txn)) => {
                  unsafe {
                    (*wasm_core).core.process_transaction(&txn);
                    (*wasm_core).add_apps();
                    (*wasm_core).render();
                  }
                }
                Ok(SocketMessage::Listening(register)) => {
                  unsafe {
                    (*wasm_core).remote_tables.insert(register);
                    // Send over the table we have now
                    match (*wasm_core).core.get_table(*register.table_id.unwrap()) {
                      Some(table) => {
                        // Decompose the table into changes for a transaction
                        let mut changes = vec![];
                        changes.push(Change::NewTable{table_id: table.id, rows: table.rows, columns: table.columns});
                        for ((_,column_ix), column_alias) in table.store.column_index_to_alias.iter() {
                          changes.push(Change::SetColumnAlias{table_id: table.id, column_ix: *column_ix, column_alias: *column_alias});
                        } 
                        let mut values = vec![];
                        for i in 1..=table.rows {
                          for j in 1..=table.columns {
                            let (value, _) = table.get_unchecked(i,j);
                            values.push((TableIndex::Index(i), TableIndex::Index(j), value));
                          }
                        }
                        changes.push(Change::Set{table_id: table.id, values});
                        let txn = Transaction{changes};
                        // Send the transaction to the remote core
                        let message = bincode::serialize(&SocketMessage::Transaction(txn)).unwrap();
                        cloned_ws.send_with_u8_array(&message);                   
                      }
                      None => (),
                    }
                  }
                }
                msg => log!("{:?}", msg),
              }
            },
          }
        } else {
          log!("Unhandled Message {:?}", e.data());
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
        // Upon an open connection, send the server a list of tables to which we are listening
        unsafe {
          for input_table_id in (*wasm_core).core.runtime.needed_registers.iter() {
            let result = bincode::serialize(&SocketMessage::Listening(input_table_id.clone())).unwrap();
            cloned_ws.send_with_u8_array(&result);
          }
        }
      }) as Box<dyn FnMut(JsValue)>);
      ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
      onopen_callback.forget();
    }

    // On Close
    {
      let onclose_callback = Closure::wrap(Box::new(move |event: web_sys::Event| {
        log!("Websocket Closed.");
      }) as Box<dyn FnMut(_)>);
      ws.set_onclose(Some(&onclose_callback.as_ref().unchecked_ref()));
      onclose_callback.forget();
    }

    // Todo, make sef.websocket into a vector of websockets.
    self.websocket = Some(ws);*/
    Ok(())
  }

  pub fn add_timers(&mut self) {
    /*
    let window = web_sys::window().expect("no global `window` exists");
   
    match self.core.get_table(hash_str("time/timer")) {
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
                    let table = (*wasm_core).core.get_table(hash_str("time/timer")).unwrap();
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
    }    */
  }

  pub fn load_compressed_blocks(&mut self, encoded_miniblocks: String) {
    let compressed_miniblocks = decode(encoded_miniblocks).unwrap();
    let serialized_miniblocks = decompress_to_vec(compressed_miniblocks.as_slice()).expect("Failed to decompress!");
    self.load_blocks(serialized_miniblocks);
  }

  pub fn load_blocks(&mut self, serialized_miniblocks: Vec<u8>) -> Result<(),JsValue> {
    let miniblocks: Vec<MiniBlock> = match bincode::deserialize(&serialized_miniblocks) {
      Ok(miniblocks) => miniblocks,
      Err(x) => {
        return Err(JsValue::from_str("5239"));
      }
    };
    let mut blocks: Vec<Block> = Vec::new();
    let blocks = miniblocks.iter().map(|b| MiniBlock::maximize_block(&b)).collect::<Vec<Block>>();
    let len = blocks.len();
    self.core.insert_blocks(blocks);

    println!("{:?}", self.core.blocks);

    //self.add_timers();
    self.add_apps();
    //self.render();
    log!("Loaded {} blocks.", len);
    Ok(())
  }

  pub fn process_transaction(&mut self) {
    
    self.core.process_transaction(&self.changes);
    /*match &self.websocket {
      Some(ws) => {
        for changed_register in &self.core.runtime.aggregate_changed_this_round {
          match (self.remote_tables.get(&changed_register),self.core.get_table(*changed_register.table_id.unwrap())) {
            (Some(listeners),Some(table)) => {
              let mut changes = vec![];
              let mut values = vec![];
              for i in 1..=table.rows {
                for j in 1..=table.columns {
                  let (value, _) = table.get_unchecked(i,j);
                  values.push((TableIndex::Index(i), TableIndex::Index(j), value));
                }
              }
              changes.push(Change::Set{table_id: table.id, values});                  
              let txn = Transaction{changes};
              let message = bincode::serialize(&SocketMessage::Transaction(txn)).unwrap();
              // Send the transaction over the websocket to the remote core
              ws.send_with_u8_array(&message);
            }
            _ => (),
          }
        }       
      }
      _ => (),
    }*/
    self.changes.clear();
  }

  pub fn init(&mut self) -> Result<(), JsValue> {
    let wasm_core = self as *mut WasmCore;

    // Set up some callbacks for events.
    {
      let key_closure = |table_id| { 
        Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
          let key = event.key();
          // TODO Make this safe
          unsafe {
            (*wasm_core).changes.push(Change::Set((
              table_id, vec![
                (TableIndex::Index(1), 
                TableIndex::Alias(*KEY),
                Value::from_string(&key))])));    
           (*wasm_core).event_id += 1;
            let eid = (*wasm_core).event_id;
            (*wasm_core).changes.push(Change::Set((
              table_id, vec![
                (TableIndex::Index(1), 
                TableIndex::Alias(*EVENT__ID),
                Value::U32(eid))])));
            (*wasm_core).process_transaction();
            (*wasm_core).render();
            //let table = (*wasm_core).core.get_table(hash_str("balls"));
            //log!("{:?}", table);
          }
        }) as Box<dyn FnMut(_)>)
      };
      let keydown_callback = key_closure(*HTML_EVENT_KEY__DOWN);
      self.document.add_event_listener_with_callback("keydown", keydown_callback.as_ref().unchecked_ref())?;
      let keyup_callback = key_closure(*HTML_EVENT_KEY__UP);
      self.document.add_event_listener_with_callback("keyup", keyup_callback.as_ref().unchecked_ref())?;
      keydown_callback.forget();
      keyup_callback.forget();
    }
    
    {
      let pointer_closure = |table_id| { 
        Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
          //let target = event.target().unwrap();
          //let target_element = target.dyn_ref::<web_sys::HtmlElement>().unwrap();
          //let target_table_id = target_element.id().parse::<u64>().unwrap();
          //log!("{:?}", target_element.id().parse::<u64>().unwrap());

          let x = event.offset_x();
          let y = event.offset_y();
          //log!("event: {:?} {:?}", x, y);
          // TODO Make this safe
          unsafe {
            (*wasm_core).changes.push(Change::Set((
              table_id, vec![(
                TableIndex::Index(1), 
                TableIndex::Alias(*X),
                Value::I32(x as i32))])));    
            (*wasm_core).changes.push(Change::Set((
              table_id, vec![(
                TableIndex::Index(1), 
                TableIndex::Alias(*Y),
                Value::I32(y as i32))])));              
            /*(*wasm_core).changes.push(Change::Set{
              table_id: table_id, values: vec![
              (TableIndex::Index(1), 
              TableIndex::Alias(*TARGET),
              Value::from_id(target_table_id))],
            });*/            
            (*wasm_core).event_id += 1;
            let eid = (*wasm_core).event_id;
            (*wasm_core).changes.push(Change::Set((
              table_id, vec![(
                TableIndex::Index(1), 
                TableIndex::Alias(*EVENT__ID),
                Value::U32(eid))])));  
            (*wasm_core).process_transaction();
            (*wasm_core).render();
            //let table = (*wasm_core).core.get_table(hash_str("clicked"));
            //log!("{:?}", table);
          }
        }) as Box<dyn FnMut(_)>)
      };
      let move_callback = pointer_closure(*HTML_EVENT_POINTER__MOVE);
      self.document.add_event_listener_with_callback("pointermove", move_callback.as_ref().unchecked_ref())?;
      let down_callback = pointer_closure(*HTML_EVENT_POINTER__DOWN);
      self.document.add_event_listener_with_callback("pointerdown", down_callback.as_ref().unchecked_ref())?;
      let up_callback = pointer_closure(*HTML_EVENT_POINTER__UP);
      self.document.add_event_listener_with_callback("pointerup", up_callback.as_ref().unchecked_ref())?;
      move_callback.forget();
      down_callback.forget();
      up_callback.forget();
    }

    {
      let onhashchange_closure = Closure::wrap(Box::new(move |event: web_sys::HashChangeEvent| {
        unsafe { 
          let location = (*wasm_core).window.location(); 
          let mut hash = location.hash().unwrap();
          if hash.len() > 1 {
            hash = hash[1..].to_string();
          }

          (*wasm_core).changes.push(Change::Set((
            *HTML_LOCATION, vec![(
              TableIndex::Index(1), 
              TableIndex::Alias(*HASH),
              Value::from_string(&hash))])));    
          //(*wasm_core).changes.push(Change::InternString{string: hash});
          (*wasm_core).process_transaction();
          (*wasm_core).render();
        }
      }) as Box<dyn FnMut(_)>);
      self.window.set_onhashchange(Some(onhashchange_closure.as_ref().unchecked_ref()));
      onhashchange_closure.forget();
    }
    
    let location = self.window.location();
    let hash = location.hash()?;
    if hash.len() > 1 {
      let hash = hash[1..].to_string();
    }
    let host = location.host()?;
    let hostname = location.hostname()?;
    let href = location.href()?;
    let origin = location.origin()?;
    let pathname = location.pathname()?;
    let port = location.port()?;
    let protocol = location.protocol()?;
    let mut search = location.search()?;
    if search.len() > 1 {
      search = search[1..].to_string();
    }
    let mut changes = vec![Change::Set((
      *HTML_LOCATION, vec![
      (TableIndex::Index(1), TableIndex::Alias(*HASH), Value::from_string(&hash)),
      (TableIndex::Index(1), TableIndex::Alias(*HOST), Value::from_string(&host)),
      (TableIndex::Index(1), TableIndex::Alias(*HOST__NAME), Value::from_string(&hostname)),
      (TableIndex::Index(1), TableIndex::Alias(*HREF), Value::from_string(&href)),
      (TableIndex::Index(1), TableIndex::Alias(*ORIGIN), Value::from_string(&origin)),
      (TableIndex::Index(1), TableIndex::Alias(*PATH__NAME), Value::from_string(&pathname)),
      (TableIndex::Index(1), TableIndex::Alias(*PORT), Value::from_string(&port)),
      (TableIndex::Index(1), TableIndex::Alias(*PROTOCOL), Value::from_string(&protocol)),
      (TableIndex::Index(1), TableIndex::Alias(*SEARCH), Value::from_string(&search))]
    )), 
    /*Change::InternString{string: hash}, 
    Change::InternString{string: host}, 
    Change::InternString{string: hostname}, 
    Change::InternString{string: href}, 
    Change::InternString{string: origin}, 
    Change::InternString{string: pathname}, 
    Change::InternString{string: port}, 
    Change::InternString{string: protocol}, 
    Change::InternString{string: search}*/
    ];
    self.changes.append(&mut changes);
    self.process_transaction();
    
    Ok(())
  }

  pub fn add_apps(&mut self) -> Result<(), JsValue> {
    let wasm_core = self as *mut WasmCore;
    let table = self.core.get_table("html/app");
    match table {
      Ok(app_table) => {        

        let app_table_brrw = app_table.borrow();
        for row in 1..=app_table_brrw.rows as usize {
          match (app_table_brrw.get(&TableIndex::Index(row), &TableIndex::Alias(*ROOT)), 
                 app_table_brrw.get(&TableIndex::Index(row), &TableIndex::Alias(*CONTAINS))) {
            (Ok(Value::String(root)), Ok(contents)) => {
              let root_id = root.hash();
              match self.apps.contains(&root_id) {
                true => continue, // app already added
                false => {
                  self.apps.insert(root_id.clone());
                  match self.document.get_element_by_id(&root.to_string()) {
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
    self.render_canvases();
    Ok(())
  }

  fn render_value(&mut self, value: Value) -> Result<web_sys::Element, JsValue> {
    let mut div = self.document.create_element("div")?;
    match value {
      Value::String(chars) => {
        let contents_string = chars.to_string();
        div.set_inner_html(&contents_string);
      },
      Value::U16(x) => div.set_inner_html(&format!("{:?}", x)),
      Value::U8(x) => div.set_inner_html(&format!("{:?}", x)),
      Value::Reference(table_id) => {
        let table = self.core.get_table_by_id(*table_id.unwrap()).unwrap();
        let rendered_ref = self.make_element(&table.borrow())?;
        div.append_child(&rendered_ref)?;
      }
      _ => (), // TODO Unhandled Boolean and Empty
    }
    Ok(div)
  }

  fn make_element(&mut self, table: &Table) -> Result<web_sys::Element, JsValue> {
    let wasm_core = self as *mut WasmCore;
    let mut container: web_sys::Element = self.document.create_element("div")?;
    let element_id = hash_str(&format!("div-{:?}", table.id));
    container.set_id(&format!("{:?}",element_id));
    container.set_attribute("table-id", &format!("{}", table.id))?;
    // First check to see if the table has a "type" column. If it doesn't, just render the table
    match table.column_alias_to_ix.contains_key(&*TYPE) {
      true => {
        for row in 1..=table.rows {
          match table.get(&TableIndex::Index(row), &TableIndex::Alias(*TYPE))  {
            Ok(Value::String(kind)) => {
              let raw_kind = kind.hash();
              // ---------------------
              // RENDER A DIV
              // ---------------------
              if raw_kind == *DIV {
                // Get contents
                match table.get(&TableIndex::Index(row), &TableIndex::Alias(*CONTAINS)) {
                  Ok(contents) => {
                    let element_id = hash_str(&format!("div-{:?}-{:?}", table.id, row));
                    let rendered = self.render_value(contents)?;
                    rendered.set_id(&format!("{:?}",element_id));
                    container.append_child(&rendered)?;
                  }
                  x => {log!("4733 {:?}",x);},
                }
              }
              // ---------------------
              // RENDER A LINK
              // ---------------------
              else if raw_kind == *A {
                // Get contents
                match (table.get(&TableIndex::Index(row), &TableIndex::Alias(*HREF)),
                      table.get(&TableIndex::Index(row), &TableIndex::Alias(*CONTAINS))) {
                  (Ok(Value::String(href)), Ok(contents)) => {
                    let element_id = hash_str(&format!("div-{:?}-{:?}", table.id, row));
                    let rendered = self.render_value(contents)?;
                    rendered.set_id(&format!("{:?}",element_id));
                    let mut link: web_sys::Element = self.document.create_element("a")?;
                    link.set_attribute("href",&href.to_string())?;
                    let element_id = href.hash();
                    link.set_id(&format!("{:?}",element_id));
                    link.append_child(&rendered)?;
                    container.append_child(&link)?;
                  }
                  x => {log!("4734 {:?}", x);},
                }
              }
              // ---------------------
              // RENDER AN IMG
              // ---------------------
              else if raw_kind == *IMG {
                // Get contents
                match table.get(&TableIndex::Index(row), &TableIndex::Alias(*SRC)) {
                  Ok(Value::String(src)) => {
                    let mut img: web_sys::Element = self.document.create_element("img")?;
                    let element_id = hash_str(&format!("img-{:?}-{:?}", table.id, row));
                    img.set_attribute("src", &src.to_string())?;
                    img.set_id(&format!("{:?}",element_id));
                    container.append_child(&img)?;
                  }
                  x => {log!("4735 {:?}", x);},
                }
              }
              // ---------------------
              // RENDER A BUTTON
              // ---------------------
              /*else if raw_kind == *BUTTON {
                // Get contents
                match table.get(&TableIndex::Index(row), &TableIndex::Alias(*CONTAINS)) {
                  Some((contents,_)) => {
                    let element_id = hash_str(&format!("div-{:?}-{:?}", table.id, row));
                    let rendered = self.render_value(contents)?;
                    rendered.set_id(&format!("{:?}",element_id));
                    let mut button: web_sys::Element = self.document.create_element("button")?;
                    let element_id = hash_str(&format!("button-{:?}-{:?}", table.id, row));
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
                    let mut canvas: web_sys::Element = self.document.create_element("canvas")?;
                    let element_id = hash_str(&format!("canvas-{:?}-{:?}", table.id, row));
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
                        let mut slider: web_sys::Element = self.document.create_element("input")?;
                        let mut slider: web_sys::HtmlInputElement = slider
                          .dyn_into::<web_sys::HtmlInputElement>()
                          .map_err(|_| ())
                          .unwrap();
                        let element_id = hash_str(&format!("slider-{:?}-{:?}", table.id, row));
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
              }*/
            }
            x => log!("54894 {:?}",x),
            Err(x) => log!("32134 {:?}",x), // TODO Alert there is no type
          }
        }
      }
      // There's no Type column, so we are going to treat the table as a generic thing and just turn it into divs
      false => {
        // Make a div for each row
        for row in 1..=table.rows {
          let mut row_div = self.document.create_element("div")?;
          let element_id = hash_str(&format!("div-{:?}-{:?}", table.id, row));
          row_div.set_id(&format!("{:?}",element_id));
          // Make an internal div for each cell 
          for column in 1..=table.cols {
            // Get contents
            match table.get(&TableIndex::Index(row), &TableIndex::Index(column)) {
              Ok(contents) => {
                let mut cell_div = self.document.create_element("div")?;
                let element_id = hash_str(&format!("div-{:?}-{:?}-{:?}", table.id, row, column));
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
    }
    Ok(container)
  }

  pub fn render_canvases(&mut self) -> Result<(), JsValue> {
    let wasm_core = self as *mut WasmCore;
    for canvas_id in &self.canvases {
      match self.document.get_element_by_id(&format!("{}",canvas_id)) {
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
    /*
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
            for row in 1..=parameters_table.rows {
              match (parameters_table.get_f64(&TableIndex::Index(row), &TableIndex::Alias(*X)),
                    parameters_table.get_f64(&TableIndex::Index(row), &TableIndex::Alias(*Y)),
                    parameters_table.get_f64(&TableIndex::Index(row), &TableIndex::Alias(*WIDTH)),
                    parameters_table.get_f64(&TableIndex::Index(row), &TableIndex::Alias(*HEIGHT))) {
                (Some(x), Some(y), Some(width), Some(height)) => {
                  let stroke = get_stroke_string(&parameters_table,row, *STROKE);
                  let fill = get_stroke_string(&parameters_table,row, *FILL);
                  let line_width = get_line_width(&parameters_table,row);
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
              }
          // ---------------------
          // RENDER TEXT
          // ---------------------    
          } else if shape == *TEXT {
            for row in 1..=parameters_table.rows {
              match (parameters_table.get(&TableIndex::Index(row), &TableIndex::Alias(*TEXT)),
                    parameters_table.get_f64(&TableIndex::Index(row), &TableIndex::Alias(*X)),
                    parameters_table.get_f64(&TableIndex::Index(row), &TableIndex::Alias(*Y))) {
                (Some((text_value,_)), Some(x), Some(y)) => {
                  let stroke = get_stroke_string(&parameters_table,row, *STROKE);
                  let fill = get_stroke_string(&parameters_table,row, *FILL);
                  let line_width = get_line_width(&parameters_table,row);
                  let text = get_property(&parameters_table, row, *TEXT);

                  context.save();
                  context.set_fill_style(&JsValue::from_str(&fill));
                  context.set_line_width(line_width);
                  match parameters_table.get_reference(&TableIndex::Index(row), &TableIndex::Alias(*FONT)) {
                    Some(font_table_id) => {
                      let font_table = self.core.get_table(font_table_id).unwrap();
                      let size = get_property(&font_table, row, *SIZE);
                      let face = match &*get_property(&font_table, row, *FACE) {
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
          // RENDER AN IMAGE
          // --------------------- 
          } else if shape == *IMAGE {
            for row in 1..=parameters_table.rows {
              match (parameters_table.get_string(&TableIndex::Index(row), &TableIndex::Alias(*SOURCE)),
                      parameters_table.get_f64(&TableIndex::Index(row), &TableIndex::Alias(*X)),
                      parameters_table.get_f64(&TableIndex::Index(row), &TableIndex::Alias(*Y)),
                      parameters_table.get_f64(&TableIndex::Index(row), &TableIndex::Alias(*ROTATE))) {
                (Some((source_string,_)), Some(x), Some(y), Some(rotation)) => {
                  let source_hash = hash_str(&source_string);
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
            }
          } else {
            log!("Unknown canvas element");
          }
        },
        _ => {log!("Missing shape or parameters table");}
      }
    }*/ 
    Ok(())
  }

}
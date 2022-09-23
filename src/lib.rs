#![recursion_limit="256"]
#![feature(alloc)]
#![feature(drain_filter)]
#![feature(get_mut_unchecked)]
#![allow(warnings)]
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
extern crate mech_html;
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
  //math_sin, 
  //math_cos, 
  //math_floor,
  //math_round,
};
use mech_html::*;
use mech_html::{
  elements::*,
  shapes::*,
};
use web_sys::{ErrorEvent, MessageEvent, WebSocket, FileReader};
use std::sync::Arc;

mod websocket;

static PI: f64 = 3.141592654;

pub use self::websocket::*;

#[macro_export]
macro_rules! log {
  ( $( $t:tt )* ) => {
    web_sys::console::log_1(&format!( $( $t )* ).into());
  }
}

#[wasm_bindgen]
pub struct WasmCore {
  core: mech_core::Core,
  //programs: Vec<Program>,
  changes: Vec<Change>,
  images: HashMap<u64, web_sys::HtmlImageElement>,
  canvases: HashSet<u64>,
  /*nodes: HashMap<u64, Vec<u64>>,*/
  websocket: Option<web_sys::WebSocket>,
  remote_tables: HashSet<(TableId,RegisterIndex,RegisterIndex)>,
  event_id: u64,
  timers: HashMap<usize,Closure<dyn FnMut()>>,
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
    let html_code = r#"
HTML
=====

This is the HTML machine. It's inside of the wasm crate right now, but I'll move it soon. to its proper crate in the machines directory.

Timers
  #time/timer = [|period<s> ticks<u64>|]

Pointer events
  #html/event/pointer-move = [|x<f32> y<f32> target<string> event-id<u64>| 0 0 "" 0]
  #html/event/pointer-down = [|x<f32> y<f32> target<string> event-id<u64>| 0 0 "" 0]
  #html/event/pointer-up = [|x<f32> y<f32> target<string> event-id<u64>| 0 0 "" 0]

Keyboard events
  #html/event/key-up = [|key<string> event-id<u64>| "" 0]
  #html/event/key-down = [|key<string> event-id<u64>| "" 0]"#;

    let mut compiler = Compiler::new();
    let sections = compiler.compile_str(&html_code).unwrap();
    mech.load_sections(sections);

    WasmCore {
      core: mech,
      //programs: Vec::new(),
      changes: Vec::new(),
      images: HashMap::new(),
      canvases: HashSet::new(),
      /*nodes: HashMap::new(),*/
      websocket: None,
      remote_tables: HashSet::new(),
      event_id: 0,
      timers: HashMap::new(),
      apps: HashSet::new(),
      window: web_sys::window().unwrap(),
      document: web_sys::window().unwrap().document().unwrap(),
    }
  }
  
  pub fn connect_remote_core(&mut self, address: String) -> Result<(),JsValue> {
    connect_remote_core(self,address)
  }

  pub fn add_timers(&mut self) -> Result<(),JsValue> {
   
    let window = web_sys::window().expect("no global `window` exists");
    
    match self.core.get_table("time/timer") {
      Ok(timers_table) => {
        let timers_table_brrw = timers_table.borrow();
        for row in 1..=timers_table_brrw.rows {
          match self.timers.entry(row) {
            Entry::Occupied(timer) => {
              // TODO Do something here to alert that the timer was already added
            },
            Entry::Vacant(v) => {
              self.changes.push(Change::Set((
                *TIME_TIMER, vec![
                (TableIndex::Index(1), 
                TableIndex::Alias(*TICKS),
                Value::U64(U64::new(0)))],
              )));             
              self.process_transaction();
              match timers_table_brrw.get(&TableIndex::Index(row), &TableIndex::Alias(*PERIOD)) {
                Ok(Value::Time(period)) => {
                  let wasm_core = self as *mut WasmCore;
                  let closure = || { 
                    Closure::wrap(Box::new(move || {
                      unsafe{
                        let timer_table = (*wasm_core).core.get_table("time/timer").unwrap();
                        let timer_table_brrw = timer_table.borrow();
                        match timer_table_brrw.get(&TableIndex::Index(row), &TableIndex::Alias(*TICKS)) {
                          Ok(Value::U64(ticks)) => {
                            (*wasm_core).changes.push(Change::Set((
                              *TIME_TIMER, vec![
                              (TableIndex::Index(row), 
                              TableIndex::Alias(*TICKS),
                              Value::U64(U64::new(ticks.unwrap()+1)))],
                            )));           
                            (*wasm_core).process_transaction();
                            (*wasm_core).render();
                            //let table = (*wasm_core).core.get_table("circle").unwrap();
                            //log!("{:?}",(*wasm_core).core);
                          }
                          x => {log!("6868 {:?}", x);},
                        }
                      }
                    }) as Box<dyn FnMut()>)
                  };
                  let timer_callback = closure();
                  let id = window.set_interval_with_callback_and_timeout_and_arguments_0(
                    timer_callback.as_ref().unchecked_ref(),
                    (period.unwrap() * 1000.0) as i32
                  ).unwrap();
                  self.timers.insert(row,timer_callback);
                }
                x => {log!("6869 {:?}", x);},
              }
            }
          }
        }
      }
      x => {log!("6870 {:?}", x);},
    }   
    Ok(())
  }

  pub fn load_compressed_blocks(&mut self, encoded_miniblocks: String) {
    let compressed_miniblocks = decode(encoded_miniblocks).unwrap();
    let serialized_miniblocks = decompress_to_vec(compressed_miniblocks.as_slice()).expect("Failed to decompress!");
    self.load_sections(serialized_miniblocks);
  }

  pub fn load_sections(&mut self, serialized_miniblocks: Vec<u8>) -> Result<(),JsValue> {
    let miniblocks: Vec<Vec<MiniBlock>> = match bincode::deserialize(&serialized_miniblocks) {
      Ok(miniblocks) => miniblocks,
      Err(x) => {
        return Err(JsValue::from_str("5239"));
      }
    };
    let mut len = 0;
    let mut sections = vec![];
    for section in miniblocks {
      let mut blocks: Vec<Block> = Vec::new();
      let blocks = section.iter().map(|b| MiniBlock::maximize_block(&b)).collect::<Vec<Block>>();
      len += blocks.len();
      sections.push(blocks);
    }
    self.core.load_sections(sections);
    self.core.schedule_blocks();
    self.add_timers();
    self.add_apps();
    self.render();
    log!("Loaded {} blocks.", len);
    Ok(())
  }

  pub fn process_transaction(&mut self) {
    // Collect the new table messages
    let mut set_tables: Vec<(TableId,RegisterIndex,RegisterIndex)> = vec![];
    for change in self.changes.iter() {
      match change {
        // If any new tables are sent, mark them as remote tables
        // Remote tables are any tables that are not defined in the
        // current core, but may be used (written to or read from).
        Change::NewTable{table_id,..} => {
          self.remote_tables.insert((TableId::Global(*table_id),RegisterIndex::All,RegisterIndex::All));
        }
        Change::Set((table_id,data)) => {
          set_tables.push((TableId::Global(*table_id),RegisterIndex::All,RegisterIndex::All));
        }
        _ => (),
      }
    }
    self.core.process_transaction(&self.changes);
    // For any tables that have changed which are remote,
    // send a record of those changes back to the sender.
    for register in set_tables {
      match self.core.schedule.trigger_to_output.get(&register) {
        Some(out_registers) => {
          for (table_id,_,_) in self.remote_tables.intersection(out_registers) {
            let table = self.core.get_table_by_id(*table_id.unwrap()).unwrap();
            let changes = table.borrow().data_to_changes();
            match &self.websocket {
              Some(ws) => {
                let message = bincode::serialize(&SocketMessage::Transaction(changes)).unwrap();
                ws.send_with_u8_array(&message);        
              }
              _ => (),
            }
          }
        }
        _ => (),
      }
    }
    // TODO Send changes to remote cores via websocket
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
                Value::U64(U64::new(eid)))])));
            (*wasm_core).process_transaction();
            (*wasm_core).render();
            //log!("Keydown");
            //let table = (*wasm_core).core.get_table("balls");
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
                Value::F32(F32::new(x as f32)))])));    
            (*wasm_core).changes.push(Change::Set((
              table_id, vec![(
                TableIndex::Index(1), 
                TableIndex::Alias(*Y),
                Value::F32(F32::new(y as f32)))])));              
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
                Value::U64(U64::new(eid)))])));  
            (*wasm_core).process_transaction();
            (*wasm_core).render();
            //let table = (*wasm_core).core.get_table_by_id(hash_str("balls"));
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
          if hash.len() > 6 && hash[0..6].to_string()=="debug=".to_string() {
            let table = (*wasm_core).core.get_table_by_id(hash_str(&hash[6..]));
            log!("{:?}", table);
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
    match self.core.get_table("html/app") {
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
                      let app = render_value(contents,&self.document,&self.core)?;
                      drawing_area.append_child(&app)?;
                    }
                    x => log!("4845 {:?}",x),
                  }
                }
              }
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

  pub fn render(&mut self) -> Result<(), JsValue> {
    let wasm_core = self as *mut WasmCore;
    self.draw_canvases();
    Ok(())
  }

  pub fn draw_canvases(&mut self) -> Result<(), JsValue> {
    let canvases = self.document.get_elements_by_tag_name("canvas");
    for i in 0..canvases.length() {
      let canvas = canvases.get_with_index(i).unwrap();
      let canvas: web_sys::HtmlCanvasElement = canvas
                    .dyn_into::<web_sys::HtmlCanvasElement>()
                    .map_err(|_| ())
                    .unwrap();
      draw_canvas(&canvas,&self.core);
    }
    Ok(())
  }

}
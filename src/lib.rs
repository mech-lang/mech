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

mod shapes;
mod elements;
mod websocket;

static PI: f64 = 3.141592654;

pub use self::shapes::*;
pub use self::elements::*;
pub use self::websocket::*;

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
  images: HashMap<u64, web_sys::HtmlImageElement>,
  canvases: HashSet<u64>,
  /*nodes: HashMap<u64, Vec<u64>>,
  websocket: Option<web_sys::WebSocket>,
  remote_tables: HashSet<Register>,*/
  event_id: u32,
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
    WasmCore {
      core: mech,
      //programs: Vec::new(),
      changes: Vec::new(),
      images: HashMap::new(),
      canvases: HashSet::new(),
      /*nodes: HashMap::new(),
      websocket: None,
      remote_tables: HashSet::new(),*/
      event_id: 0,
      timers: HashMap::new(),
      apps: HashSet::new(),
      window: web_sys::window().unwrap(),
      document: web_sys::window().unwrap().document().unwrap(),
    }
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
                Value::U64(0))],
              )));             
              self.process_transaction();
              match timers_table_brrw.get(&TableIndex::Index(row), &TableIndex::Alias(*PERIOD)) {
                Ok(Value::I32(period)) => {
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
                              Value::U64(ticks+1))],
                            )));           
                            (*wasm_core).process_transaction();
                            (*wasm_core).render();
                            //let table = (*wasm_core).core.get_table_by_name("mouth").unwrap();
                            //log!("{:?}", table);
                            //log!("{:?}", table.get_f32(&TableIndex::Index(1),&TableIndex::Index(4)).unwrap());
                          }
                          x => {log!("6868 {:?}", x);},
                        }
                      }
                    }) as Box<dyn FnMut()>)
                  };
                  let timer_callback = closure();
                  let id = window.set_interval_with_callback_and_timeout_and_arguments_0(
                    timer_callback.as_ref().unchecked_ref(),
                    period
                  ).unwrap();
                  self.timers.insert(row,timer_callback);
                }
                x => {log!("6868 {:?}", x);},
              }
            }
          }
        }
      }
      x => {log!("6868 {:?}", x);},
    }   
    Ok(())
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
    //self.add_timers();
    self.add_apps();
    self.render();
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
      Value::Reference(TableId::Global(table_id)) => {
        let table = self.core.get_table_by_id(table_id).unwrap();
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
              // Render an HTML element
              if raw_kind == *DIV { render_div(table,&mut container,wasm_core)?; }
              else if raw_kind == *A { render_link(table,&mut container,wasm_core)?; }
              else if raw_kind == *IMG { render_img(table,&mut container,wasm_core)?; }
              else if raw_kind == *BUTTON { render_button(table,&mut container,wasm_core)?; }
              else if raw_kind == *SLIDER { render_slider(table,&mut container,wasm_core)?; }
              else if raw_kind == *CANVAS { render_canvas(table,&mut container,wasm_core)?; }
              else {
                log!("4744 {:?}", raw_kind);
              }
            }
            x => log!("4745 {:?}",x),
            Err(x) => log!("4746 {:?}",x),
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
              x => log!("4747 {:?}",x),
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
  
    let wasm_core = self as *mut WasmCore;
    let context = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();

    // Get the elements table for this canvas
    let elements_table_id_string = canvas.get_attribute("elements").unwrap();
    let elements_table_id: u64 = elements_table_id_string.parse::<u64>().unwrap();
    let elements_table = self.core.get_table_by_id(elements_table_id).unwrap();
    let elements_table_brrw = elements_table.borrow();
    let context = Rc::new(context);
    context.clear_rect(0.0, 0.0, canvas.width().into(), canvas.height().into());
    for row in 1..=elements_table_brrw.rows as usize {
      match (elements_table_brrw.get(&TableIndex::Index(row), &TableIndex::Alias(*SHAPE)),
      elements_table_brrw.get(&TableIndex::Index(row), &TableIndex::Alias(*PARAMETERS))) {
        (Ok(Value::String(shape)), Ok(Value::Reference(parameters_table_id))) => {
          let shape = shape.hash();
          let parameters_table = self.core.get_table_by_id(*parameters_table_id.unwrap()).unwrap();
          // Render a shape
          if shape == *CIRCLE { render_circle(parameters_table,&context)?; }
          else if shape == *ELLIPSE { render_ellipse(parameters_table,&context)?; }
          else if shape == *ARC { render_arc(parameters_table,&context)?; }
          else if shape == *RECTANGLE { render_rectangle(parameters_table,&context)?; } 
          else if shape == *TEXT { render_text(parameters_table,&context,wasm_core)?; }
          else if shape == *PATH { render_path(parameters_table,&context,wasm_core)?; }
          else if shape == *IMAGE { render_image(parameters_table,&context,wasm_core)?; }
          else {
            log!("5869");
          }
        },
        x => {log!("5870 {:?}", x);},
      }
    }
    Ok(())
  }

}
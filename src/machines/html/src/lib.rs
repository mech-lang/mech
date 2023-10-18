#![recursion_limit="256"]
#![feature(alloc)]
#![feature(get_mut_unchecked)]
#![allow(warnings)]
extern crate wasm_bindgen;
extern crate hashbrown;
#[macro_use]
extern crate alloc;
extern crate core;
extern crate web_sys;
extern crate mech_core;
extern crate mech_utilities;
extern crate bincode;
#[macro_use]
extern crate lazy_static;
extern crate miniz_oxide;
extern crate base64;
//use mech_core::{Interner, Transaction};
//use mech_core::Value;

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
use mech_core::*;
use mech_utilities::{SocketMessage, MiniBlock};
use web_sys::{ErrorEvent, MessageEvent, WebSocket, FileReader};
use std::sync::Arc;

pub mod shapes;
pub mod elements;

pub use self::shapes::*;
pub use self::elements::*;

static PI: f64 = 3.14159265358979323846264338327950288;

#[macro_export]
macro_rules! log {
  ( $( $t:tt )* ) => {
    web_sys::console::log_1(&format!( $( $t )* ).into());
  }
}

lazy_static! {
  pub static ref HTML_APP: u64 = hash_str("html/app");
  pub static ref DIV: u64 = hash_str("div");
  pub static ref A: u64 = hash_str("a");
  pub static ref IMG: u64 = hash_str("img");
  pub static ref SRC: u64 = hash_str("src");
  pub static ref CONTAINS: u64 = hash_str("contains");
  pub static ref ROOT: u64 = hash_str("root");
  pub static ref TYPE: u64 = hash_str("type");
  pub static ref KIND: u64 = hash_str("kind");
  pub static ref HREF: u64 = hash_str("href");
  pub static ref BUTTON: u64 = hash_str("button");
  pub static ref SLIDER: u64 = hash_str("slider");
  pub static ref MIN: u64 = hash_str("min");
  pub static ref MAX: u64 = hash_str("max");
  pub static ref VALUE: u64 = hash_str("value");
  pub static ref CANVAS: u64 = hash_str("canvas");
  pub static ref PARAMETERS: u64 = hash_str("parameters");
  pub static ref HEIGHT: u64 = hash_str("height");
  pub static ref WIDTH: u64 = hash_str("width");
  pub static ref SHAPE: u64 = hash_str("shape");
  pub static ref CIRCLE: u64 = hash_str("circle");
  pub static ref RECTANGLE: u64 = hash_str("rectangle");
  pub static ref LINE: u64 = hash_str("line");
  pub static ref PATH: u64 = hash_str("path");
  pub static ref START__POINT: u64 = hash_str("start-point");
  pub static ref LINE__WIDTH: u64 = hash_str("line-width");
  pub static ref START__ANGLE: u64 = hash_str("start-angle");
  pub static ref END__ANGLE: u64 = hash_str("end-angle");
  pub static ref QUADRATIC: u64 = hash_str("quadratic");
  pub static ref CONTROL__POINT: u64 = hash_str("control-point");
  pub static ref CONTROL__POINTS: u64 = hash_str("control-points");
  pub static ref END__POINT: u64 = hash_str("end-point");
  pub static ref X1: u64 = hash_str("x1");
  pub static ref X2: u64 = hash_str("x2");
  pub static ref Y1: u64 = hash_str("y1");
  pub static ref Y2: u64 = hash_str("y2");
  pub static ref RADIUS: u64 = hash_str("radius");
  pub static ref STROKE: u64 = hash_str("stroke");
  pub static ref FILL: u64 = hash_str("fill");
  pub static ref CENTER__X: u64 = hash_str("center-x");
  pub static ref CENTER__Y: u64 = hash_str("center-y");
  pub static ref IMAGE: u64 = hash_str("image");
  pub static ref X: u64 = hash_str("x");
  pub static ref Y: u64 = hash_str("y");
  pub static ref ROTATE: u64 = hash_str("rotate");
  pub static ref TRANSLATE: u64 = hash_str("translate");
  pub static ref SOURCE: u64 = hash_str("source");
  pub static ref TIME_TIMER: u64 = hash_str("time/timer");
  pub static ref PERIOD: u64 = hash_str("period");
  pub static ref TICKS: u64 = hash_str("ticks");
  pub static ref HTML_EVENT_POINTER__MOVE: u64 = hash_str("html/event/pointer-move");
  pub static ref HTML_EVENT_POINTER__DOWN: u64 = hash_str("html/event/pointer-down");
  pub static ref HTML_EVENT_POINTER__UP: u64 = hash_str("html/event/pointer-up");
  pub static ref HTML_EVENT_KEY__DOWN: u64 = hash_str("html/event/key-down");
  pub static ref HTML_EVENT_KEY__UP: u64 = hash_str("html/event/key-up");
  pub static ref TARGET: u64 = hash_str("target");
  pub static ref KEY: u64 = hash_str("key");
  pub static ref EVENT__ID: u64 = hash_str("event-id");
  pub static ref ARC: u64 = hash_str("arc");
  pub static ref ELLIPSE: u64 = hash_str("ellipse");
  pub static ref MAJOR__AXIS: u64 = hash_str("major-axis");
  pub static ref MINOR__AXIS: u64 = hash_str("minor-axis");
  pub static ref STARTING__ANGLE: u64 = hash_str("starting-angle");
  pub static ref ENDING__ANGLE: u64 = hash_str("ending-angle");
  pub static ref TEXT: u64 = hash_str("text");
  pub static ref FONT: u64 = hash_str("font");
  pub static ref SIZE: u64 = hash_str("size");
  pub static ref FACE: u64 = hash_str("face");
  pub static ref STYLE: u64 = hash_str("style");
  pub static ref WEIGHT: u64 = hash_str("weight");
  pub static ref BOLD: u64 = hash_str("bold");
  pub static ref NORMAL: u64 = hash_str("normal");
  pub static ref ITALIC: u64 = hash_str("italic");
  pub static ref FAMILY: u64 = hash_str("family");
  pub static ref DIRECTION: u64 = hash_str("direction");
  pub static ref ALIGNMENT: u64 = hash_str("alignment");
  pub static ref START: u64 = hash_str("start");
  pub static ref END: u64 = hash_str("end");
  pub static ref LEFT: u64 = hash_str("left");
  pub static ref RIGHT: u64 = hash_str("right");
  pub static ref CENTER: u64 = hash_str("center");
  pub static ref BEZIER: u64 = hash_str("bezier");
  pub static ref HTML_LOCATION: u64 = hash_str("html/location");
  pub static ref HASH: u64 = hash_str("hash");
  pub static ref HOST: u64 = hash_str("host");
  pub static ref HOST__NAME: u64 = hash_str("host-name");
  pub static ref ORIGIN: u64 = hash_str("origin");
  pub static ref PATH__NAME: u64 = hash_str("path-name");
  pub static ref PORT: u64 = hash_str("port");
  pub static ref PROTOCOL: u64 = hash_str("protocol");
  pub static ref SEARCH: u64 = hash_str("search");
  pub static ref SCALE: u64 = hash_str("scale");
}

pub fn render_value(value: Value, document: &web_sys::Document, core: &mech_core::Core) -> Result<web_sys::Element, JsValue> {
  let mut div = document.create_element("div")?;
  match value {
    Value::String(chars) => {
      let contents_string = chars.to_string();
      div.set_inner_html(&contents_string);
    },
    Value::F32(x) => div.set_inner_html(&format!("{:.2?}", x)),
    Value::F64(x) => div.set_inner_html(&format!("{:?}", x)),
    Value::U128(x) => div.set_inner_html(&format!("{:?}", x)),
    Value::U64(x) => div.set_inner_html(&format!("{:?}", x)),
    Value::U32(x) => div.set_inner_html(&format!("{:?}", x)),
    Value::U16(x) => div.set_inner_html(&format!("{:?}", x)),
    Value::U8(x) => div.set_inner_html(&format!("{:?}", x)),
    Value::I128(x) => div.set_inner_html(&format!("{:?}", x)),
    Value::I64(x) => div.set_inner_html(&format!("{:?}", x)),
    Value::I32(x) => div.set_inner_html(&format!("{:?}", x)),
    Value::I16(x) => div.set_inner_html(&format!("{:?}", x)),
    Value::I8(x) => div.set_inner_html(&format!("{:?}", x)),
    Value::Reference(TableId::Global(table_id)) => {
      let table = core.get_table_by_id(table_id).unwrap();
      let rendered_ref = make_element(&table.borrow(), document, core)?;
      div.append_child(&rendered_ref)?;
    }
    x => log!("4745 {:?}",x),
  }
  Ok(div)
}

pub fn draw_canvas(canvas: &web_sys::HtmlCanvasElement, core: &mech_core::Core) -> Result<(), JsValue> {
  
  let context = canvas
      .get_context("2d")
      .unwrap()
      .unwrap()
      .dyn_into::<web_sys::CanvasRenderingContext2d>()
      .unwrap();

  // Get the elements table for this canvas
  let elements_table_id_string = canvas.get_attribute("elements").unwrap();
  let elements_table_id: u64 = elements_table_id_string.parse::<u64>().unwrap();
  let elements_table = core.get_table_by_id(elements_table_id).unwrap();
  let elements_table_brrw = elements_table.borrow();
  let context = Rc::new(context);
  context.clear_rect(0.0, 0.0, canvas.width().into(), canvas.height().into());
  for row in 1..=elements_table_brrw.rows as usize {
    match (elements_table_brrw.get(&TableIndex::Index(row), &TableIndex::Alias(*SHAPE)),
    elements_table_brrw.get(&TableIndex::Index(row), &TableIndex::Alias(*PARAMETERS))) {
      (Ok(Value::String(shape)), Ok(Value::Reference(parameters_table_id))) => {
        let shape = shape.hash();
        let parameters_table = core.get_table_by_id(*parameters_table_id.unwrap()).unwrap();
        // Render a shape
        if shape == *CIRCLE { render_circle(parameters_table,&context)?; }
        else if shape == *ELLIPSE { render_ellipse(parameters_table,&context)?; }
        else if shape == *ARC { render_arc(parameters_table,&context)?; }
        else if shape == *RECTANGLE { render_rectangle(parameters_table,&context)?; } 
        else if shape == *TEXT { render_text(parameters_table,&context,core)?; }
        else if shape == *PATH { render_path(parameters_table,&context,core)?; }
        else if shape == *IMAGE { render_image(parameters_table,&context,core)?; }
        else {
          log!("5869");
        }
      },
      x => {log!("5870 {:?}", x);},
    }
  }
  Ok(())
}

pub fn make_element(table: &Table, document: &web_sys::Document, core: &mech_core::Core) -> Result<web_sys::Element, JsValue> {
  let mut container: web_sys::Element = document.create_element("div")?;
  let element_id = hash_str(&format!("div-{:?}", table.id));
  container.set_id(&format!("{:?}",element_id));
  container.set_attribute("table-id", &format!("{}", table.id))?;
  // First check to see if the table has a "type" column. If it doesn't, just render the table
  match table.col_map.get_index(&*KIND) {
    Ok(_) => {
      for row in 1..=table.rows {
        match table.get(&TableIndex::Index(row), &TableIndex::Alias(*KIND))  {
          Ok(Value::String(kind)) => {
            let raw_kind = kind.hash();
            // Render an HTML element
            if raw_kind == *DIV { render_div(table,&mut container, document, core)?; }
            else if raw_kind == *A { render_link(table,&mut container, document, core)?; }
            else if raw_kind == *IMG { render_img(table,&mut container,document, core)?; }
            else if raw_kind == *BUTTON { render_button(table, &mut container, document, core)?; }
            else if raw_kind == *SLIDER { render_slider(table, &mut container, document, core)?; }
            else if raw_kind == *CANVAS { render_canvas(table, &mut container, document, core)?; }
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
    Err(_) => {
      // Make a div for each row
      for row in 1..=table.rows {
        let mut row_div = document.create_element("div")?;
        let element_id = hash_str(&format!("div-{:?}-{:?}", table.id, row));
        row_div.set_id(&format!("{:?}",element_id));
        // Make an internal div for each cell 
        for column in 1..=table.cols {
          // Get contents
          match table.get(&TableIndex::Index(row), &TableIndex::Index(column)) {
            Ok(contents) => {
              let mut cell_div = document.create_element("div")?;
              let element_id = hash_str(&format!("div-{:?}-{:?}-{:?}", table.id, row, column));
              let rendered = render_value(contents, document, core)?;
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
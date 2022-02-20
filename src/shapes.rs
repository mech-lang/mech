
use mech_core::*;
use std::cell::RefCell;
use std::rc::Rc;
use web_sys::CanvasRenderingContext2d;
use wasm_bindgen::JsValue;
use crate::*;

// Define a function to make this a lot easier
fn get_stroke_string(parameters_table: &Table, row: usize, alias: u64) -> String { 
  match parameters_table.get(&TableIndex::Index(row), &TableIndex::Alias(alias))  {
    Ok(Value::U128(stroke)) => {
      let mut color_string: String = "#".to_string();
      color_string = format!("{}{:02x}", color_string, stroke);
      log!("{:?}", color_string);
      color_string
    }
    _ => "#000000".to_string(),
  }
}

fn get_line_width(parameters_table: &Table, row: usize) -> f64 {
  match parameters_table.get(&TableIndex::Index(row), &TableIndex::Alias(*LINE__WIDTH))  {
    Ok(Value::F32(line_width)) => line_width as f64,
    _ => 1.0,
  }
}

fn get_property(parameters_table: &Table, row: usize, alias: u64) -> String {
  match parameters_table.get(&TableIndex::Index(row), &TableIndex::Alias(alias))  {
    Ok(Value::F32(property)) => format!("{:?}", property),
    Ok(Value::String(property)) => property.to_string(),
    _ => "".to_string()
  }
}

pub fn render_circle(parameters_table: Rc<RefCell<Table>>, context: &Rc<CanvasRenderingContext2d>) -> Result<(),JsValue> {
  let parameters_table_brrw = parameters_table.borrow();
  for row in 1..=parameters_table_brrw.rows {
    match (parameters_table_brrw.get(&TableIndex::Index(row), &TableIndex::Alias(*CENTER__X)),
      parameters_table_brrw.get(&TableIndex::Index(row), &TableIndex::Alias(*CENTER__Y)),
      parameters_table_brrw.get(&TableIndex::Index(row), &TableIndex::Alias(*RADIUS))) {
    (Ok(Value::F32(cx)), Ok(Value::F32(cy)), Ok(Value::F32(radius))) => {
      let stroke = get_stroke_string(&parameters_table_brrw,row, *STROKE);
      let fill = get_stroke_string(&parameters_table_brrw,row, *FILL);
      let line_width = get_line_width(&parameters_table_brrw,row);
      context.save();
      context.begin_path();
      context.arc(cx.into(), cy.into(), radius.into(), 0.0, 2.0 * PI);
      context.set_fill_style(&JsValue::from_str(&fill));
      context.fill();
      context.set_stroke_style(&JsValue::from_str(&stroke));
      context.set_line_width(line_width.into());    
      context.stroke();                
      context.restore();
    }
      x => {log!("5854 {:?}", x);},
    }        
  }
  Ok(())
}

pub fn render_ellipse(parameters_table: Rc<RefCell<Table>>, context: &Rc<CanvasRenderingContext2d>) -> Result<(),JsValue> {
  let parameters_table_brrw = parameters_table.borrow();
  for row in 1..=parameters_table_brrw.rows {
    match (parameters_table_brrw.get(&TableIndex::Index(row), &TableIndex::Alias(*CENTER__X)),
          parameters_table_brrw.get(&TableIndex::Index(row), &TableIndex::Alias(*CENTER__Y)),
          parameters_table_brrw.get(&TableIndex::Index(row), &TableIndex::Alias(*MAJOR__AXIS)),
          parameters_table_brrw.get(&TableIndex::Index(row), &TableIndex::Alias(*MINOR__AXIS))) {
      (Ok(Value::F32(cx)), Ok(Value::F32(cy)), Ok(Value::F32(maja)), Ok(Value::F32(mina))) => {
        let stroke = get_stroke_string(&parameters_table_brrw,row, *STROKE);
        let fill = get_stroke_string(&parameters_table_brrw,row, *FILL);
        let line_width = get_line_width(&parameters_table_brrw,row);
        context.save();
        context.begin_path();
        context.ellipse(cx.into(), cy.into(), maja.into(), mina.into(), 0.0, 0.0, 2.0 * PI);
        context.set_fill_style(&JsValue::from_str(&fill));
        context.fill();
        context.set_stroke_style(&JsValue::from_str(&stroke));
        context.set_line_width(line_width.into());    
        context.stroke();                
        context.restore();
      }
      x => {log!("5855 {:?}", x);},
    }   
  }     
  Ok(())
}

pub fn render_arc(parameters_table: Rc<RefCell<Table>>, context: &Rc<CanvasRenderingContext2d>) -> Result<(),JsValue> {
  let parameters_table_brrw = parameters_table.borrow();
  for row in 1..=parameters_table_brrw.rows {
    match (parameters_table_brrw.get(&TableIndex::Index(row), &TableIndex::Alias(*CENTER__X)),
          parameters_table_brrw.get(&TableIndex::Index(row), &TableIndex::Alias(*CENTER__Y)),
          parameters_table_brrw.get(&TableIndex::Index(row), &TableIndex::Alias(*STARTING__ANGLE)),
          parameters_table_brrw.get(&TableIndex::Index(row), &TableIndex::Alias(*ENDING__ANGLE)),
          parameters_table_brrw.get(&TableIndex::Index(row), &TableIndex::Alias(*RADIUS))) {
      (Ok(Value::F32(cx)), Ok(Value::F32(cy)), Ok(Value::F32(sa)), Ok(Value::F32(ea)), Ok(Value::F32(radius))) => {
        let stroke = get_stroke_string(&parameters_table_brrw,row, *STROKE);
        let fill = get_stroke_string(&parameters_table_brrw,row, *FILL);
        let line_width = get_line_width(&parameters_table_brrw,row);
        context.save();
        context.begin_path();
        context.arc(cx.into(), cy.into(), radius.into(), sa as f64 * PI / 180.0, ea as f64 * PI / 180.0);
        context.set_fill_style(&JsValue::from_str(&fill));
        context.fill();
        context.set_stroke_style(&JsValue::from_str(&stroke));
        context.set_line_width(line_width);    
        context.stroke();                
        context.restore();
      }
      x => {log!("5856 {:?}", x);},
    }        
  }
  Ok(())
}
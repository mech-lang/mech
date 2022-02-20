
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

pub fn render_rectangle(parameters_table: Rc<RefCell<Table>>, context: &Rc<CanvasRenderingContext2d>) -> Result<(),JsValue> {
  let parameters_table_brrw = parameters_table.borrow();
  for row in 1..=parameters_table_brrw.rows {
    match (parameters_table_brrw.get(&TableIndex::Index(row), &TableIndex::Alias(*X)),
          parameters_table_brrw.get(&TableIndex::Index(row), &TableIndex::Alias(*Y)),
          parameters_table_brrw.get(&TableIndex::Index(row), &TableIndex::Alias(*WIDTH)),
          parameters_table_brrw.get(&TableIndex::Index(row), &TableIndex::Alias(*HEIGHT))) {
      (Ok(Value::F32(x)), Ok(Value::F32(y)), Ok(Value::F32(width)), Ok(Value::F32(height))) => {
        let stroke = get_stroke_string(&parameters_table_brrw,row, *STROKE);
        let fill = get_stroke_string(&parameters_table_brrw,row, *FILL);
        let line_width = get_line_width(&parameters_table_brrw,row);
        context.save();
        context.set_fill_style(&JsValue::from_str(&fill));
        context.fill_rect(x.into(),y.into(),width.into(),height.into());
        context.set_stroke_style(&JsValue::from_str(&stroke));
        context.set_line_width(line_width);
        context.stroke_rect(x.into(),y.into(),width.into(),height.into());
        context.restore();
      }
      x => {log!("5857 {:?}", x);},
    }
  }
  Ok(())
}

pub fn render_text(parameters_table: Rc<RefCell<Table>>, context: &Rc<CanvasRenderingContext2d>, wasm_core: *mut WasmCore) -> Result<(),JsValue> {
  let parameters_table_brrw = parameters_table.borrow();
  for row in 1..=parameters_table_brrw.rows {
    match (parameters_table_brrw.get(&TableIndex::Index(row), &TableIndex::Alias(*TEXT)),
          parameters_table_brrw.get(&TableIndex::Index(row), &TableIndex::Alias(*X)),
          parameters_table_brrw.get(&TableIndex::Index(row), &TableIndex::Alias(*Y))) {
      (Ok(Value::String(text_value)), Ok(Value::F32(x)), Ok(Value::F32(y))) => {
        let stroke = get_stroke_string(&parameters_table_brrw,row, *STROKE);
        let fill = get_stroke_string(&parameters_table_brrw,row, *FILL);
        let line_width = get_line_width(&parameters_table_brrw,row);
        let text = get_property(&parameters_table_brrw, row, *TEXT);

        context.save();
        context.set_fill_style(&JsValue::from_str(&fill));
        context.set_line_width(line_width);
        match parameters_table_brrw.get(&TableIndex::Index(row), &TableIndex::Alias(*FONT)) {
          Ok(Value::Reference(font_table_id)) => {
            let font_table = unsafe{(*wasm_core).core.get_table_by_id(*font_table_id.unwrap()).unwrap()};
            let font_table_brrw = font_table.borrow();
            let size = get_property(&font_table_brrw, row, *SIZE);
            let face = match &*get_property(&font_table_brrw, row, *FACE) {
              "" => "sans-serif".to_string(),
              x => x.to_string(),
            };
            let font_string = format!("{}px {}", size, face);
            context.set_font(&*font_string);
          }
          _ => (),
        }
        context.fill_text(&text,x.into(),y.into());
        context.restore();
      }
      x => {log!("5858 {:?}", x);},
    }
  }
  Ok(())
}

pub fn render_line(parameters_table: Rc<RefCell<Table>>, context: &Rc<CanvasRenderingContext2d>) -> Result<(),JsValue> {
  let parameters_table_brrw = parameters_table.borrow();
  for row in 1..=parameters_table_brrw.rows {
    match (parameters_table_brrw.get(&TableIndex::Index(row), &TableIndex::Alias(*X)),
          parameters_table_brrw.get(&TableIndex::Index(row), &TableIndex::Alias(*Y))) {
      (Ok(Value::F32(x)),Ok(Value::F32(y))) => {
        context.line_to(x.into(), y.into());
      }
      x => {log!("5859 {:?}", x);},
    }
  }
  Ok(())
}

pub fn render_quadratic(parameters_table: Rc<RefCell<Table>>, context: &Rc<CanvasRenderingContext2d>, wasm_core: *mut WasmCore) -> Result<(),JsValue> {
  let parameters_table_brrw = parameters_table.borrow();
  for row in 1..=parameters_table_brrw.rows {
    let parameters_table_brrw = parameters_table.borrow();
    match (parameters_table_brrw.get(&TableIndex::Index(1), &TableIndex::Alias(*CONTROL__POINT)),
          parameters_table_brrw.get(&TableIndex::Index(1), &TableIndex::Alias(*END__POINT))) {
      (Ok(Value::Reference(TableId::Global(control__point_table_id))),Ok(Value::Reference(TableId::Global(end__point_table_id)))) => {
        let control__point_table = unsafe{(*wasm_core).core.get_table_by_id(control__point_table_id).unwrap()};
        let end__point_table = unsafe{(*wasm_core).core.get_table_by_id(end__point_table_id).unwrap()};
        let control__point_table_brrw = control__point_table.borrow();
        let end__point_table_brrw = end__point_table.borrow();
        match (control__point_table_brrw.get(&TableIndex::Index(1), &TableIndex::Alias(*X)),
              control__point_table_brrw.get(&TableIndex::Index(1), &TableIndex::Alias(*Y)),
              end__point_table_brrw.get(&TableIndex::Index(1), &TableIndex::Alias(*X)),
              end__point_table_brrw.get(&TableIndex::Index(1), &TableIndex::Alias(*Y))) {
          (Ok(Value::F32(cx)),Ok(Value::F32(cy)),Ok(Value::F32(ex)),Ok(Value::F32(ey))) => {
            context.quadratic_curve_to(cx.into(), cy.into(), ex.into(), ey.into());
          }
          x => {log!("5860 {:?}", x);},
        }
      }
      x => {log!("5861 {:?}", x);},
    }
  }
  Ok(())
}

pub fn render_bezier(parameters_table: Rc<RefCell<Table>>, context: &Rc<CanvasRenderingContext2d>, wasm_core: *mut WasmCore) -> Result<(),JsValue> {
  let parameters_table_brrw = parameters_table.borrow();
  match (parameters_table_brrw.get(&TableIndex::Index(1), &TableIndex::Alias(*CONTROL__POINTS)),
        parameters_table_brrw.get(&TableIndex::Index(1), &TableIndex::Alias(*END__POINT))) {
    (Ok(Value::Reference(TableId::Global(control__point_table_id))),Ok(Value::Reference(TableId::Global(end__point_table_id)))) => {
      let control__point_table = unsafe{(*wasm_core).core.get_table_by_id(control__point_table_id).unwrap()};
      let end__point_table = unsafe{(*wasm_core).core.get_table_by_id(end__point_table_id).unwrap()};
      let control__point_table_brrw = control__point_table.borrow();
      let end__point_table_brrw = end__point_table.borrow();
      match (control__point_table_brrw.get(&TableIndex::Index(1), &TableIndex::Alias(*X)),
            control__point_table_brrw.get(&TableIndex::Index(1), &TableIndex::Alias(*Y)),
            control__point_table_brrw.get(&TableIndex::Index(2), &TableIndex::Alias(*X)),
            control__point_table_brrw.get(&TableIndex::Index(2), &TableIndex::Alias(*Y)),
            end__point_table_brrw.get(&TableIndex::Index(1), &TableIndex::Alias(*X)),
            end__point_table_brrw.get(&TableIndex::Index(1), &TableIndex::Alias(*Y))) {
        (Ok(Value::F32(cx1)),Ok(Value::F32(cy1)),Ok(Value::F32(cx2)),Ok(Value::F32(cy2)),Ok(Value::F32(ex)),Ok(Value::F32(ey))) => {
          context.bezier_curve_to(cx1.into(), cy1.into(), cx2.into(), cy2.into(), ex.into(), ey.into());
        }
        x => {log!("5862 {:?}", x);},
      }
    }
    x => {log!("5863 {:?}", x);},
  }
  Ok(())
}

pub fn render_arc_path(parameters_table: Rc<RefCell<Table>>, context: &Rc<CanvasRenderingContext2d>, wasm_core: *mut WasmCore) -> Result<(),JsValue> {
  let parameters_table_brrw = parameters_table.borrow();
  match (parameters_table_brrw.get(&TableIndex::Index(1), &TableIndex::Alias(*CENTER__X)),
         parameters_table_brrw.get(&TableIndex::Index(1), &TableIndex::Alias(*CENTER__Y)),
         parameters_table_brrw.get(&TableIndex::Index(1), &TableIndex::Alias(*STARTING__ANGLE)),
         parameters_table_brrw.get(&TableIndex::Index(1), &TableIndex::Alias(*ENDING__ANGLE)),
         parameters_table_brrw.get(&TableIndex::Index(1), &TableIndex::Alias(*RADIUS))) {
    (Ok(Value::F32(cx)),Ok(Value::F32(cy)),Ok(Value::F32(sa)),Ok(Value::F32(ea)),Ok(Value::F32(radius))) => {
      context.arc(cx.into(), cy.into(), radius.into(), sa as f64 * PI / 180.0, ea as f64 * PI / 180.0);
    }
    x => {log!("5864 {:?}", x);},
  }
  Ok(())
}  

pub fn render_path(parameters_table: Rc<RefCell<Table>>, context: &Rc<CanvasRenderingContext2d>, wasm_core: *mut WasmCore) -> Result<(),JsValue> {
  let parameters_table_brrw = parameters_table.borrow();
  context.save();
  let rotate = match parameters_table_brrw.get(&TableIndex::Index(1), &TableIndex::Alias(*ROTATE)) {
    Ok(Value::F32(rotate)) => rotate,
    _ => 0.0,
  };
  let (tx,ty) = match parameters_table_brrw.get(&TableIndex::Index(1), &TableIndex::Alias(*TRANSLATE)) {
    Ok(Value::Reference(TableId::Global(translate_table_id))) => {
      let translate_table = unsafe{(*wasm_core).core.get_table_by_id(translate_table_id).unwrap()};
      let translate_table_brrw = translate_table.borrow();
      match (translate_table_brrw.get(&TableIndex::Index(1), &TableIndex::Alias(*X)),
              translate_table_brrw.get(&TableIndex::Index(1), &TableIndex::Alias(*Y))) {
        (Ok(Value::F32(tx)),Ok(Value::F32(ty))) => (tx,ty),
        _ => (0.0,0.0),
      }
    },
    _ => (0.0,0.0),
  };
  context.translate(tx.into(),ty.into());
  context.rotate(rotate as f64 * PI / 180.0);
  context.begin_path();
  
  match (parameters_table_brrw.get(&TableIndex::Index(1), &TableIndex::Alias(*START__POINT)),
          parameters_table_brrw.get(&TableIndex::Index(1), &TableIndex::Alias(*CONTAINS))) {
    (Ok(Value::Reference(start_point_id)), Ok(Value::Reference(TableId::Global(contains_table_id)))) => {
      let start_point_table = unsafe{(*wasm_core).core.get_table_by_id(*start_point_id.unwrap()).unwrap()};
      let start_point_table_brrw = start_point_table.borrow();
      match (start_point_table_brrw.get(&TableIndex::Index(1), &TableIndex::Alias(*X)),
              start_point_table_brrw.get(&TableIndex::Index(1), &TableIndex::Alias(*Y))) {
          (Ok(Value::F32(x)),Ok(Value::F32(y))) => {
            context.move_to(x.into(), y.into());
          // Get the contained shapes
          let contains_table = unsafe{(*wasm_core).core.get_table_by_id(contains_table_id).unwrap()};
          let contains_table_brrw = contains_table.borrow();
          for i in 1..=contains_table_brrw.rows {
            match (contains_table_brrw.get(&TableIndex::Index(i), &TableIndex::Alias(*SHAPE)),
                    contains_table_brrw.get(&TableIndex::Index(i), &TableIndex::Alias(*PARAMETERS))) {
              (Ok(Value::String(shape)),Ok(Value::Reference(TableId::Global(parameters_table_id)))) => {
                let shape = shape.hash();
                let parameters_table = unsafe{(*wasm_core).core.get_table_by_id(parameters_table_id).unwrap()};
                // Render a path element
                if shape == *LINE { render_line(parameters_table,&context)?; }
                else if shape == *QUADRATIC { render_quadratic(parameters_table,&context,wasm_core)?; }
                else if shape == *BEZIER { render_bezier(parameters_table,&context,wasm_core)?; }
                else if shape == *ARC { render_arc_path(parameters_table,&context,wasm_core)?; }
              }
              x => {log!("5865 {:?}", x);},
            }
          }
        }
        x => {log!("5866 {:?}", x);},
      }
      let stroke = get_stroke_string(&parameters_table_brrw,1, *STROKE);
      let line_width = get_line_width(&parameters_table_brrw,1);

      // Only set the stroke if it's included as a field
      match parameters_table_brrw.get(&TableIndex::Index(1), &TableIndex::Alias(*FILL))  {
        Ok(_) => {
          let fill = get_stroke_string(&parameters_table_brrw,1, *FILL);
          context.set_fill_style(&JsValue::from_str(&fill));
          context.fill();
        }
        _ => (),
      }
      context.set_stroke_style(&JsValue::from_str(&stroke));
      context.set_line_width(line_width);
      context.stroke();
    }
    x => {log!("5867 {:?}", x);},
  }
  //context.close_path();
  context.restore();
  Ok(())
}

pub fn render_image(parameters_table: Rc<RefCell<Table>>, context: &Rc<CanvasRenderingContext2d>, wasm_core: *mut WasmCore) -> Result<(),JsValue> {
  let parameters_table_brrw = parameters_table.borrow();
  for row in 1..=parameters_table_brrw.rows {
    match (parameters_table_brrw.get(&TableIndex::Index(row), &TableIndex::Alias(*SOURCE)),
          parameters_table_brrw.get(&TableIndex::Index(row), &TableIndex::Alias(*X)),
          parameters_table_brrw.get(&TableIndex::Index(row), &TableIndex::Alias(*Y)),
          parameters_table_brrw.get(&TableIndex::Index(row), &TableIndex::Alias(*ROTATE))) {
      (Ok(Value::String(source)), Ok(Value::F32(x)), Ok(Value::F32(y)), Ok(Value::F32(rotation))) => {
        let source_hash = source.hash();
        match unsafe{(*wasm_core).images.entry(source_hash)} {
          Entry::Occupied(img_entry) => {
            let img = img_entry.get();
            let ix = img.width() as f64 / 2.0;
            let iy = img.height() as f64 / 2.0;
            context.save();
            context.translate(x.into(), y.into());
            context.rotate(rotation as f64 * PI / 180.0);
            context.draw_image_with_html_image_element(&img, -ix as f64, -iy as f64);
            context.restore();
          },
          Entry::Vacant(v) => {
            let mut img = web_sys::HtmlImageElement::new().unwrap();
            img.set_src(&source.to_string());
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
      x => {log!("5868 {:?}", x);},
    }
  }
  Ok(())
}

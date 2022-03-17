use mech_core::*;
use wasm_bindgen::JsValue;
use web_sys::*;

use crate::*;

pub fn render_div(table: &Table, container: &mut web_sys::Element, wasm_core: *mut WasmCore) -> Result<(),JsValue> {
  for row in 1..=table.rows {
    // Get contents
    match table.get(&TableIndex::Index(row), &TableIndex::Alias(*CONTAINS)) {
      Ok(contents) => {
        let element_id = hash_str(&format!("div-{:?}-{:?}", table.id, row));
        let rendered = unsafe{(*wasm_core).render_value(contents)?};
        rendered.set_id(&format!("{:?}",element_id));
        container.append_child(&rendered)?;
      }
      x => {log!("4733 {:?}",x);},
    }
  }
  Ok(())
}

pub fn render_link(table: &Table, container: &mut web_sys::Element, wasm_core: *mut WasmCore) -> Result<(),JsValue> {
  for row in 1..=table.rows {
    match (table.get(&TableIndex::Index(row), &TableIndex::Alias(*HREF)),
           table.get(&TableIndex::Index(row), &TableIndex::Alias(*CONTAINS))) {
    (Ok(Value::String(href)), Ok(contents)) => {
      let element_id = hash_str(&format!("div-{:?}-{:?}", table.id, row));
      let rendered = unsafe{(*wasm_core).render_value(contents)?};
      rendered.set_id(&format!("{:?}",element_id));
      let mut link: web_sys::Element = unsafe{(*wasm_core).document.create_element("a")?};
      link.set_attribute("href",&href.to_string())?;
      let element_id = href.hash();
      link.set_id(&format!("{:?}",element_id));
      link.append_child(&rendered)?;
      container.append_child(&link)?;
    }
    x => {log!("4734 {:?}", x);},
    }
  }
  Ok(())
}

pub fn render_img(table: &Table, container: &mut web_sys::Element, wasm_core: *mut WasmCore) -> Result<(),JsValue> {
  for row in 1..=table.rows {
    match table.get(&TableIndex::Index(row), &TableIndex::Alias(*SRC)) {
      Ok(Value::String(src)) => {
        let mut img: web_sys::Element = unsafe{(*wasm_core).document.create_element("img")?};
        let element_id = hash_str(&format!("img-{:?}-{:?}", table.id, row));
        img.set_attribute("src", &src.to_string())?;
        img.set_id(&format!("{:?}",element_id));
        container.append_child(&img)?;
      }
      x => {log!("4735 {:?}", x);},
    }
  }
  Ok(())
}

pub fn render_button(table: &Table, container: &mut web_sys::Element, wasm_core: *mut WasmCore) -> Result<(),JsValue> {
  for row in 1..=table.rows {
    match table.get(&TableIndex::Index(row), &TableIndex::Alias(*CONTAINS)) {
      Ok(contents) => {
        let element_id = hash_str(&format!("div-{:?}-{:?}", table.id, row));
        let rendered = unsafe{(*wasm_core).render_value(contents)?};
        rendered.set_id(&format!("{:?}",element_id));
        let mut button: web_sys::Element = unsafe{(*wasm_core).document.create_element("button")?};
        let element_id = hash_str(&format!("button-{:?}-{:?}", table.id, row));
        button.set_id(&format!("{:?}",element_id));
        button.append_child(&rendered)?;
        container.append_child(&button)?;
      }
      x => {log!("4736 {:?}", x);},
    }
  }
  Ok(())
}

pub fn render_slider(table: &Table, container: &mut web_sys::Element, wasm_core: *mut WasmCore) -> Result<(),JsValue> {
  for row in 1..=table.rows {
    match (table.get(&TableIndex::Index(row), &TableIndex::Alias(*MIN)),
           table.get(&TableIndex::Index(row), &TableIndex::Alias(*MAX)),
           table.get(&TableIndex::Index(row), &TableIndex::Alias(*VALUE))) {
        (Ok(Value::F32(min)), Ok(Value::F32(max)), Ok(Value::F32(value))) => {
        let mut slider: web_sys::Element = unsafe{(*wasm_core).document.create_element("input")?};
        let mut slider: web_sys::HtmlInputElement = slider
          .dyn_into::<web_sys::HtmlInputElement>()
          .map_err(|_| ())
          .unwrap();
        let element_id = hash_str(&format!("slider-{:?}-{:?}", table.id, row));
        slider.set_attribute("type","range");
        slider.set_attribute("min", &format!("{:?}", min));
        slider.set_attribute("max", &format!("{:?}", max));
        slider.set_attribute("value", &format!("{:?}", value));
        slider.set_attribute("row", &format!("{:?}", row));
        slider.set_attribute("table", &format!("{:?}", table.id));
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
                let change = Change::Set((
                  table_id, vec![ 
                    (TableIndex::Index(row),
                    TableIndex::Alias(*VALUE),
                    Value::F32(F32::new(slider_value as f32)))]));
                // TODO Make this safe
                unsafe {
                  let table = (*wasm_core).core.get_table_by_id(table_id).unwrap();
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
      }
      x => {log!("4739 {:?}", x);},
    }
  }
  Ok(())
}

pub fn render_canvas(table: &Table, container: &mut web_sys::Element, wasm_core: *mut WasmCore) -> Result<(),JsValue> {
  for row in 1..=table.rows {
    match table.get(&TableIndex::Index(row), &TableIndex::Alias(*CONTAINS)) {
      Ok(contents) => {
        let mut canvas: web_sys::Element = unsafe{(*wasm_core).document.create_element("canvas")?};
        let element_id = hash_str(&format!("canvas-{:?}-{:?}", table.id, row));
        canvas.set_id(&format!("{:?}",element_id));
        unsafe{(*wasm_core).canvases.insert(element_id)};
        // Is there a parameters field?
        match table.get(&TableIndex::Index(row), &TableIndex::Alias(*PARAMETERS)) {
          Ok(Value::Reference(parameters_table_id)) => {
            let parameters_table = unsafe{(*wasm_core).core.get_table_by_id(*parameters_table_id.unwrap()).unwrap()};
            let parameters_table_brrw = parameters_table.borrow();
            match (parameters_table_brrw.get(&TableIndex::Index(1), &TableIndex::Alias(*HEIGHT)),
            parameters_table_brrw.get(&TableIndex::Index(1), &TableIndex::Alias(*WIDTH))) {
              (Ok(Value::F32(height)),Ok(Value::F32(width))) => {
                canvas.set_attribute("height", &format!("{:?}",height));
                canvas.set_attribute("width", &format!("{:?}",width));
              }
              x => {log!("4740 {:?}", x);},
            }
          }
          x => {log!("4741 {:?}", x);},
        }
        // Add the contents
        match table.get(&TableIndex::Index(row), &TableIndex::Alias(*CONTAINS)) {
          Ok(Value::Reference(contains_table_id)) => {
            canvas.set_attribute("elements", &format!("{}",contains_table_id.unwrap()));
          }
          x => {log!("4742 {:?}", x);},
        }
        container.append_child(&canvas)?;
      }
      x => {log!("4743 {:?}", x);},
    }
  }
  Ok(())
}




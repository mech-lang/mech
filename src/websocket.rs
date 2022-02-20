/*
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
  }*/


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
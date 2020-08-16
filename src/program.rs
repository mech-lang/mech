// # Program

// # Prelude
extern crate bincode;
extern crate libloading;
extern crate colored;
use colored::*;

use std::thread::{self, JoinHandle};
use std::collections::{HashMap, HashSet, Bound, BTreeMap};
use std::collections::hash_map::Entry;
use std::mem;
use std::fs::{OpenOptions, File, canonicalize, create_dir};
use std::io::{Write, BufReader, BufWriter};
use std::sync::Arc;
use std::rc::Rc;
use std::cell::RefCell;
use std::path::{Path, PathBuf};

use mech_core::{Core, humanize, Register, Transaction, Change, Error};
use mech_core::{Value, ValueMethods, ValueIterator, Index};
use mech_core::Block;
use mech_core::{Table, TableId};
use mech_core::hash_string;
use mech_syntax::compiler::Compiler;
use mech_utilities::{RunLoopMessage, MechCode, Machine, MachineRegistrar, MachineDeclaration};
use crossbeam_channel::Sender;
use crossbeam_channel::Receiver;

use libloading::Library;
use std::io::copy;

use time;

struct Registrar {
  machines: HashMap<u64, Box<dyn Machine>>,
}

impl Registrar {
  fn new() -> Registrar {
    Registrar {
      machines: HashMap::default(),
    }
  }
}

impl MachineRegistrar for Registrar {
  fn register_machine(&mut self, machine: Box<dyn Machine>) {
    self.machines.insert(machine.id(), machine);
  }
}

fn download_machine(machine_name: &str, name: &str, path_str: &str, ver: &str, outgoing: Option<crossbeam_channel::Sender<ClientMessage>>) -> Result<Library,Box<std::error::Error>> {
  create_dir("machines");

  let machine_file_path = format!("machines/{}",machine_name);
  {
    let path = Path::new(path_str);
    // Download from the web
    if path.to_str().unwrap().starts_with("https") {
      match outgoing {
        Some(sender) => {sender.send(ClientMessage::String(format!("{} {} v{}", "[Downloading]".bright_cyan(), name, ver)));}
        None => (),
      }
      let machine_url = format!("{}/{}", path_str, machine_name);
      let mut response = reqwest::get(machine_url.as_str())?;
      let mut dest = File::create(machine_file_path.clone())?;
      copy(&mut response, &mut dest)?;
    // Load from a local directory
    } else {
      match outgoing {
        Some(sender) => {sender.send(ClientMessage::String(format!("{} {} v{}", "[Loading]".bright_cyan(), name, ver)));}
        None => (),
      }
      let machine_path = format!("{}{}", path_str, machine_name);
      println!("{:?}", machine_path);
      let path = Path::new(&machine_path);
      let mut dest = File::create(machine_file_path.clone())?;
      let mut f = File::open(path)?;
      copy(&mut f, &mut dest)?;
    }
  }
  let machine_file_path = format!("machines/{}",machine_name);
  let machine = Library::new(machine_file_path).expect("Can't load library");
  Ok(machine)
}

// ## Program

pub struct Program {
  pub name: String,
  pub mech: Core,
  pub cores: HashMap<u64,Core>,
  pub input_map: HashMap<Register,HashSet<u64>>,
  pub libraries: HashMap<String, Library>,
  pub machines: HashMap<u64, Box<dyn Machine>>,
  pub machine_repository: HashMap<String, (String, String)>,
  capacity: usize,
  pub incoming: Receiver<RunLoopMessage>,
  pub outgoing: Sender<RunLoopMessage>,
  pub errors: Vec<Error>,
  programs: u64,
  pub listeners: HashSet<Register>,
}

impl Program {
  pub fn new(name:&str, capacity: usize, outgoing: Sender<RunLoopMessage>, incoming: Receiver<RunLoopMessage>) -> Program {
    let mut mech = Core::new(capacity);
    mech.load_standard_library();
    let mech_code = hash_string("mech/code");
    let txn = Transaction{changes: vec![Change::NewTable{table_id: mech_code, rows: 1, columns: 1}]};
    mech.process_transaction(&txn);
    Program { 
      name: name.to_owned(), 
      capacity,
      machine_repository: HashMap::new(), 
      mech,
      cores: HashMap::new(),
      libraries: HashMap::new(),
      machines: HashMap::new(),
      input_map: HashMap::new(),
      incoming,
      outgoing,
      errors: Vec::new(),
      programs: 0,
      listeners: HashSet::new(),
    }
  }

  pub fn compile_program(&mut self, input: String) {
    let mut compiler = Compiler::new();
    compiler.compile_string(input.clone());
    self.mech.register_blocks(compiler.blocks);
    //self.errors.append(&mut self.mech.runtime.errors.clone());
    let mech_code = hash_string("mech/code");
    self.programs += 1;
    //let txn = Transaction::from_change(Change::Set{table: mech_code, row: Index::Index(self.programs), column: Index::Index(1), value: Value::from_str(&input.clone())});
    //self.outgoing.send(RunLoopMessage::Transaction(txn));
  }

  pub fn compile_fragment(&mut self, input: String) {
    let mut compiler = Compiler::new();
    compiler.compile_string(input.clone());
    for mut block in compiler.blocks {
      block.id = (self.mech.runtime.blocks.len() + 1) as u64;
      self.mech.runtime.ready_blocks.insert(block.id);
      self.mech.register_blocks(vec![block]);
    }
    //self.errors.append(&mut self.mech.runtime.errors.clone());
    let mech_code = hash_string("mech/code");
    self.programs += 1;
    //let txn = Transaction::from_change(Change::Set{table: mech_code, row: Index::Index(self.programs), column: Index::Index(1), value: Value::from_str(&input.clone())});
    //self.outgoing.send(RunLoopMessage::Transaction(txn));
  }

  pub fn download_dependencies(&mut self, outgoing: Option<crossbeam_channel::Sender<ClientMessage>>) -> Result<(),Box<std::error::Error>> {
    
    if self.machine_repository.len() == 0 {
      // Download machine_repository index
      let registry_url = "https://gitlab.com/mech-lang/machines/directory/-/raw/master/machines.mec";
      let mut response = reqwest::get(registry_url)?.text()?;
      let mut registry_compiler = Compiler::new();
      registry_compiler.compile_string(response);
      let mut registry_core = Core::new(100);
      registry_core.load_standard_library(); 
      registry_core.register_blocks(registry_compiler.blocks);
      registry_core.step();

      // Convert the machine listing into a hash map
      let registry_table = registry_core.get_table(hash_string("mech/machines")).unwrap();
      for row in 0..registry_table.rows {
        let row_index = Index::Index(row+1);
        let name = registry_table.get_string(&registry_table.get(&row_index, &Index::Index(1)).unwrap().as_string().unwrap()).unwrap().to_string();
        let version = registry_table.get_string(&registry_table.get(&row_index, &Index::Index(2)).unwrap().as_string().unwrap()).unwrap().to_string();
        let url = registry_table.get_string(&registry_table.get(&row_index, &Index::Index(3)).unwrap().as_string().unwrap()).unwrap().to_string();
        self.machine_repository.insert(name, (version, url));
      }
    }

    // Do it for the mech core
    for (fun_name_id, fun) in self.mech.runtime.functions.iter_mut() {
      let fun_name = self.mech.runtime.database.borrow().store.strings.get(&fun_name_id).unwrap().clone();
      let m: Vec<_> = fun_name.split('/').collect();
      #[cfg(unix)]
      let machine_name = format!("libmech_{}.so", m[0]);
      #[cfg(windows)]
      let machine_name = format!("mech_{}.dll", m[0]);
      match (&fun, self.machine_repository.get(m[0])) {
        (None, Some((ver, path))) => {
          let library = self.libraries.entry(m[0].to_string()).or_insert_with(||{
            match File::open(format!("machines/{}",machine_name)) {
              Ok(_) => {
                Library::new(format!("machines/{}",machine_name)).expect("Can't load library")
              }
              _ => download_machine(&machine_name, m[0], path, ver, outgoing.clone()).unwrap()
            }
          });       
          let native_rust = unsafe {
            // Replace slashes with underscores and then add a null terminator
            let mut s = format!("{}\0", fun_name.replace("/","_"));
            let error_msg = format!("Symbol {} not found",s);
            let m = library.get::<extern "C" fn(arguments: &Vec<(u64, ValueIterator)>, out: &mut ValueIterator)>(s.as_bytes()).expect(&error_msg);
            m.into_raw()
          };
          *fun = Some(*native_rust);
        },
        _ => (),
      }
    }
    /*
    let mut changes = Vec::new();
    for needed_table in self.mech.runtime.input.difference(&self.mech.runtime.defined_tables) {
      let needed_table_name = self.mech.store.names.get(needed_table.table.unwrap()).unwrap();
      let m: Vec<_> = needed_table_name.split('/').collect();
      #[cfg(unix)]
      let machine_name = format!("libmech_{}.so", m[0]);
      #[cfg(windows)]
      let machine_name = format!("mech_{}.dll", m[0]);
      match self.machine_repository.get(m[0]) {
        Some((ver, path)) => {
          let library = self.libraries.entry(m[0].to_string()).or_insert_with(||{
            match File::open(format!("machines/{}",machine_name)) {
              Ok(_) => {
                Library::new(format!("machines/{}",machine_name)).expect("Can't load library")
              }
              _ => download_machine(&machine_name, m[0], path, ver, outgoing.clone()).unwrap()
            }
          });          
          // Replace slashes with underscores and then add a null terminator
          let mut s = format!("{}\0", needed_table_name.replace("/","_"));
          let error_msg = format!("Symbol {} not found",s);
          let mut registrar = Registrar::new();
          unsafe{
            let declaration = library.get::<*mut MachineDeclaration>(s.as_bytes()).unwrap().read();
            let mut init_changes = (declaration.register)(&mut registrar, self.outgoing.clone());
            changes.append(&mut init_changes);
          }        
          self.machines.extend(registrar.machines);
        },
        _ => (),
      }
    }
    let txn = Transaction{changes};
    self.mech.process_transaction(&txn);
*/
    /*
    // Do it for the the other core
    for core in self.cores.values_mut() {
      for (fun_name, fun) in core.runtime.functions.iter_mut() {
        let m: Vec<_> = fun_name.split('/').collect();
        #[cfg(unix)]
        let machine_name = format!("libmech_{}.so", m[0]);
        #[cfg(windows)]
        let machine_name = format!("mech_{}.dll", m[0]);
        match (&fun, self.machine_repository.get(m[0])) {
          (None, Some((ver, path))) => {
            let library = self.libraries.entry(m[0].to_string()).or_insert_with(||{
              match File::open(format!("machines/{}",machine_name)) {
                Ok(_) => {
                  Library::new(format!("machines/{}",machine_name)).expect("Can't load library")
                }
                _ => download_machine(&machine_name, m[0], path, ver, outgoing.clone()).unwrap()
              }
            });          
            let native_rust = unsafe {
              // Replace slashes with underscores and then add a null terminator
              let mut s = format!("{}\0", fun_name.replace("/","_"));
              let error_msg = format!("Symbol {} not found",s);
              let m = library.get::<extern "C" fn(Vec<(String, Rc<RefCell<Table>>)>, Rc<RefCell<Table>>)>(s.as_bytes()).expect(&error_msg);
              m.into_raw()
            };
            *fun = Some(*native_rust);
          },
          _ => (),
        }
      }
    }*/
    
    Ok(())
  }

  pub fn clear(&mut self) {
    //self.mech.clear();
  }

}

// ## Run Loop

// Client messages are sent to the client from the run loop

#[derive(Debug, Clone)]
pub enum ClientMessage {
  Stop,
  Pause,
  Resume,
  Clear,
  Time(usize),
  NewBlocks(usize),
  Table(Option<Table>),
  Transaction(Transaction),
  String(String),
  //Block(Block),
  StepDone,
  Done,
}

pub struct RunLoop {
  pub name: String,
  thread: JoinHandle<()>,
  pub outgoing: Sender<RunLoopMessage>,
  pub incoming: Receiver<ClientMessage>,
}

impl RunLoop {

  pub fn wait(self) {
    self.thread.join().unwrap();
  }

  pub fn close(&self) {
    match self.outgoing.send(RunLoopMessage::Stop) {
      Ok(..) => (),
      Err(..) => (),
    }
  }

  pub fn send(&self, msg: RunLoopMessage) -> Result<(),&str> {
    match self.outgoing.send(msg) {
      Ok(_) => Ok(()),
      Err(_) => Err("Failed to send message"),
    }
  }

  pub fn receive(&self) -> Result<ClientMessage,&str> {
    match self.incoming.recv() {
      Ok(message) => Ok(message),
      Err(_) => Err("Failed to send message"),
    }
  }

  pub fn is_empty(&self) -> bool {
    self.incoming.is_empty()
  }

  pub fn channel(&self) -> Sender<RunLoopMessage> {
    self.outgoing.clone()
  }

}

// ## Persister

pub enum PersisterMessage {
    Stop,
    Write(Vec<Change>),
}

pub struct Persister {
    thread: JoinHandle<()>,
    outgoing: Sender<PersisterMessage>,
    loaded: Vec<Change>,
}

impl Persister {
  pub fn new(path_ref:&str) -> Persister {
    let (outgoing, incoming) = crossbeam_channel::unbounded();
    let path = path_ref.to_string();
    let thread = thread::spawn(move || {
      let file = OpenOptions::new().append(true).create(true).open(&path).unwrap();
      let mut writer = BufWriter::new(file);
      loop {
        match incoming.recv().unwrap() {
          PersisterMessage::Stop => { break; }
          PersisterMessage::Write(items) => {
            for item in items {
              let result = bincode::serialize(&item).unwrap();
              if let Err(e) = writer.write_all(&result) {
                panic!("Can't persist! {:?}", e);
              }
            }
            writer.flush().unwrap();
          }
        }
      }
    });
    Persister { outgoing, thread, loaded: vec![] }
  }

  pub fn load(&mut self, path: &str) {
    let file = match File::open(path) {
      Ok(f) => f,
      Err(_) => {
        ////println!("Unable to load db: {}", path);
        return;
      }
    };
    let mut reader = BufReader::new(file);
    loop {
      let result:Result<Change, _> = bincode::deserialize_from(&mut reader);
      match result {
        Ok(change) => {
          self.loaded.push(change);
        },
        Err(info) => {
          ////println!("ran out {:?}", info);
          break;
        }
      }
    }
  }

  pub fn send(&self, changes: Vec<Change>) {
    self.outgoing.send(PersisterMessage::Write(changes)).unwrap();
  }

  pub fn wait(self) {
    self.thread.join().unwrap();
  }

    pub fn get_channel(&self) -> Sender<PersisterMessage> {
    self.outgoing.clone()
  }

  pub fn get_changes(&mut self) -> Vec<Change> {
    mem::replace(&mut self.loaded, vec![])
  }

  pub fn close(&self) {
    self.outgoing.send(PersisterMessage::Stop).unwrap();
  }
}
// ## Program Runner

pub struct ProgramRunner {
  pub name: String,
  //pub persistence_channel: Option<Sender<PersisterMessage>>,
}

impl ProgramRunner {

  pub fn new(name:&str, capacity: usize) -> ProgramRunner {
    // Start a new program
    //let mut program = Program::new(name, capacity);

    // Start a persister
    /*
    let persist_name = format!("{}.mdb", name);
    let mut persister = Persister::new(&persist_name);
    persister.load(&persist_name);
    let changes = persister.get_changes();

    // Load database
    ////println!("{} Applying {} stored changes...", BrightCyan.paint(format!("[{}]", name)), changes.len());    
    for change in changes {
      program.mech.process_transaction(&Transaction::from_change(change));
    }*/
    
    ProgramRunner {
      name: name.to_owned(),
      //program,
      // TODO Use the persistence file specified by the user
      //persistence_channel: Some(persister.get_channel()),
      //persistence_channel: None,
    }
  }

  /*pub fn load_program(&mut self, input: String) -> Result<(),Box<std::error::Error>> {
    self.program.compile_program(input);
    Ok(())
  }

  pub fn load_core(&mut self, mut core: Core) {
    core.id = (self.program.cores.len() + 1) as u64;
    for input_register in &core.input {
      let input = self.program.input_map.entry(input_register.clone()).or_insert(HashSet::new());
      input.insert(core.id);
    }

    let table = core.get_table("#data".to_string()).unwrap();
    self.program.mech.remote_tables.push(table.clone());

    self.program.cores.insert(core.id, core);
  }*/

  pub fn add_persist_channel(&mut self, persister:&mut Persister) {
    //self.persistence_channel = Some(persister.get_channel());
  }

  pub fn run(self) -> RunLoop {
    //let name = self.name;
    //let outgoing = self.program.outgoing.clone();
    let (outgoing, program_incoming) = crossbeam_channel::unbounded();
    let runloop_outgoing = outgoing.clone();
    let (client_outgoing, incoming) = crossbeam_channel::unbounded();
    //let mut program = self.program;
    //let persistence_channel = self.persistence_channel;

    let thread = thread::Builder::new().name(self.name.to_owned()).spawn(move || {

      let mut program = Program::new("new program", 100, outgoing.clone(), program_incoming);

      program.download_dependencies(Some(client_outgoing.clone()));

      // Step cores
      program.mech.step();
      for core in program.cores.values_mut() {
        core.step();
      }
      extern crate ws;
      use ws::{connect, Handler, Sender, Handshake, Result, Message, CloseCode};
/*
// Our Handler struct.
// Here we explicity indicate that the Client needs a Sender,
// whereas a closure captures the Sender for us automatically.
struct Client {
  out: Sender,
}

// We implement the Handler trait for Client so that we can get more
// fine-grained control of the connection.
impl Handler for Client {

  // `on_open` will be called only after the WebSocket handshake is successful
  // so at this point we know that the connection is ready to send/receive messages.
  // We ignore the `Handshake` for now, but you could also use this method to setup
  // Handler state or reject the connection based on the details of the Request
  // or Response, such as by checking cookies or Auth headers.
  fn on_open(&mut self, _: Handshake) -> Result<()> {
      // Now we don't need to call unwrap since `on_open` returns a `Result<()>`.
      // If this call fails, it will only result in this connection disconnecting.
      self.out.send("Hello WebSocket")
  }

  // `on_message` is roughly equivalent to the Handler closure. It takes a `Message`
  // and returns a `Result<()>`.
  fn on_message(&mut self, msg: Message) -> Result<()> {
      // Close the connection when we get a response from the server
      println!("Got message: {}", msg);
      Ok(())
      //self.out.close(CloseCode::Normal)
  }
}
//connect("ws://127.0.0.1:3012/ws/", |out| Client { out: out } ).unwrap();
let thread = thread::Builder::new().name("wsthread".to_string()).spawn(move || {
    println!("Connecting to websocket!");
    // Connect to the url and call the closure
    if let Err(error) = connect("ws://127.0.0.1:3012/ws/", |out| {
      Client { out: out }
    }) {
        // Inform the user of failure
        println!("Failed to create WebSocket due to: {:?}", error);
    }
  });
  */
      /*  
  use tungstenite::{connect, Message};
  use url::Url;

  println!("Attepmpting to connect to websocket...");
  match connect(Url::parse("ws://localhost:3012/ws/").unwrap()) {
    Ok((mut socket, response)) => {
      println!("Connected to the server");
      println!("Response HTTP code: {}", response.status());
      println!("Response contains the following headers:");
      for (ref header, _value) in response.headers() {
          println!("* {}", header);
      }
    
      socket
          .write_message(Message::Text("Hello WebSocket".into()))
          .unwrap();
      loop {
          let msg = socket.read_message().expect("Error reading message");
          println!("Received: {}", msg);
      }
    }
    Err(e) => println!("ERROR::: {:?}", e),
  }
*/

//(mut socket, response)



// socket.close(None);
      // Check to see if there are any remote cores...
      // Simple websocket client.
      /*
use std::time::Duration;
use std::{io, thread};

use actix::io::SinkWrite;
use actix::*;
use actix_codec::Framed;
use awc::{
    error::WsProtocolError,
    ws::{Codec, Frame, Message},
    BoxedSocket, Client,
};
use bytes::Bytes;
use futures::stream::{SplitSink, StreamExt};


    //let sys = System::new("websocket-client");
    Arbiter::spawn(async {
        let result = Client::new()
            .ws("http://127.0.0.1:3012/ws/")
            .connect()
            .await
            .map_err(|e| {
                println!("Error: {}", e);
            });
            //(response, framed)
        println!("{:?}", result);
        /*let (sink, stream) = framed.split();
        let addr = ChatClient::create(|ctx| {
            ChatClient::add_stream(stream, ctx);
            ChatClient(SinkWrite::new(sink, ctx))
        });*/
/*
        // start console loop
        thread::spawn(move || loop {
            let mut cmd = String::new();
            if io::stdin().read_line(&mut cmd).is_err() {
                println!("error");
                return;
            }
            addr.do_send(ClientCommand(cmd));
        });*/
    });
    println!("Down here!!!");
    //let run = sys.run();
    //println!("{:?}", run);
*/
/*
struct ChatClient(SinkWrite<Message, SplitSink<Framed<BoxedSocket, Codec>, Message>>);

#[derive(Message)]
#[rtype(result = "()")]
struct ClientCommand(String);

impl Actor for ChatClient {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        // start heartbeats otherwise server will disconnect after 10 seconds
        self.hb(ctx)
    }

    fn stopped(&mut self, _: &mut Context<Self>) {
        println!("Disconnected");

        // Stop application on disconnect
        System::current().stop();
    }
}

impl ChatClient {
    fn hb(&self, ctx: &mut Context<Self>) {
        ctx.run_later(Duration::new(1, 0), |act, ctx| {
            act.0.write(Message::Ping(Bytes::from_static(b""))).unwrap();
            act.hb(ctx);

            // client should also check for a timeout here, similar to the
            // server code
        });
    }
}

/// Handle stdin commands
impl Handler<ClientCommand> for ChatClient {
    type Result = ();

    fn handle(&mut self, msg: ClientCommand, _ctx: &mut Context<Self>) {
        self.0.write(Message::Text(msg.0)).unwrap();
    }
}

/// Handle server websocket messages
impl StreamHandler<Result<Frame, WsProtocolError>> for ChatClient {
    fn handle(&mut self, msg: Result<Frame, WsProtocolError>, _: &mut Context<Self>) {
        if let Ok(Frame::Text(txt)) = msg {
            println!("Server: {:?}", txt)
        }
    }

    fn started(&mut self, _ctx: &mut Context<Self>) {
        println!("Connected");
    }

    fn finished(&mut self, ctx: &mut Context<Self>) {
        println!("Server disconnected");
        ctx.stop()
    }
}

impl actix::io::WriteHandler<WsProtocolError> for ChatClient {}
*/


      // Send the first done to the client to indicate that the program is initialized
      client_outgoing.send(ClientMessage::Done);
      let mut paused = false;
      'runloop: loop {
        match (program.incoming.recv(), paused) {
          (Ok(RunLoopMessage::Transaction(txn)), false) => {
            use std::time::Instant;
            //let pre_changes = program.mech.store.len();
            let start_ns = time::precise_time_ns();
            program.mech.process_transaction(&txn);
            //let delta_changes = program.mech.store.len() - pre_changes;
            let end_ns = time::precise_time_ns();
            let time = (end_ns - start_ns) as f64;              
            //program.compile_string(String::from(text.clone()));
            ////println!("{:?}", program.mech);
            //println!("Txn took {:0.4?} ms ({:0.0?} cps)", time / 1_000_000.0, delta_changes as f64 / (time / 1.0e9));
            //println!("{}", program.mech.get_table("ball".to_string()).unwrap().borrow().rows);
            /*let mut changes: Vec<Change> = Vec::new();
            for i in pre_changes..program.mech.store.len() {
              let change = &program.mech.store.changes[i-1];
              match change {
                Change::Set{table, ..} => {
                  match program.listeners.get(&TableId::Global(*table)) {
                    Some(_) => changes.push(change.clone()),
                    _ => (),
                  }
                }
                _ => ()
              } 
            }
            if !changes.is_empty() {
              let txn = Transaction::from_changeset(changes);
              client_outgoing.send(ClientMessage::Transaction(txn));
            }*/
            client_outgoing.send(ClientMessage::StepDone);
          },
          (Ok(RunLoopMessage::Listening(ref register)), _) => {
            println!("Someone is listening for: {:?}", register);
            /*
            match program.mech.output.get(register) {
              Some(_) => {
                // We produce a table for which they're listening, so let's mark that
                // so we can send updates
                program.listeners.insert(register.clone()); 
                // Send over the table we have now
                let table_ref = program.mech.get_table_by_id(&register.table);
                client_outgoing.send(ClientMessage::Table(Some(table_ref.unwrap().borrow().clone())));
              }, 
              _ => (),
            }*/
            
          },
          (Ok(RunLoopMessage::Stop), _) => { 
            client_outgoing.send(ClientMessage::Stop);
            break 'runloop;
          },
          (Ok(RunLoopMessage::GetTable(table_id)), _) => { 
            let table_msg = ClientMessage::Table(program.mech.get_table(table_id));
            client_outgoing.send(table_msg);
          },
          (Ok(RunLoopMessage::Pause), false) => { 
            paused = true;
            client_outgoing.send(ClientMessage::Pause);
          },
          (Ok(RunLoopMessage::Resume), true) => {
            paused = false;
            //program.mech.resume();
            client_outgoing.send(ClientMessage::Resume);
          },
          (Ok(RunLoopMessage::StepBack), _) => {
            if !paused {
              paused = true;
            }
            //program.mech.step_back_one();
            //client_outgoing.send(ClientMessage::Time(program.mech.offset));
          }
          (Ok(RunLoopMessage::StepForward), true) => {
            //program.mech.step_forward_one();
            //client_outgoing.send(ClientMessage::Time(program.mech.offset));
          } 
          (Ok(RunLoopMessage::Code(code_tuple)), _) => {
            let block_count = program.mech.runtime.blocks.len();
            match code_tuple {
              (0, MechCode::String(code)) => {
                let mut compiler = Compiler::new(); 
                compiler.compile_string(code);
                
                program.mech.register_blocks(compiler.blocks);
                program.download_dependencies(Some(client_outgoing.clone()));
                program.mech.step();
                /*
                for register in &program.mech.runtime.changed_this_round {
                  match program.machines.get(&table) {
                    // Invoke the machine!
                    Some(machine) => {
                      
                      for change in &program.mech.store.changes {
                        match change {
                          Change::Set{table_id: change_table, ..} => {
                            if table == change_table {
                              machine.on_change(&change);
                            }
                          }
                          _ => (),
                        }
                      }

                    },
                    _ => (),
                  }
                }*/
                client_outgoing.send(ClientMessage::StepDone);
              },
              (0, MechCode::MiniBlocks(miniblocks)) => {
                let mut blocks: Vec<Block> = Vec::new();
                for miniblock in miniblocks {
                  let mut block = Block::new(100);
                  for tfms in miniblock.transformations {
                    block.register_transformations(tfms);
                  }
                  block.plan = miniblock.plan.clone();
                  let store = unsafe{&mut *Arc::get_mut_unchecked(&mut block.store)};
                  for (key, value) in miniblock.strings {
                    store.strings.insert(key, value.to_string());
                  }
                  block.gen_id();
                  blocks.push(block);
                }
                program.mech.register_blocks(blocks);
                program.download_dependencies(Some(client_outgoing.clone()));
                program.mech.step();
                client_outgoing.send(ClientMessage::StepDone);
              }
              (ix, code) => {

              }
            }
          }
          (Ok(RunLoopMessage::EchoCode(code)), _) => {
            /*
            // Reset #ans
             match program.mech.get_table("ans".to_string()) {
              Some(table) => {
                table.borrow_mut().clear();
              },
              None => (),
            };

            // Compile and run code
            let mut compiler = Compiler::new();
            compiler.compile_string(code);
            program.mech.register_blocks(compiler.blocks);
            program.download_dependencies(Some(client_outgoing.clone()));
            program.mech.step();

            // Get the result
            let echo_table = match program.mech.get_table("ans".to_string()) {
              Some(table) => Some(table.borrow().clone()),
              None => None,
            };

            // Send it
            client_outgoing.send(ClientMessage::Table(echo_table));*/
            client_outgoing.send(ClientMessage::StepDone);
          } 
          (Ok(RunLoopMessage::Clear), _) => {
            program.clear();
            client_outgoing.send(ClientMessage::Clear);
          },
          (Ok(RunLoopMessage::PrintCore(core_id)), _) => {
            match core_id {
              None => client_outgoing.send(ClientMessage::String(format!("{:?}",program.cores.len() + 1))),
              Some(0) => client_outgoing.send(ClientMessage::String(format!("{:?}",program.mech))),
              Some(core_id) => client_outgoing.send(ClientMessage::String(format!("{:?}",program.cores.get(&core_id)))),
            };
          },
          (Ok(RunLoopMessage::PrintRuntime), _) => {
            //println!("{:?}", program.mech.runtime);
            client_outgoing.send(ClientMessage::String(format!("{:?}",program.mech.runtime)));
          },
          (Ok(RunLoopMessage::Blocks(miniblocks)), _) => {
            let mut blocks: Vec<Block> = Vec::new();
            for miniblock in miniblocks {
              let mut block = Block::new(100);
              for tfms in miniblock.transformations {
                block.register_transformations(tfms);
              }
              blocks.push(block);
            }
            program.mech.register_blocks(blocks);
            program.mech.step();
            client_outgoing.send(ClientMessage::StepDone);
          }
          (Err(_), _) => {
            break 'runloop
          },
          x => println!("{:?}", x),
        }
        client_outgoing.send(ClientMessage::Done);
      }
      /*if let Some(channel) = persistence_channel {
        channel.send(PersisterMessage::Stop);
      }*/
    }).unwrap();
    RunLoop { name: self.name, thread, outgoing: runloop_outgoing, incoming }
  }

  /*pub fn colored_name(&self) -> term_painter::Painted<String> {
    BrightCyan.paint(format!("[{}]", &self.name))
  }*/

}


// New runtime
// requirements:
// pass all tests
// robust units
// all number types
// Unicode
// async blocks
// parallel operators
// rewind capability
// pre-serialized memory layout
// performance target: 10 million updates per 60Hz cycle
// stack allocated tables
// matrix library in std

use std::{borrow::Cow, convert::TryInto, str::FromStr};
use wgpu::util::DeviceExt;

// Indicates a u32 overflow in an intermediate Collatz value
const OVERFLOW: u32 = 0xffffffff;

async fn run() {
  let numbers = vec![1;1_00_000];
  let result = execute_gpu(&numbers).await.unwrap();
  //println!("{:?}", result);
}

async fn execute_gpu(numbers: &[u32]) -> Option<Vec<u32>> {
    // Instantiates instance of WebGPU
    let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);

    // `request_adapter` instantiates the general connection to the GPU
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await?;

    // `request_device` instantiates the feature specific connection to the GPU, defining some parameters,
    //  `features` being the available features.
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
            },
            None,
        )
        .await
        .unwrap();

    let info = adapter.get_info();
    println!("{:?}", info);
    // skip this on LavaPipe temporarily
    if info.vendor == 0x10005 {
        return None;
    }

    execute_gpu_inner(&device, &queue, numbers).await
}

async fn execute_gpu_inner(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    numbers: &[u32],
) -> Option<Vec<u32>> {
    // Loads the shader from WGSL
    let cs_module = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(
          r#"[[block]]
struct PrimeIndices {
    data: [[stride(4)]] array<u32>;
}; // this is used as both input and output for convenience

[[group(0), binding(0)]]
var<storage> v_indices: [[access(read_write)]] PrimeIndices;

fn add(n_base: u32) -> u32{
    var n: u32 = n_base;
    return n_base + 1u;
}

[[stage(compute), workgroup_size(1)]]
fn main([[builtin(global_invocation_id)]] global_id: vec3<u32>) {
    v_indices.data[global_id.x] = add(v_indices.data[global_id.x]);
}"#
        )),
      flags: wgpu::ShaderFlags::empty(),
    });

    // Gets the size in bytes of the buffer.
    let slice_size = numbers.len() * std::mem::size_of::<u32>();
    let size = slice_size as wgpu::BufferAddress;

    // Instantiates buffer without data.
    // `usage` of buffer specifies how it can be used:
    //   `BufferUsage::MAP_READ` allows it to be read (outside the shader).
    //   `BufferUsage::COPY_DST` allows it to be the destination of the copy.
    let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        usage: wgpu::BufferUsage::MAP_READ | wgpu::BufferUsage::COPY_DST,
        mapped_at_creation: false,
    });

    // Instantiates buffer with data (`numbers`).
    // Usage allowing the buffer to be:
    //   A storage buffer (can be bound within a bind group and thus available to a shader).
    //   The destination of a copy.
    //   The source of a copy.
    let storage_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Storage Buffer"),
        contents: bytemuck::cast_slice(numbers),
        usage: wgpu::BufferUsage::STORAGE
            | wgpu::BufferUsage::COPY_DST
            | wgpu::BufferUsage::COPY_SRC,
    });

    // A bind group defines how buffers are accessed by shaders.
    // It is to WebGPU what a descriptor set is to Vulkan.
    // `binding` here refers to the `binding` of a buffer in the shader (`layout(set = 0, binding = 0) buffer`).

    // A pipeline specifies the operation of a shader

    // Instantiates the pipeline.
    let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: None,
        module: &cs_module,
        entry_point: "main",
    });

    // Instantiates the bind group, once again specifying the binding of buffers.
    let bind_group_layout = compute_pipeline.get_bind_group_layout(0);
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: storage_buffer.as_entire_binding(),
        }],
    });
    let start_ns0 = time::precise_time_ns();
    // A command encoder executes one or many pipelines.
    // It is to WebGPU what a command buffer is to Vulkan.
    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
      let m = 100_000;
      let start_ns0 = time::precise_time_ns();
      for _ in 0..m   {
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None });
        cpass.set_pipeline(&compute_pipeline);
        cpass.set_bind_group(0, &bind_group, &[]);
        cpass.insert_debug_marker("add");
        cpass.dispatch(numbers.len() as u32, 1, 1); // Number of cells to run, the (x,y,z) size of item being processed
      }
      let end_ns0 = time::precise_time_ns();
      let time = (end_ns0 - start_ns0) as f64;
      println!("{:0.2?}Hz", 1.0 / ((time / 1_000_000_000.0) / m as f64));
    }
    // Sets adds copy operation to command encoder.
    // Will copy data from storage buffer on GPU to staging buffer on CPU.
    encoder.copy_buffer_to_buffer(&storage_buffer, 0, &staging_buffer, 0, size);

    // Submits command encoder for processing
    queue.submit(Some(encoder.finish()));

    // Note that we're not calling `.await` here.
    let buffer_slice = staging_buffer.slice(..);
    // Gets the future representing when `staging_buffer` can be read from
    let buffer_future = buffer_slice.map_async(wgpu::MapMode::Read);

    // Poll the device in a blocking manner so that our future resolves.
    // In an actual application, `device.poll(...)` should
    // be called in an event loop or on another thread.
    device.poll(wgpu::Maintain::Wait);

    // Awaits until `buffer_future` can be read from
    if let Ok(()) = buffer_future.await {
        // Gets contents of buffer
        let data = buffer_slice.get_mapped_range();
        // Since contents are got in bytes, this converts these bytes back to u32
        let result = data
            .chunks_exact(4)
            .map(|b| u32::from_ne_bytes(b.try_into().unwrap()))
            .collect();

        // With the current interface, we have to make sure all mapped views are
        // dropped before we unmap the buffer.
        drop(data);
        staging_buffer.unmap(); // Unmaps buffer from memory
                                // If you are familiar with C++ these 2 lines can be thought of similarly to:
                                //   delete myPointer;
                                //   myPointer = NULL;
                                // It effectively frees the memory


        // Returns data from buffer
        Some(result)
    } else {
        panic!("failed to run compute on gpu!")
    }
}

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
        pollster::block_on(run());
    }
    #[cfg(target_arch = "wasm32")]
    {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init().expect("could not initialize logger");
        wasm_bindgen_futures::spawn_local(run());
    }
}

/*

use std::sync::Arc;
use std::cell::RefCell;
use std::fmt;
use std::ptr;
use std::rc::Rc;
use hashbrown::{HashMap, HashSet};
use seahash;

use rayon::prelude::*;
use std::collections::VecDeque;
use std::thread;

pub type MechFunction = extern "C" fn(arguments: &mut Vec<Vec<f64>>);
pub type Column = Rc<RefCell<Vec<f64>>>;

pub fn hash_string(input: &str) -> u64 {
  seahash::hash(input.to_string().as_bytes()) & 0x00FFFFFFFFFFFFFF
}

#[derive(Clone)]
pub struct Table {
  pub id: u64,
  pub rows: usize,
  pub cols: usize,
  data: Vec<Column>,
}

impl Table {
  pub fn new(id: u64, rows: usize, cols: usize) -> Table {
    let mut table = Table {
      id,
      rows,
      cols,
      data: Vec::with_capacity(cols),
    };
    for col in 0..cols {
      table.data.push(Rc::new(RefCell::new(vec![0.0; rows])));
    }
    table
  }

  pub fn get(&self, row: usize, col: usize) -> Option<f64> {
    if col < self.cols && row < self.rows {
      Some(self.data[col].borrow()[row])
    } else {
      None
    }
  }

  pub fn set(&self, row: usize, col: usize, val: f64) -> Result<(),()> {
    if col < self.cols && row < self.rows {
      self.data[col].borrow_mut()[row] = val;
      Ok(())
    } else {
      Err(())
    }
  }

  pub fn get_column_unchecked(&self, col: usize) -> Column {
    self.data[col].clone()
  }

}

impl fmt::Debug for Table {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    for row in 0..self.rows {
      write!(f,"│ ")?;
      for col in 0..self.cols {
        let v = self.get(row,col).unwrap();
        write!(f,"{:0.2?} │ ", v)?;
      }
      write!(f,"\n")?;
    }
    Ok(())
  }
}

// binop vector-vector          -- lhs: &Vec<f64>,     rhs: &Vec<f64>    out: &mut Vec<f64>
// binop vector-vector in-place -- lhs: &mut Vec<f64>  rhs: &Vec<f64>
// binop vector-scalar          -- lhs: &Vec<f64>,     rhs: f64          out: &mut Vec<f64>
// binop vector-scalar in-place -- lhs: &mut Vec<f64>  rhs: f64
// truth vector-vector          -- lhs: &Vec<bool>     rhs: &Vec<bool>   out: &mut Vec<bool>
// comp  vector-scalar          -- lhs: &Vec<f64>      rhs: f64          out: &mut Vec<bool>
// set   vector-scalar          -- ix: &Vec<bool>      x:   f64          out: &mut Vec<f64>
// set   vector-vector          -- ix: &Vec<bool>      x:   &Vec<f64>    out: &mut Vec<f64>

pub type ArgF64 = Column;
pub type ArgBool = Rc<RefCell<Vec<bool>>>;
pub type OutF64 = Column;
pub type OutBool = Rc<RefCell<Vec<bool>>>;

#[derive(Debug)]
enum Transformation {
  ParAddVVIP((OutF64, ArgF64)),  
  ParAddVSIP((OutF64, ArgF64)),
  ParMultiplyVS((ArgF64, ArgF64, OutF64)),
  ParOrVV((ArgBool,ArgBool,OutBool)),
  ParLessThanVS((ArgF64,f64,OutBool)),
  ParGreaterThanVS((ArgF64,f64,OutBool)),
  ParSetVS((ArgBool,f64,OutF64)),
  ParSetVV((ArgBool,ArgF64,OutF64)),
}

impl Transformation {
  pub fn solve(&mut self) {
    match &*self {
      // MATH
      Transformation::ParAddVVIP((lhs, rhs)) => { lhs.borrow_mut().par_iter_mut().zip(&(*rhs.borrow())).for_each(|(lhs, rhs)| *lhs += rhs); }
      Transformation::ParAddVSIP((lhs, rhs)) => { 
        let rhs = rhs.borrow()[0];
        lhs.borrow_mut().par_iter_mut().for_each(|lhs| *lhs += rhs); 
      }
      Transformation::ParMultiplyVS((lhs, rhs, out)) => { 
        let rhs = rhs.borrow()[0];
        out.borrow_mut().par_iter_mut().zip(&(*lhs.borrow())).for_each(|(out, lhs)| *out = *lhs * rhs); 
      }
      // COMPARE
      Transformation::ParGreaterThanVS((lhs, rhs, out)) => { out.borrow_mut().par_iter_mut().zip(&(*lhs.borrow())).for_each(|(out, lhs)| *out = *lhs > *rhs); }
      Transformation::ParLessThanVS((lhs, rhs, out)) => { out.borrow_mut().par_iter_mut().zip(&(*lhs.borrow())).for_each(|(out, lhs)| *out = *lhs < *rhs); }
      // LOGIC
      Transformation::ParOrVV((lhs, rhs, out)) => { out.borrow_mut().par_iter_mut().zip(&(*lhs.borrow())).zip(&(*rhs.borrow())).for_each(|((out, lhs),rhs)| *out = *lhs || *rhs); }
      // SET
      Transformation::ParSetVS((ix, rhs, out)) => {
        out.borrow_mut().par_iter_mut().zip(&(*ix.borrow())).for_each(|(out,ix)| {
          if *ix == true {
            *out = *rhs
          }});          
      }
      Transformation::ParSetVV((ix, rhs, out)) => {
        out.borrow_mut().par_iter_mut().zip(&(*ix.borrow())).zip(&(*rhs.borrow())).for_each(|((out,ix),x)| if *ix == true {
          *out = *x
        });          
      }
    }
  }
}

pub type Change = (u64, Vec<(usize, usize, f64)>);

pub type Transaction = Vec<Change>;

struct Core {
  blocks: Vec<Rc<RefCell<Block>>>,
  database: Database,
}

impl Core {

  pub fn new() -> Core {
    Core {
      blocks: Vec::new(),
      database: Database::new(),
    }
  }

  pub fn process_transaction(&mut self, txn: &Transaction) -> Result<(),()> {
    for (table_id, adds) in txn {
      match self.database.get_table_by_id(table_id) {
        Some(table) => {
          for (row,col,val) in adds {
            match table.set(*row, *col, *val) {
              Err(_) => {
                // Index out of bounds.
                return Err(());
              }
              _ => (),
            }
          }
        }
        _ => {
          // Table doesn't exist
          return Err(());
        }
      }
    }
    self.step();
    Ok(())
  }

  pub fn insert_table(&mut self, table: Table) -> Option<Table> {
    self.database.insert_table(table)
  }

  pub fn get_table(&mut self, table_name: &str) -> Option<&Table> {
    self.database.get_table(table_name)
  }

  pub fn insert_block(&mut self, block: Block) {
    self.blocks.push(Rc::new(RefCell::new(block)));
  }

  pub fn step(&mut self) {
    for ref mut block in &mut self.blocks.iter() {
      block.borrow_mut().solve();
    }
  }
}

struct Database {
  tables: HashMap<u64,Table>,
}

impl Database {
  pub fn new() -> Database {
    Database {
      tables: HashMap::new(),
    }
  }

  pub fn insert_table(&mut self, table: Table) -> Option<Table> {
    self.tables.insert(table.id, table)
  }

  pub fn get_table(&mut self, table_name: &str) -> Option<&Table> {
    self.tables.get(&hash_string(table_name))
  }

  pub fn get_table_by_id(&mut self, table_id: &u64) -> Option<&Table> {
    self.tables.get(table_id)
  }

}

pub type Plan = Vec<Rc<RefCell<Transformation>>>;

struct Block {
  id: u64,
  plan: Plan,
}

impl Block {
  pub fn new() -> Block {
    Block {
      id: 0,
      plan: Vec::new(),
    }
  }

  pub fn gen_id(&mut self) -> u64 {
    self.id = hash_string(&format!("{:?}", self.plan));
    self.id
  }

  pub fn id(&self) -> u64 {
    self.id
  }

  pub fn add_tfm(&mut self, tfm: Transformation) {
    self.plan.push(Rc::new(RefCell::new(tfm)));
  }

  pub fn solve(&mut self) {
    for ref mut tfm in &mut self.plan.iter() {
      tfm.borrow_mut().solve();
    }
  }

}

fn main() {
  let sizes: Vec<usize> = vec![1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7].iter().map(|x| *x as usize).collect();
  let mut total_time = VecDeque::new();  
  let start_ns0 = time::precise_time_ns();
  let n = 1e6 as usize;

  // Create a core
  let mut core = Core::new();

  {
    // #time/timer += [period: 60Hz]
    let time_timer = Table::new(hash_string("time/timer"),1,2);
    time_timer.set(0,0,60.0);
    core.insert_table(time_timer.clone());

    // #gravity = 1
    let gravity = Table::new(hash_string("gravity"),1,1);
    gravity.set(0,0,1.0);
    core.insert_table(gravity.clone());

    // #gravity = 1
    let cosnt1 = Table::new(hash_string("-0.8"),1,1);
    cosnt1.set(0,0,-0.8);
    core.insert_table(cosnt1.clone());

    // Create balls
    // #balls = [x: 0:n y: 0:n vx: 3.0 vy: 4.0]
    let balls = Table::new(hash_string("balls"),n,4);
    for i in 0..n {
      balls.set(i,0,i as f64);
      balls.set(i,1,i as f64);
      balls.set(i,2,3.0);
      balls.set(i,3,4.0);
    }
    core.insert_table(balls.clone());
  }

  // Table
  let balls = core.get_table("balls").unwrap();
  let mut x = balls.get_column_unchecked(0);
  let mut y = balls.get_column_unchecked(1);
  let mut vx = balls.get_column_unchecked(2);
  let mut vy = balls.get_column_unchecked(3);

  let gravity = core.get_table("gravity").unwrap();
  let mut g = gravity.get_column_unchecked(0);

  let const1 = core.get_table("-0.8").unwrap();
  let mut c1 = const1.get_column_unchecked(0);

  // Temp Vars
  let mut x2 = Rc::new(RefCell::new(vec![0.0; n]));
  let mut y2 = Rc::new(RefCell::new(vec![0.0; n]));
  let mut vy2 = Rc::new(RefCell::new(vec![0.0; n]));
  let mut iy = Rc::new(RefCell::new(vec![false; n]));
  let mut iyy = Rc::new(RefCell::new(vec![false; n]));
  let mut iy_or = Rc::new(RefCell::new(vec![false; n]));
  let mut ix = Rc::new(RefCell::new(vec![false; n]));
  let mut ixx = Rc::new(RefCell::new(vec![false; n]));
  let mut ix_or = Rc::new(RefCell::new(vec![false; n]));
  let mut vx2 = Rc::new(RefCell::new(vec![0.0; n]));

  // Update the block positions on each tick of the timer  
  let mut block1 = Block::new();
  // #ball.x := #ball.x + #ball.vx
  block1.add_tfm(Transformation::ParAddVVIP((x.clone(), vx.clone())));
  // #ball.y := #ball.y + #ball.vy    
  block1.add_tfm(Transformation::ParAddVVIP((y.clone(), vy.clone())));
  // #ball.vy := #ball.vy + #gravity
  block1.add_tfm(Transformation::ParAddVSIP((vy.clone(), g.clone())));
  block1.gen_id();

  // Keep the balls within the boundary height
  let mut block2 = Block::new();
  // iy = #ball.y > #boundary.height
  block2.add_tfm(Transformation::ParGreaterThanVS((y.clone(), 500.0, iy.clone())));
  // iyy = #ball.y < 0
  block2.add_tfm(Transformation::ParLessThanVS((y.clone(), 0.0, iyy.clone())));
  // #ball.y{iy} := #boundary.height
  block2.add_tfm(Transformation::ParSetVS((iy.clone(), 500.0, y.clone())));
  // #ball.vy{iy | iyy} := #ball.vy * -0.80
  block2.add_tfm(Transformation::ParOrVV((iy.clone(), iyy.clone(), iy_or.clone())));
  block2.add_tfm(Transformation::ParMultiplyVS((vy.clone(), c1.clone(), vy2.clone())));
  block2.add_tfm(Transformation::ParSetVV((iy_or.clone(), vy2.clone(), vy.clone())));
  block2.gen_id();

  // Keep the balls within the boundary width
  let mut block3 = Block::new();
  // ix = #ball.x > #boundary.width
  block3.add_tfm(Transformation::ParGreaterThanVS((x.clone(), 500.0, ix.clone())));
  // ixx = #ball.x < 0
  block3.add_tfm(Transformation::ParLessThanVS((x.clone(), 0.0, ixx.clone())));
  // #ball.x{ix} := #boundary.width
  block3.add_tfm(Transformation::ParSetVS((ix.clone(), 500.0, x.clone())));
  // #ball.vx{ix | ixx} := #ball.vx * -0.80
  block3.add_tfm(Transformation::ParOrVV((ix.clone(), ixx.clone(), ix_or.clone())));
  block3.add_tfm(Transformation::ParMultiplyVS((vx.clone(), c1.clone(), vx2.clone())));
  block3.add_tfm(Transformation::ParSetVV((ix_or.clone(), vx2.clone(), vx.clone())));
  block3.gen_id();

  core.insert_block(block1);
  core.insert_block(block2);
  core.insert_block(block3);

  for i in 0..2000 {
    let txn = vec![(hash_string("time/timer"), vec![(0, 1, i as f64)])];
    let start_ns = time::precise_time_ns();

    core.process_transaction(&txn);

    let end_ns = time::precise_time_ns();
    let time = (end_ns - start_ns) as f64;
    total_time.push_back(time);
    if total_time.len() > 1000 {
      total_time.pop_front();
    }
    
  }
  let average_time: f64 = total_time.iter().sum::<f64>() / total_time.len() as f64; 
  println!("{:e} - {:0.2?}Hz", n, 1.0 / (average_time / 1_000_000_000.0));
  let end_ns0 = time::precise_time_ns();
  let time = (end_ns0 - start_ns0) as f64;
  println!("{:0.4?} s", time / 1e9);
}
*/
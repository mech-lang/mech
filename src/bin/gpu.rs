use rayon::prelude::*;
use std::{borrow::Cow, convert::TryInto, str::FromStr};
use wgpu::util::DeviceExt;

// Indicates a u32 overflow in an intermediate Collatz value
const OVERFLOW: u32 = 0xffffffff;

async fn run() {
  let numbers = vec![1.0;1_000_000];
  let result = execute_gpu(&numbers).await;
}

async fn execute_gpu(numbers: &[f32]) {
    // Instantiates instance of WebGPU
    let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);

    // `request_adapter` instantiates the general connection to the GPU
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptionsBase{
          power_preference: wgpu::PowerPreference::HighPerformance, 
          compatible_surface: None,
        })
        .await.unwrap();

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

    let mut result: Vec<f32> = numbers.to_vec(); 
    let start_ns0 = time::precise_time_ns();
    let n = 1000;
    for _ in 0..n as usize {
      result = execute_gpu_inner(&device, &queue, &result).await.unwrap();
    }
    let end_ns0 = time::precise_time_ns();
    let time = (end_ns0 - start_ns0) as f64;
    println!("{:0.2?}Hz", 1.0 / ((time / 1_000_000_000.0) / n as f64));
}

async fn execute_gpu_inner(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    numbers: &[f32],
) -> Option<Vec<f32>> {
    // Loads the shader from WGSL
    let cs_module = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(
          r#"[[block]]
struct DataBuf {
    data: [[stride(4)]] array<f32>;
};

[[group(0), binding(0)]]
var<storage> v1: [[access(read_write)]] DataBuf;

[[stage(compute), workgroup_size(1)]]
fn main([[builtin(global_invocation_id)]] global_id: vec3<u32>) {
    // TODO: a more interesting computation than this.
    v1.data[global_id.x] = v1.data[global_id.x] + 1.0f;
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
        label: Some("x"),
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
    // A command encoder executes one or many pipelines.
    // It is to WebGPU what a command buffer is to Vulkan.
    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
  
    let start_ns0 = time::precise_time_ns();
    {
      let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None });
      cpass.set_pipeline(&compute_pipeline);
      cpass.set_bind_group(0, &bind_group, &[]);
      cpass.insert_debug_marker("add");
      cpass.dispatch(numbers.len() as u32, 1, 1); // Number of cells to run, the (x,y,z) size of item being processed
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
    let result: Option<Vec<f32>> = if let Ok(()) = buffer_future.await {
      let end_ns0 = time::precise_time_ns();
      let time = (end_ns0 - start_ns0) as f64;
        // Gets contents of buffer
        let data = buffer_slice.get_mapped_range();
        // Since contents are got in bytes, this converts these bytes back to u32
        let result = data
            .chunks_exact(4)
            .map(|b| f32::from_ne_bytes(b.try_into().unwrap()))
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
    };
    result
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
    let mut x: Vec<f64> = vec![1.0;1_000_000];
    let mut y = vec![1.0;1_000_000];
    let mut vx = vec![1.0;1_000_000];
    let mut vy = vec![1.0;1_000_000];
    let n = 1000;
    let start_ns0 = time::precise_time_ns();
    for _ in 0..n as usize {
      for (((ref mut x,ref mut y), ref mut vx), ref mut vy) in &mut x.iter_mut().zip(&mut y).zip(&mut vx).zip(&mut vy) {
        **x += **vx;
        **y += **vy;
        **vy += 1.0;
        if **x > 500.0 {
          **x = 500.0;
          **vx *= -0.8;
        } else 
        if **x < 0.0 {
          **x = 0.0;
          **vx *= -0.8;
        }
        if **y > 500.0 {
          **y = 500.0;
          **vy *= -0.8;
        } else 
        if **y < 0.0 {
          **y = 0.0;
          **vy *= -0.8;
        }

      }
    }
    let end_ns0 = time::precise_time_ns();
    let time = (end_ns0 - start_ns0) as f64;
    println!("{:0.2?}Hz", 1.0 / ((time / 1_000_000_000.0) / n as f64));    
}

/*
//! This is the main file of the project. It contains structures used by all other parts of the
//! engine and the main method

mod config;
mod galaxygen;
mod render;

use {
    cgmath::{Matrix4, Point3, Vector3},
    config::{Config, Construction},
    ron::de::from_reader,
    std::{env, f32::consts::PI, fs::File},
};

#[derive(Clone, Copy, Debug)]
#[repr(C)]
/// An object with a position, velocity and mass that can be sent to the GPU.
pub struct Particle {
    /// Position
    pos: Point3<f32>, // 4, 8, 12

    /// The radius of the particle (currently unused)
    radius: f32, // 16

    /// Velocity
    vel: Vector3<f32>, // 4, 8, 12
    _p: f32, // 16

    /// Mass
    mass: f64, // 4, 8
    _p2: [f32; 2], // 12, 16
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
/// All variables that define the state of the program. Will be sent to the GPU.
pub struct Globals {
    /// The camera matrix (projection x view matrix)
    matrix: Matrix4<f32>, // 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15
    /// The current camera position (used for particle size)
    camera_pos: Point3<f32>, // 16, 17, 18
    /// The number of particles
    particles: u32, // 19
    /// Newton's law of gravitation has problems with 1D particles, this value works against
    /// gravitation in close ranges.
    safety: f64, // 20, 21
    /// How much time passes each frame
    delta: f32, // 22

    _p: f32, // 23
}

impl Particle {
    fn new(pos: Point3<f32>, vel: Vector3<f32>, mass: f64, density: f64) -> Self {
        Self {
            pos,
            // V = 4/3*pi*r^3
            // V = m/ d
            // 4/3*pi*r^3 = m / d
            // r^3 = 3*m / (4*d*pi)
            // r = cbrt(3*m / (4*d*pi))
            radius: (3.0 * mass / (4.0 * density * PI as f64)).cbrt() as f32,
            vel,
            mass,
            _p: 0.0,
            _p2: [0.0; 2],
        }
    }
}

pub type dvec3 = Vec<(f32,f32,f32)>;

fn length2(v: Vector3<f32>) -> f32 {
    v.x * v.x + v.y * v.y + v.z * v.z
}

fn normalize(v: Vector3<f32>) -> Vector3<f32> {
    v
}

const G: f32 = 6.67408e-11;

fn run_sim(data_old: &Vec<Particle>, data: &mut Vec<Particle>) {
    // Get index of current particle
    let delta = 36e0;
    let safety = 1e20;

    for i in 0..data_old.len() {
        // Gravity
        let mut temp: Vector3<f32> = Vector3::new(0.0, 0.0, 0.0);

        // Go through all other particles...
        for j in  0..data_old.len() {
            // Skip self
            if(j == i) { continue; }

            // If a single particle with no mass is encountered, the entire loop
            // terminates (because they are sorted by mass)
            if(data_old[j].mass == 0.0) { break; }

            let diff: Vector3<f32> = data_old[j].pos - data_old[i].pos;
            temp += normalize(diff) * data_old[j].mass as f32 / (length2(diff)+safety);
        }

        // Update data
        data[i].vel += temp * G * delta;
        let v = data[i].vel;
        data[i].pos += v * delta;
    }
}


fn main() {
    let config = read_config().unwrap_or_else(|| {
        println!("Using default config.");
        default_config()
    });

    // Construct particles from config
    let particles = config.construct_particles();

    let globals = Globals {
        matrix: Matrix4::from_translation(Vector3::new(0.0, 0.0, 0.0)),
        camera_pos: config.camera_pos.into(),
        particles: particles.len() as u32,
        safety: config.safety,
        delta: 0.0,
        _p: 0.0,
    };

    /*let old_particles = particles.clone();
    let mut new_particles = particles.clone();
    loop {
    let start_ns = time::precise_time_ns();

    run_sim(&old_particles,&mut new_particles);

    let end_ns = time::precise_time_ns();
    let time = (end_ns - start_ns) as f64;
    println!("{:0.2?}Hz", 1.0 / (time / 1_000_000_000.0));
    }*/
    
    render::run(globals, particles);
}

/// Read configuration file
fn read_config() -> Option<Config> {
    let input_path = env::args().nth(1)?;
    let f = File::open(&input_path).expect("Failed opening file!");
    let config = from_reader(f).expect("Failed to parse config!");

    Some(config)
}

fn default_config() -> Config {
    Config {
        camera_pos: [0.0, 0.0, 1e10],
        safety: 1e20,
        constructions: vec![
            Construction::Galaxy {
                center_pos: [-1e11, -1e11, 0.0],
                center_vel: [10e6, 0.0, 0.0],
                center_mass: 1e35,
                amount: 500000,
                normal: [1.0, 0.0, 0.0],
            },
            Construction::Galaxy {
                center_pos: [1e11, 1e11, 0.0],
                center_vel: [0.0, 0.0, 0.0],
                center_mass: 3e35,
                amount: 500000,
                normal: [1.0, 1.0, 0.0],
            },
        ],
    }
}

*/
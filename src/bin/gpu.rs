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
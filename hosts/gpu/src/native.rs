use std::{
    collections::BTreeMap,
    fmt,
    sync::mpsc,
    time::{Duration, Instant},
};

use wgpu::util::DeviceExt;

use super::{GpuBindingAccess, GpuProgram};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GpuExecutionError {
    AdapterUnavailable,
    DeviceRequest(String),
    MissingInput(String),
    InputLength {
        name: String,
        expected: u64,
        actual: usize,
    },
    BufferMap(String),
    ChannelClosed,
}

impl fmt::Display for GpuExecutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for GpuExecutionError {}

#[derive(Clone, Debug)]
pub struct GpuExecutionProfile {
    pub adapter: String,
    pub setup: Duration,
    pub pipeline_and_upload: Duration,
    pub dispatch_and_readback: Duration,
    pub total: Duration,
    pub outputs: BTreeMap<String, Vec<f32>>,
}

impl GpuProgram {
    /// Dispatches the generated kernel through wgpu. The same path works over
    /// Metal, Vulkan, Direct3D 12, and WebGPU-capable native backends.
    pub fn run_gpu(
        &self,
        inputs: &BTreeMap<String, Vec<f32>>,
    ) -> Result<BTreeMap<String, Vec<f32>>, GpuExecutionError> {
        self.run_gpu_profiled(inputs).map(|profile| profile.outputs)
    }

    pub fn run_gpu_profiled(
        &self,
        inputs: &BTreeMap<String, Vec<f32>>,
    ) -> Result<GpuExecutionProfile, GpuExecutionError> {
        pollster::block_on(self.run_gpu_profiled_async(inputs))
    }

    pub async fn run_gpu_async(
        &self,
        inputs: &BTreeMap<String, Vec<f32>>,
    ) -> Result<BTreeMap<String, Vec<f32>>, GpuExecutionError> {
        self.run_gpu_profiled_async(inputs)
            .await
            .map(|profile| profile.outputs)
    }

    pub async fn run_gpu_profiled_async(
        &self,
        inputs: &BTreeMap<String, Vec<f32>>,
    ) -> Result<GpuExecutionProfile, GpuExecutionError> {
        let total_started = Instant::now();
        let setup_started = Instant::now();
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .ok_or(GpuExecutionError::AdapterUnavailable)?;
        let adapter_name = {
            let info = adapter.get_info();
            format!("{} ({:?})", info.name, info.backend)
        };
        let required_storage_buffers = self.bindings.len() as u32;
        let adapter_limits = adapter.limits();
        if required_storage_buffers > adapter_limits.max_storage_buffers_per_shader_stage {
            return Err(GpuExecutionError::DeviceRequest(format!(
                "kernel needs {required_storage_buffers} storage buffers, adapter supports {}",
                adapter_limits.max_storage_buffers_per_shader_stage
            )));
        }
        let required_limits = wgpu::Limits {
            max_storage_buffers_per_shader_stage: required_storage_buffers,
            ..wgpu::Limits::downlevel_defaults()
        };
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Mech GPU program"),
                    required_features: wgpu::Features::empty(),
                    required_limits,
                },
                None,
            )
            .await
            .map_err(|error| GpuExecutionError::DeviceRequest(error.to_string()))?;
        let setup = setup_started.elapsed();

        let pipeline_started = Instant::now();
        let mut buffers = Vec::with_capacity(self.bindings.len());
        for binding in &self.bindings {
            let buffer = match binding.access {
                GpuBindingAccess::Read => {
                    let values = inputs
                        .get(&binding.name)
                        .ok_or_else(|| GpuExecutionError::MissingInput(binding.name.clone()))?;
                    if values.len() != binding.elements as usize {
                        return Err(GpuExecutionError::InputLength {
                            name: binding.name.clone(),
                            expected: binding.elements,
                            actual: values.len(),
                        });
                    }
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some(&binding.name),
                        contents: bytemuck::cast_slice(values),
                        usage: wgpu::BufferUsages::STORAGE,
                    })
                }
                GpuBindingAccess::ReadWrite => device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(&binding.name),
                    size: binding.elements * std::mem::size_of::<f32>() as u64,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                }),
            };
            buffers.push(buffer);
        }

        let layout_entries = self
            .bindings
            .iter()
            .map(|binding| wgpu::BindGroupLayoutEntry {
                binding: binding.binding,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage {
                        read_only: binding.access == GpuBindingAccess::Read,
                    },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            })
            .collect::<Vec<_>>();
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Mech GPU bindings"),
            entries: &layout_entries,
        });
        let bind_entries = self
            .bindings
            .iter()
            .zip(&buffers)
            .map(|(binding, buffer)| wgpu::BindGroupEntry {
                binding: binding.binding,
                resource: buffer.as_entire_binding(),
            })
            .collect::<Vec<_>>();
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Mech GPU bind group"),
            layout: &bind_group_layout,
            entries: &bind_entries,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Generated Mech WGSL"),
            source: wgpu::ShaderSource::Wgsl(self.wgsl.clone().into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Mech GPU pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Mech GPU pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        });
        let pipeline_and_upload = pipeline_started.elapsed();

        let dispatch_started = Instant::now();
        let mut readbacks = Vec::new();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Mech GPU command encoder"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Mech GPU compute pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(self.workgroup_count(), 1, 1);
        }
        for (index, binding) in self.bindings.iter().enumerate() {
            if binding.access != GpuBindingAccess::ReadWrite {
                continue;
            }
            let size = binding.elements * std::mem::size_of::<f32>() as u64;
            let readback = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Mech GPU readback"),
                size,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            encoder.copy_buffer_to_buffer(&buffers[index], 0, &readback, 0, size);
            readbacks.push((binding.name.clone(), readback));
        }
        queue.submit(Some(encoder.finish()));

        let mut outputs = BTreeMap::new();
        for (name, readback) in readbacks {
            let slice = readback.slice(..);
            let (sender, receiver) = mpsc::channel();
            slice.map_async(wgpu::MapMode::Read, move |result| {
                let _ = sender.send(result);
            });
            device.poll(wgpu::Maintain::Wait);
            receiver
                .recv()
                .map_err(|_| GpuExecutionError::ChannelClosed)?
                .map_err(|error| GpuExecutionError::BufferMap(error.to_string()))?;
            let mapped = slice.get_mapped_range();
            outputs.insert(name, bytemuck::cast_slice::<u8, f32>(&mapped).to_vec());
            drop(mapped);
            readback.unmap();
        }
        Ok(GpuExecutionProfile {
            adapter: adapter_name,
            setup,
            pipeline_and_upload,
            dispatch_and_readback: dispatch_started.elapsed(),
            total: total_started.elapsed(),
            outputs,
        })
    }
}

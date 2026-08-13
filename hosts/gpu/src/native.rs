use std::{
    collections::BTreeMap,
    fmt,
    sync::Arc,
    sync::mpsc,
    time::{Duration, Instant},
};

use wgpu::util::DeviceExt;

use super::{GpuBindingAccess, GpuBindingKind, GpuProgram};

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
    InvalidFeedback(String),
    WorkgroupLimit {
        required: u32,
        supported: u32,
    },
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

#[derive(Debug)]
pub struct ResidentGpuSession {
    adapter: String,
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    bind_groups: [wgpu::BindGroup; 2],
    input_buffers: BTreeMap<String, (Arc<wgpu::Buffer>, u64)>,
    output_buffers: [BTreeMap<String, Arc<wgpu::Buffer>>; 2],
    output_elements: BTreeMap<String, u64>,
    workgroups: u32,
    next_group: usize,
    last_output_group: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct ResidentDispatchProfile {
    pub adapter: String,
    pub turns: u32,
    pub dispatch: Duration,
    pub readback: Duration,
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
            let buffer = match binding.kind {
                GpuBindingKind::Input(_) => {
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
                GpuBindingKind::StateRead(slot) => {
                    let state = self
                        .states
                        .iter()
                        .find(|state| state.slot == slot)
                        .expect("state binding references known state");
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some(&binding.name),
                        contents: bytemuck::cast_slice(&state.initializer),
                        usage: wgpu::BufferUsages::STORAGE,
                    })
                }
                GpuBindingKind::StateWrite(_) | GpuBindingKind::Output(_) => {
                    device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some(&binding.name),
                        size: binding.elements * std::mem::size_of::<f32>() as u64,
                        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                        mapped_at_creation: false,
                    })
                }
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
        for output in &self.outputs {
            let size = output.elements * std::mem::size_of::<f32>() as u64;
            let readback = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Mech GPU readback"),
                size,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            encoder.copy_buffer_to_buffer(&buffers[output.binding as usize], 0, &readback, 0, size);
            readbacks.push((output.name.clone(), readback));
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

    pub fn prepare_resident(
        &self,
        inputs: &BTreeMap<String, Vec<f32>>,
    ) -> Result<ResidentGpuSession, GpuExecutionError> {
        pollster::block_on(self.prepare_resident_async(inputs))
    }

    pub async fn prepare_resident_async(
        &self,
        inputs: &BTreeMap<String, Vec<f32>>,
    ) -> Result<ResidentGpuSession, GpuExecutionError> {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .ok_or(GpuExecutionError::AdapterUnavailable)?;
        let adapter_info = adapter.get_info();
        let adapter_name = format!("{} ({:?})", adapter_info.name, adapter_info.backend);
        let adapter_limits = adapter.limits();
        let required_storage_buffers = self.bindings.len() as u32;
        if required_storage_buffers > adapter_limits.max_storage_buffers_per_shader_stage {
            return Err(GpuExecutionError::DeviceRequest(format!(
                "kernel needs {required_storage_buffers} storage buffers, adapter supports {}",
                adapter_limits.max_storage_buffers_per_shader_stage
            )));
        }
        let workgroups = self.workgroup_count();
        if workgroups > adapter_limits.max_compute_workgroups_per_dimension {
            return Err(GpuExecutionError::WorkgroupLimit {
                required: workgroups,
                supported: adapter_limits.max_compute_workgroups_per_dimension,
            });
        }
        let required_limits = wgpu::Limits {
            max_storage_buffers_per_shader_stage: required_storage_buffers,
            ..wgpu::Limits::downlevel_defaults()
        };
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Mech resident GPU program"),
                    required_features: wgpu::Features::empty(),
                    required_limits,
                },
                None,
            )
            .await
            .map_err(|error| GpuExecutionError::DeviceRequest(error.to_string()))?;

        let mut state_buffers = BTreeMap::new();
        let mut fixed_buffers = BTreeMap::new();
        let mut input_buffers = BTreeMap::new();
        for state in &self.states {
            let initial = Arc::new(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Mech resident initial state"),
                    contents: bytemuck::cast_slice(&state.initializer),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                }),
            );
            let alternate = Arc::new(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Mech resident alternate state"),
                size: state.elements * std::mem::size_of::<f32>() as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            }));
            state_buffers.insert(state.slot, [initial, alternate]);
        }
        for binding in &self.bindings {
            if !matches!(binding.kind, GpuBindingKind::Input(_)) {
                continue;
            }
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
            let buffer = Arc::new(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&binding.name),
                    contents: bytemuck::cast_slice(values),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                }),
            );
            fixed_buffers.insert(binding.binding, Arc::clone(&buffer));
            input_buffers.insert(binding.name.clone(), (buffer, binding.elements));
        }

        let mut group_buffers: [Vec<Arc<wgpu::Buffer>>; 2] = [Vec::new(), Vec::new()];
        for binding in &self.bindings {
            match binding.kind {
                GpuBindingKind::Input(_) => {
                    let buffer = fixed_buffers[&binding.binding].clone();
                    group_buffers[0].push(buffer.clone());
                    group_buffers[1].push(buffer);
                }
                GpuBindingKind::StateRead(slot) => {
                    group_buffers[0].push(state_buffers[&slot][0].clone());
                    group_buffers[1].push(state_buffers[&slot][1].clone());
                }
                GpuBindingKind::StateWrite(slot) => {
                    group_buffers[0].push(state_buffers[&slot][1].clone());
                    group_buffers[1].push(state_buffers[&slot][0].clone());
                }
                GpuBindingKind::Output(_) => {
                    let buffer = Arc::new(device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some(&binding.name),
                        size: binding.elements * std::mem::size_of::<f32>() as u64,
                        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                        mapped_at_creation: false,
                    }));
                    group_buffers[0].push(buffer.clone());
                    group_buffers[1].push(buffer.clone());
                }
            }
        }

        let mut output_buffers: [BTreeMap<String, Arc<wgpu::Buffer>>; 2] =
            [BTreeMap::new(), BTreeMap::new()];
        let mut output_elements = BTreeMap::new();
        for output in &self.outputs {
            for group in 0..2 {
                output_buffers[group].insert(
                    output.name.clone(),
                    group_buffers[group][output.binding as usize].clone(),
                );
            }
            output_elements.insert(output.name.clone(), output.elements);
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
            label: Some("Mech resident GPU bindings"),
            entries: &layout_entries,
        });
        let bind_groups = [0, 1].map(|group| {
            let entries = self
                .bindings
                .iter()
                .zip(&group_buffers[group])
                .map(|(binding, buffer)| wgpu::BindGroupEntry {
                    binding: binding.binding,
                    resource: buffer.as_entire_binding(),
                })
                .collect::<Vec<_>>();
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Mech resident GPU bind group"),
                layout: &bind_group_layout,
                entries: &entries,
            })
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Generated resident Mech WGSL"),
            source: wgpu::ShaderSource::Wgsl(self.wgsl.clone().into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Mech resident GPU pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Mech resident GPU pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        });

        Ok(ResidentGpuSession {
            adapter: adapter_name,
            device,
            queue,
            pipeline,
            bind_groups,
            input_buffers,
            output_buffers,
            output_elements,
            workgroups,
            next_group: 0,
            last_output_group: None,
        })
    }
}

impl ResidentGpuSession {
    pub fn adapter_name(&self) -> &str {
        &self.adapter
    }

    pub fn write_input(&self, name: &str, values: &[f32]) -> Result<(), GpuExecutionError> {
        let Some((buffer, elements)) = self.input_buffers.get(name) else {
            return Err(GpuExecutionError::MissingInput(name.to_owned()));
        };
        if values.len() != *elements as usize {
            return Err(GpuExecutionError::InputLength {
                name: name.to_owned(),
                expected: *elements,
                actual: values.len(),
            });
        }
        self.queue
            .write_buffer(buffer, 0, bytemuck::cast_slice(values));
        Ok(())
    }

    pub fn submit_turns(&mut self, turns: u32) -> Result<Duration, GpuExecutionError> {
        if turns == 0 {
            return Err(GpuExecutionError::InvalidFeedback(
                "resident dispatch needs at least one turn".to_owned(),
            ));
        }
        let started = Instant::now();
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Mech resident GPU turns"),
            });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Mech resident GPU compute pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            for _ in 0..turns {
                let group = self.next_group;
                pass.set_bind_group(0, &self.bind_groups[group], &[]);
                pass.dispatch_workgroups(self.workgroups, 1, 1);
                self.last_output_group = Some(group);
                self.next_group = 1 - group;
            }
        }
        self.queue.submit(Some(encoder.finish()));
        Ok(started.elapsed())
    }

    pub fn dispatch_turns(&mut self, turns: u32) -> Result<Duration, GpuExecutionError> {
        let started = Instant::now();
        self.submit_turns(turns)?;
        self.device.poll(wgpu::Maintain::Wait);
        Ok(started.elapsed())
    }

    pub fn read_outputs(
        &self,
    ) -> Result<(Duration, BTreeMap<String, Vec<f32>>), GpuExecutionError> {
        let group = self.last_output_group.ok_or_else(|| {
            GpuExecutionError::InvalidFeedback("no resident turns have run".to_owned())
        })?;
        let started = Instant::now();
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Mech resident GPU readback"),
            });
        let mut readbacks = Vec::new();
        for (name, buffer) in &self.output_buffers[group] {
            let size = self.output_elements[name] * std::mem::size_of::<f32>() as u64;
            let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Mech resident GPU readback buffer"),
                size,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            encoder.copy_buffer_to_buffer(buffer, 0, &readback, 0, size);
            readbacks.push((name.clone(), readback));
        }
        self.queue.submit(Some(encoder.finish()));

        let mut outputs = BTreeMap::new();
        for (name, readback) in readbacks {
            let slice = readback.slice(..);
            let (sender, receiver) = mpsc::channel();
            slice.map_async(wgpu::MapMode::Read, move |result| {
                let _ = sender.send(result);
            });
            self.device.poll(wgpu::Maintain::Wait);
            receiver
                .recv()
                .map_err(|_| GpuExecutionError::ChannelClosed)?
                .map_err(|error| GpuExecutionError::BufferMap(error.to_string()))?;
            let mapped = slice.get_mapped_range();
            outputs.insert(name, bytemuck::cast_slice::<u8, f32>(&mapped).to_vec());
            drop(mapped);
            readback.unmap();
        }
        Ok((started.elapsed(), outputs))
    }

    pub fn run_turns(&mut self, turns: u32) -> Result<ResidentDispatchProfile, GpuExecutionError> {
        let dispatch = self.dispatch_turns(turns)?;
        let (readback, outputs) = self.read_outputs()?;
        Ok(ResidentDispatchProfile {
            adapter: self.adapter.clone(),
            turns,
            dispatch,
            readback,
            outputs,
        })
    }
}

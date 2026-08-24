use std::{
    collections::BTreeMap,
    fmt,
    sync::Arc,
    sync::mpsc,
    time::{Duration, Instant},
};

use mech_core::CellSlotId;
use wgpu::util::DeviceExt;

use super::{
    ElementwiseKernel, GpuBindingAccess, GpuExecutionBindingRole, GpuExecutionPlan,
    GpuKernelPlanSource, GpuPhysicalOutputPlan, GpuPlanInitialValues, GpuPlanScalar,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GpuExecutionError {
    AdapterUnavailable,
    DeviceRequest(String),
    MissingInput(String),
    UnknownInput(String),
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
    InvalidPlan(String),
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

impl ElementwiseKernel {
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
        let execution_plan =
            GpuExecutionPlan::build(GpuKernelPlanSource::Elementwise(self), inputs)
                .map_err(|failure| GpuExecutionError::InvalidPlan(failure.to_string()))?;
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
        let adapter_limits = adapter.limits();
        let (required_limits, workgroups) =
            required_device_limits_for_plan(&execution_plan, &adapter_limits)?;
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
        let mut buffers = BTreeMap::new();
        for binding in &execution_plan.bindings {
            let usage = wgpu::BufferUsages::STORAGE
                | if matches!(
                    binding.role,
                    GpuExecutionBindingRole::StateWrite | GpuExecutionBindingRole::Output
                ) {
                    wgpu::BufferUsages::COPY_SRC
                } else {
                    wgpu::BufferUsages::empty()
                };
            let buffer = match &binding.initial_values {
                Some(GpuPlanInitialValues::F32(values)) => {
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some(&binding.name),
                        contents: bytemuck::cast_slice(values),
                        usage,
                    })
                }
                Some(GpuPlanInitialValues::U32(values)) => {
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some(&binding.name),
                        contents: bytemuck::cast_slice(values),
                        usage,
                    })
                }
                None => device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(&binding.name),
                    size: binding.elements * scalar_size(binding.scalar),
                    usage,
                    mapped_at_creation: false,
                }),
            };
            buffers.insert(binding.binding, buffer);
        }

        let layout_entries = execution_plan
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
        let bind_entries = execution_plan
            .bindings
            .iter()
            .map(|binding| wgpu::BindGroupEntry {
                binding: binding.binding,
                resource: buffers[&binding.binding].as_entire_binding(),
            })
            .collect::<Vec<_>>();
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Mech GPU bind group"),
            layout: &bind_group_layout,
            entries: &bind_entries,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Generated Mech WGSL"),
            source: wgpu::ShaderSource::Wgsl(execution_plan.wgsl.clone().into()),
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
            pass.dispatch_workgroups(workgroups, 1, 1);
        }
        for output in &execution_plan.physical_outputs {
            let size = output.sample_elements * std::mem::size_of::<f32>() as u64;
            let readback = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Mech GPU readback"),
                size,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            let binding = physical_output_binding(&execution_plan, output)?;
            encoder.copy_buffer_to_buffer(&buffers[&binding], 0, &readback, 0, size);
            readbacks.push((output.aliases.clone(), readback));
        }
        queue.submit(Some(encoder.finish()));

        let mut outputs = BTreeMap::new();
        for (aliases, readback) in readbacks {
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
            let values = bytemuck::cast_slice::<u8, f32>(&mapped).to_vec();
            for name in aliases {
                outputs.insert(name, values.clone());
            }
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
        let execution_plan =
            GpuExecutionPlan::build(GpuKernelPlanSource::Elementwise(self), inputs)
                .map_err(|failure| GpuExecutionError::InvalidPlan(failure.to_string()))?;
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
        let (required_limits, workgroups) =
            required_device_limits_for_plan(&execution_plan, &adapter_limits)?;
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
        for state in &execution_plan.states {
            let slot = CellSlotId::new(state.slot);
            let initial = Arc::new(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Mech resident initial state"),
                    contents: bytemuck::cast_slice(&state.initial_values),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                }),
            );
            let alternate = Arc::new(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Mech resident alternate state"),
                size: state.elements * std::mem::size_of::<f32>() as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            }));
            state_buffers.insert(slot, [initial, alternate]);
        }
        for binding in &execution_plan.bindings {
            if binding.role != GpuExecutionBindingRole::Input {
                continue;
            }
            let Some(GpuPlanInitialValues::F32(values)) = &binding.initial_values else {
                return Err(GpuExecutionError::InvalidPlan(format!(
                    "input binding `{}` has no f32 initializer",
                    binding.name
                )));
            };
            let buffer = Arc::new(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&binding.name),
                    contents: bytemuck::cast_slice(values),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                }),
            );
            fixed_buffers.insert(binding.binding, buffer.clone());
            input_buffers.insert(binding.name.clone(), (buffer, binding.elements));
        }

        let mut group_buffers: [BTreeMap<u32, Arc<wgpu::Buffer>>; 2] =
            [BTreeMap::new(), BTreeMap::new()];
        for binding in &execution_plan.bindings {
            match binding.role {
                GpuExecutionBindingRole::Input => {
                    let buffer = fixed_buffers[&binding.binding].clone();
                    group_buffers[0].insert(binding.binding, buffer.clone());
                    group_buffers[1].insert(binding.binding, buffer);
                }
                GpuExecutionBindingRole::StateRead => {
                    let slot = CellSlotId::new(binding.slot);
                    group_buffers[0].insert(binding.binding, state_buffers[&slot][0].clone());
                    group_buffers[1].insert(binding.binding, state_buffers[&slot][1].clone());
                }
                GpuExecutionBindingRole::StateWrite => {
                    let slot = CellSlotId::new(binding.slot);
                    group_buffers[0].insert(binding.binding, state_buffers[&slot][1].clone());
                    group_buffers[1].insert(binding.binding, state_buffers[&slot][0].clone());
                }
                GpuExecutionBindingRole::Output => {
                    let buffer = Arc::new(device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some(&binding.name),
                        size: binding.elements * scalar_size(binding.scalar),
                        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                        mapped_at_creation: false,
                    }));
                    group_buffers[0].insert(binding.binding, buffer.clone());
                    group_buffers[1].insert(binding.binding, buffer.clone());
                }
                GpuExecutionBindingRole::IntegrityFault => {
                    return Err(GpuExecutionError::InvalidPlan(
                        "elementwise plan unexpectedly contains an integrity binding".to_owned(),
                    ));
                }
            }
        }

        let mut output_buffers: [BTreeMap<String, Arc<wgpu::Buffer>>; 2] =
            [BTreeMap::new(), BTreeMap::new()];
        let mut output_elements = BTreeMap::new();
        for output in &execution_plan.outputs {
            for group in 0..2 {
                let physical = execution_plan
                    .physical_outputs
                    .iter()
                    .find(|physical| physical.id == output.physical_output)
                    .expect("validated execution plan references a physical output");
                let buffer = if let Some(binding) = physical.binding {
                    group_buffers[group][&binding].clone()
                } else {
                    let slot = CellSlotId::new(output.slot);
                    state_buffers[&slot][1 - group].clone()
                };
                output_buffers[group].insert(output.name.clone(), buffer);
            }
            output_elements.insert(output.name.clone(), output.elements);
        }

        let layout_entries = execution_plan
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
            let entries = execution_plan
                .bindings
                .iter()
                .map(|binding| wgpu::BindGroupEntry {
                    binding: binding.binding,
                    resource: group_buffers[group][&binding.binding].as_entire_binding(),
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
            source: wgpu::ShaderSource::Wgsl(execution_plan.wgsl.clone().into()),
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

fn scalar_size(scalar: GpuPlanScalar) -> u64 {
    match scalar {
        GpuPlanScalar::F32 | GpuPlanScalar::U32 => 4,
    }
}

fn physical_output_binding(
    plan: &GpuExecutionPlan,
    output: &GpuPhysicalOutputPlan,
) -> Result<u32, GpuExecutionError> {
    output
        .binding
        .or_else(|| {
            plan.bindings
                .iter()
                .find(|binding| {
                    binding.role == GpuExecutionBindingRole::StateWrite
                        && binding.slot == output.slot
                })
                .map(|binding| binding.binding)
        })
        .ok_or_else(|| {
            GpuExecutionError::InvalidPlan(format!(
                "physical output {} has no readable binding",
                output.id
            ))
        })
}

fn required_device_limits_for_plan(
    plan: &GpuExecutionPlan,
    adapter_limits: &wgpu::Limits,
) -> Result<(wgpu::Limits, u32), GpuExecutionError> {
    let required_storage_buffers = plan.bindings.len() as u32;
    if required_storage_buffers > adapter_limits.max_storage_buffers_per_shader_stage {
        return Err(GpuExecutionError::DeviceRequest(format!(
            "kernel needs {required_storage_buffers} storage buffers, adapter supports {}",
            adapter_limits.max_storage_buffers_per_shader_stage
        )));
    }
    let workgroups = plan.dispatch_elements.div_ceil(plan.workgroup_size);
    if workgroups > adapter_limits.max_compute_workgroups_per_dimension {
        return Err(GpuExecutionError::WorkgroupLimit {
            required: workgroups,
            supported: adapter_limits.max_compute_workgroups_per_dimension,
        });
    }
    Ok((
        wgpu::Limits {
            max_storage_buffers_per_shader_stage: required_storage_buffers,
            max_compute_workgroups_per_dimension: workgroups,
            ..wgpu::Limits::downlevel_defaults()
        },
        workgroups,
    ))
}

impl ResidentGpuSession {
    pub fn adapter(&self) -> &str {
        &self.adapter
    }

    /// Replaces one declared resident input without rebuilding the pipeline or
    /// resetting feedback state. Queue ordering makes the upload visible to the
    /// next dispatch submitted through this session.
    pub fn update_input(&self, name: &str, values: &[f32]) -> Result<(), GpuExecutionError> {
        let (buffer, expected) = self
            .input_buffers
            .get(name)
            .ok_or_else(|| GpuExecutionError::UnknownInput(name.to_owned()))?;
        if values.len() != *expected as usize {
            return Err(GpuExecutionError::InputLength {
                name: name.to_owned(),
                expected: *expected,
                actual: values.len(),
            });
        }
        self.queue
            .write_buffer(buffer, 0, bytemuck::cast_slice(values));
        Ok(())
    }

    pub fn dispatch_turns(&mut self, turns: u32) -> Result<Duration, GpuExecutionError> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GpuBinding, GpuBindingAccess, GpuBindingKind};
    use mech_core::CellSlotId;

    #[test]
    fn all_gpu_execution_modes_reject_oversized_workgroup_counts() {
        let program = ElementwiseKernel {
            compute: super::super::empty_compute_program(),
            wgsl: String::new(),
            bindings: Vec::new(),
            outputs: Vec::new(),
            states: Vec::new(),
            input_slots: BTreeMap::new(),
            constants: BTreeMap::new(),
            dispatch_elements: u64::from(super::super::WORKGROUP_SIZE) * 2,
        };
        let mut limits = wgpu::Limits::downlevel_defaults();
        limits.max_compute_workgroups_per_dimension = 1;

        let plan =
            GpuExecutionPlan::build(GpuKernelPlanSource::Elementwise(&program), &BTreeMap::new())
                .unwrap();
        assert_eq!(
            required_device_limits_for_plan(&plan, &limits).unwrap_err(),
            GpuExecutionError::WorkgroupLimit {
                required: 2,
                supported: 1,
            }
        );
    }

    #[test]
    fn all_gpu_execution_modes_request_the_validated_limits() {
        let program = ElementwiseKernel {
            compute: super::super::empty_compute_program(),
            wgsl: String::new(),
            bindings: vec![GpuBinding {
                binding: 0,
                name: "output".to_owned(),
                access: GpuBindingAccess::ReadWrite,
                elements: u64::from(super::super::WORKGROUP_SIZE) * 2,
                kind: GpuBindingKind::Output(CellSlotId::new(0)),
            }],
            outputs: Vec::new(),
            states: Vec::new(),
            input_slots: BTreeMap::new(),
            constants: BTreeMap::new(),
            dispatch_elements: u64::from(super::super::WORKGROUP_SIZE) * 2,
        };
        let limits = wgpu::Limits {
            max_storage_buffers_per_shader_stage: 8,
            max_compute_workgroups_per_dimension: 8,
            ..wgpu::Limits::downlevel_defaults()
        };

        let plan =
            GpuExecutionPlan::build(GpuKernelPlanSource::Elementwise(&program), &BTreeMap::new())
                .unwrap();
        let (requested, workgroups) = required_device_limits_for_plan(&plan, &limits).unwrap();
        assert_eq!(workgroups, 2);
        assert_eq!(requested.max_storage_buffers_per_shader_stage, 1);
        assert_eq!(requested.max_compute_workgroups_per_dimension, 2);
    }
}

//! Direct Metal resident execution for the native benchmark lane.
//!
//! This is deliberately an opt-in backend. The portable resident path remains
//! WGPU; this module lets macOS measure the same generated Mech shader with a
//! direct Metal command encoder, which is the boundary used by the Mojo
//! native-Metal reference.

use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

use mech_core::CellSlotId;
use metal::{CompileOptions, Device, MTLResourceOptions, MTLSize};
use naga::{
    ResourceBinding,
    back::msl::{self, BindTarget, EntryPointResourceMap, EntryPointResources},
    front::wgsl,
    proc::{BoundsCheckPolicies, BoundsCheckPolicy},
    valid::{Capabilities, ValidationFlags, Validator},
};

use super::{
    BatchedExecutionError, BatchedIntegrityFault, FixedShapeKernel, component_major_values,
    instance_major_values,
};
const FAULT_WORDS: usize = 2;

/// A direct Metal resident session for a generated fixed-shape Mech kernel.
///
/// This backend is only compiled on macOS with the `metal-native` feature. It
/// keeps the same double-buffer publication boundary and compact fault status
/// as the WGPU session, so checked and unchecked measurements remain directly
/// comparable; only command encoding is different.
pub struct BatchedResidentMetalSession {
    adapter: String,
    queue: metal::CommandQueue,
    pipeline: metal::ComputePipelineState,
    input_buffers: Vec<metal::Buffer>,
    state_buffers: Vec<(CellSlotId, [metal::Buffer; 2])>,
    constraints: Box<[super::BatchedConstraint]>,
    fault_buffer: Option<metal::Buffer>,
    sizes_buffer: metal::Buffer,
    instances: usize,
    next_group: usize,
    last_output_group: Option<usize>,
    faults: super::BatchedFaultRecorder,
}

impl FixedShapeKernel {
    /// Prepare a direct Metal session from the same backend-neutral scalar IR
    /// used by the portable WGPU resident executor.
    pub fn prepare_metal(
        &self,
        inputs: &BTreeMap<String, Vec<f32>>,
    ) -> Result<BatchedResidentMetalSession, BatchedExecutionError> {
        let expanded_inputs = self.expand_inputs(inputs)?;
        let broadcast_inputs = self
            .inputs
            .iter()
            .filter(|input| {
                inputs
                    .get(&input.name)
                    .is_some_and(|values| values.len() == input.shape.elements())
            })
            .map(|input| input.slot)
            .collect::<BTreeSet<_>>();
        let shader = self.component_major_wgsl(&broadcast_inputs);
        let (msl, entry_point) = compile_msl(
            &shader,
            self.inputs.len(),
            self.states.len(),
            !self.constraints.is_empty(),
        )?;
        let device = Device::system_default()
            .ok_or_else(|| BatchedExecutionError::Native("Metal device unavailable".to_owned()))?;
        let options = CompileOptions::new();
        options.set_fast_math_enabled(false);
        let library = device
            .new_library_with_source(&msl, &options)
            .map_err(BatchedExecutionError::Native)?;
        let function = library
            .get_function(&entry_point, None)
            .map_err(BatchedExecutionError::Native)?;
        let pipeline = device
            .new_compute_pipeline_state_with_function(&function)
            .map_err(BatchedExecutionError::Native)?;
        let queue = device.new_command_queue();

        let mut input_buffers = Vec::with_capacity(self.inputs.len());
        for input in &self.inputs {
            let values = expanded_inputs
                .get(&input.slot)
                .ok_or_else(|| BatchedExecutionError::MissingInput(input.name.clone()))?;
            let values =
                component_major_values(values, input.shape.elements(), self.instances as usize);
            input_buffers.push(buffer_from_f32(&device, &values));
        }

        let mut state_buffers = Vec::with_capacity(self.states.len());
        for state in &self.states {
            let expanded = state
                .initializer
                .iter()
                .copied()
                .cycle()
                .take(state.shape.elements() * self.instances as usize)
                .collect::<Vec<_>>();
            let values =
                component_major_values(&expanded, state.shape.elements(), self.instances as usize);
            let initial = buffer_from_f32(&device, &values);
            let alternate = buffer_from_len(&device, values.len());
            state_buffers.push((state.slot, [initial, alternate]));
        }

        let fault_buffer =
            (!self.constraints.is_empty()).then(|| buffer_from_u32(&device, &[0; FAULT_WORDS]));
        let mut sizes = input_buffers
            .iter()
            .map(|buffer| buffer.length() as u32)
            .collect::<Vec<_>>();
        for (_, buffers) in &state_buffers {
            sizes.push(buffers[0].length() as u32);
            sizes.push(buffers[1].length() as u32);
        }
        if let Some(fault) = &fault_buffer {
            sizes.push(fault.length() as u32);
        }
        let sizes_buffer = buffer_from_u32(&device, &sizes);
        Ok(BatchedResidentMetalSession {
            adapter: "Apple Metal (direct command encoder)".to_owned(),
            queue,
            pipeline,
            input_buffers,
            state_buffers,
            constraints: self.constraints.clone().into_boxed_slice(),
            fault_buffer,
            sizes_buffer,
            instances: self.instances as usize,
            next_group: 0,
            last_output_group: None,
            faults: super::BatchedFaultRecorder::default(),
        })
    }
}

impl BatchedResidentMetalSession {
    pub fn adapter(&self) -> &str {
        &self.adapter
    }

    /// Dispatches resident turns with one Metal command buffer and host wait
    /// per turn, matching the Mojo timing boundary.
    pub fn dispatch_turns(&mut self, turns: u32) -> Result<Duration, BatchedExecutionError> {
        if turns == 0 {
            return Err(BatchedExecutionError::ZeroTurns);
        }
        let started = Instant::now();
        for _ in 0..turns {
            let attempted_turn = self.faults.next_turn();
            let group = self.next_group;
            if let Some(fault) = &self.fault_buffer {
                clear_fault(fault);
            }
            let command_buffer = self.queue.new_command_buffer();
            let encoder = command_buffer.new_compute_command_encoder();
            encoder.set_compute_pipeline_state(&self.pipeline);
            let mut binding = 0;
            for input in &self.input_buffers {
                encoder.set_buffer(binding, Some(input), 0);
                binding += 1;
            }
            for (_, state) in &self.state_buffers {
                encoder.set_buffer(binding, Some(&state[group]), 0);
                binding += 1;
                encoder.set_buffer(binding, Some(&state[1 - group]), 0);
                binding += 1;
            }
            if let Some(fault) = &self.fault_buffer {
                encoder.set_buffer(binding, Some(fault), 0);
                binding += 1;
            }
            encoder.set_buffer(binding, Some(&self.sizes_buffer), 0);
            encoder.dispatch_threads(
                MTLSize::new(self.instances as u64, 1, 1),
                MTLSize::new(crate::WORKGROUP_SIZE as u64, 1, 1),
            );
            encoder.end_encoding();
            command_buffer.commit();
            command_buffer.wait_until_completed();

            if let Some(fault) = &self.fault_buffer {
                let words = read_u32(fault, FAULT_WORDS);
                if words[0] != 0 {
                    let packed = words[1];
                    let code = packed & 0xff;
                    let Some(constraint) = self.constraints.get(code.saturating_sub(1) as usize)
                    else {
                        return Err(BatchedExecutionError::Native(format!(
                            "Metal returned unknown integrity constraint code {code}"
                        )));
                    };
                    let fault = BatchedIntegrityFault {
                        attempted_turn,
                        instance: packed >> 8,
                        constraint: constraint.id,
                        constraint_name: constraint.name.clone(),
                    };
                    return Err(self.faults.record(fault));
                }
            }
            self.last_output_group = Some(group);
            self.next_group = 1 - group;
        }
        Ok(started.elapsed())
    }

    pub fn read_state(&self) -> Result<BTreeMap<CellSlotId, Vec<f32>>, BatchedExecutionError> {
        let group = self.last_output_group.ok_or_else(|| {
            BatchedExecutionError::Native("Metal session has no output".to_owned())
        })?;
        // `last_output_group` records the source group used by the last
        // dispatch; the published candidate was written to its opposite.
        let output_group = 1 - group;
        self.read_state_group(output_group)
    }

    /// Reads the currently published state, including the initial state or
    /// the estimate retained after a rejected candidate.
    pub fn read_published_state(
        &self,
    ) -> Result<BTreeMap<CellSlotId, Vec<f32>>, BatchedExecutionError> {
        let output_group = self.last_output_group.map_or(0, |group| 1 - group);
        self.read_state_group(output_group)
    }

    fn read_state_group(
        &self,
        output_group: usize,
    ) -> Result<BTreeMap<CellSlotId, Vec<f32>>, BatchedExecutionError> {
        let mut result = BTreeMap::new();
        for (slot, buffers) in &self.state_buffers {
            let elements = buffers[0].length() as usize / std::mem::size_of::<f32>();
            let values = read_f32(&buffers[output_group], elements);
            result.insert(
                *slot,
                instance_major_values(&values, elements / self.instances, self.instances),
            );
        }
        Ok(result)
    }
}

fn buffer_from_f32(device: &Device, values: &[f32]) -> metal::Buffer {
    buffer_from_bytes(device, bytemuck::cast_slice(values))
}

fn buffer_from_u32(device: &Device, values: &[u32]) -> metal::Buffer {
    buffer_from_bytes(device, bytemuck::cast_slice(values))
}

fn buffer_from_len(device: &Device, elements: usize) -> metal::Buffer {
    device.new_buffer(
        (elements * std::mem::size_of::<f32>()) as u64,
        MTLResourceOptions::StorageModeShared,
    )
}

fn buffer_from_bytes(device: &Device, bytes: &[u8]) -> metal::Buffer {
    device.new_buffer_with_data(
        bytes.as_ptr().cast(),
        bytes.len() as u64,
        MTLResourceOptions::StorageModeShared,
    )
}

fn clear_fault(buffer: &metal::Buffer) {
    let bytes =
        unsafe { std::slice::from_raw_parts_mut(buffer.contents().cast::<u32>(), FAULT_WORDS) };
    bytes[0] = 0;
    bytes[1] = u32::MAX;
}

fn read_u32(buffer: &metal::Buffer, count: usize) -> Vec<u32> {
    unsafe { std::slice::from_raw_parts(buffer.contents().cast::<u32>(), count).to_vec() }
}

fn read_f32(buffer: &metal::Buffer, count: usize) -> Vec<f32> {
    unsafe { std::slice::from_raw_parts(buffer.contents().cast::<f32>(), count).to_vec() }
}

fn compile_msl(
    source: &str,
    input_count: usize,
    state_count: usize,
    checked: bool,
) -> Result<(String, String), BatchedExecutionError> {
    let module = wgsl::parse_str(source)
        .map_err(|error| BatchedExecutionError::Native(error.emit_to_string(source)))?;
    let info = Validator::new(ValidationFlags::all(), Capabilities::all())
        .validate(&module)
        .map_err(|error| BatchedExecutionError::Native(error.to_string()))?;
    let mut resources = EntryPointResources::default();
    let binding_count = input_count + state_count * 2 + usize::from(checked);
    for binding in 0..binding_count {
        let mutable = binding >= input_count
            && binding < input_count + state_count * 2
            && (binding - input_count) % 2 == 1;
        resources.resources.insert(
            ResourceBinding {
                group: 0,
                binding: binding as u32,
            },
            BindTarget {
                buffer: Some(binding as u8),
                mutable: mutable || (checked && binding + 1 == binding_count),
                ..Default::default()
            },
        );
    }
    resources.sizes_buffer = Some(binding_count as u8);
    let options = msl::Options {
        lang_version: (3, 0),
        per_entry_point_map: EntryPointResourceMap::from([("main".to_owned(), resources)]),
        inline_samplers: Vec::new(),
        spirv_cross_compatibility: false,
        fake_missing_bindings: false,
        bounds_check_policies: BoundsCheckPolicies {
            index: BoundsCheckPolicy::Unchecked,
            buffer: BoundsCheckPolicy::Unchecked,
            image_load: BoundsCheckPolicy::Unchecked,
            image_store: BoundsCheckPolicy::Unchecked,
            binding_array: BoundsCheckPolicy::Unchecked,
        },
        zero_initialize_workgroup_memory: true,
    };
    let (source, info) =
        msl::write_string(&module, &info, &options, &msl::PipelineOptions::default())
            .map_err(|error| BatchedExecutionError::Native(error.to_string()))?;
    let entry_point = info
        .entry_point_names
        .into_iter()
        .next()
        .ok_or_else(|| BatchedExecutionError::Native("Metal shader has no entry point".to_owned()))?
        .map_err(|error| BatchedExecutionError::Native(error.to_string()))?;
    if std::env::var_os("MECH_DUMP_MSL").is_some() {
        println!("--- generated MSL ---\n{source}--- end generated MSL ---");
    }
    Ok((source, entry_point))
}

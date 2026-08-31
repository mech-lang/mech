//! Direct Metal resident execution for apples-to-apples macOS measurements.
//!
//! This backend intentionally sits beside the portable `wgpu` backend. It
//! consumes the same generated WGSL, translates it with Naga, and submits the
//! resulting function through Metal directly. That makes the cost of the
//! `wgpu` command path visible instead of attributing it to Mech's kernel.

use std::{
    collections::BTreeMap,
    mem,
    time::{Duration, Instant},
};

use metal::{Buffer, CommandQueue, ComputePipelineState, Device, MTLResourceOptions, MTLSize};
use naga::{
    AddressSpace, ArraySize, TypeInner,
    back::msl,
    front::wgsl,
    valid::{Capabilities, ValidationFlags, Validator},
};

use crate::{BatchedExecutionError, FixedShapeKernel, WORKGROUP_SIZE};
use mech_core::{CellSlotId, IntegrityConstraintId};

const FAULT_WORDS: usize = 2;

struct MetalState {
    slot: CellSlotId,
    elements: usize,
    read_binding: u32,
    write_binding: u32,
    buffers: [Buffer; 2],
}

/// A direct Metal resident session over the same fixed-shape Mech kernel used
/// by the portable WGPU backend.
pub struct MetalResidentGpuSession {
    queue: CommandQueue,
    pipeline: ComputePipelineState,
    inputs: Vec<(u32, Buffer)>,
    sizes: Buffer,
    sizes_binding: u32,
    states: Vec<MetalState>,
    fault: Option<Buffer>,
    constraints: Vec<(IntegrityConstraintId, Box<str>)>,
    workgroups: u64,
    next_group: usize,
    last_output_group: Option<usize>,
    attempted_turns: u64,
    checked: bool,
}

impl FixedShapeKernel {
    /// Compiles the generated Mech WGSL to MSL and prepares a direct Metal
    /// session. Unlike the WGPU path, no portable command encoder is involved.
    #[allow(clippy::missing_const_for_fn)]
    pub fn prepare_native_metal(
        &self,
        inputs: &BTreeMap<String, Vec<f32>>,
    ) -> Result<MetalResidentGpuSession, BatchedExecutionError> {
        MetalResidentGpuSession::prepare(self, inputs)
    }
}

impl MetalResidentGpuSession {
    pub fn prepare(
        kernel: &FixedShapeKernel,
        inputs: &BTreeMap<String, Vec<f32>>,
    ) -> Result<Self, BatchedExecutionError> {
        let device = Device::system_default()
            .ok_or_else(|| BatchedExecutionError::Native("Metal device unavailable".to_owned()))?;
        let queue = device.new_command_queue();
        let (msl_source, entry_point, size_bindings) = wgsl_to_msl(kernel.wgsl())?;
        let options = metal::CompileOptions::new();
        options.set_language_version(metal::MTLLanguageVersion::V2_2);
        let library = device
            .new_library_with_source(&msl_source, &options)
            .map_err(|error| {
                BatchedExecutionError::Native(format!("Metal shader compile: {error}"))
            })?;
        let function = library.get_function(&entry_point, None).map_err(|error| {
            BatchedExecutionError::Native(format!("Metal entry point: {error}"))
        })?;
        let pipeline = device
            .new_compute_pipeline_state_with_function(&function)
            .map_err(|error| BatchedExecutionError::Native(format!("Metal pipeline: {error}")))?;

        let physical_inputs = kernel.physical_inputs(inputs)?;
        let input_buffers = physical_inputs
            .iter()
            .map(|input| {
                let buffer = device.new_buffer_with_data(
                    input.initial_values.as_ptr() as *const _,
                    (input.initial_values.len() * mem::size_of::<f32>()) as metal::NSUInteger,
                    MTLResourceOptions::StorageModeShared,
                );
                (input.binding, buffer)
            })
            .collect();

        let states = kernel
            .physical_states()
            .into_iter()
            .map(|state| {
                let initial = device.new_buffer_with_data(
                    state.initial_values.as_ptr() as *const _,
                    (state.initial_values.len() * mem::size_of::<f32>()) as metal::NSUInteger,
                    MTLResourceOptions::StorageModeShared,
                );
                let alternate = device.new_buffer(
                    (state.elements * mem::size_of::<f32>()) as u64,
                    MTLResourceOptions::StorageModeShared,
                );
                MetalState {
                    slot: state.slot,
                    elements: state.elements,
                    read_binding: state.read_binding,
                    write_binding: state.write_binding,
                    buffers: [initial, alternate],
                }
            })
            .collect::<Vec<_>>();

        let checked = kernel.integrity_buffer().is_some();
        let fault = checked.then(|| {
            let values = [0_u32, u32::MAX];
            device.new_buffer_with_data(
                values.as_ptr() as *const _,
                (FAULT_WORDS * mem::size_of::<u32>()) as metal::NSUInteger,
                MTLResourceOptions::StorageModeShared,
            )
        });
        let sizes_binding =
            (physical_inputs.len() + states.len() * 2 + usize::from(checked)) as u32;
        let mut buffer_lengths = BTreeMap::new();
        for input in &physical_inputs {
            buffer_lengths.insert(input.binding, input.elements as u32);
        }
        for state in &states {
            buffer_lengths.insert(state.read_binding, state.elements as u32);
            buffer_lengths.insert(state.write_binding, state.elements as u32);
        }
        if let Some(integrity) = kernel.integrity_buffer() {
            buffer_lengths.insert(integrity.binding, FAULT_WORDS as u32);
        }
        let size_words = size_bindings
            .iter()
            .map(|(_, index)| *index as usize + 1)
            .max()
            .unwrap_or(0);
        let mut size_values = vec![0_u32; size_words];
        for (binding, index) in size_bindings {
            size_values[index as usize] = *buffer_lengths.get(&binding).ok_or_else(|| {
                BatchedExecutionError::Native(format!(
                    "Metal has no physical buffer for WGSL binding {binding}"
                ))
            })?;
        }
        let sizes = device.new_buffer_with_data(
            size_values.as_ptr() as *const _,
            (size_values.len() * mem::size_of::<u32>()) as metal::NSUInteger,
            MTLResourceOptions::StorageModeShared,
        );
        let constraints = kernel
            .named_integrity_constraints()
            .map(|(id, name)| (id, name.into()))
            .collect();
        Ok(Self {
            queue,
            pipeline,
            inputs: input_buffers,
            sizes,
            sizes_binding,
            states,
            fault,
            constraints,
            workgroups: u64::from(kernel.workgroup_count()),
            next_group: 0,
            last_output_group: None,
            attempted_turns: 0,
            checked,
        })
    }

    pub fn adapter(&self) -> &'static str {
        "Metal (native)"
    }

    pub fn dispatch_turns(&mut self, turns: u32) -> Result<Duration, BatchedExecutionError> {
        if turns == 0 {
            return Err(BatchedExecutionError::ZeroTurns);
        }
        let started = Instant::now();
        for _ in 0..turns {
            let attempted_turn = self.attempted_turns.saturating_add(1);
            if self.checked {
                self.clear_fault();
            }
            let read_group = self.next_group;
            let write_group = 1 - read_group;
            let command_buffer = self.queue.new_command_buffer();
            let encoder = command_buffer.new_compute_command_encoder();
            encoder.set_compute_pipeline_state(&self.pipeline);
            for (binding, buffer) in &self.inputs {
                encoder.set_buffer(u64::from(*binding), Some(buffer), 0);
            }
            for state in &self.states {
                encoder.set_buffer(
                    u64::from(state.read_binding),
                    Some(&state.buffers[read_group]),
                    0,
                );
                encoder.set_buffer(
                    u64::from(state.write_binding),
                    Some(&state.buffers[write_group]),
                    0,
                );
            }
            if let Some(fault) = &self.fault {
                let binding = self.inputs.len() as u64 + self.states.len() as u64 * 2;
                encoder.set_buffer(binding, Some(fault), 0);
            }
            encoder.set_buffer(u64::from(self.sizes_binding), Some(&self.sizes), 0);
            encoder.dispatch_thread_groups(
                MTLSize {
                    width: self.workgroups,
                    height: 1,
                    depth: 1,
                },
                MTLSize {
                    width: u64::from(WORKGROUP_SIZE),
                    height: 1,
                    depth: 1,
                },
            );
            encoder.end_encoding();
            command_buffer.commit();
            command_buffer.wait_until_completed();
            if let Some((instance, code)) = self.read_fault()? {
                let constraint_index = code as usize - 1;
                let Some((constraint, name)) = self.constraints.get(constraint_index) else {
                    return Err(BatchedExecutionError::Native(format!(
                        "Metal returned unknown integrity constraint code {code}"
                    )));
                };
                return Err(BatchedExecutionError::Integrity(
                    crate::BatchedIntegrityFault {
                        attempted_turn,
                        instance,
                        constraint: *constraint,
                        constraint_name: name.clone(),
                    },
                ));
            }
            self.next_group = write_group;
            self.last_output_group = Some(write_group);
            self.attempted_turns = attempted_turn;
        }
        Ok(started.elapsed())
    }

    pub fn read_state(&self) -> Result<BTreeMap<CellSlotId, Vec<f32>>, BatchedExecutionError> {
        let group = self
            .last_output_group
            .ok_or_else(|| BatchedExecutionError::Native("no Metal turns have run".to_owned()))?;
        Ok(self
            .states
            .iter()
            .map(|state| {
                let values = unsafe {
                    std::slice::from_raw_parts(
                        state.buffers[group].contents() as *const f32,
                        state.elements,
                    )
                };
                (state.slot, values.to_vec())
            })
            .collect())
    }

    fn clear_fault(&self) {
        let Some(fault) = &self.fault else { return };
        unsafe {
            let values = std::slice::from_raw_parts_mut(fault.contents() as *mut u32, FAULT_WORDS);
            values[0] = 0;
            values[1] = u32::MAX;
        }
    }

    fn read_fault(&self) -> Result<Option<(u32, u32)>, BatchedExecutionError> {
        let Some(fault) = &self.fault else {
            return Ok(None);
        };
        let values =
            unsafe { std::slice::from_raw_parts(fault.contents() as *const u32, FAULT_WORDS) };
        if values[0] == 0 {
            return Ok(None);
        }
        let packed = values[1];
        let code = packed & 0xff;
        if code == 0 {
            return Err(BatchedExecutionError::Native(
                "Metal returned an empty integrity constraint code".to_owned(),
            ));
        }
        Ok(Some((packed >> 8, code)))
    }
}

fn wgsl_to_msl(source: &str) -> Result<(String, String, Vec<(u32, u32)>), BatchedExecutionError> {
    let module = wgsl::parse_str(source)
        .map_err(|error| BatchedExecutionError::Native(format!("WGSL parse for Metal: {error}")))?;
    let info = Validator::new(ValidationFlags::all(), Capabilities::all())
        .validate(&module)
        .map_err(|error| {
            BatchedExecutionError::Native(format!("WGSL validate for Metal: {error}"))
        })?;
    let mut resources = BTreeMap::new();
    let mut size_bindings = Vec::new();
    for (handle, variable) in module.global_variables.iter() {
        if let Some(binding) = variable.binding.clone() {
            if matches!(variable.space, AddressSpace::Storage { .. }) {
                if let TypeInner::Array {
                    size: ArraySize::Dynamic,
                    ..
                } = &module.types[variable.ty].inner
                {
                    size_bindings.push((binding.binding, handle.index() as u32));
                }
            }
            resources.insert(
                binding.clone(),
                msl::BindTarget {
                    buffer: Some(binding.binding as msl::Slot),
                    mutable: true,
                    ..Default::default()
                },
            );
        }
    }
    let per_entry_point_map = msl::EntryPointResourceMap::from([(
        "main".to_owned(),
        msl::EntryPointResources {
            resources,
            sizes_buffer: Some(
                (size_bindings
                    .iter()
                    .map(|(binding, _)| *binding)
                    .max()
                    .unwrap_or(0)
                    .saturating_add(1)) as msl::Slot,
            ),
            ..Default::default()
        },
    )]);
    let options = msl::Options {
        lang_version: (2, 2),
        per_entry_point_map,
        fake_missing_bindings: false,
        ..Default::default()
    };
    let (source, translation) =
        msl::write_string(&module, &info, &options, &msl::PipelineOptions::default())
            .map_err(|error| BatchedExecutionError::Native(format!("WGSL to MSL: {error}")))?;
    let entry_point = translation
        .entry_point_names
        .into_iter()
        .next()
        .ok_or_else(|| BatchedExecutionError::Native("Metal shader has no entry point".to_owned()))?
        .map_err(|error| {
            BatchedExecutionError::Native(format!("WGSL to MSL entry point: {error}"))
        })?;
    Ok((source, entry_point, size_bindings))
}

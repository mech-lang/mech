use super::{MemoryBudgetLimits, MemoryPlanError, MemoryTargetKind, SlotLayout};
use crate::Value;
use crate::snapshot::{Complex32Bits, Rational64Value};

#[cfg(feature = "no_std")]
use alloc::string::String;
#[cfg(not(feature = "no_std"))]
use std::string::String;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TargetPrimitiveLayouts {
    pub bool_slot: SlotLayout,
    pub u8_slot: SlotLayout,
    pub u16_slot: SlotLayout,
    pub u32_slot: SlotLayout,
    pub u64_slot: SlotLayout,
    pub u128_slot: SlotLayout,
    pub i8_slot: SlotLayout,
    pub i16_slot: SlotLayout,
    pub i32_slot: SlotLayout,
    pub i64_slot: SlotLayout,
    pub i128_slot: SlotLayout,
    pub f32_slot: SlotLayout,
    pub f64_slot: SlotLayout,
    pub c64_slot: SlotLayout,
    pub r64_slot: SlotLayout,
    pub id_slot: SlotLayout,
    pub index_slot: SlotLayout,
    pub atom_slot: SlotLayout,
    pub string_header: SlotLayout,
    pub canonical_value_handle: SlotLayout,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TargetMemoryProfile {
    pub kind: MemoryTargetKind,
    pub primitives: TargetPrimitiveLayouts,
    pub limits: MemoryBudgetLimits,
    pub maximum_addressable_bytes: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GpuMemoryLimits {
    pub max_buffer_size: u64,
    pub max_storage_buffer_binding_size: u64,
    pub max_storage_buffers_per_shader_stage: u32,
    pub max_bindings_per_bind_group: u32,
    pub max_compute_workgroups_per_dimension: u32,
    pub max_compute_invocations_per_workgroup: u32,
    pub max_compute_workgroup_size_x: u32,
    pub min_storage_buffer_offset_alignment: u32,
}

const RESIDENT_MAX_OUTPUT_ELEMENTS: u64 = 65_536;
const RESIDENT_MAX_BYTES: u64 = 16 * 1024 * 1024;
const RESIDENT_MAX_RETAINED_NODES: u64 = 65_536;
const RESIDENT_MAX_COMPARISON_WORK: u64 = 65_536;
const RESIDENT_MAX_COMPUTE_WORK: u64 = 16_777_216;
const GPU_MAX_SCALAR_INSTRUCTIONS: u64 = 16_777_216;

impl TargetMemoryProfile {
    pub fn current_direct_host() -> Result<Self, MemoryPlanError> {
        current_host(MemoryTargetKind::DirectHost)
    }

    pub fn current_resident_cpu() -> Result<Self, MemoryPlanError> {
        let mut primitives = host_primitives()?;
        primitives.bool_slot = rust_layout::<u8>()?;
        primitives.index_slot = rust_layout::<u64>()?;
        primitives.f64_slot = rust_layout::<f64>()?;
        primitives.string_header = rust_layout::<String>()?;
        primitives.canonical_value_handle = rust_layout::<Option<Value>>()?;
        Ok(Self {
            kind: MemoryTargetKind::ResidentCpu,
            primitives,
            limits: MemoryBudgetLimits {
                max_output_elements: Some(RESIDENT_MAX_OUTPUT_ELEMENTS),
                max_output_bytes: Some(RESIDENT_MAX_BYTES),
                max_temporary_bytes: Some(RESIDENT_MAX_BYTES),
                max_cloned_bytes: Some(RESIDENT_MAX_BYTES),
                max_retained_nodes: Some(RESIDENT_MAX_RETAINED_NODES),
                max_comparison_work: Some(RESIDENT_MAX_COMPARISON_WORK),
                max_compute_work: Some(RESIDENT_MAX_COMPUTE_WORK),
                ..MemoryBudgetLimits::default()
            },
            maximum_addressable_bytes: current_addressable_bytes(),
        })
    }

    pub fn current_native_host() -> Result<Self, MemoryPlanError> {
        current_host(MemoryTargetKind::NativeHost)
    }

    pub fn current_wasm_host() -> Result<Self, MemoryPlanError> {
        current_host(MemoryTargetKind::WasmHost)
    }

    pub fn gpu(limits: GpuMemoryLimits) -> Result<Self, MemoryPlanError> {
        validate_alignment(limits.min_storage_buffer_offset_alignment)?;
        let scalar = SlotLayout {
            bytes: 4,
            alignment: 4,
        };
        let maximum_buffer = limits
            .max_buffer_size
            .min(limits.max_storage_buffer_binding_size);
        let maximum_bindings = limits
            .max_storage_buffers_per_shader_stage
            .min(limits.max_bindings_per_bind_group);
        Ok(Self {
            kind: MemoryTargetKind::Gpu,
            primitives: TargetPrimitiveLayouts {
                bool_slot: scalar,
                u8_slot: scalar,
                u16_slot: scalar,
                u32_slot: scalar,
                u64_slot: scalar,
                u128_slot: scalar,
                i8_slot: scalar,
                i16_slot: scalar,
                i32_slot: scalar,
                i64_slot: scalar,
                i128_slot: scalar,
                f32_slot: scalar,
                f64_slot: scalar,
                c64_slot: scalar,
                r64_slot: scalar,
                id_slot: scalar,
                index_slot: scalar,
                atom_slot: scalar,
                string_header: scalar,
                canonical_value_handle: scalar,
            },
            limits: MemoryBudgetLimits {
                max_scalar_instructions: Some(GPU_MAX_SCALAR_INSTRUCTIONS),
                max_storage_buffer_bytes: Some(maximum_buffer),
                max_storage_bindings: Some(maximum_bindings),
                ..MemoryBudgetLimits::default()
            },
            maximum_addressable_bytes: limits.max_buffer_size,
        })
    }
}

fn current_host(kind: MemoryTargetKind) -> Result<TargetMemoryProfile, MemoryPlanError> {
    Ok(TargetMemoryProfile {
        kind,
        primitives: host_primitives()?,
        limits: MemoryBudgetLimits::default(),
        maximum_addressable_bytes: current_addressable_bytes(),
    })
}

fn current_addressable_bytes() -> u64 {
    usize::MAX as u64
}

fn host_primitives() -> Result<TargetPrimitiveLayouts, MemoryPlanError> {
    Ok(TargetPrimitiveLayouts {
        bool_slot: rust_layout::<bool>()?,
        u8_slot: rust_layout::<u8>()?,
        u16_slot: rust_layout::<u16>()?,
        u32_slot: rust_layout::<u32>()?,
        u64_slot: rust_layout::<u64>()?,
        u128_slot: rust_layout::<u128>()?,
        i8_slot: rust_layout::<i8>()?,
        i16_slot: rust_layout::<i16>()?,
        i32_slot: rust_layout::<i32>()?,
        i64_slot: rust_layout::<i64>()?,
        i128_slot: rust_layout::<i128>()?,
        f32_slot: rust_layout::<f32>()?,
        f64_slot: rust_layout::<f64>()?,
        c64_slot: rust_layout::<Complex32Bits>()?,
        r64_slot: rust_layout::<Rational64Value>()?,
        id_slot: rust_layout::<u64>()?,
        index_slot: rust_layout::<usize>()?,
        atom_slot: rust_layout::<()>()?,
        string_header: rust_layout::<String>()?,
        canonical_value_handle: rust_layout::<Value>()?,
    })
}

fn rust_layout<T>() -> Result<SlotLayout, MemoryPlanError> {
    let alignment = u32::try_from(core::mem::align_of::<T>()).map_err(|_| {
        MemoryPlanError::ArithmeticOverflow {
            field: "target primitive alignment",
        }
    })?;
    validate_alignment(alignment)?;
    Ok(SlotLayout {
        bytes: u64::try_from(core::mem::size_of::<T>()).map_err(|_| {
            MemoryPlanError::ArithmeticOverflow {
                field: "target primitive bytes",
            }
        })?,
        alignment,
    })
}

pub(crate) fn validate_alignment(alignment: u32) -> Result<(), MemoryPlanError> {
    if alignment == 0 || !alignment.is_power_of_two() {
        return Err(MemoryPlanError::InvalidAlignment { alignment });
    }
    Ok(())
}

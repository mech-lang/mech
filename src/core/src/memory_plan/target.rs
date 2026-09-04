use super::{MemoryBudgetLimits, MemoryTargetKind, SlotLayout};

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

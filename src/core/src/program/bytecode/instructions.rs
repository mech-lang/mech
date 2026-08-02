use super::ApplicationRequirement;

#[cfg(feature = "no_std")]
use alloc::vec::Vec;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Opcode {
    ConstLoad = 0x01,
    RuntimeNullary = 0x10,
    RuntimeUnary = 0x11,
    RuntimeBinary = 0x12,
    RuntimeTernary = 0x13,
    RuntimeQuaternary = 0x14,
    RuntimeVariadic = 0x15,
    HostCall = 0x20,
    ResourceRead = 0x21,
    ResourceWrite = 0x22,
    ResourceSend = 0x23,
    Return = 0xFF,
}

impl Opcode {
    pub(crate) fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x01 => Some(Self::ConstLoad),
            0x10 => Some(Self::RuntimeNullary),
            0x11 => Some(Self::RuntimeUnary),
            0x12 => Some(Self::RuntimeBinary),
            0x13 => Some(Self::RuntimeTernary),
            0x14 => Some(Self::RuntimeQuaternary),
            0x15 => Some(Self::RuntimeVariadic),
            0x20 => Some(Self::HostCall),
            0x21 => Some(Self::ResourceRead),
            0x22 => Some(Self::ResourceWrite),
            0x23 => Some(Self::ResourceSend),
            0xFF => Some(Self::Return),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BytecodeInstruction {
    ConstLoad {
        dst: u32,
        constant: u32,
    },
    RuntimeNullary {
        function: u64,
        dst: u32,
    },
    RuntimeUnary {
        function: u64,
        dst: u32,
        src: u32,
    },
    RuntimeBinary {
        function: u64,
        dst: u32,
        lhs: u32,
        rhs: u32,
    },
    RuntimeTernary {
        function: u64,
        dst: u32,
        a: u32,
        b: u32,
        c: u32,
    },
    RuntimeQuaternary {
        function: u64,
        dst: u32,
        a: u32,
        b: u32,
        c: u32,
        d: u32,
    },
    RuntimeVariadic {
        function: u64,
        dst: u32,
        arguments: Vec<u32>,
    },
    HostCall {
        requirement: u32,
        dst: u32,
        arguments: Vec<u32>,
    },
    ResourceRead {
        requirement: u32,
        dst: u32,
    },
    ResourceWrite {
        requirement: u32,
        dst: u32,
        src: u32,
    },
    ResourceSend {
        requirement: u32,
        dst: u32,
        src: u32,
    },
    Return {
        src: u32,
    },
}

impl BytecodeInstruction {
    pub(crate) fn remap_requirement(&mut self, remap: &[u32]) {
        let requirement = match self {
            Self::HostCall { requirement, .. }
            | Self::ResourceRead { requirement, .. }
            | Self::ResourceWrite { requirement, .. }
            | Self::ResourceSend { requirement, .. } => requirement,
            _ => return,
        };
        let Ok(requirement_index) = usize::try_from(*requirement) else {
            return;
        };
        if let Some(final_id) = remap.get(requirement_index) {
            *requirement = *final_id;
        }
    }

    pub fn runtime_function(&self) -> Option<u64> {
        match self {
            Self::RuntimeNullary { function, .. }
            | Self::RuntimeUnary { function, .. }
            | Self::RuntimeBinary { function, .. }
            | Self::RuntimeTernary { function, .. }
            | Self::RuntimeQuaternary { function, .. }
            | Self::RuntimeVariadic { function, .. } => Some(*function),
            _ => None,
        }
    }

    pub fn requirement<'a>(
        &self,
        requirements: &'a [ApplicationRequirement],
    ) -> Option<&'a ApplicationRequirement> {
        let index = match self {
            Self::HostCall { requirement, .. }
            | Self::ResourceRead { requirement, .. }
            | Self::ResourceWrite { requirement, .. }
            | Self::ResourceSend { requirement, .. } => *requirement,
            _ => return None,
        };
        requirements.get(usize::try_from(index).ok()?)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ImplementationMemoryClass {
    NoAdditionalScratch,
    CloneInput { input: u16 },
    MatrixSolve,
    CanonicalFinalize,
    CanonicalSortUnique,
}

use crate::{
    FloatWidth, FunctionMatrixElement, FunctionValueRepresentation, IntegerWidth, ScalarMemoryKind,
};

use super::{
    MemoryLifetime, MemorySpace, MemoryTargetKind, PhysicalStorageDescriptor, PlannedSlotKind,
    TargetMemoryProfile,
};

/// Adapts the exact R4 runtime representation to the closed R5 physical-slot
/// vocabulary. Semantic topology and dimensions continue to come from the
/// resolved descriptor and are never inferred here.
pub fn physical_storage_descriptor(
    representation: FunctionValueRepresentation,
    target: &TargetMemoryProfile,
    lifetime: MemoryLifetime,
) -> PhysicalStorageDescriptor {
    use FunctionValueRepresentation as Representation;
    let slot = match representation {
        Representation::String => PlannedSlotKind::StringHeader,
        Representation::Matrix {
            element: FunctionMatrixElement::String,
            ..
        } => PlannedSlotKind::StringHeader,
        Representation::Matrix { element, .. } => matrix_slot(element),
        Representation::U8 => fixed(ScalarMemoryKind::Unsigned(IntegerWidth::W8)),
        Representation::U16 => fixed(ScalarMemoryKind::Unsigned(IntegerWidth::W16)),
        Representation::U32 => fixed(ScalarMemoryKind::Unsigned(IntegerWidth::W32)),
        Representation::U64 => fixed(ScalarMemoryKind::Unsigned(IntegerWidth::W64)),
        Representation::U128 => fixed(ScalarMemoryKind::Unsigned(IntegerWidth::W128)),
        Representation::I8 => fixed(ScalarMemoryKind::Signed(IntegerWidth::W8)),
        Representation::I16 => fixed(ScalarMemoryKind::Signed(IntegerWidth::W16)),
        Representation::I32 => fixed(ScalarMemoryKind::Signed(IntegerWidth::W32)),
        Representation::I64 => fixed(ScalarMemoryKind::Signed(IntegerWidth::W64)),
        Representation::I128 => fixed(ScalarMemoryKind::Signed(IntegerWidth::W128)),
        Representation::F32 => fixed(ScalarMemoryKind::Floating(FloatWidth::W32)),
        Representation::F64 => fixed(ScalarMemoryKind::Floating(FloatWidth::W64)),
        Representation::C64 => fixed(ScalarMemoryKind::Complex(FloatWidth::W64)),
        Representation::R64 => fixed(ScalarMemoryKind::Rational64),
        Representation::Bool => fixed(ScalarMemoryKind::Bool),
        Representation::Id => fixed(ScalarMemoryKind::Id),
        Representation::Index => fixed(ScalarMemoryKind::Index),
        Representation::Atom => fixed(ScalarMemoryKind::Atom),
        Representation::AnyValue
        | Representation::Empty
        | Representation::Enum
        | Representation::Record
        | Representation::Map
        | Representation::Set
        | Representation::Table
        | Representation::Tuple
        | Representation::Kind
        | Representation::MutableValueCell => PlannedSlotKind::CanonicalValueHandle,
    };
    PhysicalStorageDescriptor {
        capabilities: crate::runtime_storage::actual_backing_capabilities(representation),
        slot,
        space: match target.kind {
            MemoryTargetKind::ResidentCpu => MemorySpace::ResidentCpu,
            MemoryTargetKind::Gpu => MemorySpace::Device { region: 0 },
            MemoryTargetKind::DirectHost
            | MemoryTargetKind::NativeHost
            | MemoryTargetKind::WasmHost => MemorySpace::Host,
        },
        lifetime,
        reusable_turn_temporary: matches!(lifetime, MemoryLifetime::Turn { .. }),
    }
}

const fn fixed(kind: ScalarMemoryKind) -> PlannedSlotKind {
    PlannedSlotKind::FixedScalar(kind)
}

fn matrix_slot(element: FunctionMatrixElement) -> PlannedSlotKind {
    match element {
        FunctionMatrixElement::Index => fixed(ScalarMemoryKind::Index),
        FunctionMatrixElement::Bool => fixed(ScalarMemoryKind::Bool),
        FunctionMatrixElement::String | FunctionMatrixElement::Value => {
            PlannedSlotKind::CanonicalValueHandle
        }
        FunctionMatrixElement::U8 => fixed(ScalarMemoryKind::Unsigned(IntegerWidth::W8)),
        FunctionMatrixElement::U16 => fixed(ScalarMemoryKind::Unsigned(IntegerWidth::W16)),
        FunctionMatrixElement::U32 => fixed(ScalarMemoryKind::Unsigned(IntegerWidth::W32)),
        FunctionMatrixElement::U64 => fixed(ScalarMemoryKind::Unsigned(IntegerWidth::W64)),
        FunctionMatrixElement::U128 => fixed(ScalarMemoryKind::Unsigned(IntegerWidth::W128)),
        FunctionMatrixElement::I8 => fixed(ScalarMemoryKind::Signed(IntegerWidth::W8)),
        FunctionMatrixElement::I16 => fixed(ScalarMemoryKind::Signed(IntegerWidth::W16)),
        FunctionMatrixElement::I32 => fixed(ScalarMemoryKind::Signed(IntegerWidth::W32)),
        FunctionMatrixElement::I64 => fixed(ScalarMemoryKind::Signed(IntegerWidth::W64)),
        FunctionMatrixElement::I128 => fixed(ScalarMemoryKind::Signed(IntegerWidth::W128)),
        FunctionMatrixElement::F32 => fixed(ScalarMemoryKind::Floating(FloatWidth::W32)),
        FunctionMatrixElement::F64 => fixed(ScalarMemoryKind::Floating(FloatWidth::W64)),
        FunctionMatrixElement::C64 => fixed(ScalarMemoryKind::Complex(FloatWidth::W64)),
        FunctionMatrixElement::R64 => fixed(ScalarMemoryKind::Rational64),
    }
}

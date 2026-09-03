//! Private adapter from transitional runtime representations to actual backing capabilities.

use crate::{
    FloatWidth, FunctionMatrixElement, FunctionMatrixRepresentation, FunctionMatrixStoragePattern,
    FunctionValueRepresentation, IntegerWidth, PositionalAddressingCapability, ScalarMemoryKind,
    StorageAccessCapabilities, StorageAccountingCapability, StorageAddressingCapabilities,
    StorageCanonicalizationCapabilities, StorageCapabilityDescriptor, StorageElementKind,
    StorageExtentCapability, StorageOwnershipCapabilities, StoragePublicationCapabilities,
    StorageTopology,
};

#[cfg(feature = "no_std")]
use alloc::vec;
#[cfg(not(feature = "no_std"))]
use std::vec;

const WHOLE_VALUE: StorageAddressingCapabilities = StorageAddressingCapabilities {
    whole_value: true,
    positional: PositionalAddressingCapability::None,
    named_members: false,
    keyed_members: false,
    arbitrary_regions: false,
};

const NO_CANONICALIZATION: StorageCanonicalizationCapabilities =
    StorageCanonicalizationCapabilities {
        self_describing: false,
        recursive: false,
        tagged: false,
        ordered_keys: false,
        unique_keys: false,
    };

const STANDARD_ACCESS: StorageAccessCapabilities = StorageAccessCapabilities {
    readable: true,
    writable: true,
    replaceable: true,
    region_mutable: false,
    canonical_snapshot: true,
};

const STANDARD_OWNERSHIP: StorageOwnershipCapabilities = StorageOwnershipCapabilities {
    shared_read: true,
    exclusive_write: true,
    owned_value: true,
    detachable: true,
};

const ATOMIC_PUBLICATION: StoragePublicationCapabilities = StoragePublicationCapabilities {
    atomic_replace: true,
    preserves_previous_on_failure: true,
};

fn descriptor(
    topology: StorageTopology,
    extent: StorageExtentCapability,
    addressing: StorageAddressingCapabilities,
    canonicalization: StorageCanonicalizationCapabilities,
    region_mutable: bool,
    accounting: StorageAccountingCapability,
) -> StorageCapabilityDescriptor {
    StorageCapabilityDescriptor {
        topology,
        extent,
        addressing,
        canonicalization,
        access: StorageAccessCapabilities {
            region_mutable,
            ..STANDARD_ACCESS
        },
        ownership: STANDARD_OWNERSHIP,
        publication: ATOMIC_PUBLICATION,
        accounting,
    }
}

fn scalar(kind: ScalarMemoryKind) -> StorageCapabilityDescriptor {
    descriptor(
        StorageTopology::Scalar(kind),
        StorageExtentCapability::Single,
        WHOLE_VALUE,
        NO_CANONICALIZATION,
        false,
        StorageAccountingCapability::FixedScalar,
    )
}

fn matrix_element(element: FunctionMatrixElement) -> Option<ScalarMemoryKind> {
    Some(match element {
        FunctionMatrixElement::Index => ScalarMemoryKind::Index,
        FunctionMatrixElement::Bool => ScalarMemoryKind::Bool,
        FunctionMatrixElement::String => ScalarMemoryKind::String,
        FunctionMatrixElement::U8 => ScalarMemoryKind::Unsigned(IntegerWidth::W8),
        FunctionMatrixElement::U16 => ScalarMemoryKind::Unsigned(IntegerWidth::W16),
        FunctionMatrixElement::U32 => ScalarMemoryKind::Unsigned(IntegerWidth::W32),
        FunctionMatrixElement::U64 => ScalarMemoryKind::Unsigned(IntegerWidth::W64),
        FunctionMatrixElement::U128 => ScalarMemoryKind::Unsigned(IntegerWidth::W128),
        FunctionMatrixElement::I8 => ScalarMemoryKind::Signed(IntegerWidth::W8),
        FunctionMatrixElement::I16 => ScalarMemoryKind::Signed(IntegerWidth::W16),
        FunctionMatrixElement::I32 => ScalarMemoryKind::Signed(IntegerWidth::W32),
        FunctionMatrixElement::I64 => ScalarMemoryKind::Signed(IntegerWidth::W64),
        FunctionMatrixElement::I128 => ScalarMemoryKind::Signed(IntegerWidth::W128),
        FunctionMatrixElement::F32 => ScalarMemoryKind::Floating(FloatWidth::W32),
        FunctionMatrixElement::F64 => ScalarMemoryKind::Floating(FloatWidth::W64),
        FunctionMatrixElement::C64 => ScalarMemoryKind::Complex(FloatWidth::W64),
        FunctionMatrixElement::R64 => ScalarMemoryKind::Rational64,
        FunctionMatrixElement::Value => return None,
    })
}

fn matrix_extent(representation: FunctionMatrixRepresentation) -> StorageExtentCapability {
    use FunctionMatrixRepresentation::*;
    match representation {
        Matrix1 => StorageExtentCapability::FixedDimensions(vec![1, 1].into_boxed_slice()),
        Matrix2 => StorageExtentCapability::FixedDimensions(vec![2, 2].into_boxed_slice()),
        Matrix3 => StorageExtentCapability::FixedDimensions(vec![3, 3].into_boxed_slice()),
        Matrix4 => StorageExtentCapability::FixedDimensions(vec![4, 4].into_boxed_slice()),
        Matrix2x3 => StorageExtentCapability::FixedDimensions(vec![2, 3].into_boxed_slice()),
        Matrix3x2 => StorageExtentCapability::FixedDimensions(vec![3, 2].into_boxed_slice()),
        RowVector2 => StorageExtentCapability::FixedDimensions(vec![1, 2].into_boxed_slice()),
        RowVector3 => StorageExtentCapability::FixedDimensions(vec![1, 3].into_boxed_slice()),
        RowVector4 => StorageExtentCapability::FixedDimensions(vec![1, 4].into_boxed_slice()),
        Vector2 => StorageExtentCapability::FixedDimensions(vec![2, 1].into_boxed_slice()),
        Vector3 => StorageExtentCapability::FixedDimensions(vec![3, 1].into_boxed_slice()),
        Vector4 => StorageExtentCapability::FixedDimensions(vec![4, 1].into_boxed_slice()),
        RowVectorD => {
            StorageExtentCapability::ResizableDimensions(vec![Some(1), None].into_boxed_slice())
        }
        VectorD => {
            StorageExtentCapability::ResizableDimensions(vec![None, Some(1)].into_boxed_slice())
        }
        MatrixD => {
            StorageExtentCapability::ResizableDimensions(vec![None, None].into_boxed_slice())
        }
    }
}

fn matrix(
    element: FunctionMatrixElement,
    representation: FunctionMatrixRepresentation,
) -> StorageCapabilityDescriptor {
    let Some(element) = matrix_element(element) else {
        return opaque();
    };
    descriptor(
        StorageTopology::DenseSequence {
            element: StorageElementKind::Scalar(element),
        },
        matrix_extent(representation),
        StorageAddressingCapabilities {
            positional: PositionalAddressingCapability::Rank(2),
            arbitrary_regions: true,
            ..WHOLE_VALUE
        },
        StorageCanonicalizationCapabilities {
            recursive: true,
            ..NO_CANONICALIZATION
        },
        true,
        StorageAccountingCapability::CanonicalSnapshot,
    )
}

fn opaque() -> StorageCapabilityDescriptor {
    StorageCapabilityDescriptor {
        topology: StorageTopology::Opaque,
        extent: StorageExtentCapability::Any,
        addressing: WHOLE_VALUE,
        canonicalization: NO_CANONICALIZATION,
        access: StorageAccessCapabilities {
            readable: false,
            writable: false,
            replaceable: false,
            region_mutable: false,
            canonical_snapshot: false,
        },
        ownership: StorageOwnershipCapabilities {
            shared_read: false,
            exclusive_write: false,
            owned_value: false,
            detachable: false,
        },
        publication: StoragePublicationCapabilities {
            atomic_replace: false,
            preserves_previous_on_failure: false,
        },
        accounting: StorageAccountingCapability::CanonicalSnapshot,
    }
}

pub(crate) fn actual_backing_capabilities(
    representation: FunctionValueRepresentation,
) -> StorageCapabilityDescriptor {
    use FunctionValueRepresentation::*;
    match representation {
        AnyValue => StorageCapabilityDescriptor {
            topology: StorageTopology::CanonicalValue,
            extent: StorageExtentCapability::Any,
            addressing: StorageAddressingCapabilities {
                whole_value: true,
                positional: PositionalAddressingCapability::AnyRank,
                named_members: true,
                keyed_members: true,
                arbitrary_regions: true,
            },
            canonicalization: StorageCanonicalizationCapabilities {
                self_describing: true,
                recursive: true,
                tagged: true,
                ordered_keys: true,
                unique_keys: true,
            },
            access: StorageAccessCapabilities {
                region_mutable: true,
                ..STANDARD_ACCESS
            },
            ownership: STANDARD_OWNERSHIP,
            publication: ATOMIC_PUBLICATION,
            accounting: StorageAccountingCapability::CanonicalSnapshot,
        },
        U8 => scalar(ScalarMemoryKind::Unsigned(IntegerWidth::W8)),
        U16 => scalar(ScalarMemoryKind::Unsigned(IntegerWidth::W16)),
        U32 => scalar(ScalarMemoryKind::Unsigned(IntegerWidth::W32)),
        U64 => scalar(ScalarMemoryKind::Unsigned(IntegerWidth::W64)),
        U128 => scalar(ScalarMemoryKind::Unsigned(IntegerWidth::W128)),
        I8 => scalar(ScalarMemoryKind::Signed(IntegerWidth::W8)),
        I16 => scalar(ScalarMemoryKind::Signed(IntegerWidth::W16)),
        I32 => scalar(ScalarMemoryKind::Signed(IntegerWidth::W32)),
        I64 => scalar(ScalarMemoryKind::Signed(IntegerWidth::W64)),
        I128 => scalar(ScalarMemoryKind::Signed(IntegerWidth::W128)),
        F32 => scalar(ScalarMemoryKind::Floating(FloatWidth::W32)),
        F64 => scalar(ScalarMemoryKind::Floating(FloatWidth::W64)),
        C64 => scalar(ScalarMemoryKind::Complex(FloatWidth::W64)),
        R64 => scalar(ScalarMemoryKind::Rational64),
        Bool => scalar(ScalarMemoryKind::Bool),
        Id => scalar(ScalarMemoryKind::Id),
        Index => scalar(ScalarMemoryKind::Index),
        Atom => scalar(ScalarMemoryKind::Atom),
        String => descriptor(
            StorageTopology::Scalar(ScalarMemoryKind::String),
            StorageExtentCapability::Single,
            StorageAddressingCapabilities {
                positional: PositionalAddressingCapability::Rank(1),
                arbitrary_regions: true,
                ..WHOLE_VALUE
            },
            NO_CANONICALIZATION,
            true,
            StorageAccountingCapability::CanonicalSnapshot,
        ),
        Matrix {
            element,
            storage: FunctionMatrixStoragePattern::Exact(representation),
        } => matrix(element, representation),
        Matrix {
            storage: FunctionMatrixStoragePattern::AnyStorage,
            ..
        }
        | Empty
        | Enum
        | Record
        | Map
        | Set
        | Table
        | Tuple
        | Kind
        | MutableValueCell => opaque(),
    }
}

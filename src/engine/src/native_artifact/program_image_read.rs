//! Private wire representations for the build-owned descriptor loader.
//! These are not semantic validators and must never become public Deserialize
//! implementations for constructor-private native proofs.
#[cfg(any(test, feature = "pure_text", feature = "map_ops"))]
use super::{NativeMatchAction, NativeMatchTerminator, NativeMatchValue, NativeScalarOp};
#[cfg(test)]
use super::{NativeMatchBlock, NativeMatchOrigin};
#[cfg(any(test, feature = "pure_text", feature = "map_ops"))]
use serde::Deserialize;

#[path = "program_image_canonical_json.rs"]
pub(crate) mod canonical_json;

#[path = "program_image_raw_json.rs"]
#[cfg(any(test, feature = "pure_text", feature = "map_ops"))]
pub(crate) mod raw_json;

#[path = "program_image_plan.rs"]
#[cfg(any(feature = "pure_text", feature = "map_ops"))]
mod plan;
#[cfg(any(feature = "pure_text", feature = "map_ops"))]
pub use plan::{
    decode_build_owned_native_plan_body, decode_build_owned_native_plan_image,
    decode_build_owned_native_plan_value,
};

#[path = "program_image_collections.rs"]
#[cfg(any(feature = "pure_text", feature = "map_ops"))]
mod collections;

#[path = "program_image_iteration.rs"]
#[cfg(any(feature = "pure_text", feature = "map_ops"))]
mod iteration;
#[path = "program_image_library.rs"]
#[cfg(any(feature = "pure_text", feature = "map_ops"))]
pub(crate) mod library;
#[path = "program_image_loops.rs"]
#[cfg(any(feature = "pure_text", feature = "map_ops"))]
mod loops;
#[path = "program_image_nodes.rs"]
#[cfg(any(feature = "pure_text", feature = "map_ops"))]
mod nodes;
#[path = "program_image_values.rs"]
pub(crate) mod values;

#[cfg(any(test, feature = "pure_text", feature = "map_ops"))]
macro_rules! scalar_operations {
    ($($op:ident),+ $(,)?) => {
        #[derive(Deserialize)]
        enum WireOp { $($op),+ }
        impl From<WireOp> for NativeScalarOp {
            fn from(op: WireOp) -> Self { match op { $(WireOp::$op => Self::$op),+ } }
        }
    };
}
#[cfg(any(test, feature = "pure_text", feature = "map_ops"))]
scalar_operations! {
    U64BitAnd, U64BitOr, U64BitXor, U64BitNot, U64ShlWrap, U64ShrWrap, U64Clz, U64Ctz, U64WrappingAdd, U64WrappingSub, U64WrappingMul,
    U8Eq, U8Ne, U8Lt, U8Le, U8Gt, U8Ge, U16Eq, U16Ne, U16Lt, U16Le, U16Gt, U16Ge, U32Eq, U32Ne, U32Lt, U32Le, U32Gt, U32Ge,
    U64Add, U64Sub, U64Mul, U64Div, U64Rem, U64Eq, U64Ne, U64Lt, U64Le, U64Gt, U64Ge,
    BoolAnd, BoolOr, BoolXor, BoolNot, BoolEq, BoolNe,
    I64Add, I64Sub, I64Mul, I64Div, I64Rem, I64Neg, I64Eq, I64Ne, I64Lt, I64Le, I64Gt, I64Ge,
    F64Add, F64Sub, F64Mul, F64Div, F64Neg, F64Sqrt, F64Eq, F64Ne, F64Lt, F64Le, F64Gt, F64Ge,
    U64ToI64, I64ToU64, U64ToF64, I64ToF64, F64ToU64, F64ToI64,
    U8ToU16, U8ToU32, U8ToU64, U16ToU8, U16ToU32, U16ToU64, U32ToU8, U32ToU16, U32ToU64, U64ToU8, U64ToU16, U64ToU32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg(any(test, feature = "pure_text", feature = "map_ops"))]
struct WireOrigin {
    node: u32,
    block: Option<u32>,
    local: Option<u32>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg(any(test, feature = "pure_text", feature = "map_ops"))]
enum WireAction {
    Primitive {
        op: WireOp,
        operands: Box<[u32]>,
        output: u32,
    },
    Copy {
        source: u32,
        output: u32,
    },
    None,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg(any(test, feature = "pure_text", feature = "map_ops"))]
enum WireTerminator {
    Jump(u32),
    Branch {
        condition: u32,
        yes: u32,
        no: u32,
    },
    LiteralBranch {
        left: u32,
        right: u32,
        yes: u32,
        no: u32,
    },
    Complete,
    Reject,
    IntegrityFailure,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg(test)]
struct WireBlock {
    origin: WireOrigin,
    action: WireAction,
    terminator: WireTerminator,
}

/// Reconstruct a block already checked by this build's canonical compiler.
///
/// # Safety
/// The caller must keep the returned metadata confined until it establishes
/// that `image` is the unchanged output of the same pinned compiler/Object
/// build. Execution may use it only with that build's checked indices, operand
/// types and selected operation witness. Parsing establishes wire shape only.
/// Keep this private until the enclosing loader verifies its build association.
#[cfg(test)]
pub(super) unsafe fn decode_control_block(
    image: &serde_json::Value,
) -> Result<NativeMatchBlock, serde_json::Error> {
    let block: WireBlock = serde_json::from_value(image.clone())?;
    Ok(NativeMatchBlock {
        origin: NativeMatchOrigin {
            node: mech_core::NodeId::new(block.origin.node),
            block: block.origin.block,
            local: block.origin.local,
        },
        action: action(block.action),
        terminator: terminator(block.terminator),
    })
}

#[cfg(any(test, feature = "pure_text", feature = "map_ops"))]
fn action(wire: WireAction) -> NativeMatchAction {
    match wire {
        WireAction::Primitive {
            op,
            operands,
            output,
        } => NativeMatchAction::Primitive {
            op: op.into(),
            operands: operands
                .into_vec()
                .into_iter()
                .map(NativeMatchValue)
                .collect(),
            output: NativeMatchValue(output),
        },
        WireAction::Copy { source, output } => NativeMatchAction::Copy {
            source: NativeMatchValue(source),
            output: NativeMatchValue(output),
        },
        WireAction::None => NativeMatchAction::None,
    }
}
#[cfg(any(test, feature = "pure_text", feature = "map_ops"))]
fn terminator(wire: WireTerminator) -> NativeMatchTerminator {
    match wire {
        WireTerminator::Jump(next) => NativeMatchTerminator::Jump(next),
        WireTerminator::Branch { condition, yes, no } => NativeMatchTerminator::Branch {
            condition: NativeMatchValue(condition),
            yes,
            no,
        },
        WireTerminator::LiteralBranch {
            left,
            right,
            yes,
            no,
        } => NativeMatchTerminator::LiteralBranch {
            left: NativeMatchValue(left),
            right: NativeMatchValue(right),
            yes,
            no,
        },
        WireTerminator::Complete => NativeMatchTerminator::Complete,
        WireTerminator::Reject => NativeMatchTerminator::Reject,
        WireTerminator::IntegrityFailure => NativeMatchTerminator::IntegrityFailure,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn private_control_wire_roundtrips_without_a_public_proof_deserializer() {
        let block = NativeMatchBlock {
            origin: NativeMatchOrigin {
                node: mech_core::NodeId::new(2),
                block: Some(1),
                local: Some(0),
            },
            action: NativeMatchAction::Primitive {
                op: NativeScalarOp::U64Add,
                operands: Box::new([NativeMatchValue(0), NativeMatchValue(1)]),
                output: NativeMatchValue(2),
            },
            terminator: NativeMatchTerminator::Complete,
        };
        let image = serde_json::to_value(&block).unwrap();
        // SAFETY: this test owns the block and its unchanged serialized image;
        // it inspects metadata only and never admits or executes synthetic code.
        let loaded = unsafe { decode_control_block(&image) }.unwrap();
        assert_eq!(serde_json::to_value(loaded).unwrap(), image);
    }
    #[test]
    fn narrow_comparisons_round_trip_in_build_owned_images() {
        for op in [
            NativeScalarOp::U8Eq,
            NativeScalarOp::U8Ne,
            NativeScalarOp::U8Lt,
            NativeScalarOp::U8Le,
            NativeScalarOp::U8Gt,
            NativeScalarOp::U8Ge,
            NativeScalarOp::U16Eq,
            NativeScalarOp::U16Ne,
            NativeScalarOp::U16Lt,
            NativeScalarOp::U16Le,
            NativeScalarOp::U16Gt,
            NativeScalarOp::U16Ge,
            NativeScalarOp::U32Eq,
            NativeScalarOp::U32Ne,
            NativeScalarOp::U32Lt,
            NativeScalarOp::U32Le,
            NativeScalarOp::U32Gt,
            NativeScalarOp::U32Ge,
        ] {
            let encoded = serde_json::to_value(op).unwrap();
            let decoded: WireOp = serde_json::from_value(encoded).unwrap();
            assert_eq!(NativeScalarOp::from(decoded), op);
        }
    }
    #[test]
    fn wire_rejects_unknown_fields_and_opcodes() {
        let valid = serde_json::json!({"origin":{"node":0,"block":null,"local":null},
            "action":"None","terminator":"Complete"});
        let mut extra = valid.clone();
        extra["authority"] = true.into();
        let mut opcode = valid;
        opcode["action"] =
            serde_json::json!({"Primitive":{"op":"AmbientRead","operands":[],"output":0}});
        for image in [extra, opcode] {
            // SAFETY: deliberately malformed, inert test data must fail parsing
            // before any native proof is returned or any execution is attempted.
            assert!(unsafe { decode_control_block(&image) }.is_err());
        }
    }
}

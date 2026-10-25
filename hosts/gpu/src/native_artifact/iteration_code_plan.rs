//! Compiler-only view over the SAME canonical scalar/mixed loop CFG.
use super::NativeCompileError;
use mech_engine::native_artifact::{
    NativeControlLoopRef, NativeIterationArtifactPlan, NativeIterationBlock, NativeMatchValue,
    NativeScalarKind, NativeScalarValue,
};

#[derive(Clone, Copy)]
pub(super) enum CodePlan<'a> {
    Scalar(&'a NativeIterationArtifactPlan),
    #[cfg(feature = "canonical-native-program")]
    Program(&'a mech_engine::native_artifact::NativeProgramArtifactPlan),
}
impl<'a> CodePlan<'a> {
    pub fn blocks(self) -> &'a [NativeIterationBlock] {
        match self {
            Self::Scalar(p) => p.blocks(),
            #[cfg(feature = "canonical-native-program")]
            Self::Program(p) => p.blocks(),
        }
    }
    pub fn iteration_count(self) -> usize {
        match self {
            Self::Scalar(p) => p.iterations().len(),
            #[cfg(feature = "canonical-native-program")]
            Self::Program(p) => p.iterations().len(),
        }
    }
    pub fn iteration(self, index: usize) -> NativeControlLoopRef<'a> {
        match self {
            Self::Scalar(p) => (&p.iterations()[index]).into(),
            #[cfg(feature = "canonical-native-program")]
            Self::Program(p) => p.iterations()[index].as_ref(),
        }
    }
    pub fn is_program(self) -> bool {
        match self {
            Self::Scalar(_) => false,
            #[cfg(feature = "canonical-native-program")]
            Self::Program(_) => true,
        }
    }
    pub fn entry(self) -> u32 {
        match self {
            Self::Scalar(p) => p.entry(),
            #[cfg(feature = "canonical-native-program")]
            Self::Program(p) => p.entry(),
        }
    }
    pub fn activation_boundary(self) -> Option<u32> {
        match self {
            Self::Scalar(_) => None,
            #[cfg(feature = "canonical-native-program")]
            Self::Program(plan) => plan.activation_boundary(),
        }
    }
    pub fn value_count(self) -> usize {
        match self {
            Self::Scalar(p) => p.values().len(),
            #[cfg(feature = "canonical-native-program")]
            Self::Program(p) => p.values().len(),
        }
    }
    pub fn input_count(self) -> usize {
        match self {
            Self::Scalar(p) => p.inputs().len(),
            #[cfg(feature = "canonical-native-program")]
            Self::Program(p) => p.transport_input_count(),
        }
    }
    pub fn output_count(self) -> usize {
        match self {
            Self::Scalar(p) => p.outputs().len(),
            #[cfg(feature = "canonical-native-program")]
            Self::Program(p) => p.transport_output_count(),
        }
    }
    pub fn constant_count(self) -> usize {
        match self {
            Self::Scalar(p) => p.constants().len(),
            #[cfg(feature = "canonical-native-program")]
            Self::Program(p) => p.constants().len(),
        }
    }
    pub fn kind(self, value: NativeMatchValue) -> Option<NativeScalarKind> {
        match self {
            Self::Scalar(p) => p.values().get(value.get() as usize).map(|ty| ty.kind()),
            #[cfg(feature = "canonical-native-program")]
            Self::Program(p) => p
                .values()
                .get(value.get() as usize)
                .and_then(|ty| ty.scalar_kind()),
        }
    }
    pub fn input(self, index: usize) -> NativeMatchValue {
        match self {
            Self::Scalar(p) => p.inputs()[index].value(),
            #[cfg(feature = "canonical-native-program")]
            Self::Program(p) => p.transport_input_value(index),
        }
    }
    #[cfg(feature = "canonical-native-program")]
    pub fn output(self, index: usize) -> NativeMatchValue {
        match self {
            Self::Scalar(p) => p.outputs()[index].value(),
            Self::Program(p) => p.transport_output_value(index),
        }
    }
    pub fn constant(
        self,
        index: usize,
    ) -> Result<(NativeMatchValue, Option<u64>), NativeCompileError> {
        let (value, scalar) = match self {
            Self::Scalar(p) => (
                p.constants()[index].value(),
                Some(p.constants()[index].scalar()),
            ),
            #[cfg(feature = "canonical-native-program")]
            Self::Program(p) => {
                let c = &p.constants()[index];
                let scalar = match (self.kind(c.value()), c.constant().data()) {
                    (None, _) => None,
                    (Some(NativeScalarKind::Bool), mech_core::ValueData::Bool(v)) => {
                        Some(NativeScalarValue::Bool(*v))
                    }
                    (Some(NativeScalarKind::U64), mech_core::ValueData::U64(v)) => {
                        Some(NativeScalarValue::U64(*v))
                    }
                    (Some(NativeScalarKind::I64), mech_core::ValueData::I64(v)) => {
                        Some(NativeScalarValue::I64(*v))
                    }
                    (Some(NativeScalarKind::F64), mech_core::ValueData::F64(v)) => {
                        Some(NativeScalarValue::F64(*v))
                    }
                    _ => {
                        return Err(NativeCompileError::InvalidPlan {
                            node: None,
                            reason: "constant scalar domain mismatch",
                        });
                    }
                };
                (c.value(), scalar)
            }
        };
        Ok((
            value,
            scalar.map(|s| match s {
                NativeScalarValue::Bool(v) => u64::from(v),
                NativeScalarValue::U64(v) => v,
                NativeScalarValue::I64(v) => v as u64,
                NativeScalarValue::F64(v) => v.bits(),
            }),
        ))
    }
    pub fn validate(self) -> Result<(), NativeCompileError> {
        match self {
            Self::Scalar(p) => super::iteration_compiler::validate(p),
            #[cfg(feature = "canonical-native-program")]
            Self::Program(p) => p
                .validate_current_visits()
                .map_err(|reason| NativeCompileError::InvalidPlan { node: None, reason }),
        }?;
        for block in self.blocks() {
            if let mech_engine::native_artifact::NativeIterationAction::Scalar(
                mech_engine::native_artifact::NativeMatchAction::Primitive {
                    op,
                    operands,
                    output,
                    ..
                },
            ) = block.action()
            {
                let (args, result) = op.signature();
                if self.kind(*output) != Some(result)
                    || operands.len() != args.len()
                    || !operands
                        .iter()
                        .zip(args)
                        .all(|(&value, &kind)| self.kind(value) == Some(kind))
                {
                    return Err(NativeCompileError::InvalidPlan {
                        node: Some(block.origin().node()),
                        reason: "mixed primitive scalar domain mismatch",
                    });
                }
            }
        }
        Ok(())
    }
}

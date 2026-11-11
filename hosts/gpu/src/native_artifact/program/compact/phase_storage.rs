//! Invocation-local argument/result space for synchronous numeric phase calls.
//! A call consumes current inputs and finishes before its caller loads results;
//! neither packet address is a source value or retained by the array helpers.
use crate::native_artifact::{NativeCompileError, compiler::backend};
use cranelift_codegen::ir::{StackSlot, StackSlotData, StackSlotKind};
use cranelift_frontend::FunctionBuilder;

#[derive(Clone, Copy)]
pub(super) struct Geometry {
    calls: usize,
    input_words: usize,
    output_words: usize,
    separate_bytes: usize,
}

impl Geometry {
    pub fn derive(
        requirements: impl IntoIterator<Item = (usize, usize)>,
    ) -> Result<Option<Self>, NativeCompileError> {
        let mut geometry = Self {
            calls: 0,
            input_words: 0,
            output_words: 0,
            separate_bytes: 0,
        };
        for (input, output) in requirements {
            geometry.calls += 1;
            geometry.input_words = geometry.input_words.max(input);
            geometry.output_words = geometry.output_words.max(output);
            let bytes = input
                .checked_add(output)
                .and_then(|n| n.checked_mul(8))
                .ok_or_else(|| backend("native phase packet geometry overflows"))?;
            geometry.separate_bytes = geometry
                .separate_bytes
                .checked_add(bytes)
                .ok_or_else(|| backend("native phase packet total overflows"))?;
        }
        Ok((geometry.calls != 0).then_some(geometry))
    }

    pub fn reserve(self, b: &mut FunctionBuilder<'_>) -> Result<Slots, NativeCompileError> {
        // Distinct nonempty slots also preserve disjoint addresses for an empty
        // input signature. No contents are assumed initialized by this reserve.
        let mut allocate =
            |words: usize| {
                let bytes = words
                    .max(1)
                    .checked_mul(8)
                    .and_then(|n| u32::try_from(n).ok())
                    .ok_or_else(|| backend("native phase packet exceeds stack-slot extent"))?;
                Ok(b.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    bytes,
                    3,
                )))
            };
        Ok(Slots {
            input: allocate(self.input_words)?,
            output: allocate(self.output_words)?,
            geometry: self,
        })
    }

    pub fn summary(self) -> serde_json::Value {
        serde_json::json!({"calls":self.calls,"slots":2,"input_bytes":self.input_words.max(1)*8,
            "output_bytes":self.output_words.max(1)*8,"previous_separate_bytes":self.separate_bytes,
            "invocation_local":true,"input_output_disjoint":true,"synchronous_non_escaping":true})
    }
}

pub(super) struct Slots {
    input: StackSlot,
    output: StackSlot,
    geometry: Geometry,
}

impl Slots {
    pub fn for_call(
        &self,
        input: usize,
        output: usize,
    ) -> Result<(StackSlot, StackSlot), NativeCompileError> {
        if input > self.geometry.input_words || output > self.geometry.output_words {
            return Err(backend(
                "native phase packet exceeds its prepared shared extent",
            ));
        }
        Ok((self.input, self.output))
    }
}

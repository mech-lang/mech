//! Descriptive source selection, shared geometry and full source cost bounds.
//! Neither an opcode nor an estimate grants a view, invocation or publication.
use super::NativePreparedLibraryCall;
use crate::native_artifact::{
    NativeCollectionPrimitive as P, NativeProgramLibraryTemplate, NativeScalarOp,
};
use mech_core::{ResourceDemand, Schema, SchemaBody, WorkDemand};

fn f64(schema: &Schema) -> bool {
    match schema.body() {
        SchemaBody::FloatingPoint(mech_core::FloatWidth::W64) => {
            schema.dimension_parameters().is_empty()
        }
        SchemaBody::Matrix {
            element,
            dimensions,
        } => {
            dimensions.len() == 2
                && element.as_ref() == &SchemaBody::FloatingPoint(mech_core::FloatWidth::W64)
        }
        _ => false,
    }
}
impl NativeProgramLibraryTemplate {
    /// The maintained matrix-product factory, with two typed matrix sources.
    /// Scalar dot results and other numeric kinds retain their own routes.
    pub fn f64_matrix_product(&self) -> bool {
        self.primitive() == P::MatrixMatmul
            && self.inputs().len() == 2
            && self
                .inputs()
                .iter()
                .chain([self.output()])
                .all(|s| f64(s) && matches!(s.body(), SchemaBody::Matrix { .. }))
    }

    /// A private registration witness attached to the maintained factory.
    /// Current dimensions, source values and complete admission remain separate.
    pub fn f64_matrix_binary_op(&self) -> Option<NativeScalarOp> {
        let op = match self.primitive() {
            P::MatrixAdd => NativeScalarOp::F64Add,
            P::MatrixSubtract => NativeScalarOp::F64Sub,
            P::MatrixMultiply => NativeScalarOp::F64Mul,
            P::MatrixDivide => NativeScalarOp::F64Div,
            _ => return None,
        };
        (self.inputs().len() == 2
            && self.inputs().iter().all(f64)
            && f64(self.output())
            && matches!(self.output().body(), SchemaBody::Matrix { .. })
            && self
                .inputs()
                .iter()
                .any(|s| matches!(s.body(), SchemaBody::Matrix { .. })))
        .then_some(op)
    }
}
impl NativePreparedLibraryCall {
    pub fn f64_matrix_product_geometry(
        &self,
        left: [usize; 2],
        right: [usize; 2],
    ) -> Option<[u64; 3]> {
        if !self.template().f64_matrix_product() {
            return None;
        }
        crate::resident::numeric::native_product_geometry(left, right)
            .ok()
            .map(|axes| axes.map(|n| n as u64))
    }
    pub fn f64_matrix_product_source_bound(
        &self,
        inputs: u64,
        output: u64,
        inner: u64,
    ) -> Option<(ResourceDemand, u64)> {
        if !self.template().f64_matrix_product() {
            return None;
        }
        // The ordinary F64 product charges two arithmetic operations per
        // term. Fund those and the complete emitted load/store/loop work;
        // the common envelope also retains all original planning/publication.
        self.f64_matrix_source_bound(inputs, output, output.checked_mul(inner)?)
    }

    pub fn f64_matrix_binary_geometry(
        &self,
        left: [usize; 2],
        right: [usize; 2],
    ) -> Option<[u64; 4]> {
        self.template().f64_matrix_binary_op()?;
        let [rows, columns, mut left_mode, mut right_mode] =
            crate::resident::numeric::native_binary_geometry(
                (left[0], left[1]),
                (right[0], right[1]),
            )
            .ok()?;
        // Logical geometry prefers exact before scalar for a 1x1 result.
        // A true scalar schema is physically an inline word even in that case;
        // a 1x1 matrix still owns a typed source bank.
        for ((schema, dimensions), mode) in self
            .template()
            .inputs()
            .iter()
            .zip([left, right])
            .zip([&mut left_mode, &mut right_mode])
        {
            if matches!(schema.body(), SchemaBody::FloatingPoint(_)) {
                if dimensions != [1, 1] {
                    return None;
                }
                *mode = 0;
            }
        }
        Some([rows, columns, left_mode, right_mode])
    }
    /// Complete original metadata, draft/canonical construction, current source
    /// copies, physical staging and publication. This retains the old source
    /// prices even though generated arithmetic needs only one typed result bank.
    /// All arithmetic is checked; overflow declines the optional native route.
    pub fn f64_matrix_binary_source_bound(
        &self,
        inputs: u64,
        output: u64,
    ) -> Option<(ResourceDemand, u64)> {
        self.template().f64_matrix_binary_op()?;
        self.f64_matrix_source_bound(inputs, output, 0)
    }
    fn f64_matrix_source_bound(
        &self,
        inputs: u64,
        output: u64,
        terms: u64,
    ) -> Option<(ResourceDemand, u64)> {
        let schema = self.schema_context_allocation_bound_bytes()?;
        let binding = self.binding_metadata_bytes();
        let cell = core::mem::size_of::<mech_core::ValueDataDraft>() as u64;
        let cells = inputs
            .checked_add(output)?
            .checked_add(8)?
            .checked_mul(cell.checked_mul(16)?.checked_add(1024)?)?;
        let temporary = cells
            .checked_add(binding.checked_mul(128)?)?
            .checked_add(schema.checked_mul(32)?)?
            .checked_add(65536)?;
        let published = output
            .checked_add(1)?
            .checked_mul(cell.checked_mul(8)?.checked_add(256)?)?
            .checked_add(binding)?
            .checked_add(schema)?
            .checked_add(4096)?;
        let compute = self
            .binding_work()
            .checked_mul(128)?
            .checked_add(temporary.checked_mul(16)?)?
            .checked_add(published.checked_mul(16)?)?
            .checked_add(inputs.checked_add(output)?.checked_mul(1024)?)?
            .checked_add(terms.checked_mul(16)?)?
            .checked_add(65536)?;
        let comparison = published.checked_add(temporary)?;
        let nodes = inputs
            .checked_add(output)?
            .checked_mul(8)?
            .checked_add(binding)?
            .checked_add(schema)?
            .checked_add(256)?;
        let replay = compute
            .checked_mul(128)?
            .checked_add(temporary.checked_mul(128)?)?
            .checked_add(schema.checked_mul(128)?)?
            .checked_add(65536)?;
        Some((
            ResourceDemand {
                persistent_bytes: published,
                turn_peak_bytes: temporary,
                cloned_bytes: temporary,
                retained_nodes: nodes,
                output_elements: output,
                work: WorkDemand {
                    compute,
                    comparison,
                    ..Default::default()
                },
                ..Default::default()
            },
            replay,
        ))
    }
}

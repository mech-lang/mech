//! A pointwise source closure feeds each original ordered sum term directly.
//! Reuse the ordinary reduction traversal; no producer bank or runtime dispatch.
use super::{
    numeric_product_epilogues::Epilogue,
    numeric_sum::{self, Terms},
};
use cranelift_codegen::ir::{Block, Value};
use cranelift_frontend::FunctionBuilder;
impl Terms for Epilogue<'_> {
    fn emit(
        &self,
        b: &mut FunctionBuilder<'_>,
        row: Value,
        column: Value,
        index: Value,
        vector: bool,
    ) -> Value {
        self.term(b, row, column, index, vector)
    }
}
pub(super) fn emit(
    b: &mut FunctionBuilder<'_>,
    output: Value,
    count: Value,
    inner: Value,
    axis: usize,
    terms: &Epilogue<'_>,
    done: Block,
) {
    numeric_sum::emit_terms(b, output, count, inner, axis, terms, done);
}

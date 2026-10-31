//! Ordered field transport, not source arithmetic or control evaluation.
//! Reuse the same arena/owners and each original admission/retirement in order.
use super::*;
impl Invocation<'_, '_, '_> {
    pub(super) fn project_frame_batch(
        &mut self,
        site: usize,
        args: [u64; 3],
    ) -> Result<Option<u64>, Error> {
        let descriptor = self.descriptor;
        let metadata = descriptor.metadata();
        let batch = metadata
            .projections
            .entry(site)
            .ok_or_else(|| invalid("projection batch outside its prepared entry"))?;
        if args != [0; 3] || self.state.consumed || self.state.scalar_path.is_some() {
            return Err(invalid("duplicate or malformed projection batch"));
        }
        if self.worker.is_some() || self.has_uninventoried_private_owners() {
            return Ok(Some(0));
        }
        // Static transport rows have one source, one entry and only normal
        // progress. The existing visit method keeps all boundary checks and
        // original debit/error order. No operation stream is interpreted.
        for (index, &field_site) in batch.sites.iter().enumerate() {
            if index != 0 {
                self.visit(u64::from(field_site))?;
            }
            // The original action owns schema checks, publication, temporary
            // owner drops and lexical retirement, at unchanged prices.
            if self.buffer_frame_action(
                field_site as usize,
                GlobalHelper::StructuralObservation,
                &[],
            )? != Some(None)
            {
                return Err(invalid("projection batch lost its managed source action"));
            }
            self.state.consumed = true;
        }
        Ok(Some(1))
    }
}

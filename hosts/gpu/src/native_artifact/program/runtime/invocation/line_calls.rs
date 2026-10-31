//! Synchronous invocation orchestration for this owner.
use super::*;
impl Invocation<'_, '_, '_> {
    pub(in crate::native_artifact::program::runtime) fn line_matrix(
        &self,
        value: NativeMatchValue,
    ) -> Option<&Rc<NativeTextLineMatrix>> {
        let ordinal = self
            .descriptor
            .metadata()
            .line_matrices
            .matrices
            .get(value.get() as usize)
            .copied()
            .flatten()?;
        self.state.line_matrices.views.get(ordinal)?.as_ref()
    }
    pub(in crate::native_artifact::program::runtime) fn line_matrix_coexistence(
        &mut self,
        skip: Option<NativeMatchValue>,
    ) -> Result<ResourceDemand, Error> {
        if self.state.line_matrices.views.is_empty() {
            return Ok(ResourceDemand::default());
        }
        self.state.control.work.charge(
            NativeExecutionPhase::Setup,
            amount(self.state.line_matrices.views.len(), 256, 256)?,
        )?;
        let ordinal = skip.and_then(|value| {
            self.descriptor.metadata().line_matrices.matrices[value.get() as usize]
        });
        self.state
            .line_matrices
            .retained_demand(ordinal, &self.state.library.memory_budget())
    }
    pub(in crate::native_artifact::program::runtime) fn line_matrix_call(
        &mut self,
        site: usize,
        words: &[u64],
    ) -> Result<Option<Option<u64>>, Error> {
        let descriptor = self.descriptor;
        let Some(role) = descriptor.metadata().line_matrices.sites[site] else {
            return Ok(None);
        };
        if words != [0] {
            return Err(invalid("line matrix managed operand lane differs"));
        }
        self.state
            .control
            .work
            .charge(NativeExecutionPhase::Setup, 512)?;
        let ordinal = match role {
            Site::Produce(index) | Site::Length(index) => index,
        };
        if let Some(captured) = &mut self.state.line_matrices.captured
            && captured.ordinal == ordinal
            && self.prefix.as_ref().is_some_and(|prefix| !prefix.input.eof)
        {
            // The original visits still happen. Their source-dependent
            // admissions and numeric result remain pending until real EOF.
            // Generated code never reads/stores a count in this phase.
            match role {
                Site::Produce(_) if !captured.produced => captured.produced = true,
                Site::Length(_) if captured.produced && captured.length.is_none() => {
                    captured.length = Some(site);
                }
                _ => return Err(invalid("repeated or unordered captured line producer")),
            }
            return Ok(Some(None));
        }
        let region = &descriptor.metadata().line_matrices.regions[ordinal];
        let call = descriptor.metadata().calls[site]
            .as_ref()
            .ok_or_else(|| invalid("line matrix selected call absent"))?;
        let prior = self.selected_join_coexistence(Some(region.matrix()))?;
        let failure = |error| Error::Library {
            origin: descriptor.metadata().plan.blocks()[site].origin(),
            error,
        };
        match role {
            Site::Produce(_) => {
                if self.state.line_matrices.views[ordinal].is_some() {
                    return Err(invalid("line matrix producer repeated"));
                }
                let source = self
                    .state
                    .arena
                    .copy_current(region.source().get(), &mut self.state.control.work)?;
                match call
                    .text_line_matrix_or_canonical(&source, self.limits, &self.state.library, prior)
                    .map_err(failure)?
                {
                    NativeTextLineMatrixChoice::View(view) => {
                        if !view.is_ascii() {
                            return Err(invalid("native line view lacks byte identity"));
                        }
                        self.state
                            .arena
                            .retire_current(region.matrix().get(), &mut self.state.control.work)?;
                        self.state.line_matrices.views[ordinal] = Some(Rc::new(view));
                    }
                    NativeTextLineMatrixChoice::Canonical(value) => {
                        self.state.arena.put(
                            region.matrix().get(),
                            &value,
                            &mut self.state.control.work,
                        )?;
                    }
                }
                #[cfg(all(test, feature = "canonical-native-program"))]
                tests::observe(self, ordinal);
                Ok(Some(None))
            }
            Site::Length(_) => {
                let Some(view) = self.state.line_matrices.views[ordinal].as_ref() else {
                    return Ok(None);
                };
                let count = call
                    .text_line_matrix_length(view, self.limits, &self.state.library, prior)
                    .map_err(failure)?;
                Ok(Some(Some(count)))
            }
        }
    }
}

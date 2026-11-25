//! Owns one adjacent loop-threading and identity-forwarding sequence.
use super::{identity_routes, optimize_ordered, threaded_edges};
use crate::native_artifact::{
    NativeCompileError,
    codegen::options::{CompilationControls, OptimizationPass},
    compiler::backend,
};
use cranelift_codegen::{Context, ir::Block, isa::TargetIsa};

#[derive(Clone, Copy)]
pub(in crate::native_artifact) enum RouteCleanup {
    Ordered,
    Structural { combined_verification: bool },
}

#[derive(Clone, Copy, Default)]
pub(in crate::native_artifact) struct IdentityRouteOptions<'a> {
    origins: Option<&'a [Block]>,
    equate_transports: bool,
}

impl<'a> IdentityRouteOptions<'a> {
    pub fn new(origins: Option<&'a [Block]>, equate_transports: bool) -> Self {
        Self {
            origins,
            equate_transports,
        }
    }
}

pub(in crate::native_artifact) struct RouteSequence<'a> {
    controls: &'a CompilationControls,
    context: &'a mut Context,
    isa: &'a dyn TargetIsa,
    cleanup: RouteCleanup,
}

impl<'a> RouteSequence<'a> {
    pub fn new(
        controls: &'a CompilationControls,
        context: &'a mut Context,
        isa: &'a dyn TargetIsa,
        cleanup: RouteCleanup,
    ) -> Self {
        Self {
            controls,
            context,
            isa,
            cleanup,
        }
    }

    pub fn thread_loops(&mut self) -> Result<threaded_edges::Report, NativeCompileError> {
        if !self.controls.enabled(OptimizationPass::CfgCleanup)
            || !self.controls.enabled(OptimizationPass::LoopEdgeThreading)
        {
            return Ok(threaded_edges::Report::default());
        }
        self.context.func.dfg.resolve_all_aliases();
        self.build_graphs();
        let report = threaded_edges::optimize(
            self.controls.inputs(),
            &mut self.context.func,
            &self.context.cfg,
            &self.context.domtree,
        );
        self.controls.record_route_work(|work| {
            work.loop_searches += 1;
            work.cfg_requests += 1;
            work.cfg_builds += 1;
            work.dom_requests += 1;
            work.dom_builds += 1;
            work.loop_edges_rewritten += report.threaded_edges;
        });
        if report.threaded_edges != 0 {
            self.finish_route()?;
        }
        Ok(report)
    }

    pub fn forward_identity(
        &mut self,
        options: IdentityRouteOptions<'_>,
    ) -> Result<identity_routes::Report, NativeCompileError> {
        if !self.controls.enabled(OptimizationPass::CfgCleanup)
            || !self.controls.enabled(OptimizationPass::IdentityRoutes)
        {
            return Ok(identity_routes::Report::default());
        }
        self.context.func.dfg.resolve_all_aliases();
        self.build_graphs();
        let report = identity_routes::optimize(
            &mut self.context.func,
            &self.context.cfg,
            &self.context.domtree,
            options.origins,
            options.equate_transports,
        );
        self.controls.record_route_work(|work| {
            work.identity_searches += 1;
            work.cfg_requests += 1;
            work.cfg_builds += 1;
            work.dom_requests += 1;
            work.dom_builds += 1;
            work.identity_edges_rewritten += report.threaded_edges;
        });
        if report.threaded_edges != 0 {
            self.finish_route()?;
        }
        Ok(report)
    }

    fn build_graphs(&mut self) {
        self.context.compute_cfg();
        self.context.compute_domtree();
    }

    fn finish_route(&mut self) -> Result<(), NativeCompileError> {
        match self.cleanup {
            RouteCleanup::Ordered => {
                optimize_ordered(self.controls, self.context, self.isa, true)?;
                self.context.func.dfg.resolve_all_aliases();
                self.context.verify(self.isa).map_err(backend)?;
            }
            RouteCleanup::Structural {
                combined_verification: true,
            } => {
                self.context
                    .compute_flowgraph_cleanup_and_verify(self.isa)
                    .map_err(backend)?;
            }
            RouteCleanup::Structural {
                combined_verification: false,
            } => {
                self.context
                    .compute_flowgraph_and_cleanup(self.isa)
                    .map_err(backend)?;
                self.context.verify(self.isa).map_err(backend)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "route_sequence_tests.rs"]
mod tests;

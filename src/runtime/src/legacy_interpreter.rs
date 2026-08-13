//! Temporary developer-only access to the pre-resident interpreter.
//!
//! Shipping products must use the production resident loaders instead. This
//! facade exists only to keep migration tests and full-development tools
//! usable until E1 removes the old executor.

use mech_core::{MResult, MechSourceCode, Program};

use crate::{MechRuntime, ModuleBuildOptions, RuntimeContext, RuntimeValueSnapshot, SourceRequest};

/// Explicit, temporary authority to execute through the legacy interpreter.
pub struct LegacyInterpreter<'runtime> {
    runtime: &'runtime mut MechRuntime,
}

impl MechRuntime {
    /// Enter the temporary developer-only interpreter facade.
    pub fn legacy_interpreter(&mut self) -> LegacyInterpreter<'_> {
        LegacyInterpreter { runtime: self }
    }
}

impl LegacyInterpreter<'_> {
    pub fn run_string(&mut self, source: &str) -> MResult<RuntimeValueSnapshot> {
        self.runtime.run_string(source)
    }

    pub fn run_string_with_context(
        &mut self,
        context: &mut RuntimeContext,
        source: &str,
    ) -> MResult<RuntimeValueSnapshot> {
        self.runtime.run_string_with_context(context, source)
    }

    pub fn run_source_with_context(
        &mut self,
        context: &mut RuntimeContext,
        source: &MechSourceCode,
    ) -> MResult<RuntimeValueSnapshot> {
        self.runtime.run_source_with_context(context, source)
    }

    pub fn run_tree(&mut self, tree: &Program) -> MResult<RuntimeValueSnapshot> {
        self.runtime.run_tree(tree)
    }

    pub fn run_tree_with_context(
        &mut self,
        context: &mut RuntimeContext,
        tree: &Program,
    ) -> MResult<RuntimeValueSnapshot> {
        self.runtime.run_tree_with_context(context, tree)
    }

    pub fn resolve_and_run_root_module(
        &mut self,
        request: impl Into<SourceRequest>,
        options: ModuleBuildOptions<'_>,
    ) -> MResult<RuntimeValueSnapshot> {
        self.runtime.resolve_and_run_root_module(request, options)
    }

    pub fn resolve_and_run_root_module_with_context(
        &mut self,
        context: &mut RuntimeContext,
        request: impl Into<SourceRequest>,
        options: ModuleBuildOptions<'_>,
    ) -> MResult<RuntimeValueSnapshot> {
        self.runtime
            .resolve_and_run_root_module_with_context(context, request, options)
    }

    #[cfg(feature = "invariant_define")]
    pub fn resolve_and_run_root_module_report(
        &mut self,
        request: impl Into<SourceRequest>,
        options: ModuleBuildOptions<'_>,
    ) -> MResult<crate::RuntimeRootModuleExecutionReport> {
        self.runtime
            .resolve_and_run_root_module_report(request, options)
    }

    pub fn install_bytecode_with_context(
        &mut self,
        context: &mut RuntimeContext,
        bytecode: &[u8],
    ) -> MResult<RuntimeValueSnapshot> {
        self.runtime
            .install_bytecode_with_context(context, bytecode)
    }
}

/// Compatibility syntax for migration-only tests, examples, and benchmarks.
///
/// New developer tools must use [`MechRuntime::legacy_interpreter`] visibly.
#[doc(hidden)]
pub trait LegacyInterpreterTestExt {
    fn run_string(&mut self, source: &str) -> MResult<RuntimeValueSnapshot>;
    fn run_string_with_context(
        &mut self,
        context: &mut RuntimeContext,
        source: &str,
    ) -> MResult<RuntimeValueSnapshot>;
    fn run_source_with_context(
        &mut self,
        context: &mut RuntimeContext,
        source: &MechSourceCode,
    ) -> MResult<RuntimeValueSnapshot>;
    fn run_source(&mut self, source: &MechSourceCode) -> MResult<RuntimeValueSnapshot>;
    fn run_tree(&mut self, tree: &Program) -> MResult<RuntimeValueSnapshot>;
    fn run_tree_with_context(
        &mut self,
        context: &mut RuntimeContext,
        tree: &Program,
    ) -> MResult<RuntimeValueSnapshot>;
    fn resolve_and_run_root_module(
        &mut self,
        request: impl Into<SourceRequest>,
        options: ModuleBuildOptions<'_>,
    ) -> MResult<RuntimeValueSnapshot>;
    fn resolve_and_run_root_module_with_context(
        &mut self,
        context: &mut RuntimeContext,
        request: impl Into<SourceRequest>,
        options: ModuleBuildOptions<'_>,
    ) -> MResult<RuntimeValueSnapshot>;
    #[cfg(feature = "invariant_define")]
    fn resolve_and_run_root_module_report(
        &mut self,
        request: impl Into<SourceRequest>,
        options: ModuleBuildOptions<'_>,
    ) -> MResult<crate::RuntimeRootModuleExecutionReport>;
    fn install_bytecode_with_context(
        &mut self,
        context: &mut RuntimeContext,
        bytecode: &[u8],
    ) -> MResult<RuntimeValueSnapshot>;
}

impl LegacyInterpreterTestExt for MechRuntime {
    fn run_string(&mut self, source: &str) -> MResult<RuntimeValueSnapshot> {
        self.legacy_interpreter().run_string(source)
    }

    fn run_string_with_context(
        &mut self,
        context: &mut RuntimeContext,
        source: &str,
    ) -> MResult<RuntimeValueSnapshot> {
        self.legacy_interpreter()
            .run_string_with_context(context, source)
    }

    fn run_source_with_context(
        &mut self,
        context: &mut RuntimeContext,
        source: &MechSourceCode,
    ) -> MResult<RuntimeValueSnapshot> {
        self.legacy_interpreter()
            .run_source_with_context(context, source)
    }

    fn run_source(&mut self, source: &MechSourceCode) -> MResult<RuntimeValueSnapshot> {
        let mut context = self.runtime_context()?;
        self.legacy_interpreter()
            .run_source_with_context(&mut context, source)
    }

    fn run_tree(&mut self, tree: &Program) -> MResult<RuntimeValueSnapshot> {
        self.legacy_interpreter().run_tree(tree)
    }

    fn run_tree_with_context(
        &mut self,
        context: &mut RuntimeContext,
        tree: &Program,
    ) -> MResult<RuntimeValueSnapshot> {
        self.legacy_interpreter()
            .run_tree_with_context(context, tree)
    }

    fn resolve_and_run_root_module(
        &mut self,
        request: impl Into<SourceRequest>,
        options: ModuleBuildOptions<'_>,
    ) -> MResult<RuntimeValueSnapshot> {
        self.legacy_interpreter()
            .resolve_and_run_root_module(request, options)
    }

    fn resolve_and_run_root_module_with_context(
        &mut self,
        context: &mut RuntimeContext,
        request: impl Into<SourceRequest>,
        options: ModuleBuildOptions<'_>,
    ) -> MResult<RuntimeValueSnapshot> {
        self.legacy_interpreter()
            .resolve_and_run_root_module_with_context(context, request, options)
    }

    #[cfg(feature = "invariant_define")]
    fn resolve_and_run_root_module_report(
        &mut self,
        request: impl Into<SourceRequest>,
        options: ModuleBuildOptions<'_>,
    ) -> MResult<crate::RuntimeRootModuleExecutionReport> {
        self.legacy_interpreter()
            .resolve_and_run_root_module_report(request, options)
    }

    fn install_bytecode_with_context(
        &mut self,
        context: &mut RuntimeContext,
        bytecode: &[u8],
    ) -> MResult<RuntimeValueSnapshot> {
        self.legacy_interpreter()
            .install_bytecode_with_context(context, bytecode)
    }
}

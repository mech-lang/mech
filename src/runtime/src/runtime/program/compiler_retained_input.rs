//! Retained native-build inputs from the existing canonical compilation pass.
//! These owners describe provenance, not a serialized program or execution authority.

use std::{cell::RefCell, sync::Arc};

use mech_core::{FunctionCatalog, MResult};
use mech_engine::ProgramCompilationProduct;

use crate::{ModuleBuildOptions, SourceDocument, SourceRequest};

use super::{ProgramCompiler, ProgramCompilerView};

/// One exact admitted source revision. There is no arbitrary-document constructor.
#[derive(Clone, Debug)]
pub struct CanonicalCompilationSource {
    canonical_uri: String,
    document: SourceDocument,
}

impl CanonicalCompilationSource {
    pub fn canonical_uri(&self) -> &str {
        &self.canonical_uri
    }

    pub fn document(&self) -> &SourceDocument {
        &self.document
    }
}

/// Source owners and the exact injected catalog that compiled their artifact.
/// Requested roots retain caller order; dependencies retain compiler discovery order.
pub struct CanonicalCompilationSources {
    function_catalog: Arc<FunctionCatalog>,
    roots: Box<[CanonicalCompilationSource]>,
    dependencies: Box<[CanonicalCompilationSource]>,
}

impl CanonicalCompilationSources {
    pub fn function_catalog(&self) -> &Arc<FunctionCatalog> {
        &self.function_catalog
    }

    pub fn roots(&self) -> &[CanonicalCompilationSource] {
        &self.roots
    }

    pub fn dependencies(&self) -> &[CanonicalCompilationSource] {
        &self.dependencies
    }
}

/// One compilation result and its retained inputs, produced together without
/// resolving or parsing any source a second time. Splitting this record grants
/// no launch, capability or native-entry authority.
pub struct RetainedCanonicalCompilation {
    product: ProgramCompilationProduct,
    sources: CanonicalCompilationSources,
}

impl RetainedCanonicalCompilation {
    pub fn product(&self) -> &ProgramCompilationProduct {
        &self.product
    }

    pub fn sources(&self) -> &CanonicalCompilationSources {
        &self.sources
    }

    pub fn into_parts(self) -> (ProgramCompilationProduct, CanonicalCompilationSources) {
        (self.product, self.sources)
    }
}

#[derive(Default)]
pub(super) struct RetainedSourceCollector {
    roots: Vec<CanonicalCompilationSource>,
    dependencies: Vec<CanonicalCompilationSource>,
}

impl ProgramCompilerView<'_> {
    /// Called only after ordinary admission at the actual compiler ownership
    /// sites, not by a resolver wrapper or a speculative resolution probe.
    pub(super) fn retain_compilation_source(
        &self,
        canonical_uri: &str,
        document: &SourceDocument,
        requested_root: bool,
    ) {
        let Some(collector) = self.retained_sources else {
            return;
        };
        let mut collector = collector.borrow_mut();
        // URI/revision metadata alone is not the document owner. Keep every
        // distinct snapshot actually compiled without adding conflict rules or
        // comparing/reconstructing source. The retained owners keep pointers live.
        if collector
            .roots
            .iter()
            .chain(&collector.dependencies)
            .any(|source| {
                source.canonical_uri == canonical_uri
                    && std::ptr::eq(source.document.snapshot(), document.snapshot())
                    && source.document.nominal_origin() == document.nominal_origin()
                    && source.document.nominal_package_id() == document.nominal_package_id()
            })
        {
            return;
        }
        let source = CanonicalCompilationSource {
            canonical_uri: canonical_uri.to_owned(),
            document: document.clone(),
        };
        if requested_root {
            collector.roots.push(source);
        } else {
            collector.dependencies.push(source);
        }
    }
}

impl ProgramCompiler {
    /// Compile once through the same authoritative ordered-root implementation,
    /// retaining its original documents and exact catalog for later packaging.
    /// Existing APIs keep their product-only behavior and retain no extra owners.
    pub fn compile_canonical_roots_retained(
        &mut self,
        requests: &[SourceRequest],
        options: ModuleBuildOptions<'_>,
    ) -> MResult<RetainedCanonicalCompilation> {
        let collector = RefCell::new(RetainedSourceCollector::default());
        let mut view = self.view();
        view.retained_sources = Some(&collector);
        let product = view.compile_canonical_roots(requests, options)?;
        drop(view);
        let collector = collector.into_inner();
        Ok(RetainedCanonicalCompilation {
            product,
            sources: CanonicalCompilationSources {
                function_catalog: Arc::clone(&self.function_catalog),
                roots: collector.roots.into_boxed_slice(),
                dependencies: collector.dependencies.into_boxed_slice(),
            },
        })
    }
}

#[cfg(all(test, any(feature = "full_source", feature = "full_compiler")))]
#[path = "compiler_retained_input_tests.rs"]
mod tests;

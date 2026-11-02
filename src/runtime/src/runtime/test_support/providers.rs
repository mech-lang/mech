#[cfg(feature = "resident-routing-source")]
use mech_core::FunctionCatalogBuilder;
#[cfg(feature = "resident-routing-source")]
use mech_core::MResult;

use super::super::RuntimeBuilder;
#[cfg(feature = "resident-routing-source")]
use crate::{
    RuntimeAfterCommitEffect, RuntimeEffectMetadata, RuntimeHostInputDriver,
    RuntimeHostInputSource, RuntimeIngress,
};

#[cfg(feature = "resident-routing-source")]
pub(crate) struct TestAfterCommitEffect {
    metadata: RuntimeEffectMetadata,
    delivery: Box<dyn FnMut() -> MResult<()>>,
}

#[cfg(feature = "resident-routing-source")]
impl TestAfterCommitEffect {
    pub(crate) fn new(
        metadata: RuntimeEffectMetadata,
        delivery: impl FnMut() -> MResult<()> + 'static,
    ) -> Self {
        Self {
            metadata,
            delivery: Box::new(delivery),
        }
    }
}

#[cfg(feature = "resident-routing-source")]
impl std::fmt::Debug for TestAfterCommitEffect {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TestAfterCommitEffect")
            .field("metadata", &self.metadata)
            .finish_non_exhaustive()
    }
}

#[cfg(feature = "resident-routing-source")]
impl RuntimeAfterCommitEffect for TestAfterCommitEffect {
    fn metadata(&self) -> RuntimeEffectMetadata {
        self.metadata.clone()
    }

    fn deliver(&mut self) -> MResult<()> {
        (self.delivery)()
    }
}

#[cfg(feature = "resident-routing-source")]
#[derive(Debug, Default)]
struct TestLiveInputDriver {
    live: bool,
}

#[cfg(feature = "resident-routing-source")]
impl RuntimeHostInputDriver for TestLiveInputDriver {
    fn drives(&self, source: &RuntimeHostInputSource) -> bool {
        source.base_uri().starts_with("test://")
    }

    fn attach(&mut self, _ingress: RuntimeIngress) -> MResult<()> {
        Ok(())
    }

    fn start(&mut self) -> MResult<()> {
        self.live = true;
        Ok(())
    }

    fn stop(&mut self) -> MResult<()> {
        self.live = false;
        Ok(())
    }

    fn is_live(&self) -> bool {
        self.live
    }
}

pub(crate) fn test_runtime_builder() -> RuntimeBuilder {
    #[cfg(feature = "resident-routing-source")]
    {
        let mut catalog = FunctionCatalogBuilder::new();
        mech_engine::install_intrinsic_runtime(&mut catalog).unwrap();
        mech_engine::install_intrinsic_source(&mut catalog).unwrap();
        return RuntimeBuilder::new()
            .function_catalog(std::sync::Arc::new(catalog.build().unwrap()))
            .test_input_driver(TestLiveInputDriver::default());
    }
    #[cfg(not(feature = "resident-routing-source"))]
    RuntimeBuilder::new()
}

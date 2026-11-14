//! A lexical invariant executes only at its selected visit. Its fixed Bool
//! result lives in the already admitted scratch arena; no payload is hidden.
use super::*;

impl ReactiveInstance {
    pub(super) fn execute_lexical_assertion(
        &mut self,
        assertion: super::super::ActivatedAssertion,
        epoch: InstanceEpoch,
    ) -> Result<bool, ResidentExecutionError> {
        let fail = |error| ResidentExecutionError::Kernel {
            node: assertion.artifact_node,
            error,
        };
        (|| -> Result<(), ResidentKernelError> {
            budget::PreparedKernel::new(
                (),
                budget::resident_cost! {
                    compute_work: 2,
                    ..budget::KernelCostEstimate::default()
                },
            )
            .admit_control()?
            .into_plan();
            Ok(())
        })()
        .map_err(fail)?;
        match self.read_location(assertion.condition, epoch) {
            Some(ResidentValueRef::Bool([1])) => {}
            Some(ResidentValueRef::Bool([0])) => {
                return Err(ResidentExecutionError::LexicalIntegrity {
                    node: assertion.artifact_node,
                    block: assertion.block,
                    local: assertion.local,
                });
            }
            _ => return Err(fail(ResidentKernelError::InvalidInput)),
        }
        copy_input(
            &mut self.workspace.scratch,
            assertion.write.region,
            ResidentValueRef::Bool(&[1]),
        )
        .map_err(|error| error.at(fail(ResidentKernelError::InvalidOutput)))?;
        Ok(true)
    }
}

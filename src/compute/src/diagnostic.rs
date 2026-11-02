use std::{error::Error, fmt};

use mech_core::NodeId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComputeDiagnosticCode {
    IntegrityConstraintsUnsupported,
    StateUnsupported,
    OpaqueOperationContract,
    ExternalInteractionUnsupported,
    PortContractUnsupported,
    SchemaUnsupported,
    DynamicShapeUnsupported,
    OperationUnsupported,
    ArityUnsupported,
    ShapeMismatch,
    ConstantUnsupported,
    ArtifactMalformed,
    PlacementConstraintUnsatisfied,
    DerivedBroadcastRequiresMaterialization,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComputeDiagnostic {
    pub code: ComputeDiagnosticCode,
    pub node: Option<NodeId>,
    pub operation: Option<String>,
    pub detail: String,
}

impl fmt::Display for ComputeDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(node) = self.node {
            write!(formatter, "node {}: ", node.get())?;
        }
        if let Some(operation) = &self.operation {
            write!(formatter, "{operation}: ")?;
        }
        write!(formatter, "{}", self.detail)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComputeAdmissionError {
    pub diagnostics: Vec<ComputeDiagnostic>,
}

impl ComputeAdmissionError {
    pub fn diagnostics(&self) -> &[ComputeDiagnostic] {
        &self.diagnostics
    }
}

impl fmt::Display for ComputeAdmissionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "compute planning rejected the program with {} diagnostic(s)",
            self.diagnostics.len()
        )?;
        for diagnostic in &self.diagnostics {
            write!(formatter, "\n- {diagnostic}")?;
        }
        Ok(())
    }
}

impl Error for ComputeAdmissionError {}

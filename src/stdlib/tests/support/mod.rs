#![allow(
    dead_code,
    reason = "shared canonical source helpers are selected by focused integration tests"
)]

use mech_core::{
    ReactiveInstanceId, ResolvedType, ResolvedValueDescriptor, Schema, Value, ValueData,
};
use mech_engine::resident::{
    ActivationFacts, ResidentActivationError, ResidentExecutionError, activate,
};
use mech_engine::{
    ArtifactBuildError, CanonicalSourceFrontend, SourceDocumentOutputKind, SourceSemanticError,
};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

#[derive(Debug)]
pub enum CanonicalEvaluationError {
    Parse(String),
    Source(SourceSemanticError),
    Artifact(ArtifactBuildError),
    Activation(ResidentActivationError),
    Execution(ResidentExecutionError),
    MissingProgramOutput,
}

#[derive(Debug)]
pub struct EvaluatedValue {
    value: Value,
    declared_schema: Schema,
}

impl EvaluatedValue {
    pub fn data(&self) -> &ValueData {
        self.value.data()
    }
}

impl CanonicalEvaluationError {
    pub fn source(&self) -> Option<&SourceSemanticError> {
        match self {
            Self::Source(error) => Some(error),
            _ => None,
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::Parse(message) => message.clone(),
            Self::Source(error) => format!("{}: {}", error.code, error.message),
            other => format!("{other:?}"),
        }
    }
}

pub fn evaluate(source: &str) -> Result<EvaluatedValue, CanonicalEvaluationError> {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x523), Revision(1), source)
            .map_err(|error| CanonicalEvaluationError::Parse(format!("{error:?}")))?,
        ParseConfig::default(),
    );
    if !parsed.diagnostics.is_empty() {
        return Err(CanonicalEvaluationError::Parse(format!(
            "{:#?}",
            parsed.diagnostics
        )));
    }
    let document = DocumentSyntax::cast(parsed.syntax()).ok_or_else(|| {
        CanonicalEvaluationError::Parse("canonical parser did not publish a document root".into())
    })?;
    let compiled = CanonicalSourceFrontend
        .compile_document(&document)
        .map_err(CanonicalEvaluationError::Source)?;
    let output = compiled
        .document_outputs()
        .iter()
        .rev()
        .find(|output| output.kind == SourceDocumentOutputKind::Program)
        .map(|output| output.output as usize)
        .ok_or(CanonicalEvaluationError::MissingProgramOutput)?;
    let declared_schema = compiled
        .schemas()
        .get(compiled.program().outputs[output].schema)
        .expect("canonical output schema is present before artifact compilation")
        .clone();
    let artifact = compiled
        .compile_artifact()
        .map_err(CanonicalEvaluationError::Artifact)?;
    let mut instance = activate(
        ReactiveInstanceId::new(0x523, 0),
        &artifact,
        &mech_stdlib::source_catalog(),
        &ActivationFacts::default(),
    )
    .map_err(CanonicalEvaluationError::Activation)?;
    instance
        .turn(&[])
        .map_err(CanonicalEvaluationError::Execution)?;
    let value = instance
        .copied_output(output)
        .map_err(CanonicalEvaluationError::Activation)?;
    Ok(EvaluatedValue {
        value,
        declared_schema,
    })
}

pub fn descriptor(value: &EvaluatedValue) -> ResolvedValueDescriptor {
    ResolvedValueDescriptor::from_schema(value.declared_schema.clone(), value.value.shape().clone())
        .expect("resident output schema and shape form a resolved descriptor")
}

pub fn resolved_type(value: &EvaluatedValue) -> ResolvedType {
    descriptor(value).resolved_type().clone()
}

pub fn current_extents(value: &EvaluatedValue) -> Box<[u64]> {
    descriptor(value)
        .current_extents()
        .expect("resident output dimensions resolve against its retained shape")
}

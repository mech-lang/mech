use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{
    ElementwiseKernel, FixedShapeKernel, GpuBindingAccess, GpuBindingRole, WORKGROUP_SIZE,
};

pub const GPU_EXECUTION_PLAN_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug)]
pub enum GpuKernelPlanSource<'a> {
    Elementwise(&'a ElementwiseKernel),
    FixedShape(&'a FixedShapeKernel),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GpuPlanKernelKind {
    Elementwise,
    FixedShape,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GpuPlanBindingAccess {
    Read,
    ReadWrite,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GpuPlanBindingRole {
    Input,
    StateRead,
    StateWrite,
    Output,
    IntegrityFault,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GpuPlanScalar {
    F32,
    U32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GpuPlanLayout {
    RowMajor,
    ColumnMajor,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "scalar", content = "values", rename_all = "kebab-case")]
pub enum GpuPlanInitialValues {
    F32(Vec<f32>),
    U32(Vec<u32>),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GpuPlanBinding {
    pub binding: u32,
    pub name: String,
    pub access: GpuPlanBindingAccess,
    pub role: GpuPlanBindingRole,
    pub slot: u32,
    pub elements: u64,
    pub scalar: GpuPlanScalar,
    pub initial_values: Option<GpuPlanInitialValues>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GpuPlanState {
    pub slot: u32,
    pub elements: u64,
    pub elements_per_instance: u64,
    pub initial_values: Vec<f32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GpuPlanOutput {
    pub name: String,
    pub slot: u32,
    pub physical_output: u32,
    pub elements: u64,
    pub elements_per_instance: u64,
    pub dimensions: Vec<u64>,
    pub sample_dimensions: Vec<u64>,
    pub physical_layout: GpuPlanLayout,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GpuPhysicalOutputPlan {
    pub id: u32,
    pub slot: u32,
    pub binding: Option<u32>,
    pub sample_elements: u64,
    pub aliases: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GpuPlanConstraint {
    pub code: u32,
    pub id: u32,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GpuExecutionPlan {
    pub version: u32,
    pub kernel_kind: GpuPlanKernelKind,
    pub wgsl: String,
    pub workgroup_size: u32,
    pub dispatch_elements: u32,
    pub bindings: Vec<GpuPlanBinding>,
    pub states: Vec<GpuPlanState>,
    pub outputs: Vec<GpuPlanOutput>,
    pub physical_outputs: Vec<GpuPhysicalOutputPlan>,
    pub constraints: Vec<GpuPlanConstraint>,
}

impl GpuExecutionPlan {
    pub fn build(
        source: GpuKernelPlanSource<'_>,
        input_values: &BTreeMap<String, Vec<f32>>,
    ) -> Result<Self, GpuExecutionPlanError> {
        let plan = match source {
            GpuKernelPlanSource::Elementwise(kernel) => {
                build_elementwise_plan(kernel, input_values)?
            }
            GpuKernelPlanSource::FixedShape(kernel) => {
                build_fixed_shape_plan(kernel, input_values)?
            }
        };
        plan.validate()?;
        Ok(plan)
    }

    pub fn validate(&self) -> Result<(), GpuExecutionPlanError> {
        if self.version != GPU_EXECUTION_PLAN_VERSION {
            return Err(GpuExecutionPlanError::UnsupportedVersion(self.version));
        }
        if self.workgroup_size == 0 || self.dispatch_elements == 0 {
            return Err(GpuExecutionPlanError::Invalid(
                "workgroup and dispatch sizes must be nonzero".to_owned(),
            ));
        }
        let bindings = self
            .bindings
            .iter()
            .map(|binding| binding.binding)
            .collect::<BTreeSet<_>>();
        if bindings.len() != self.bindings.len() {
            return Err(GpuExecutionPlanError::Invalid(
                "GPU binding numbers must be unique".to_owned(),
            ));
        }
        if self.bindings.iter().any(|binding| binding.elements == 0) {
            return Err(GpuExecutionPlanError::Invalid(
                "GPU bindings must contain at least one scalar".to_owned(),
            ));
        }
        for binding in &self.bindings {
            let Some(initial) = &binding.initial_values else {
                continue;
            };
            let length = match initial {
                GpuPlanInitialValues::F32(values) if binding.scalar == GpuPlanScalar::F32 => {
                    values.len()
                }
                GpuPlanInitialValues::U32(values) if binding.scalar == GpuPlanScalar::U32 => {
                    values.len()
                }
                _ => {
                    return Err(GpuExecutionPlanError::Invalid(format!(
                        "GPU binding `{}` initializer scalar does not match its declaration",
                        binding.name
                    )));
                }
            };
            if u64::try_from(length).ok() != Some(binding.elements) {
                return Err(GpuExecutionPlanError::Invalid(format!(
                    "GPU binding `{}` has {length} initial values but declares {} elements",
                    binding.name, binding.elements
                )));
            }
        }
        let physical = self
            .physical_outputs
            .iter()
            .map(|output| output.id)
            .collect::<BTreeSet<_>>();
        if physical.len() != self.physical_outputs.len() {
            return Err(GpuExecutionPlanError::Invalid(
                "physical output identifiers must be unique".to_owned(),
            ));
        }
        let mut physical_aliases = BTreeSet::new();
        for output in &self.physical_outputs {
            if output.sample_elements == 0 || output.aliases.is_empty() {
                return Err(GpuExecutionPlanError::Invalid(format!(
                    "physical output {} must have a nonempty sample and alias set",
                    output.id
                )));
            }
            if let Some(binding) = output.binding {
                if !bindings.contains(&binding) {
                    return Err(GpuExecutionPlanError::Invalid(format!(
                        "physical output {} references unknown binding {binding}",
                        output.id
                    )));
                }
            }
            for alias in &output.aliases {
                if !physical_aliases.insert(alias.as_str()) {
                    return Err(GpuExecutionPlanError::Invalid(format!(
                        "logical output alias `{alias}` appears in more than one physical transfer"
                    )));
                }
            }
        }
        for output in &self.outputs {
            if !physical.contains(&output.physical_output) {
                return Err(GpuExecutionPlanError::Invalid(format!(
                    "logical output `{}` references unknown physical output {}",
                    output.name, output.physical_output
                )));
            }
            let Some(physical_output) = self
                .physical_outputs
                .iter()
                .find(|physical| physical.id == output.physical_output)
            else {
                unreachable!("physical output identity was checked above")
            };
            if !physical_output.aliases.contains(&output.name)
                || physical_output.slot != output.slot
                || physical_output.sample_elements != output.elements_per_instance
            {
                return Err(GpuExecutionPlanError::Invalid(format!(
                    "logical output `{}` does not match physical output {}",
                    output.name, output.physical_output
                )));
            }
        }
        if physical_aliases.len() != self.outputs.len() {
            return Err(GpuExecutionPlanError::Invalid(
                "physical aliases and logical outputs are not one-to-one".to_owned(),
            ));
        }
        for state in &self.states {
            if state.elements == 0
                || state.elements_per_instance == 0
                || state.initial_values.len() as u64 != state.elements
            {
                return Err(GpuExecutionPlanError::Invalid(format!(
                    "resident state {} has an invalid physical shape or initializer",
                    state.slot
                )));
            }
            for role in [
                GpuPlanBindingRole::StateRead,
                GpuPlanBindingRole::StateWrite,
            ] {
                if !self.bindings.iter().any(|binding| {
                    binding.role == role
                        && binding.slot == state.slot
                        && binding.elements == state.elements
                }) {
                    return Err(GpuExecutionPlanError::Invalid(format!(
                        "resident state {} has no matching {role:?} binding",
                        state.slot
                    )));
                }
            }
        }
        let constraint_codes = self
            .constraints
            .iter()
            .map(|constraint| constraint.code)
            .collect::<BTreeSet<_>>();
        if constraint_codes.len() != self.constraints.len()
            || constraint_codes.contains(&0)
            || (!self.constraints.is_empty()
                && !self
                    .bindings
                    .iter()
                    .any(|binding| binding.role == GpuPlanBindingRole::IntegrityFault))
        {
            return Err(GpuExecutionPlanError::Invalid(
                "integrity constraints require unique nonzero codes and one fault binding"
                    .to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GpuExecutionPlanError {
    Invalid(String),
    UnsupportedVersion(u32),
}

impl fmt::Display for GpuExecutionPlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(reason) => write!(formatter, "invalid GPU execution plan: {reason}"),
            Self::UnsupportedVersion(version) => write!(
                formatter,
                "GPU execution plan version {version} is unsupported; expected {GPU_EXECUTION_PLAN_VERSION}"
            ),
        }
    }
}

impl std::error::Error for GpuExecutionPlanError {}

fn build_elementwise_plan(
    kernel: &ElementwiseKernel,
    input_values: &BTreeMap<String, Vec<f32>>,
) -> Result<GpuExecutionPlan, GpuExecutionPlanError> {
    let mut bindings = Vec::new();
    for binding in kernel.bindings() {
        let initial_values = match binding.role() {
            GpuBindingRole::Input => {
                let values = input_values.get(&binding.name).ok_or_else(|| {
                    GpuExecutionPlanError::Invalid(format!(
                        "elementwise input `{}` has no initializer",
                        binding.name
                    ))
                })?;
                if values.len() as u64 != binding.elements {
                    return Err(GpuExecutionPlanError::Invalid(format!(
                        "elementwise input `{}` has {} values; expected {}",
                        binding.name,
                        values.len(),
                        binding.elements
                    )));
                }
                Some(GpuPlanInitialValues::F32(values.clone()))
            }
            GpuBindingRole::StateRead => {
                let (_, _, initializer) = kernel
                    .state_initializers()
                    .find(|(slot, _, _)| *slot == binding.slot())
                    .ok_or_else(|| {
                        GpuExecutionPlanError::Invalid(format!(
                            "state-read binding `{}` has no state initializer",
                            binding.name
                        ))
                    })?;
                Some(GpuPlanInitialValues::F32(initializer.to_vec()))
            }
            GpuBindingRole::StateWrite | GpuBindingRole::Output => None,
        };
        bindings.push(GpuPlanBinding {
            binding: binding.binding,
            name: binding.name.clone(),
            access: binding.access.into(),
            role: binding.role().into(),
            slot: binding.slot().get(),
            elements: binding.elements,
            scalar: GpuPlanScalar::F32,
            initial_values,
        });
    }
    let states = kernel
        .state_initializers()
        .map(|(slot, elements, initializer)| GpuPlanState {
            slot: slot.get(),
            elements,
            elements_per_instance: elements,
            initial_values: initializer.to_vec(),
        })
        .collect();
    let outputs = logical_outputs(
        GpuPlanKernelKind::Elementwise,
        kernel.compute_program(),
        1,
        &bindings,
    )?;
    let physical_outputs = physical_outputs(&outputs, &bindings)?;
    Ok(GpuExecutionPlan {
        version: GPU_EXECUTION_PLAN_VERSION,
        kernel_kind: GpuPlanKernelKind::Elementwise,
        wgsl: kernel.wgsl().to_owned(),
        workgroup_size: WORKGROUP_SIZE,
        dispatch_elements: u32::try_from(kernel.dispatch_elements()).map_err(|_| {
            GpuExecutionPlanError::Invalid("elementwise dispatch exceeds u32".to_owned())
        })?,
        bindings,
        states,
        outputs,
        physical_outputs,
        constraints: Vec::new(),
    })
}

fn build_fixed_shape_plan(
    kernel: &FixedShapeKernel,
    input_values: &BTreeMap<String, Vec<f32>>,
) -> Result<GpuExecutionPlan, GpuExecutionPlanError> {
    if kernel.integrity_buffer().is_some() && kernel.instances() >= (1 << 24) {
        return Err(GpuExecutionPlanError::Invalid(
            "checked GPU fault records support fewer than 2^24 instances".to_owned(),
        ));
    }
    let physical_inputs = kernel
        .physical_inputs(input_values)
        .map_err(|failure| GpuExecutionPlanError::Invalid(failure.to_string()))?;
    let physical_states = kernel.physical_states();
    let mut bindings = physical_inputs
        .iter()
        .map(|input| GpuPlanBinding {
            binding: input.binding,
            name: input.name.clone(),
            access: GpuPlanBindingAccess::Read,
            role: GpuPlanBindingRole::Input,
            slot: input.slot.get(),
            elements: input.elements as u64,
            scalar: GpuPlanScalar::F32,
            initial_values: Some(GpuPlanInitialValues::F32(input.initial_values.clone())),
        })
        .collect::<Vec<_>>();
    for state in &physical_states {
        bindings.push(GpuPlanBinding {
            binding: state.read_binding,
            name: format!("state.{}.read", state.slot.get()),
            access: GpuPlanBindingAccess::Read,
            role: GpuPlanBindingRole::StateRead,
            slot: state.slot.get(),
            elements: state.elements as u64,
            scalar: GpuPlanScalar::F32,
            initial_values: None,
        });
        bindings.push(GpuPlanBinding {
            binding: state.write_binding,
            name: format!("state.{}.write", state.slot.get()),
            access: GpuPlanBindingAccess::ReadWrite,
            role: GpuPlanBindingRole::StateWrite,
            slot: state.slot.get(),
            elements: state.elements as u64,
            scalar: GpuPlanScalar::F32,
            initial_values: None,
        });
    }
    if let Some(integrity) = kernel.integrity_buffer() {
        bindings.push(GpuPlanBinding {
            binding: integrity.binding,
            name: "integrity-fault".to_owned(),
            access: GpuPlanBindingAccess::ReadWrite,
            role: GpuPlanBindingRole::IntegrityFault,
            slot: 0,
            elements: integrity.words as u64,
            scalar: GpuPlanScalar::U32,
            initial_values: Some(GpuPlanInitialValues::U32(vec![0, u32::MAX])),
        });
    }
    bindings.sort_by_key(|binding| binding.binding);
    let states = physical_states
        .iter()
        .map(|state| GpuPlanState {
            slot: state.slot.get(),
            elements: state.elements as u64,
            elements_per_instance: state.elements_per_instance as u64,
            initial_values: state.initial_values.clone(),
        })
        .collect();
    let outputs = logical_outputs(
        GpuPlanKernelKind::FixedShape,
        kernel.compute_program(),
        kernel.instances(),
        &bindings,
    )?;
    let physical_outputs = physical_outputs(&outputs, &bindings)?;
    let constraints = kernel
        .named_integrity_constraints()
        .enumerate()
        .map(|(index, (id, name))| {
            Ok(GpuPlanConstraint {
                code: u32::try_from(index + 1).map_err(|_| {
                    GpuExecutionPlanError::Invalid(
                        "integrity constraint count exceeds u32".to_owned(),
                    )
                })?,
                id: id.get(),
                name: name.to_owned(),
            })
        })
        .collect::<Result<Vec<_>, GpuExecutionPlanError>>()?;
    Ok(GpuExecutionPlan {
        version: GPU_EXECUTION_PLAN_VERSION,
        kernel_kind: GpuPlanKernelKind::FixedShape,
        wgsl: kernel.wgsl().to_owned(),
        workgroup_size: WORKGROUP_SIZE,
        dispatch_elements: kernel.instances(),
        bindings,
        states,
        outputs,
        physical_outputs,
        constraints,
    })
}

fn logical_outputs(
    kind: GpuPlanKernelKind,
    compute: &mech_compute::ComputeProgram,
    instances: u32,
    bindings: &[GpuPlanBinding],
) -> Result<Vec<GpuPlanOutput>, GpuExecutionPlanError> {
    let mut physical_ids = BTreeMap::<u32, u32>::new();
    compute
        .interface()
        .outputs
        .iter()
        .map(|output| {
            let per_instance = output.elements().map_err(|failure| {
                GpuExecutionPlanError::Invalid(format!(
                    "output `{}` has an invalid shape: {failure}",
                    output.name
                ))
            })? as u64;
            let elements = per_instance
                .checked_mul(u64::from(instances))
                .ok_or_else(|| {
                    GpuExecutionPlanError::Invalid(format!(
                        "output `{}` size overflows u64",
                        output.name
                    ))
                })?;
            let next_id = u32::try_from(physical_ids.len()).map_err(|_| {
                GpuExecutionPlanError::Invalid("physical output count exceeds u32".to_owned())
            })?;
            let physical_output = *physical_ids.entry(output.slot.get()).or_insert(next_id);
            let mut dimensions = Vec::new();
            if kind == GpuPlanKernelKind::FixedShape {
                dimensions.push(u64::from(instances));
            }
            dimensions.extend(output.dimensions.iter().copied());
            let _ = bindings;
            Ok(GpuPlanOutput {
                name: output.name.to_string(),
                slot: output.slot.get(),
                physical_output,
                elements,
                elements_per_instance: per_instance,
                dimensions,
                sample_dimensions: output.dimensions.to_vec(),
                physical_layout: if kind == GpuPlanKernelKind::FixedShape {
                    GpuPlanLayout::ColumnMajor
                } else {
                    GpuPlanLayout::RowMajor
                },
            })
        })
        .collect()
}

fn physical_outputs(
    outputs: &[GpuPlanOutput],
    bindings: &[GpuPlanBinding],
) -> Result<Vec<GpuPhysicalOutputPlan>, GpuExecutionPlanError> {
    let state_slots = bindings
        .iter()
        .filter(|binding| binding.role == GpuPlanBindingRole::StateWrite)
        .map(|binding| binding.slot)
        .collect::<BTreeSet<_>>();
    let mut physical = BTreeMap::<u32, GpuPhysicalOutputPlan>::new();
    for output in outputs {
        let binding = if state_slots.contains(&output.slot) {
            None
        } else {
            Some(
                bindings
                    .iter()
                    .find(|binding| {
                        binding.role == GpuPlanBindingRole::Output && binding.slot == output.slot
                    })
                    .ok_or_else(|| {
                        GpuExecutionPlanError::Invalid(format!(
                            "output `{}` has no physical binding",
                            output.name
                        ))
                    })?
                    .binding,
            )
        };
        let entry =
            physical
                .entry(output.physical_output)
                .or_insert_with(|| GpuPhysicalOutputPlan {
                    id: output.physical_output,
                    slot: output.slot,
                    binding,
                    sample_elements: output.elements_per_instance,
                    aliases: Vec::new(),
                });
        if entry.slot != output.slot
            || entry.binding != binding
            || entry.sample_elements != output.elements_per_instance
        {
            return Err(GpuExecutionPlanError::Invalid(format!(
                "logical output `{}` is incompatible with its physical alias group",
                output.name
            )));
        }
        entry.aliases.push(output.name.clone());
    }
    Ok(physical.into_values().collect())
}

impl From<GpuBindingAccess> for GpuPlanBindingAccess {
    fn from(access: GpuBindingAccess) -> Self {
        match access {
            GpuBindingAccess::Read => Self::Read,
            GpuBindingAccess::ReadWrite => Self::ReadWrite,
        }
    }
}

impl From<GpuBindingRole> for GpuPlanBindingRole {
    fn from(role: GpuBindingRole) -> Self {
        match role {
            GpuBindingRole::Input => Self::Input,
            GpuBindingRole::StateRead => Self::StateRead,
            GpuBindingRole::StateWrite => Self::StateWrite,
            GpuBindingRole::Output => Self::Output,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialized_execution_plan_rejects_unknown_versions() {
        let mut plan = GpuExecutionPlan {
            version: GPU_EXECUTION_PLAN_VERSION,
            kernel_kind: GpuPlanKernelKind::Elementwise,
            wgsl: "shader".to_owned(),
            workgroup_size: 64,
            dispatch_elements: 1,
            bindings: Vec::new(),
            states: Vec::new(),
            outputs: Vec::new(),
            physical_outputs: Vec::new(),
            constraints: Vec::new(),
        };
        let encoded = serde_json::to_string(&plan).unwrap();
        let decoded: GpuExecutionPlan = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, plan);
        plan.version += 1;
        assert_eq!(
            plan.validate(),
            Err(GpuExecutionPlanError::UnsupportedVersion(
                GPU_EXECUTION_PLAN_VERSION + 1
            ))
        );
    }

    #[test]
    fn invalid_binding_sizes_and_aliases_are_rejected() {
        let mut plan = GpuExecutionPlan {
            version: GPU_EXECUTION_PLAN_VERSION,
            kernel_kind: GpuPlanKernelKind::Elementwise,
            wgsl: "shader".to_owned(),
            workgroup_size: 64,
            dispatch_elements: 1,
            bindings: vec![GpuPlanBinding {
                binding: 0,
                name: "input".to_owned(),
                access: GpuPlanBindingAccess::Read,
                role: GpuPlanBindingRole::Input,
                slot: 1,
                elements: 2,
                scalar: GpuPlanScalar::F32,
                initial_values: Some(GpuPlanInitialValues::F32(vec![1.0])),
            }],
            states: Vec::new(),
            outputs: Vec::new(),
            physical_outputs: Vec::new(),
            constraints: Vec::new(),
        };
        assert!(plan.validate().is_err());
        plan.bindings[0].initial_values = Some(GpuPlanInitialValues::F32(vec![1.0, 2.0]));
        plan.outputs.push(GpuPlanOutput {
            name: "missing".to_owned(),
            slot: 2,
            physical_output: 5,
            elements: 1,
            elements_per_instance: 1,
            dimensions: vec![1],
            sample_dimensions: vec![1],
            physical_layout: GpuPlanLayout::RowMajor,
        });
        assert!(plan.validate().is_err());
    }
}

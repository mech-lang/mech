//! Read-only numeric captures of one synchronous scalar-state Tail.
//! The proof selects exact maintained factories, not arbitrary pointer inputs.
use super::readonly_numeric::width;
use mech_engine::native_artifact::{
    NativeIterationAction as Action, NativeIterationValueScope as Scope, NativePreparedLibraryCall,
    NativeProgramArtifactPlan as Plan, NativeScalarKind, NativeTailLoop,
};

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Capture {
    pub value: u32,
    pub width: u8,
    pub witness: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) enum Operation {
    Read {
        capture: usize,
        index: u32,
        output: u32,
    },
    Length {
        capture: usize,
        output: u32,
    },
}
impl Operation {
    pub fn output(&self) -> u32 {
        match *self {
            Self::Read { output, .. } | Self::Length { output, .. } => output,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Proof {
    pub captures: Vec<Capture>,
    pub actions: Vec<(usize, Operation)>,
}
impl Proof {
    pub fn words(&self) -> usize {
        self.captures.len() * 4
    }
    pub fn action(&self, site: usize) -> Option<&Operation> {
        self.actions
            .iter()
            .find(|(at, _)| *at == site)
            .map(|(_, action)| action)
    }
}

pub(in crate::native_artifact) fn derive(
    plan: &Plan,
    tail: &NativeTailLoop,
    calls: &[Option<NativePreparedLibraryCall>],
) -> Result<Proof, &'static str> {
    let iteration = plan
        .iterations()
        .iter()
        .position(|control| control.as_ref().header() == tail.header())
        .ok_or("missing scalar Tail iteration")? as u32;
    let facts = plan.facts();
    let mut captures = Vec::<Capture>::new();
    let mut actions = Vec::new();
    for (site, block) in plan
        .blocks()
        .iter()
        .enumerate()
        .filter(|(_, block)| block.iteration() == Some(iteration))
    {
        let Action::LibraryCall {
            template,
            operands,
            output,
        } = block.action()
        else {
            continue;
        };
        let call = calls
            .get(site)
            .and_then(Option::as_ref)
            .ok_or("missing selected capture factory")?;
        if call.template() != template {
            return Err("capture factory differs from source template");
        }
        let read = call.supports_unsigned_matrix_element();
        let length = call.supports_unsigned_matrix_length();
        if !(read || length) {
            continue;
        }
        let source = *operands.first().ok_or("capture has no source")?;
        let Scope::Immutable { iteration: scope } = plan.value_scopes()[source.get() as usize]
        else {
            return Err("capture source is not immutable");
        };
        if let Some(scope) = scope {
            let mut ancestor = tail.parent();
            while ancestor.is_some() && ancestor != Some(scope) {
                ancestor = plan
                    .iterations()
                    .get(ancestor.ok_or("capture ancestry ended")? as usize)
                    .ok_or("capture ancestor missing")?
                    .as_ref()
                    .parent();
            }
            if ancestor != Some(scope) {
                return Err("capture is not from a strict ancestor");
            }
        }
        // The arena definition belongs to the enclosing immutable invocation.
        // Scalar-only generated execution cannot replace or retire this owner.
        if facts
            .definition_sites(source.get())
            .map_err(|_| "capture definitions unavailable")?
            .iter()
            .any(|definition| definition.iteration() == Some(iteration))
        {
            return Err("capture source is defined in the scalar body");
        }
        if plan.values()[source.get() as usize].definition()
            != template
                .inputs()
                .first()
                .ok_or("capture witness has no source schema")?
        {
            return Err("capture source schema differs from selected schema");
        }
        let width = width(template.inputs()[0].body())
            .ok_or("capture is not a supported unsigned matrix")?;
        if (read
            && (operands.len() != 2
                || plan.values()[operands[1].get() as usize].scalar_kind()
                    != Some(NativeScalarKind::U64)))
            || (length && operands.len() != 1)
        {
            return Err("capture operands differ from selected native signature");
        }
        let capture = if let Some(at) = captures
            .iter()
            .position(|capture| capture.value == source.get())
        {
            if captures[at].width != width {
                return Err("capture width differs across selected witnesses");
            }
            at
        } else {
            captures.push(Capture {
                value: source.get(),
                width,
                witness: site,
            });
            captures.len() - 1
        };
        actions.push((
            site,
            if read {
                Operation::Read {
                    capture,
                    index: operands[1].get(),
                    output: output.get(),
                }
            } else {
                Operation::Length {
                    capture,
                    output: output.get(),
                }
            },
        ));
    }
    if actions.is_empty() {
        Err("no selected unsigned reads or lengths")
    } else {
        Ok(Proof { captures, actions })
    }
}

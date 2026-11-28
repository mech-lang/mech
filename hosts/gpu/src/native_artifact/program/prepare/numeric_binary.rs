//! Same-plan selected typed matrix operations; no values, pointers or permits.
use mech_engine::native_artifact::{
    NativeIterationAction as Action, NativeProgramArtifactPlan as Plan, NativeScalarOp,
};
pub(in crate::native_artifact) const HEADER_WORK: u64 = 32768;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) enum Kind {
    Elementwise(NativeScalarOp),
    Product,
    Sum {
        axis: usize,
        input: [Option<u64>; 2],
    },
    Transpose {
        input: [Option<u64>; 2],
    },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) struct Operation {
    pub operands: Vec<u32>,
    pub output: u32,
    pub kind: Kind,
}
#[derive(Debug)]
pub(in crate::native_artifact) struct Layout {
    sites: Box<[Option<Operation>]>,
    epilogues: Box<[Option<super::numeric_product_epilogues::Candidate>]>,
}
fn constant_axes(
    template: &mech_engine::native_artifact::NativeProgramLibraryTemplate,
) -> Option<[Option<u64>; 2]> {
    let mech_core::SchemaBody::Matrix { dimensions, .. } = template.inputs().first()?.body() else {
        return None;
    };
    if dimensions.len() != 2 {
        return None;
    }
    Some(std::array::from_fn(|at| match dimensions[at] {
        mech_core::DimensionExpr::Constant(value) => Some(value),
        _ => None,
    }))
}
impl Layout {
    pub fn derive(plan: &Plan) -> Result<Self, &'static str> {
        plan.validate_current_visits()?;
        let sites = plan
            .blocks()
            .iter()
            .enumerate()
            .map(|(site, block)| {
                let Action::LibraryCall {
                    operands, output, ..
                } = block.action()
                else {
                    return Ok(None);
                };
                let Some(template) = plan.prepared_library_template(site)? else {
                    return Ok(None);
                };
                if operands.len() != template.inputs().len()
                    || operands
                        .iter()
                        .zip(template.inputs())
                        .any(|(v, s)| plan.values()[v.get() as usize].definition() != s)
                    || plan.values()[output.get() as usize].definition() != template.output()
                {
                    return Ok(None);
                }
                let kind = template
                    .f64_matrix_binary_op()
                    .map(Kind::Elementwise)
                    .or_else(|| template.f64_matrix_product().then_some(Kind::Product))
                    .or_else(|| {
                        template.f64_matrix_sum_axis().and_then(|axis| {
                            Some(Kind::Sum {
                                axis,
                                input: constant_axes(template)?,
                            })
                        })
                    })
                    .or_else(|| {
                        template.f64_matrix_transpose().then_some(())?;
                        Some(Kind::Transpose {
                            input: constant_axes(template)?,
                        })
                    });
                Ok(kind.map(|kind| Operation {
                    operands: operands.iter().map(|v| v.get()).collect(),
                    output: output.get(),
                    kind,
                }))
            })
            .collect::<Result<Box<[_]>, &'static str>>()?;
        let mut result = Self {
            epilogues: vec![None; sites.len()].into_boxed_slice(),
            sites,
        };
        for recipe in super::numeric_product_epilogues::derive(plan, &result) {
            let entry = recipe.entry as usize;
            result.epilogues[entry] = Some(recipe);
        }
        Ok(result)
    }
    pub fn epilogue(&self, site: usize) -> Option<&super::numeric_product_epilogues::Candidate> {
        self.epilogues.get(site)?.as_ref()
    }
    pub fn epilogue_arguments(&self) -> usize {
        self.epilogues
            .iter()
            .flatten()
            .map(|r| r.scalars.len())
            .max()
            .unwrap_or(0)
    }
    pub fn at(&self, site: usize) -> Option<&Operation> {
        self.sites.get(site)?.as_ref()
    }
}

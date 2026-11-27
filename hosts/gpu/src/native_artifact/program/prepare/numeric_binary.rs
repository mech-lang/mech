//! Same-plan selected typed matrix operations; no values, pointers or permits.
use mech_engine::native_artifact::{
    NativeIterationAction as Action, NativeProgramArtifactPlan as Plan, NativeScalarOp,
};
pub(in crate::native_artifact) const HEADER_WORK: u64 = 32768;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) enum Kind {
    Elementwise(NativeScalarOp),
    Product,
    Transpose { input: [Option<u64>; 2] },
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
                        if !template.f64_matrix_transpose() {
                            return None;
                        }
                        let mech_core::SchemaBody::Matrix { dimensions, .. } =
                            template.inputs()[0].body()
                        else {
                            return None;
                        };
                        let mut input = [None; 2];
                        for (at, axis) in dimensions.iter().enumerate() {
                            if let mech_core::DimensionExpr::Constant(value) = axis {
                                input[at] = Some(*value);
                            }
                        }
                        Some(Kind::Transpose { input })
                    });
                Ok(kind.map(|kind| Operation {
                    operands: operands.iter().map(|v| v.get()).collect(),
                    output: output.get(),
                    kind,
                }))
            })
            .collect::<Result<Box<[_]>, &'static str>>()?;
        Ok(Self { sites })
    }
    pub fn at(&self, site: usize) -> Option<&Operation> {
        self.sites.get(site)?.as_ref()
    }
}

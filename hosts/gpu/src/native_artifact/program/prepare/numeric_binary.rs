//! Same-plan selected elementwise arithmetic; no values, pointers or permits.
use mech_engine::native_artifact::{
    NativeIterationAction as Action, NativeProgramArtifactPlan as Plan, NativeScalarOp,
};
pub(in crate::native_artifact) const HEADER_WORK: u64 = 32768;
#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) struct Operation {
    pub operands: [u32; 2],
    pub output: u32,
    pub op: NativeScalarOp,
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
                if operands.len() != 2
                    || operands
                        .iter()
                        .zip(template.inputs())
                        .any(|(v, s)| plan.values()[v.get() as usize].definition() != s)
                    || plan.values()[output.get() as usize].definition() != template.output()
                {
                    return Ok(None);
                }
                Ok(template.f64_matrix_binary_op().map(|op| Operation {
                    operands: [operands[0].get(), operands[1].get()],
                    output: output.get(),
                    op,
                }))
            })
            .collect::<Result<Box<[_]>, &'static str>>()?;
        Ok(Self { sites })
    }
    pub fn at(&self, site: usize) -> Option<&Operation> {
        self.sites.get(site)?.as_ref()
    }
}

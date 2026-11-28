use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) enum Expr {
    Coordinate(u8),
    Constant(u64),
    Capture(u32),
    Binary {
        op: NativeScalarOp,
        left: Box<Index>,
        right: Box<Index>,
    },
}
/// Each checked subexpression is jointly affine in the admitted coordinates.
/// Multiplication requires a coordinate-independent factor. Every endpoint
/// and corner is checked before a native stream is admitted.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) struct Index {
    pub expr: Expr,
    pub dependencies: [bool; 2],
}
impl Index {
    fn leaf(expr: Expr, axis: Option<usize>) -> Self {
        let mut dependencies = [false; 2];
        if let Some(axis) = axis {
            dependencies[axis] = true;
        }
        Self { expr, dependencies }
    }
    pub(super) fn binary(op: NativeScalarOp, left: Self, right: Self) -> Option<Self> {
        if op == NativeScalarOp::U64Mul
            && left.dependencies.iter().any(|&v| v)
            && right.dependencies.iter().any(|&v| v)
        {
            return None;
        }
        let dependencies =
            std::array::from_fn(|axis| left.dependencies[axis] || right.dependencies[axis]);
        Some(Self {
            expr: Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            },
            dependencies,
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) struct Operand {
    pub matrix: usize,
    pub indices: Vec<Index>,
}
pub(super) struct Builder<'a> {
    pub plan: &'a Plan,
    pub region: &'a Region,
    pub outer: Option<u32>,
    pub definitions: BTreeMap<u32, (u32, &'a Action)>,
    pub used: BTreeSet<u32>,
}
impl<'a> Builder<'a> {
    pub fn new(plan: &'a Plan, region: &'a Region, outer: Option<u32>) -> Option<Self> {
        let mut definitions = BTreeMap::new();
        for &site in &region.body {
            let action = plan.blocks().get(site as usize)?.action();
            let output = match action {
                Action::Scalar(Scalar::Copy { output, .. } | Scalar::Primitive { output, .. })
                | Action::LibraryCall { output, .. } => Some(output.get()),
                _ => None,
            };
            if let Some(output) = output {
                if definitions.insert(output, (site, action)).is_some() {
                    return None;
                }
            }
        }
        Some(Self {
            plan,
            region,
            outer,
            definitions,
            used: BTreeSet::new(),
        })
    }
    fn kind(&self, value: u32, kind: NativeScalarKind) -> bool {
        self.plan
            .values()
            .get(value as usize)
            .and_then(|v| v.scalar_kind())
            == Some(kind)
    }
    pub fn index(&mut self, value: u32, depth: usize) -> Option<Index> {
        if depth > 32 || !self.kind(value, NativeScalarKind::U64) {
            return None;
        }
        if value == self.region.element {
            return Some(Index::leaf(Expr::Coordinate(0), Some(0)));
        }
        if self.outer == Some(value) {
            return Some(Index::leaf(Expr::Coordinate(1), Some(1)));
        }
        if let Some(n) = constant(self.plan, value) {
            return Some(Index::leaf(Expr::Constant(n), None));
        }
        if let Some(&(site, action)) = self.definitions.get(&value) {
            self.used.insert(site);
            return match action {
                Action::Scalar(Scalar::Copy { source, .. }) => self.index(source.get(), depth + 1),
                Action::Scalar(Scalar::Primitive { op, operands, .. })
                    if matches!(
                        op,
                        NativeScalarOp::U64Add | NativeScalarOp::U64Sub | NativeScalarOp::U64Mul
                    ) && operands.len() == 2 =>
                {
                    let left = self.index(operands[0].get(), depth + 1)?;
                    let right = self.index(operands[1].get(), depth + 1)?;
                    Index::binary(*op, left, right)
                }
                _ => None,
            };
        }
        self.region
            .captures
            .contains(&value)
            .then(|| Index::leaf(Expr::Capture(value), None))
    }
    pub(super) fn read(&mut self, value: u32, depth: usize) -> Option<Operand> {
        if depth > 32 || !self.kind(value, NativeScalarKind::F64) {
            return None;
        }
        let &(site, action) = self.definitions.get(&value)?;
        self.used.insert(site);
        if let Action::Scalar(Scalar::Copy { source, .. }) = action {
            return self.read(source.get(), depth + 1);
        }
        let read = self
            .region
            .reads
            .iter()
            .find(|r| r.site == site && r.output == value)?;
        let matrix = self.region.matrices.get(read.matrix)?;
        let definition = self.plan.values().get(matrix.value as usize)?.definition();
        let SchemaBody::Matrix {
            element,
            dimensions,
        } = definition.body()
        else {
            return None;
        };
        if matrix.width != 8
            || dimensions.len() != 2
            || element.as_ref() != &SchemaBody::FloatingPoint(mech_core::FloatWidth::W64)
        {
            return None;
        }
        let Action::LibraryCall {
            template,
            operands,
            output,
        } = action
        else {
            return None;
        };
        if output.get() != value
            || operands.first()?.get() != matrix.value
            || operands[1..].iter().map(|v| v.get()).collect::<Vec<_>>() != read.indices
            || !matches!(
                (template.primitive(), operands.len()),
                (Primitive::MatrixScalarAccess, 2) | (Primitive::MatrixCellAccess, 3)
            )
            || template.inputs().len() != operands.len()
            || template.inputs()[0] != *definition
            || template.output().body() != element.as_ref()
        {
            return None;
        }
        let indices = read
            .indices
            .iter()
            .map(|&v| self.index(v, depth + 1))
            .collect::<Option<Vec<_>>>()?;
        Some(Operand {
            matrix: read.matrix,
            indices,
        })
    }
    pub fn product(&mut self, value: u32, depth: usize) -> Option<[Operand; 2]> {
        if depth > 32 || !self.kind(value, NativeScalarKind::F64) {
            return None;
        }
        let &(site, action) = self.definitions.get(&value)?;
        self.used.insert(site);
        match action {
            Action::Scalar(Scalar::Copy { source, .. }) => self.product(source.get(), depth + 1),
            Action::Scalar(Scalar::Primitive {
                op: NativeScalarOp::F64Mul,
                operands,
                ..
            }) if operands.len() == 2 => Some([
                self.read(operands[0].get(), depth + 1)?,
                self.read(operands[1].get(), depth + 1)?,
            ]),
            _ => None,
        }
    }
}

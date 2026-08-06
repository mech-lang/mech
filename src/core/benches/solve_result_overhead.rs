#![allow(clippy::result_large_err, clippy::vec_box)]

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use mech_core::{GenericError, MResult, MechError, Ref};
use nalgebra::DMatrix;
use std::{
    cell::Cell,
    collections::{BTreeSet, HashMap, HashSet},
    hint::black_box,
    time::Duration,
};

const TRIVIAL_NODE_COUNT: usize = 1_000_000;
const DISPATCH_CALL_COUNT: usize = 10_000_000;
const MIXED_WIDTH: usize = 64;
const MIXED_DEPTH: usize = 64;
const MATRIX_NODE_COUNT: usize = 16;
const MATRIX_SIDE: usize = 64;

type CellId = u64;
type NodeId = usize;

trait VoidSolve {
    fn solve(&self);
}

trait ResultSolve {
    fn solve_result(&self) -> MResult<()>;
}

trait InfallibleSolve {
    fn solve_infallible(&self);
}

trait FallibleSolve {
    fn solve_fallible(&self) -> MResult<()>;
}

enum TypedNode {
    Infallible(Box<dyn InfallibleSolve>),
    Fallible(Box<dyn FallibleSolve>),
}

impl TypedNode {
    #[inline(always)]
    fn solve_result(&self) -> MResult<()> {
        match self {
            Self::Infallible(node) => {
                node.solve_infallible();
                Ok(())
            }
            Self::Fallible(node) => node.solve_fallible(),
        }
    }
}

#[derive(Clone, Copy)]
enum TypedNodeRef<'a> {
    Infallible(&'a dyn InfallibleSolve),
    Fallible(&'a dyn FallibleSolve),
}

impl TypedNodeRef<'_> {
    #[inline(always)]
    fn solve_result(&self) -> MResult<()> {
        match self {
            Self::Infallible(node) => {
                node.solve_infallible();
                Ok(())
            }
            Self::Fallible(node) => node.solve_fallible(),
        }
    }
}

struct DispatchKernel {
    value: Cell<u64>,
}

impl DispatchKernel {
    #[inline(never)]
    fn tick(&self) {
        self.value.set(black_box(self.value.get()).wrapping_add(1));
    }
}

impl VoidSolve for DispatchKernel {
    fn solve(&self) {
        self.tick();
    }
}

impl ResultSolve for DispatchKernel {
    fn solve_result(&self) -> MResult<()> {
        self.tick();
        Ok(())
    }
}

impl InfallibleSolve for DispatchKernel {
    fn solve_infallible(&self) {
        self.tick();
    }
}

impl FallibleSolve for DispatchKernel {
    fn solve_fallible(&self) -> MResult<()> {
        self.tick();
        Ok(())
    }
}

#[inline(never)]
fn dispatch_direct(node: &DispatchKernel, calls: usize) -> u64 {
    for _ in 0..calls {
        node.tick();
    }
    node.value.get()
}

#[inline(never)]
fn dispatch_void(node: &dyn VoidSolve, calls: usize) -> u64 {
    for _ in 0..calls {
        node.solve();
    }
    calls as u64
}

#[inline(never)]
fn dispatch_result(node: &dyn ResultSolve, calls: usize) -> MResult<u64> {
    for _ in 0..calls {
        node.solve_result()?;
    }
    Ok(calls as u64)
}

#[inline(never)]
fn dispatch_typed(node: TypedNodeRef<'_>, calls: usize) -> MResult<u64> {
    for _ in 0..calls {
        node.solve_result()?;
    }
    Ok(calls as u64)
}

struct TrivialScalarKernel {
    input: Ref<f64>,
    output: Ref<f64>,
}

impl TrivialScalarKernel {
    #[inline(always)]
    fn solve_core(&self) {
        let next = *self.input.borrow() + 1.0;
        *self.output.borrow_mut() = next;
    }
}

impl VoidSolve for TrivialScalarKernel {
    fn solve(&self) {
        self.solve_core();
    }
}

impl ResultSolve for TrivialScalarKernel {
    fn solve_result(&self) -> MResult<()> {
        self.solve_core();
        Ok(())
    }
}

impl InfallibleSolve for TrivialScalarKernel {
    fn solve_infallible(&self) {
        self.solve_core();
    }
}

fn trivial_fixture(count: usize) -> (Vec<Box<TrivialScalarKernel>>, Ref<f64>) {
    let mut nodes = Vec::with_capacity(count);
    let mut input = Ref::new(1.0);
    for _ in 0..count {
        let output = Ref::new(0.0);
        nodes.push(Box::new(TrivialScalarKernel {
            input,
            output: output.clone(),
        }));
        input = output;
    }
    (nodes, input)
}

#[inline(never)]
fn run_void_nodes(nodes: &[&dyn VoidSolve]) -> usize {
    for node in nodes {
        node.solve();
    }
    nodes.len()
}

#[inline(never)]
fn run_result_nodes(nodes: &[&dyn ResultSolve]) -> MResult<usize> {
    for node in nodes {
        node.solve_result()?;
    }
    Ok(nodes.len())
}

#[inline(never)]
fn run_typed_nodes(nodes: &[TypedNodeRef<'_>]) -> MResult<usize> {
    for node in nodes {
        node.solve_result()?;
    }
    Ok(nodes.len())
}

struct GraphNode<N> {
    execution: N,
    outputs: Vec<CellId>,
}

struct Graph<N> {
    nodes: Vec<GraphNode<N>>,
    consumers: HashMap<CellId, Vec<NodeId>>,
}

impl<N> Graph<N> {
    fn from_parts(parts: Vec<(N, Vec<CellId>, Vec<CellId>)>) -> Self {
        let mut nodes = Vec::with_capacity(parts.len());
        let mut consumers: HashMap<CellId, Vec<NodeId>> = HashMap::new();
        for (id, (execution, inputs, outputs)) in parts.into_iter().enumerate() {
            for input in inputs {
                consumers.entry(input).or_default().push(id);
            }
            nodes.push(GraphNode { execution, outputs });
        }
        Self { nodes, consumers }
    }
}

struct SchedulerRun<'a, N> {
    graph: &'a Graph<N>,
    work: BTreeSet<NodeId>,
    processed: HashSet<NodeId>,
    executed_nodes: Vec<NodeId>,
    changed_nodes: Vec<NodeId>,
    dirty_cells: Vec<CellId>,
}

impl<'a, N> SchedulerRun<'a, N> {
    fn new(graph: &'a Graph<N>, dirty_cells: &[CellId]) -> Self {
        let dirty_cells = dirty_cells.iter().copied().collect::<HashSet<_>>();
        let mut work = BTreeSet::new();
        for cell in dirty_cells {
            if let Some(consumers) = graph.consumers.get(&cell) {
                work.extend(consumers.iter().copied());
            }
        }
        Self {
            graph,
            work,
            processed: HashSet::new(),
            executed_nodes: Vec::new(),
            changed_nodes: Vec::new(),
            dirty_cells: Vec::new(),
        }
    }

    #[inline(always)]
    fn pop(&mut self) -> Option<NodeId> {
        while let Some(node) = self.work.pop_first() {
            if self.processed.insert(node) {
                return Some(node);
            }
        }
        None
    }

    #[inline(always)]
    fn mark_changed(&mut self, node: NodeId) {
        self.executed_nodes.push(node);
        self.changed_nodes.push(node);
        for output in self.graph.nodes[node].outputs.iter().copied() {
            self.dirty_cells.push(output);
            if let Some(consumers) = self.graph.consumers.get(&output) {
                self.work.extend(consumers.iter().copied());
            }
        }
    }

    fn finish(self) -> SchedulerOutcome {
        SchedulerOutcome {
            executed_nodes: self.executed_nodes,
            changed_nodes: self.changed_nodes,
            dirty_cells: self.dirty_cells,
        }
    }
}

struct SchedulerOutcome {
    executed_nodes: Vec<NodeId>,
    changed_nodes: Vec<NodeId>,
    dirty_cells: Vec<CellId>,
}

impl SchedulerOutcome {
    fn checksum(&self) -> usize {
        self.executed_nodes.len() + self.changed_nodes.len() + self.dirty_cells.len()
    }
}

#[inline(never)]
fn run_void_graph(graph: &Graph<Box<dyn VoidSolve>>, dirty_cells: &[CellId]) -> SchedulerOutcome {
    let mut run = SchedulerRun::new(graph, dirty_cells);
    while let Some(node) = run.pop() {
        graph.nodes[node].execution.solve();
        run.mark_changed(node);
    }
    run.finish()
}

#[inline(never)]
fn run_result_graph(
    graph: &Graph<Box<dyn ResultSolve>>,
    dirty_cells: &[CellId],
) -> MResult<SchedulerOutcome> {
    let mut run = SchedulerRun::new(graph, dirty_cells);
    while let Some(node) = run.pop() {
        graph.nodes[node].execution.solve_result()?;
        run.mark_changed(node);
    }
    Ok(run.finish())
}

#[inline(never)]
fn run_typed_graph(graph: &Graph<TypedNode>, dirty_cells: &[CellId]) -> MResult<SchedulerOutcome> {
    let mut run = SchedulerRun::new(graph, dirty_cells);
    while let Some(node) = run.pop() {
        graph.nodes[node].execution.solve_result()?;
        run.mark_changed(node);
    }
    Ok(run.finish())
}

#[derive(Clone, Copy)]
enum ScalarOperation {
    Add,
    Affine,
    Difference,
    Average,
}

#[inline(always)]
fn solve_scalar(operation: ScalarOperation, lhs: &Ref<f64>, rhs: &Ref<f64>, output: &Ref<f64>) {
    let lhs = *lhs.borrow();
    let rhs = *rhs.borrow();
    let next = match operation {
        ScalarOperation::Add => lhs + rhs,
        ScalarOperation::Affine => lhs.mul_add(0.75, rhs * 0.25),
        ScalarOperation::Difference => lhs - rhs,
        ScalarOperation::Average => (lhs + rhs) * 0.5,
    };
    *output.borrow_mut() = next;
}

struct InfallibleScalarKernel {
    lhs: Ref<f64>,
    rhs: Ref<f64>,
    output: Ref<f64>,
    operation: ScalarOperation,
}

impl InfallibleScalarKernel {
    fn solve_core(&self) {
        solve_scalar(self.operation, &self.lhs, &self.rhs, &self.output);
    }
}

impl VoidSolve for InfallibleScalarKernel {
    fn solve(&self) {
        self.solve_core();
    }
}

impl ResultSolve for InfallibleScalarKernel {
    fn solve_result(&self) -> MResult<()> {
        self.solve_core();
        Ok(())
    }
}

impl InfallibleSolve for InfallibleScalarKernel {
    fn solve_infallible(&self) {
        self.solve_core();
    }
}

struct FallibleScalarKernel {
    lhs: Ref<f64>,
    rhs: Ref<f64>,
    output: Ref<f64>,
    operation: ScalarOperation,
}

impl FallibleScalarKernel {
    fn solve_checked(&self) -> MResult<()> {
        if !self.lhs.borrow().is_finite() || !self.rhs.borrow().is_finite() {
            return Err(MechError::new(
                GenericError {
                    msg: "non-finite benchmark input".into(),
                },
                None,
            ));
        }
        solve_scalar(self.operation, &self.lhs, &self.rhs, &self.output);
        Ok(())
    }
}

impl VoidSolve for FallibleScalarKernel {
    fn solve(&self) {
        self.solve_checked()
            .expect("mixed benchmark inputs remain finite");
    }
}

impl ResultSolve for FallibleScalarKernel {
    fn solve_result(&self) -> MResult<()> {
        self.solve_checked()
    }
}

impl FallibleSolve for FallibleScalarKernel {
    fn solve_fallible(&self) -> MResult<()> {
        self.solve_checked()
    }
}

fn build_mixed_graph<N>(
    mut infallible: impl FnMut(InfallibleScalarKernel) -> N,
    mut fallible: impl FnMut(FallibleScalarKernel) -> N,
) -> (Graph<N>, Vec<CellId>, Ref<f64>) {
    let root_cells = (0..MIXED_WIDTH)
        .map(|column| Ref::new(0.25 + column as f64 / MIXED_WIDTH as f64))
        .collect::<Vec<_>>();
    let mut cells = root_cells;
    let mut parts = Vec::with_capacity(MIXED_WIDTH * MIXED_DEPTH);
    for layer in 0..MIXED_DEPTH {
        let input_start = layer * MIXED_WIDTH;
        for column in 0..MIXED_WIDTH {
            let index = layer * MIXED_WIDTH + column;
            let lhs_index = input_start + column;
            let rhs_index = input_start + (column * 5 + layer + 1) % MIXED_WIDTH;
            let output_index = MIXED_WIDTH + index;
            let output = Ref::new(0.0);
            let operation = match index % 4 {
                0 => ScalarOperation::Add,
                1 => ScalarOperation::Affine,
                2 => ScalarOperation::Difference,
                _ => ScalarOperation::Average,
            };
            let execution = if index.is_multiple_of(16) {
                fallible(FallibleScalarKernel {
                    lhs: cells[lhs_index].clone(),
                    rhs: cells[rhs_index].clone(),
                    output: output.clone(),
                    operation,
                })
            } else {
                infallible(InfallibleScalarKernel {
                    lhs: cells[lhs_index].clone(),
                    rhs: cells[rhs_index].clone(),
                    output: output.clone(),
                    operation,
                })
            };
            parts.push((
                execution,
                vec![lhs_index as CellId, rhs_index as CellId],
                vec![output_index as CellId],
            ));
            cells.push(output);
        }
    }
    let dirty_cells = (0..MIXED_WIDTH as CellId).collect::<Vec<_>>();
    let last = cells.last().expect("mixed graph has outputs").clone();
    (Graph::from_parts(parts), dirty_cells, last)
}

struct MatrixKernel {
    lhs: Ref<DMatrix<f64>>,
    rhs: Ref<DMatrix<f64>>,
    output: Ref<DMatrix<f64>>,
}

impl MatrixKernel {
    #[inline(always)]
    fn solve_core(&self) {
        let lhs = self.lhs.as_ptr();
        let rhs = self.rhs.as_ptr();
        let output = self.output.as_mut_ptr();
        unsafe {
            (&*lhs).mul_to(&*rhs, &mut *output);
        }
    }
}

impl VoidSolve for MatrixKernel {
    fn solve(&self) {
        self.solve_core();
    }
}

impl ResultSolve for MatrixKernel {
    fn solve_result(&self) -> MResult<()> {
        self.solve_core();
        Ok(())
    }
}

impl InfallibleSolve for MatrixKernel {
    fn solve_infallible(&self) {
        self.solve_core();
    }
}

fn build_matrix_graph<N>(
    mut wrap: impl FnMut(MatrixKernel) -> N,
) -> (Graph<N>, Vec<CellId>, Ref<DMatrix<f64>>) {
    let lhs = Ref::new(DMatrix::from_fn(MATRIX_SIDE, MATRIX_SIDE, |row, column| {
        if row == column { 0.99 } else { 0.0001 }
    }));
    let rhs = Ref::new(DMatrix::from_fn(MATRIX_SIDE, MATRIX_SIDE, |row, column| {
        if row == column {
            1.0
        } else {
            ((row + column) % 7) as f64 * 0.00001
        }
    }));
    let mut previous = lhs;
    let mut parts = Vec::with_capacity(MATRIX_NODE_COUNT);
    for node in 0..MATRIX_NODE_COUNT {
        let output = Ref::new(DMatrix::zeros(MATRIX_SIDE, MATRIX_SIDE));
        parts.push((
            wrap(MatrixKernel {
                lhs: previous,
                rhs: rhs.clone(),
                output: output.clone(),
            }),
            vec![node as CellId],
            vec![(node + 1) as CellId],
        ));
        previous = output;
    }
    (Graph::from_parts(parts), vec![0], previous)
}

fn dynamic_dispatch_only(c: &mut Criterion) {
    let node = DispatchKernel {
        value: Cell::new(0),
    };
    let mut group = c.benchmark_group("solve_contract/dynamic_dispatch_only");
    group.throughput(Throughput::Elements(DISPATCH_CALL_COUNT as u64));
    group.bench_function("direct_concrete", |b| {
        b.iter(|| black_box(dispatch_direct(black_box(&node), DISPATCH_CALL_COUNT)))
    });
    group.bench_function("old_void_vtable", |b| {
        let node: &dyn VoidSolve = black_box(&node);
        b.iter(|| black_box(dispatch_void(node, DISPATCH_CALL_COUNT)))
    });
    group.bench_function("current_mresult_vtable", |b| {
        let node: &dyn ResultSolve = black_box(&node);
        b.iter(|| black_box(dispatch_result(node, DISPATCH_CALL_COUNT).unwrap()))
    });
    group.bench_function("typed_infallible_vtable", |b| {
        let node = TypedNodeRef::Infallible(black_box(&node));
        b.iter(|| black_box(dispatch_typed(black_box(node), DISPATCH_CALL_COUNT).unwrap()))
    });
    group.bench_function("typed_fallible_vtable", |b| {
        let node = TypedNodeRef::Fallible(black_box(&node));
        b.iter(|| black_box(dispatch_typed(black_box(node), DISPATCH_CALL_COUNT).unwrap()))
    });
    group.finish();
}

fn one_million_trivial_scalars(c: &mut Criterion) {
    let (kernels, last) = trivial_fixture(TRIVIAL_NODE_COUNT);
    let mut group = c.benchmark_group("solve_contract/trivial_scalar_1m");
    group.throughput(Throughput::Elements(TRIVIAL_NODE_COUNT as u64));

    {
        let nodes = kernels
            .iter()
            .map(|node| node.as_ref() as &dyn VoidSolve)
            .collect::<Vec<_>>();
        assert_eq!(run_void_nodes(&nodes), TRIVIAL_NODE_COUNT);
        assert_eq!(*last.borrow(), TRIVIAL_NODE_COUNT as f64 + 1.0);
        group.bench_function("old_void", |b| {
            b.iter(|| black_box((run_void_nodes(black_box(&nodes)), *last.borrow())))
        });
    }

    {
        let nodes = kernels
            .iter()
            .map(|node| node.as_ref() as &dyn ResultSolve)
            .collect::<Vec<_>>();
        assert_eq!(run_result_nodes(&nodes).unwrap(), TRIVIAL_NODE_COUNT);
        group.bench_function("current_mresult", |b| {
            b.iter(|| black_box((run_result_nodes(black_box(&nodes)).unwrap(), *last.borrow())))
        });
    }

    {
        let nodes = kernels
            .iter()
            .map(|node| TypedNodeRef::Infallible(node.as_ref()))
            .collect::<Vec<_>>();
        assert_eq!(run_typed_nodes(&nodes).unwrap(), TRIVIAL_NODE_COUNT);
        group.bench_function("typed_split", |b| {
            b.iter(|| black_box((run_typed_nodes(black_box(&nodes)).unwrap(), *last.borrow())))
        });
    }
    group.finish();
}

fn realistic_mixed_reactive_graph(c: &mut Criterion) {
    let (void_graph, dirty, void_last) = build_mixed_graph(
        |node| Box::new(node) as Box<dyn VoidSolve>,
        |node| Box::new(node) as Box<dyn VoidSolve>,
    );
    let (result_graph, result_dirty, result_last) = build_mixed_graph(
        |node| Box::new(node) as Box<dyn ResultSolve>,
        |node| Box::new(node) as Box<dyn ResultSolve>,
    );
    let (typed_graph, typed_dirty, typed_last) = build_mixed_graph(
        |node| TypedNode::Infallible(Box::new(node)),
        |node| TypedNode::Fallible(Box::new(node)),
    );
    assert_eq!(dirty, result_dirty);
    assert_eq!(dirty, typed_dirty);
    let expected = MIXED_WIDTH * MIXED_DEPTH * 3;
    assert_eq!(run_void_graph(&void_graph, &dirty).checksum(), expected);
    assert_eq!(
        run_result_graph(&result_graph, &dirty).unwrap().checksum(),
        expected
    );
    assert_eq!(
        run_typed_graph(&typed_graph, &dirty).unwrap().checksum(),
        expected
    );

    let mut group = c.benchmark_group("solve_contract/realistic_mixed_reactive_graph");
    group.throughput(Throughput::Elements((MIXED_WIDTH * MIXED_DEPTH) as u64));
    group.bench_function("old_void", |b| {
        b.iter(|| {
            let outcome = run_void_graph(black_box(&void_graph), black_box(&dirty));
            black_box((outcome, *void_last.borrow()))
        })
    });
    group.bench_function("current_mresult", |b| {
        b.iter(|| {
            let outcome = run_result_graph(black_box(&result_graph), black_box(&dirty)).unwrap();
            black_box((outcome, *result_last.borrow()))
        })
    });
    group.bench_function("typed_split", |b| {
        b.iter(|| {
            let outcome = run_typed_graph(black_box(&typed_graph), black_box(&dirty)).unwrap();
            black_box((outcome, *typed_last.borrow()))
        })
    });
    group.finish();
}

fn matrix_heavy_graph(c: &mut Criterion) {
    let (void_graph, dirty, void_last) =
        build_matrix_graph(|node| Box::new(node) as Box<dyn VoidSolve>);
    let (result_graph, result_dirty, result_last) =
        build_matrix_graph(|node| Box::new(node) as Box<dyn ResultSolve>);
    let (typed_graph, typed_dirty, typed_last) =
        build_matrix_graph(|node| TypedNode::Infallible(Box::new(node)));
    assert_eq!(dirty, result_dirty);
    assert_eq!(dirty, typed_dirty);
    let expected = MATRIX_NODE_COUNT * 3;
    assert_eq!(run_void_graph(&void_graph, &dirty).checksum(), expected);
    assert_eq!(
        run_result_graph(&result_graph, &dirty).unwrap().checksum(),
        expected
    );
    assert_eq!(
        run_typed_graph(&typed_graph, &dirty).unwrap().checksum(),
        expected
    );

    let mut group = c.benchmark_group("solve_contract/matrix_heavy_graph");
    group.throughput(Throughput::Elements(MATRIX_NODE_COUNT as u64));
    group.bench_function("old_void", |b| {
        b.iter(|| {
            let outcome = run_void_graph(black_box(&void_graph), black_box(&dirty));
            black_box((outcome, void_last.borrow()[(0, 0)]))
        })
    });
    group.bench_function("current_mresult", |b| {
        b.iter(|| {
            let outcome = run_result_graph(black_box(&result_graph), black_box(&dirty)).unwrap();
            black_box((outcome, result_last.borrow()[(0, 0)]))
        })
    });
    group.bench_function("typed_split", |b| {
        b.iter(|| {
            let outcome = run_typed_graph(black_box(&typed_graph), black_box(&dirty)).unwrap();
            black_box((outcome, typed_last.borrow()[(0, 0)]))
        })
    });
    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(30)
        .warm_up_time(Duration::from_secs(2))
        .measurement_time(Duration::from_secs(5));
    targets = dynamic_dispatch_only,
        one_million_trivial_scalars,
        realistic_mixed_reactive_graph,
        matrix_heavy_graph
}
criterion_main!(benches);

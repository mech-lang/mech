use crate::*;

#[cfg(feature = "no_std")]
use alloc::collections::BTreeSet;
#[cfg(feature = "no_std")]
use hashbrown::HashMap;

#[cfg(not(feature = "no_std"))]
use std::collections::{BTreeSet, HashMap};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ValueCellId(usize);

impl ValueCellId {
    pub fn as_usize(self) -> usize {
        self.0
    }
}

pub fn value_cell_id(value: &Value) -> Option<ValueCellId> {
    match value {
        #[cfg(feature = "u8")]
        Value::U8(value) => Some(ValueCellId(value.addr())),
        #[cfg(feature = "u16")]
        Value::U16(value) => Some(ValueCellId(value.addr())),
        #[cfg(feature = "u32")]
        Value::U32(value) => Some(ValueCellId(value.addr())),
        #[cfg(feature = "u64")]
        Value::U64(value) => Some(ValueCellId(value.addr())),
        #[cfg(feature = "u128")]
        Value::U128(value) => Some(ValueCellId(value.addr())),
        #[cfg(feature = "i8")]
        Value::I8(value) => Some(ValueCellId(value.addr())),
        #[cfg(feature = "i16")]
        Value::I16(value) => Some(ValueCellId(value.addr())),
        #[cfg(feature = "i32")]
        Value::I32(value) => Some(ValueCellId(value.addr())),
        #[cfg(feature = "i64")]
        Value::I64(value) => Some(ValueCellId(value.addr())),
        #[cfg(feature = "i128")]
        Value::I128(value) => Some(ValueCellId(value.addr())),
        #[cfg(feature = "f32")]
        Value::F32(value) => Some(ValueCellId(value.addr())),
        #[cfg(feature = "f64")]
        Value::F64(value) => Some(ValueCellId(value.addr())),
        #[cfg(any(feature = "string", feature = "variable_define"))]
        Value::String(value) => Some(ValueCellId(value.addr())),
        #[cfg(any(feature = "bool", feature = "variable_define"))]
        Value::Bool(value) => Some(ValueCellId(value.addr())),
        #[cfg(feature = "atom")]
        Value::Atom(value) => Some(ValueCellId(value.addr())),
        #[cfg(feature = "matrix")]
        Value::MatrixIndex(value) => Some(ValueCellId(value.addr())),
        #[cfg(all(feature = "matrix", feature = "bool"))]
        Value::MatrixBool(value) => Some(ValueCellId(value.addr())),
        #[cfg(all(feature = "matrix", feature = "u8"))]
        Value::MatrixU8(value) => Some(ValueCellId(value.addr())),
        #[cfg(all(feature = "matrix", feature = "u16"))]
        Value::MatrixU16(value) => Some(ValueCellId(value.addr())),
        #[cfg(all(feature = "matrix", feature = "u32"))]
        Value::MatrixU32(value) => Some(ValueCellId(value.addr())),
        #[cfg(all(feature = "matrix", feature = "u64"))]
        Value::MatrixU64(value) => Some(ValueCellId(value.addr())),
        #[cfg(all(feature = "matrix", feature = "u128"))]
        Value::MatrixU128(value) => Some(ValueCellId(value.addr())),
        #[cfg(all(feature = "matrix", feature = "i8"))]
        Value::MatrixI8(value) => Some(ValueCellId(value.addr())),
        #[cfg(all(feature = "matrix", feature = "i16"))]
        Value::MatrixI16(value) => Some(ValueCellId(value.addr())),
        #[cfg(all(feature = "matrix", feature = "i32"))]
        Value::MatrixI32(value) => Some(ValueCellId(value.addr())),
        #[cfg(all(feature = "matrix", feature = "i64"))]
        Value::MatrixI64(value) => Some(ValueCellId(value.addr())),
        #[cfg(all(feature = "matrix", feature = "i128"))]
        Value::MatrixI128(value) => Some(ValueCellId(value.addr())),
        #[cfg(all(feature = "matrix", feature = "f32"))]
        Value::MatrixF32(value) => Some(ValueCellId(value.addr())),
        #[cfg(all(feature = "matrix", feature = "f64"))]
        Value::MatrixF64(value) => Some(ValueCellId(value.addr())),
        #[cfg(all(feature = "matrix", feature = "string"))]
        Value::MatrixString(value) => Some(ValueCellId(value.addr())),
        #[cfg(all(feature = "matrix", feature = "rational"))]
        Value::MatrixR64(value) => Some(ValueCellId(value.addr())),
        #[cfg(all(feature = "matrix", feature = "complex"))]
        Value::MatrixC64(value) => Some(ValueCellId(value.addr())),
        #[cfg(feature = "matrix")]
        Value::MatrixValue(value) => Some(ValueCellId(value.addr())),
        #[cfg(feature = "complex")]
        Value::C64(value) => Some(ValueCellId(value.addr())),
        #[cfg(feature = "rational")]
        Value::R64(value) => Some(ValueCellId(value.addr())),
        #[cfg(feature = "set")]
        Value::Set(value) => Some(ValueCellId(value.addr())),
        #[cfg(feature = "map")]
        Value::Map(value) => Some(ValueCellId(value.addr())),
        #[cfg(feature = "record")]
        Value::Record(value) => Some(ValueCellId(value.addr())),
        #[cfg(feature = "table")]
        Value::Table(value) => Some(ValueCellId(value.addr())),
        #[cfg(feature = "tuple")]
        Value::Tuple(value) => Some(ValueCellId(value.addr())),
        #[cfg(feature = "enum")]
        Value::Enum(value) => Some(ValueCellId(value.addr())),
        Value::Index(value) => Some(ValueCellId(value.addr())),
        Value::MutableReference(value) => value_cell_id(&value.borrow()),
        Value::Typed(value, _) => value_cell_id(value),
        Value::Id(_) => None,
        Value::Kind(_) => None,
        Value::IndexAll => None,
        Value::EmptyKind(_) => None,
        Value::Empty => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlanNodeId(usize);

impl PlanNodeId {
    pub fn as_usize(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PlanInputMode {
    Reactive,
    Trigger,
    Sampled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlanInput {
    pub cell: ValueCellId,
    pub mode: PlanInputMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PlanInvalidationKind {
    Changed,
    Triggered,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlanInvalidation {
    pub cell: ValueCellId,
    pub kind: PlanInvalidationKind,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlanNodeSpec {
    pub inputs: Vec<PlanInput>,
    pub outputs: Vec<ValueCellId>,
}

impl PlanNodeSpec {
    pub fn explicit(inputs: Vec<PlanInput>, outputs: Vec<ValueCellId>) -> Self {
        let mut seen_inputs = BTreeSet::new();
        let mut normalized_inputs = Vec::new();
        for input in inputs {
            if seen_inputs.insert(input) {
                normalized_inputs.push(input);
            }
        }

        let mut seen_outputs = BTreeSet::new();
        let mut normalized_outputs = Vec::new();
        for output in outputs {
            if seen_outputs.insert(output) {
                normalized_outputs.push(output);
            }
        }

        Self {
            inputs: normalized_inputs,
            outputs: normalized_outputs,
        }
    }

    pub fn reactive(arguments: &[Value], output: &Value) -> MResult<Self> {
        let inputs = arguments
            .iter()
            .filter_map(|argument| {
                value_cell_id(argument).map(|cell| PlanInput {
                    cell,
                    mode: PlanInputMode::Reactive,
                })
            })
            .collect::<Vec<_>>();
        let outputs = value_cell_id(output).into_iter().collect::<Vec<_>>();
        let spec = Self::explicit(inputs, outputs);

        for input in &spec.inputs {
            if spec.outputs.contains(&input.cell) {
                return Err(MechError::new(
                    PlanInputOutputOverlap { cell: input.cell },
                    None,
                ));
            }
        }

        Ok(spec)
    }

    pub fn assignment(source: &Value, sink: &Value) -> MResult<Self> {
        let source_cell = required_cell("assignment source", source)?;
        let sink_cell = required_cell("assignment sink", sink)?;

        let input = if source_cell == sink_cell {
            PlanInput {
                cell: sink_cell,
                mode: PlanInputMode::Sampled,
            }
        } else {
            PlanInput {
                cell: source_cell,
                mode: PlanInputMode::Reactive,
            }
        };

        Ok(Self::explicit(vec![input], vec![sink_cell]))
    }

    pub fn read_modify_write(sources: &[Value], sink: &Value) -> MResult<Self> {
        let sink_cell = required_cell("read-modify-write sink", sink)?;
        let mut inputs = Vec::new();

        for source in sources {
            if let Some(source_cell) = value_cell_id(source) {
                inputs.push(PlanInput {
                    cell: source_cell,
                    mode: if source_cell == sink_cell {
                        PlanInputMode::Sampled
                    } else {
                        PlanInputMode::Reactive
                    },
                });
            }
        }

        inputs.push(PlanInput {
            cell: sink_cell,
            mode: PlanInputMode::Sampled,
        });

        Ok(Self::explicit(inputs, vec![sink_cell]))
    }
}

fn required_cell(role: &'static str, value: &Value) -> MResult<ValueCellId> {
    value_cell_id(value).ok_or_else(|| {
        MechError::new(
            PlanCellIdentityMissing {
                role,
                kind: value.kind().to_string(),
            },
            None,
        )
    })
}

#[derive(Clone, Debug)]
pub struct PlanCellIdentityMissing {
    pub role: &'static str,
    pub kind: String,
}

impl MechErrorKind for PlanCellIdentityMissing {
    fn name(&self) -> &str {
        "PlanCellIdentityMissing"
    }

    fn message(&self) -> String {
        format!(
            "planner {} value of kind {} has no stable cell identity",
            self.role, self.kind,
        )
    }
}

#[derive(Clone, Debug)]
pub struct PlanInputOutputOverlap {
    pub cell: ValueCellId,
}

impl MechErrorKind for PlanInputOutputOverlap {
    fn name(&self) -> &str {
        "PlanInputOutputOverlap"
    }

    fn message(&self) -> String {
        format!(
            "pure reactive plan node uses cell {} as both input and output",
            self.cell.as_usize(),
        )
    }
}

#[derive(Clone, Debug)]
pub struct PlanDependencyCycle {
    pub nodes: Vec<PlanNodeId>,
}

impl MechErrorKind for PlanDependencyCycle {
    fn name(&self) -> &str {
        "PlanDependencyCycle"
    }

    fn message(&self) -> String {
        let nodes = self
            .nodes
            .iter()
            .map(|node| node.as_usize().to_string())
            .collect::<Vec<_>>()
            .join(", ");

        format!("planner dependency cycle contains nodes [{}]", nodes,)
    }
}

#[derive(Clone, Debug, Default)]
pub struct PlanGraph {
    nodes: Vec<PlanNodeSpec>,
    reactive_consumers: HashMap<ValueCellId, Vec<PlanNodeId>>,
    trigger_consumers: HashMap<ValueCellId, Vec<PlanNodeId>>,
    producers: HashMap<ValueCellId, Vec<PlanNodeId>>,
}

impl PlanGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn clear(&mut self) {
        self.nodes.clear();
        self.reactive_consumers.clear();
        self.trigger_consumers.clear();
        self.producers.clear();
    }

    pub fn node_spec(&self, node: PlanNodeId) -> Option<&PlanNodeSpec> {
        self.nodes.get(node.as_usize())
    }

    pub fn add_node(&mut self, spec: PlanNodeSpec) -> PlanNodeId {
        let spec = PlanNodeSpec::explicit(spec.inputs, spec.outputs);
        let id = PlanNodeId(self.nodes.len());

        for input in &spec.inputs {
            match input.mode {
                PlanInputMode::Reactive => self
                    .reactive_consumers
                    .entry(input.cell)
                    .or_default()
                    .push(id),
                PlanInputMode::Trigger => self
                    .trigger_consumers
                    .entry(input.cell)
                    .or_default()
                    .push(id),
                PlanInputMode::Sampled => {}
            }
        }

        for output in &spec.outputs {
            self.producers.entry(*output).or_default().push(id);
        }

        self.nodes.push(spec);
        id
    }

    pub fn schedule_from(
        &self,
        invalidations: &[PlanInvalidation],
    ) -> MResult<PlanScheduleOutcome> {
        let mut pending = BTreeSet::<PlanNodeId>::new();
        let mut affected = vec![false; self.nodes.len()];
        let mut expanded = vec![false; self.nodes.len()];
        let mut unique_initial_cells = BTreeSet::new();
        let mut unique_initial_invalidations = BTreeSet::new();

        for invalidation in invalidations {
            if unique_initial_invalidations.insert(*invalidation) {
                unique_initial_cells.insert(invalidation.cell);
                self.schedule_initial(*invalidation, &mut affected, &mut pending);
            }
        }

        while let Some(node) = pop_first(&mut pending) {
            let index = node.as_usize();
            if expanded.get(index).copied().unwrap_or(true) {
                continue;
            }
            expanded[index] = true;

            if let Some(spec) = self.node_spec(node) {
                for output in &spec.outputs {
                    self.schedule_reactive_consumers(*output, &mut affected, &mut pending);
                }
            }
        }

        let mut adjacency = vec![BTreeSet::<PlanNodeId>::new(); self.nodes.len()];
        let mut indegree = vec![0usize; self.nodes.len()];

        self.add_producer_consumer_edges(&affected, &mut adjacency, &mut indegree);
        self.add_multiple_writer_edges(&affected, &mut adjacency, &mut indegree);

        let ordered_nodes = self.topological_order(&affected, &adjacency, &mut indegree)?;

        Ok(PlanScheduleOutcome {
            invalidated_cells: unique_initial_cells.len(),
            scheduled_nodes: affected.iter().filter(|value| **value).count(),
            ordered_nodes,
        })
    }

    fn schedule_initial(
        &self,
        invalidation: PlanInvalidation,
        affected: &mut [bool],
        pending: &mut BTreeSet<PlanNodeId>,
    ) {
        self.schedule_reactive_consumers(invalidation.cell, affected, pending);
        if invalidation.kind == PlanInvalidationKind::Triggered {
            self.schedule_trigger_consumers(invalidation.cell, affected, pending);
        }
    }

    fn schedule_reactive_consumers(
        &self,
        cell: ValueCellId,
        affected: &mut [bool],
        pending: &mut BTreeSet<PlanNodeId>,
    ) {
        if let Some(consumers) = self.reactive_consumers.get(&cell) {
            for consumer in consumers {
                mark_scheduled(*consumer, affected, pending);
            }
        }
    }

    fn schedule_trigger_consumers(
        &self,
        cell: ValueCellId,
        affected: &mut [bool],
        pending: &mut BTreeSet<PlanNodeId>,
    ) {
        if let Some(consumers) = self.trigger_consumers.get(&cell) {
            for consumer in consumers {
                mark_scheduled(*consumer, affected, pending);
            }
        }
    }

    fn add_producer_consumer_edges(
        &self,
        affected: &[bool],
        adjacency: &mut [BTreeSet<PlanNodeId>],
        indegree: &mut [usize],
    ) {
        for (index, spec) in self.nodes.iter().enumerate() {
            if !affected.get(index).copied().unwrap_or(false) {
                continue;
            }

            let consumer = PlanNodeId(index);
            for input in &spec.inputs {
                if let Some(producers) = self.producers.get(&input.cell) {
                    for producer in producers {
                        add_ordering_edge(*producer, consumer, affected, adjacency, indegree);
                    }
                }
            }
        }
    }

    fn add_multiple_writer_edges(
        &self,
        affected: &[bool],
        adjacency: &mut [BTreeSet<PlanNodeId>],
        indegree: &mut [usize],
    ) {
        for producers in self.producers.values() {
            let affected_producers = producers
                .iter()
                .copied()
                .filter(|producer| affected.get(producer.as_usize()).copied().unwrap_or(false))
                .collect::<Vec<_>>();

            for pair in affected_producers.windows(2) {
                add_ordering_edge(pair[0], pair[1], affected, adjacency, indegree);
            }
        }
    }

    fn topological_order(
        &self,
        affected: &[bool],
        adjacency: &[BTreeSet<PlanNodeId>],
        indegree: &mut [usize],
    ) -> MResult<Vec<PlanNodeId>> {
        let affected_count = affected.iter().filter(|value| **value).count();
        let mut ready = BTreeSet::new();
        for (index, is_affected) in affected.iter().enumerate() {
            if *is_affected && indegree[index] == 0 {
                ready.insert(PlanNodeId(index));
            }
        }

        let mut ordered_nodes = Vec::new();
        while let Some(node) = pop_first(&mut ready) {
            ordered_nodes.push(node);
            for destination in &adjacency[node.as_usize()] {
                let destination_index = destination.as_usize();
                indegree[destination_index] -= 1;
                if indegree[destination_index] == 0 {
                    ready.insert(*destination);
                }
            }
        }

        if ordered_nodes.len() != affected_count {
            let nodes = affected
                .iter()
                .enumerate()
                .filter_map(|(index, is_affected)| {
                    if *is_affected && indegree[index] > 0 {
                        Some(PlanNodeId(index))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            return Err(MechError::new(PlanDependencyCycle { nodes }, None));
        }

        Ok(ordered_nodes)
    }
}

fn add_ordering_edge(
    from: PlanNodeId,
    to: PlanNodeId,
    affected: &[bool],
    adjacency: &mut [BTreeSet<PlanNodeId>],
    indegree: &mut [usize],
) {
    if from == to {
        return;
    }

    let from_index = from.as_usize();
    let to_index = to.as_usize();
    if !affected.get(from_index).copied().unwrap_or(false)
        || !affected.get(to_index).copied().unwrap_or(false)
    {
        return;
    }

    if adjacency[from_index].insert(to) {
        indegree[to_index] += 1;
    }
}

fn mark_scheduled(node: PlanNodeId, affected: &mut [bool], pending: &mut BTreeSet<PlanNodeId>) {
    let Some(is_affected) = affected.get_mut(node.as_usize()) else {
        return;
    };

    if !*is_affected {
        *is_affected = true;
        pending.insert(node);
    }
}

fn pop_first(nodes: &mut BTreeSet<PlanNodeId>) -> Option<PlanNodeId> {
    let node = nodes.iter().next().copied()?;
    nodes.remove(&node);
    Some(node)
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlanScheduleOutcome {
    pub invalidated_cells: usize,
    pub scheduled_nodes: usize,
    pub ordered_nodes: Vec<PlanNodeId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "f64")]
    fn f64_value(value: f64) -> Value {
        Value::F64(Ref::new(value))
    }

    fn cell(value: &Value) -> ValueCellId {
        value_cell_id(value).expect("test value should have a cell id")
    }

    fn input(value: &Value, mode: PlanInputMode) -> PlanInput {
        PlanInput {
            cell: cell(value),
            mode,
        }
    }

    fn invalidation(value: &Value, kind: PlanInvalidationKind) -> PlanInvalidation {
        PlanInvalidation {
            cell: cell(value),
            kind,
        }
    }

    fn add_edge(graph: &mut PlanGraph, source: &Value, output: &Value) -> PlanNodeId {
        graph.add_node(PlanNodeSpec::explicit(
            vec![input(source, PlanInputMode::Reactive)],
            vec![cell(output)],
        ))
    }

    #[cfg(feature = "f64")]
    #[test]
    fn cell_id_tracks_underlying_scalar_allocation() {
        let reference = Ref::new(1.0);
        let first = Value::F64(reference.clone());
        let second = Value::F64(reference);
        let other = f64_value(1.0);

        assert_eq!(value_cell_id(&first), value_cell_id(&second));
        assert_ne!(value_cell_id(&first), value_cell_id(&other));
    }

    #[cfg(feature = "f64")]
    #[test]
    fn cell_id_recurses_through_mutable_reference() {
        let inner = f64_value(1.0);
        let reference = Value::MutableReference(Ref::new(inner.clone()));

        assert_eq!(value_cell_id(&reference), value_cell_id(&inner));
    }

    #[cfg(feature = "f64")]
    #[test]
    fn cell_id_recurses_through_typed_value() {
        let inner = f64_value(1.0);
        let typed = Value::Typed(Box::new(inner.clone()), ValueKind::F64);

        assert_eq!(value_cell_id(&typed), value_cell_id(&inner));
    }

    #[test]
    fn non_reference_values_have_no_cell_id() {
        assert_eq!(value_cell_id(&Value::Empty), None);
        assert_eq!(value_cell_id(&Value::Id(1)), None);
        assert_eq!(value_cell_id(&Value::IndexAll), None);
    }

    #[test]
    fn explicit_spec_deduplicates_without_reordering() {
        let a = Value::Index(Ref::new(1));
        let b = Value::Index(Ref::new(2));
        let a_cell = cell(&a);
        let b_cell = cell(&b);

        let spec = PlanNodeSpec::explicit(
            vec![
                PlanInput {
                    cell: b_cell,
                    mode: PlanInputMode::Reactive,
                },
                PlanInput {
                    cell: a_cell,
                    mode: PlanInputMode::Trigger,
                },
                PlanInput {
                    cell: b_cell,
                    mode: PlanInputMode::Reactive,
                },
                PlanInput {
                    cell: b_cell,
                    mode: PlanInputMode::Sampled,
                },
            ],
            vec![b_cell, a_cell, b_cell],
        );

        assert_eq!(
            spec.inputs,
            vec![
                PlanInput {
                    cell: b_cell,
                    mode: PlanInputMode::Reactive,
                },
                PlanInput {
                    cell: a_cell,
                    mode: PlanInputMode::Trigger,
                },
                PlanInput {
                    cell: b_cell,
                    mode: PlanInputMode::Sampled,
                },
            ]
        );
        assert_eq!(spec.outputs, vec![b_cell, a_cell]);
    }

    #[cfg(feature = "f64")]
    #[test]
    fn pure_reactive_spec_rejects_input_output_overlap() {
        let value = f64_value(1.0);
        let error = PlanNodeSpec::reactive(&[value.clone()], &value).unwrap_err();

        assert_eq!(error.kind_name(), "PlanInputOutputOverlap");
    }

    #[test]
    fn schedules_dependency_chain() {
        let a = Value::Index(Ref::new(1));
        let b = Value::Index(Ref::new(2));
        let c = Value::Index(Ref::new(3));
        let mut graph = PlanGraph::new();
        let node_0 = add_edge(&mut graph, &a, &b);
        let node_1 = add_edge(&mut graph, &b, &c);

        let outcome = graph
            .schedule_from(&[invalidation(&a, PlanInvalidationKind::Changed)])
            .unwrap();

        assert_eq!(outcome.scheduled_nodes, 2);
        assert_eq!(outcome.ordered_nodes, vec![node_0, node_1]);
    }

    #[test]
    fn skips_unrelated_branch() {
        let a = Value::Index(Ref::new(1));
        let b = Value::Index(Ref::new(2));
        let x = Value::Index(Ref::new(3));
        let y = Value::Index(Ref::new(4));
        let mut graph = PlanGraph::new();
        let node_0 = add_edge(&mut graph, &a, &b);
        add_edge(&mut graph, &x, &y);

        let outcome = graph
            .schedule_from(&[invalidation(&a, PlanInvalidationKind::Changed)])
            .unwrap();

        assert_eq!(outcome.scheduled_nodes, 1);
        assert_eq!(outcome.ordered_nodes, vec![node_0]);
    }

    #[test]
    fn deduplicates_diamond_join() {
        let a = Value::Index(Ref::new(1));
        let b = Value::Index(Ref::new(2));
        let c = Value::Index(Ref::new(3));
        let d = Value::Index(Ref::new(4));
        let mut graph = PlanGraph::new();
        let node_0 = add_edge(&mut graph, &a, &b);
        let node_1 = add_edge(&mut graph, &a, &c);
        let node_2 = graph.add_node(PlanNodeSpec::explicit(
            vec![
                input(&b, PlanInputMode::Reactive),
                input(&c, PlanInputMode::Reactive),
            ],
            vec![cell(&d)],
        ));

        let outcome = graph
            .schedule_from(&[invalidation(&a, PlanInvalidationKind::Changed)])
            .unwrap();

        assert_eq!(outcome.scheduled_nodes, 3);
        assert_eq!(outcome.ordered_nodes, vec![node_0, node_1, node_2]);
        assert_eq!(
            outcome
                .ordered_nodes
                .iter()
                .filter(|node| **node == node_2)
                .count(),
            1
        );
    }

    #[test]
    fn unions_multiple_roots() {
        let a = Value::Index(Ref::new(1));
        let b = Value::Index(Ref::new(2));
        let x = Value::Index(Ref::new(3));
        let y = Value::Index(Ref::new(4));
        let mut graph = PlanGraph::new();
        let node_0 = add_edge(&mut graph, &a, &b);
        let node_1 = add_edge(&mut graph, &x, &y);

        let outcome = graph
            .schedule_from(&[
                invalidation(&x, PlanInvalidationKind::Changed),
                invalidation(&a, PlanInvalidationKind::Changed),
                invalidation(&a, PlanInvalidationKind::Changed),
            ])
            .unwrap();

        assert_eq!(outcome.scheduled_nodes, 2);
        assert_eq!(outcome.ordered_nodes, vec![node_0, node_1]);
    }

    #[test]
    fn sampled_input_does_not_schedule() {
        let positions = Value::Index(Ref::new(1));
        let render_tick = Value::Index(Ref::new(2));
        let output = Value::Index(Ref::new(3));
        let mut graph = PlanGraph::new();
        let node = graph.add_node(PlanNodeSpec::explicit(
            vec![
                input(&positions, PlanInputMode::Sampled),
                input(&render_tick, PlanInputMode::Trigger),
            ],
            vec![cell(&output)],
        ));

        let changed_positions = graph
            .schedule_from(&[invalidation(&positions, PlanInvalidationKind::Changed)])
            .unwrap();
        let triggered_render = graph
            .schedule_from(&[invalidation(&render_tick, PlanInvalidationKind::Triggered)])
            .unwrap();

        assert_eq!(changed_positions.scheduled_nodes, 0);
        assert_eq!(changed_positions.ordered_nodes, Vec::<PlanNodeId>::new());
        assert_eq!(triggered_render.scheduled_nodes, 1);
        assert_eq!(triggered_render.ordered_nodes, vec![node]);
    }

    #[test]
    fn trigger_input_requires_triggered_invalidation() {
        let trigger = Value::Index(Ref::new(1));
        let output = Value::Index(Ref::new(2));
        let mut graph = PlanGraph::new();
        let node = graph.add_node(PlanNodeSpec::explicit(
            vec![input(&trigger, PlanInputMode::Trigger)],
            vec![cell(&output)],
        ));

        let changed = graph
            .schedule_from(&[invalidation(&trigger, PlanInvalidationKind::Changed)])
            .unwrap();
        let triggered = graph
            .schedule_from(&[invalidation(&trigger, PlanInvalidationKind::Triggered)])
            .unwrap();

        assert_eq!(changed.scheduled_nodes, 0);
        assert_eq!(triggered.scheduled_nodes, 1);
        assert_eq!(triggered.ordered_nodes, vec![node]);
    }

    #[test]
    fn reactive_input_accepts_changed_and_triggered() {
        let source = Value::Index(Ref::new(1));
        let output = Value::Index(Ref::new(2));
        let mut graph = PlanGraph::new();
        let node = add_edge(&mut graph, &source, &output);

        let changed = graph
            .schedule_from(&[invalidation(&source, PlanInvalidationKind::Changed)])
            .unwrap();
        let triggered = graph
            .schedule_from(&[invalidation(&source, PlanInvalidationKind::Triggered)])
            .unwrap();

        assert_eq!(changed.ordered_nodes, vec![node]);
        assert_eq!(triggered.ordered_nodes, vec![node]);
    }

    #[test]
    fn feedback_terminates_after_one_schedule() {
        let cell_value = Value::Index(Ref::new(1));
        let mut graph = PlanGraph::new();
        let node = graph.add_node(PlanNodeSpec::explicit(
            vec![input(&cell_value, PlanInputMode::Reactive)],
            vec![cell(&cell_value)],
        ));

        let outcome = graph
            .schedule_from(&[invalidation(&cell_value, PlanInvalidationKind::Changed)])
            .unwrap();

        assert_eq!(outcome.scheduled_nodes, 1);
        assert_eq!(outcome.ordered_nodes, vec![node]);
    }

    #[test]
    fn propagated_outputs_do_not_fire_trigger_consumers() {
        let root = Value::Index(Ref::new(1));
        let produced = Value::Index(Ref::new(2));
        let trigger_output = Value::Index(Ref::new(3));
        let mut graph = PlanGraph::new();
        let producer = add_edge(&mut graph, &root, &produced);
        let trigger_consumer = graph.add_node(PlanNodeSpec::explicit(
            vec![input(&produced, PlanInputMode::Trigger)],
            vec![cell(&trigger_output)],
        ));

        let propagated = graph
            .schedule_from(&[invalidation(&root, PlanInvalidationKind::Changed)])
            .unwrap();
        let direct_trigger = graph
            .schedule_from(&[invalidation(&produced, PlanInvalidationKind::Triggered)])
            .unwrap();

        assert_eq!(propagated.ordered_nodes, vec![producer]);
        assert_eq!(direct_trigger.ordered_nodes, vec![trigger_consumer]);
    }

    #[test]
    fn assignment_spec_uses_reactive_source_and_output_sink() {
        let source = Value::Index(Ref::new(1));
        let sink = Value::Index(Ref::new(2));
        let spec = PlanNodeSpec::assignment(&source, &sink).unwrap();

        assert_eq!(
            spec.inputs,
            vec![PlanInput {
                cell: cell(&source),
                mode: PlanInputMode::Reactive,
            }]
        );
        assert_eq!(spec.outputs, vec![cell(&sink)]);
    }

    #[test]
    fn assignment_spec_uses_sampled_input_for_same_cell() {
        let sink = Value::Index(Ref::new(1));
        let spec = PlanNodeSpec::assignment(&sink, &sink).unwrap();

        assert_eq!(
            spec.inputs,
            vec![PlanInput {
                cell: cell(&sink),
                mode: PlanInputMode::Sampled,
            }]
        );
        assert_eq!(spec.outputs, vec![cell(&sink)]);
    }

    #[test]
    fn assignment_spec_rejects_missing_source_identity() {
        let sink = Value::Index(Ref::new(1));
        let error = PlanNodeSpec::assignment(&Value::Empty, &sink).unwrap_err();

        assert_eq!(error.kind_name(), "PlanCellIdentityMissing");
        assert!(error.kind_message().contains("assignment source"));
    }

    #[test]
    fn assignment_spec_rejects_missing_sink_identity() {
        let source = Value::Index(Ref::new(1));
        let error = PlanNodeSpec::assignment(&source, &Value::Empty).unwrap_err();

        assert_eq!(error.kind_name(), "PlanCellIdentityMissing");
        assert!(error.kind_message().contains("assignment sink"));
    }

    #[test]
    fn read_modify_write_adds_sampled_sink() {
        let source = Value::Index(Ref::new(1));
        let sink = Value::Index(Ref::new(2));
        let spec = PlanNodeSpec::read_modify_write(&[source.clone()], &sink).unwrap();

        assert_eq!(
            spec.inputs,
            vec![
                PlanInput {
                    cell: cell(&source),
                    mode: PlanInputMode::Reactive,
                },
                PlanInput {
                    cell: cell(&sink),
                    mode: PlanInputMode::Sampled,
                },
            ]
        );
        assert_eq!(spec.outputs, vec![cell(&sink)]);
    }

    #[test]
    fn read_modify_write_normalizes_sink_duplicates() {
        let sink = Value::Index(Ref::new(1));
        let spec = PlanNodeSpec::read_modify_write(&[sink.clone(), sink.clone()], &sink).unwrap();

        assert_eq!(
            spec.inputs,
            vec![PlanInput {
                cell: cell(&sink),
                mode: PlanInputMode::Sampled,
            }]
        );
        assert_eq!(spec.outputs, vec![cell(&sink)]);
    }

    #[test]
    fn read_modify_write_rejects_missing_sink_identity() {
        let source = Value::Index(Ref::new(1));
        let error = PlanNodeSpec::read_modify_write(&[source], &Value::Empty).unwrap_err();

        assert_eq!(error.kind_name(), "PlanCellIdentityMissing");
        assert!(error.kind_message().contains("read-modify-write sink"));
    }

    #[cfg(all(feature = "matrix", feature = "matrixd", feature = "f64"))]
    #[test]
    fn cell_id_tracks_underlying_matrix_allocation() {
        let reference = Ref::new(na::DMatrix::from_element(2, 2, 1.0));
        let first = Value::MatrixF64(crate::structures::matrix::Matrix::DMatrix(
            reference.clone(),
        ));
        let second = Value::MatrixF64(crate::structures::matrix::Matrix::DMatrix(reference));
        let other = Value::MatrixF64(crate::structures::matrix::Matrix::DMatrix(Ref::new(
            na::DMatrix::from_element(2, 2, 1.0),
        )));

        assert_eq!(value_cell_id(&first), value_cell_id(&second));
        assert_ne!(value_cell_id(&first), value_cell_id(&other));
    }

    #[test]
    fn dependency_order_overrides_registration_order() {
        let a = Value::Index(Ref::new(1));
        let b = Value::Index(Ref::new(2));
        let c = Value::Index(Ref::new(3));
        let d = Value::Index(Ref::new(4));
        let mut graph = PlanGraph::new();
        let node_0 = graph.add_node(PlanNodeSpec::explicit(
            vec![
                input(&b, PlanInputMode::Reactive),
                input(&c, PlanInputMode::Reactive),
            ],
            vec![cell(&d)],
        ));
        let node_1 = add_edge(&mut graph, &a, &b);

        let outcome = graph
            .schedule_from(&[
                invalidation(&a, PlanInvalidationKind::Changed),
                invalidation(&c, PlanInvalidationKind::Changed),
            ])
            .unwrap();

        assert_eq!(outcome.scheduled_nodes, 2);
        assert_eq!(outcome.ordered_nodes, vec![node_1, node_0]);
    }

    #[test]
    fn sampled_dependency_orders_producer_before_consumer() {
        let positions = Value::Index(Ref::new(1));
        let render_tick = Value::Index(Ref::new(2));
        let physics_tick = Value::Index(Ref::new(3));
        let frame = Value::Index(Ref::new(4));
        let mut graph = PlanGraph::new();
        let render = graph.add_node(PlanNodeSpec::explicit(
            vec![
                input(&positions, PlanInputMode::Sampled),
                input(&render_tick, PlanInputMode::Trigger),
            ],
            vec![cell(&frame)],
        ));
        let physics = graph.add_node(PlanNodeSpec::explicit(
            vec![input(&physics_tick, PlanInputMode::Trigger)],
            vec![cell(&positions)],
        ));

        let outcome = graph
            .schedule_from(&[
                invalidation(&render_tick, PlanInvalidationKind::Triggered),
                invalidation(&physics_tick, PlanInvalidationKind::Triggered),
            ])
            .unwrap();

        assert_eq!(outcome.ordered_nodes, vec![physics, render]);
    }

    #[test]
    fn reactive_cycle_is_rejected() {
        let a = Value::Index(Ref::new(1));
        let b = Value::Index(Ref::new(2));
        let mut graph = PlanGraph::new();
        add_edge(&mut graph, &a, &b);
        add_edge(&mut graph, &b, &a);

        let error = graph
            .schedule_from(&[invalidation(&a, PlanInvalidationKind::Changed)])
            .unwrap_err();

        assert_eq!(error.kind_name(), "PlanDependencyCycle");
    }

    #[test]
    fn feedback_self_edge_is_not_a_cycle() {
        let state = Value::Index(Ref::new(1));
        let mut graph = PlanGraph::new();
        let node = graph.add_node(PlanNodeSpec::explicit(
            vec![input(&state, PlanInputMode::Reactive)],
            vec![cell(&state)],
        ));

        let outcome = graph
            .schedule_from(&[invalidation(&state, PlanInvalidationKind::Changed)])
            .unwrap();

        assert_eq!(outcome.scheduled_nodes, 1);
        assert_eq!(outcome.ordered_nodes, vec![node]);
    }

    #[test]
    fn multiple_writers_follow_registration_order() {
        let t0 = Value::Index(Ref::new(1));
        let t1 = Value::Index(Ref::new(2));
        let render = Value::Index(Ref::new(3));
        let state = Value::Index(Ref::new(4));
        let frame = Value::Index(Ref::new(5));
        let mut graph = PlanGraph::new();
        let node_0 = graph.add_node(PlanNodeSpec::explicit(
            vec![input(&t0, PlanInputMode::Trigger)],
            vec![cell(&state)],
        ));
        let node_1 = graph.add_node(PlanNodeSpec::explicit(
            vec![input(&t1, PlanInputMode::Trigger)],
            vec![cell(&state)],
        ));
        let node_2 = graph.add_node(PlanNodeSpec::explicit(
            vec![
                input(&render, PlanInputMode::Trigger),
                input(&state, PlanInputMode::Sampled),
            ],
            vec![cell(&frame)],
        ));

        let outcome = graph
            .schedule_from(&[
                invalidation(&t0, PlanInvalidationKind::Triggered),
                invalidation(&t1, PlanInvalidationKind::Triggered),
                invalidation(&render, PlanInvalidationKind::Triggered),
            ])
            .unwrap();

        assert_eq!(outcome.ordered_nodes, vec![node_0, node_1, node_2]);
    }

    #[test]
    fn clear_removes_producer_and_consumer_indexes() {
        let source = Value::Index(Ref::new(1));
        let output = Value::Index(Ref::new(2));
        let mut graph = PlanGraph::new();
        add_edge(&mut graph, &source, &output);
        graph.clear();

        let outcome = graph
            .schedule_from(&[invalidation(&source, PlanInvalidationKind::Changed)])
            .unwrap();

        assert_eq!(graph.len(), 0);
        assert_eq!(outcome.scheduled_nodes, 0);
        assert_eq!(outcome.ordered_nodes, Vec::<PlanNodeId>::new());
    }
}

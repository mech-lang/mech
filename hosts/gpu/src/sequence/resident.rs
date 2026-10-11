//! Resident executors for the typed sequence graph and its source callables.
//! No generated code or benchmark operation is invoked by this interpreter.
use super::*;
use mech_engine::native_sequence::{Kind, Program as Graph};
use std::collections::BTreeMap;

#[derive(Clone)]
struct ScalarTape {
    cells: Vec<u64>,
    parameters: Vec<(usize, Kind, usize)>,
    instructions: Vec<(u64, Kind, usize, usize, usize)>,
    output: usize,
    arity: usize,
}
impl ScalarTape {
    fn load(function: &Callable) -> Result<Self, String> {
        if !function.captures.is_empty() {
            return Err("resident scalar coefficient must be closed".into());
        }
        let graph = &function.program;
        let mut cells = vec![0; graph.constants.len() + graph.nodes.len()];
        let mut parameters = vec![];
        for (r, c) in graph.constants.iter().enumerate() {
            match c {
                Constant::Integer(n) | Constant::Float(n) => cells[r] = *n,
                Constant::Parameter(i, kind) if matches!(kind, Kind::Integer | Kind::Float) => {
                    if *i >= function.parameters.len() {
                        return Err("invalid scalar parameter".into());
                    }
                    parameters.push((r, *kind, *i));
                }
                _ => {}
            }
        }
        let instructions = graph
            .nodes
            .iter()
            .enumerate()
            .map(|(i, node)| {
                if !matches!(node.op, 19..=23 | 38) {
                    return Err(
                        "resident scalar callable contains a nonscalar operation".to_owned()
                    );
                }
                Ok((
                    node.op,
                    graph.kinds[graph.constants.len() + i],
                    node.args[0],
                    node.args.get(1).copied().unwrap_or(node.args[0]),
                    graph.constants.len() + i,
                ))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            cells,
            parameters,
            instructions,
            output: function.output,
            arity: function.parameters.len(),
        })
    }
    fn evaluate(&mut self, inputs: &[u64]) -> Result<f64, String> {
        if inputs.len() != self.arity {
            return Err("scalar callable arity".into());
        }
        for &(r, kind, index) in &self.parameters {
            let value = inputs[index];
            self.cells[r] = if kind == Kind::Integer {
                value
            } else {
                (value as f64).to_bits()
            };
        }
        for &(op, kind, a, b, dst) in &self.instructions {
            let x = self.cells[a];
            let y = self.cells[b];
            self.cells[dst] = if op == 19 {
                (x as f64).to_bits()
            } else if kind == Kind::Integer {
                match op {
                    20 => x.checked_mul(y).ok_or("unsigned arithmetic overflow")?,
                    21 => x.checked_div(y).ok_or("unsigned division by zero")?,
                    22 => x.checked_add(y).ok_or("unsigned arithmetic overflow")?,
                    23 => x.checked_sub(y).ok_or("unsigned arithmetic overflow")?,
                    _ => return Err("unsupported scalar integer operation".into()),
                }
            } else {
                let x = f64::from_bits(x);
                let y = f64::from_bits(y);
                match op {
                    20 => (x * y).to_bits(),
                    21 => (x / y).to_bits(),
                    22 => (x + y).to_bits(),
                    23 => (x - y).to_bits(),
                    38 => x.sqrt().to_bits(),
                    _ => return Err("unsupported scalar floating operation".into()),
                }
            };
        }
        Ok(f64::from_bits(self.cells[self.output]))
    }
    /// Conservative intervals preserve eager integer errors in fused producers.
    /// A proven domain avoids re-evaluating a pure coefficient grid solely to
    /// discover errors that its typed arithmetic cannot produce.
    fn proves_domain(&self, maxima: &[u64]) -> bool {
        let mut ranges = vec![None; self.cells.len()];
        for &(op, _, _, _, dst) in &self.instructions {
            let _ = op;
            ranges[dst] = None;
        }
        let first = self.instructions.first().map_or(self.cells.len(), |i| i.4);
        for (i, &n) in self.cells[..first].iter().enumerate() {
            ranges[i] = Some((n, n));
        }
        for &(r, kind, index) in &self.parameters {
            ranges[r] = if kind == Kind::Integer {
                Some((0, maxima[index]))
            } else {
                None
            };
        }
        for &(op, kind, a, b, dst) in &self.instructions {
            if kind != Kind::Integer {
                ranges[dst] = None;
                continue;
            }
            let (Some((al, ah)), Some((bl, bh))) = (ranges[a], ranges[b]) else {
                return false;
            };
            let result = match op {
                20 => al.checked_mul(bl).zip(ah.checked_mul(bh)),
                21 if bl > 0 => Some((al / bh, ah / bl)),
                22 => al.checked_add(bl).zip(ah.checked_add(bh)),
                23 => al.checked_sub(bh).zip(ah.checked_sub(bl)),
                _ => None,
            };
            if result.is_none() {
                return false;
            }
            ranges[dst] = result;
        }
        true
    }
}

const SCALAR_TILE: usize = 32;
#[derive(Clone, Copy)]
enum TileOp {
    ToFloat,
    IntegerMultiply,
    IntegerDivide,
    IntegerShift(u32),
    IntegerAdd,
    IntegerSubtract,
    FloatMultiply,
    FloatDivide,
    FloatAdd,
    FloatSubtract,
    FloatSqrt,
}
/// Interpret each source instruction across independent coefficient calls.
/// Batching amortizes dispatch and exposes elementwise loops to the host's
/// ordinary vectorization. It generates no code and substitutes no formula.
/// Checked arithmetic remains on the scalar path unless the entire invocation
/// domain proves safe; tail lanes never evaluate out-of-domain coefficients.
#[derive(Clone)]
struct ScalarTile {
    cells: Vec<[u64; SCALAR_TILE]>,
    parameters: Vec<(usize, Kind, usize)>,
    instructions: Vec<(TileOp, usize, usize, usize)>,
    output: usize,
    varying: usize,
}
impl ScalarTile {
    fn load(tape: &ScalarTape, maxima: &[u64], varying: usize) -> Option<Self> {
        if tape.arity != 2 || !tape.proves_domain(maxima) {
            return None;
        }
        let first = tape.instructions.first().map_or(tape.cells.len(), |i| i.4);
        let instructions = tape
            .instructions
            .iter()
            .map(|&(op, kind, a, b, dst)| {
                let constant_divisor = b < first && !tape.parameters.iter().any(|p| p.0 == b);
                let operation = match (op, kind) {
                    (19, _) => TileOp::ToFloat,
                    (20, Kind::Integer) => TileOp::IntegerMultiply,
                    (21, Kind::Integer) if constant_divisor && tape.cells[b].is_power_of_two() => {
                        TileOp::IntegerShift(tape.cells[b].trailing_zeros())
                    }
                    (21, Kind::Integer) => TileOp::IntegerDivide,
                    (22, Kind::Integer) => TileOp::IntegerAdd,
                    (23, Kind::Integer) => TileOp::IntegerSubtract,
                    (20, _) => TileOp::FloatMultiply,
                    (21, _) => TileOp::FloatDivide,
                    (22, _) => TileOp::FloatAdd,
                    (23, _) => TileOp::FloatSubtract,
                    (38, _) => TileOp::FloatSqrt,
                    _ => unreachable!("validated scalar operation"),
                };
                (operation, a, b, dst)
            })
            .collect();
        Some(Self {
            cells: tape
                .cells
                .iter()
                .map(|&value| [value; SCALAR_TILE])
                .collect(),
            parameters: tape.parameters.clone(),
            instructions,
            output: tape.output,
            varying,
        })
    }
    fn evaluate(&mut self, fixed: u64, start: usize, width: usize) {
        for &(r, kind, index) in &self.parameters {
            let values = &mut self.cells[r][..width];
            for (lane, dst) in values.iter_mut().enumerate() {
                let value = if index == self.varying {
                    (start + lane) as u64
                } else {
                    fixed
                };
                *dst = if kind == Kind::Integer {
                    value
                } else {
                    (value as f64).to_bits()
                };
            }
        }
        for &(op, a, b, dst) in &self.instructions {
            // SSA destinations follow every input. Disjoint slices also allow
            // vectorization without speculative aliasing or unchecked indexing.
            let (inputs, output) = self.cells.split_at_mut(dst);
            let output = &mut output[0][..width];
            let left = &inputs[a][..width];
            let right = &inputs[b][..width];
            match op {
                TileOp::ToFloat => {
                    for (dst, &x) in output.iter_mut().zip(left) {
                        *dst = (x as f64).to_bits();
                    }
                }
                TileOp::IntegerMultiply => {
                    for ((dst, &x), &y) in output.iter_mut().zip(left).zip(right) {
                        *dst = x.wrapping_mul(y);
                    }
                }
                TileOp::IntegerDivide => {
                    for ((dst, &x), &y) in output.iter_mut().zip(left).zip(right) {
                        *dst = x / y;
                    }
                }
                TileOp::IntegerShift(shift) => {
                    for (dst, &x) in output.iter_mut().zip(left) {
                        *dst = x >> shift;
                    }
                }
                TileOp::IntegerAdd => {
                    for ((dst, &x), &y) in output.iter_mut().zip(left).zip(right) {
                        *dst = x.wrapping_add(y);
                    }
                }
                TileOp::IntegerSubtract => {
                    for ((dst, &x), &y) in output.iter_mut().zip(left).zip(right) {
                        *dst = x.wrapping_sub(y);
                    }
                }
                TileOp::FloatMultiply => {
                    for ((dst, &x), &y) in output.iter_mut().zip(left).zip(right) {
                        *dst = (f64::from_bits(x) * f64::from_bits(y)).to_bits();
                    }
                }
                TileOp::FloatDivide => {
                    for ((dst, &x), &y) in output.iter_mut().zip(left).zip(right) {
                        *dst = (f64::from_bits(x) / f64::from_bits(y)).to_bits();
                    }
                }
                TileOp::FloatAdd => {
                    for ((dst, &x), &y) in output.iter_mut().zip(left).zip(right) {
                        *dst = (f64::from_bits(x) + f64::from_bits(y)).to_bits();
                    }
                }
                TileOp::FloatSubtract => {
                    for ((dst, &x), &y) in output.iter_mut().zip(left).zip(right) {
                        *dst = (f64::from_bits(x) - f64::from_bits(y)).to_bits();
                    }
                }
                TileOp::FloatSqrt => {
                    for (dst, &x) in output.iter_mut().zip(left) {
                        *dst = f64::from_bits(x).sqrt().to_bits();
                    }
                }
            }
        }
    }
    fn row(&mut self, fixed: u64, input: &[f64], divide: bool) -> f64 {
        let mut sum = 0.0;
        for (part, values) in input.chunks(SCALAR_TILE).enumerate() {
            self.evaluate(fixed, part * SCALAR_TILE, values.len());
            for (&x, &bits) in values.iter().zip(&self.cells[self.output]) {
                let coefficient = f64::from_bits(bits);
                // Preserve the original left-to-right reduction and rounding.
                sum += if divide {
                    x / coefficient
                } else {
                    x * coefficient
                };
            }
        }
        sum
    }
}
fn scalar_row(
    tape: &mut ScalarTape,
    tile: &mut Option<ScalarTile>,
    row: usize,
    input: &[f64],
    transpose: bool,
    divide: bool,
) -> Result<f64, String> {
    if let Some(tile) = tile {
        return Ok(tile.row(row as u64, input, divide));
    }
    let mut sum = 0.0;
    for (column, &x) in input.iter().enumerate() {
        let indices = if transpose {
            [column as u64, row as u64]
        } else {
            [row as u64, column as u64]
        };
        let coefficient = tape.evaluate(&indices)?;
        sum += if divide {
            x / coefficient
        } else {
            x * coefficient
        };
    }
    Ok(sum)
}

impl Context {
    /// Interpret a loaded graph using its general resident library executors.
    /// Callers retain the canonical graph between input executions.
    pub fn run_resident(&mut self, graph: &Graph) -> Result<(), String> {
        if !self.matches_constants(&graph.constants) {
            return Err("resident constant table mismatch".into());
        }
        self.resident_graph(graph, None)?;
        self.finish_execution()
    }
    fn child(
        &self,
        function: &Callable,
        captures: &[Value],
        inputs: &[Value],
    ) -> Result<Context, String> {
        if inputs.len() != function.parameters.len() || captures.len() != function.captures.len() {
            return Err("resident callable arity mismatch".into());
        }
        let mut child = Context::new(&function.program.constants, None, false);
        child.pool = self.pool.clone();
        child.scratch = self.scratch.clone();
        child.partitions = self.partitions;
        child.array_grain = self.array_grain;
        child.compact_counts = self.compact_counts;
        child.standard_map = self.standard_map;
        child.arguments = self.arguments.clone();
        child.independent_tasks = self.independent_tasks;
        child.reuse_byte_buffers = self.reuse_byte_buffers;
        for ((_, r), value) in function.captures.iter().zip(captures) {
            child.values[*r] = value.clone();
        }
        for (&r, value) in function.parameters.iter().zip(inputs) {
            child.values[r] = value.clone();
        }
        Ok(child)
    }
    fn resident_graph(&mut self, graph: &Graph, output: Option<usize>) -> Result<(), String> {
        let mut nodes = graph.nodes.clone();
        let mut uses = BTreeMap::<usize, usize>::new();
        for node in &nodes {
            for &r in &node.args {
                *uses.entry(r).or_default() += 1;
            }
        }
        // The same structural producer/consumer fusions as native emission.
        if std::env::var("MECH_BYTE_FUSION").as_deref() != Ok("0") {
            for i in 0..nodes.len() {
                if nodes[i].op == 43 {
                    let r = nodes[i].args[0];
                    if uses.get(&r) == Some(&1) && Some(r) != output {
                        if let Some(j) = r.checked_sub(graph.constants.len()) {
                            if nodes[j].op == 44 {
                                nodes[i].op = 46;
                                nodes[i].args[0] = nodes[j].args[0];
                                nodes[j].op = 47;
                                nodes[j].args.clear();
                            }
                        }
                    }
                }
            }
        }
        let mut last = BTreeMap::new();
        uses.clear();
        for (i, node) in nodes.iter().enumerate() {
            for &r in &node.args {
                last.insert(r, i);
                *uses.entry(r).or_default() += 1;
            }
        }
        let mut fused = BTreeMap::new();
        for (i, node) in nodes.iter().enumerate() {
            if matches!(node.op, 2 | 40) && uses.get(&node.args[0]) == Some(&1) {
                if let Some(j) = node.args[0].checked_sub(graph.constants.len()) {
                    if nodes[j].op == 1 {
                        fused.insert(j, 11);
                        fused.insert(i, if node.op == 40 { 41 } else { 10 });
                    }
                }
            }
        }
        for (i, node) in nodes.iter().enumerate() {
            let mut op = fused.get(&i).copied().unwrap_or(node.op);
            let mut args = [0; 5];
            for (dst, &r) in args.iter_mut().zip(&node.args) {
                *dst = r;
            }
            if matches!(op, 42 | 43 | 44 | 46)
                && last.get(&node.args[0]) == Some(&i)
                && Some(node.args[0]) != output
                && std::env::var("MECH_BYTE_REUSE").as_deref() != Ok("0")
            {
                op |= 1 << 32;
            }
            let value = self.op(op, args, 0)?;
            self.values.push(value);
            for r in node
                .args
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>()
            {
                if last.get(&r) == Some(&i) && Some(r) != output {
                    if let Value::Histogram(p) = &self.values[r] {
                        p.resolve()?;
                    }
                    if let Value::DeferredInteger(p) = &self.values[r] {
                        p.resolve()?;
                    }
                    let value = std::mem::replace(&mut self.values[r], Value::Unit);
                    self.recycle(value);
                }
            }
        }
        Ok(())
    }
    pub(super) fn resident_op(
        &mut self,
        op: u64,
        a: [usize; 5],
        consume: bool,
    ) -> Result<Value, String> {
        Ok(match op {
            5 => {
                let Some(Value::Window(window)) = self.values.get(a[0]) else {
                    return Err("expected windows".into());
                };
                let window = window.clone();
                let partitions = self.partitions;
                let compact = self.compact_counts;
                let standard = self.standard_map;
                let pool = self.pool.clone();
                let task = std::thread::spawn(move || {
                    let total = window.bytes.len().saturating_sub(window.width as usize - 1);
                    let pieces = if total < 1024 { 1 } else { partitions };
                    let count = |piece| {
                        let mut map = Counts::new(
                            window.bits as u32 * window.width as u32,
                            total,
                            compact,
                            standard,
                        );
                        let boundary = |i| total / pieces * i + (total % pieces) * i / pieces;
                        let start = boundary(piece);
                        let end = boundary(piece + 1);
                        if start < end {
                            let bytes = &window.bytes[start..end + window.width as usize - 1];
                            let bits = u32::from(window.bits);
                            let mask = u64::MAX >> (64 - bits * u32::from(window.width));
                            let mut key = 0u64;
                            for &b in &bytes[..window.width as usize - 1] {
                                key = (key << bits) | u64::from(b);
                            }
                            let mut batch = Vec::with_capacity(512);
                            for &b in &bytes[window.width as usize - 1..] {
                                key = ((key << bits) | u64::from(b)) & mask;
                                batch.push(key);
                                if batch.len() == 512 {
                                    map.increment(&batch);
                                    batch.clear();
                                }
                            }
                            map.increment(&batch);
                        }
                        map
                    };
                    let maps = if let Some(pool) = pool {
                        pool.install(|| (0..pieces).into_par_iter().map(count).collect::<Vec<_>>())
                    } else {
                        vec![count(0)]
                    };
                    let mut maps = maps.into_iter();
                    let mut map = maps.next().unwrap();
                    for other in maps {
                        map.merge(other);
                    }
                    assert_eq!(map.rows().iter().map(|(_, n)| n).sum::<u64>(), total as u64);
                    Histogram { map }
                });
                Value::Histogram(Arc::new(Pending {
                    value: OnceLock::new(),
                    task: Mutex::new(Some(task)),
                }))
            }
            20..=23 => {
                if let (Some(Value::Integer(x)), Some(Value::Integer(y))) =
                    (self.values.get(a[0]), self.values.get(a[1]))
                {
                    let result = match op {
                        20 => x.checked_mul(*y),
                        21 => x.checked_div(*y),
                        22 => x.checked_add(*y),
                        _ => x.checked_sub(*y),
                    }
                    .ok_or("unsigned overflow or division by zero")?;
                    return Ok(Value::Integer(result));
                }
                let (Some(Value::Numbers(x, sx)), Some(Value::Numbers(y, sy))) =
                    (self.values.get(a[0]), self.values.get(a[1]))
                else {
                    return Err("expected numeric operands".into());
                };
                let n = if *sx {
                    y.len()
                } else if *sy || x.len() == y.len() {
                    x.len()
                } else {
                    return Err("numeric shapes differ".into());
                };
                let mut output = self.allocate_numbers(n)?;
                for (i, dst) in output.iter_mut().enumerate() {
                    let x = x[if *sx { 0 } else { i }];
                    let y = y[if *sy { 0 } else { i }];
                    *dst = match op {
                        20 => x * y,
                        21 => x / y,
                        22 => x + y,
                        _ => x - y,
                    };
                }
                Value::Numbers(Arc::new(output), *sx && *sy)
            }
            73 => {
                let rows = self.integer(a[0])?;
                let columns = self.integer(a[1])?;
                let Some(Value::Function(function)) = self.values.get(a[2]) else {
                    return Err("expected coefficient callable".into());
                };
                if rows != 0 && columns != 0 {
                    let mut tape = ScalarTape::load(function)?;
                    if !tape.proves_domain(&[rows - 1, columns - 1]) {
                        for i in 0..rows {
                            for j in 0..columns {
                                tape.evaluate(&[i, j])?;
                            }
                        }
                    }
                }
                Value::Unit
            }
            35 | 36 => {
                let Some(Value::Numbers(input, false)) = self.values.get(a[0]) else {
                    return Err("matrix input must be vector".into());
                };
                let input = input.clone();
                let rows = usize::try_from(self.integer(a[1])?).map_err(|_| "invalid rows")?;
                let Some(Value::Function(function)) = self.values.get(a[2]) else {
                    return Err("expected coefficient callable".into());
                };
                let tape = ScalarTape::load(function)?;
                let mode = self.text(a[3])?;
                if !matches!(mode.as_slice(), b"multiply" | b"divide") {
                    return Err("invalid weight mode".into());
                }
                let divide = mode.as_slice() == b"divide";
                let transpose = op == 36;
                let maxima = if transpose {
                    [
                        input.len().saturating_sub(1) as u64,
                        rows.saturating_sub(1) as u64,
                    ]
                } else {
                    [
                        rows.saturating_sub(1) as u64,
                        input.len().saturating_sub(1) as u64,
                    ]
                };
                let tile = ScalarTile::load(&tape, &maxima, usize::from(!transpose));
                let mut output = self.allocate_numbers(rows)?;
                let chunk = rows.div_ceil(self.partitions * self.array_grain).max(1);
                let run = |output: &mut [f64]| -> Result<(), String> {
                    output
                        .par_chunks_mut(chunk)
                        .enumerate()
                        .try_for_each(|(part, output)| {
                            let mut tape = tape.clone();
                            let mut tile = tile.clone();
                            for (offset, dst) in output.iter_mut().enumerate() {
                                let i = part * chunk + offset;
                                *dst =
                                    scalar_row(&mut tape, &mut tile, i, &input, transpose, divide)?;
                            }
                            Ok(())
                        })
                };
                if let Some(pool) = &self.pool {
                    pool.install(|| run(&mut output))?;
                } else {
                    let mut tape = tape;
                    let mut tile = tile;
                    for (i, dst) in output.iter_mut().enumerate() {
                        *dst = scalar_row(&mut tape, &mut tile, i, &input, transpose, divide)?;
                    }
                }
                Value::Numbers(Arc::new(output), false)
            }
            37 => {
                let (Some(Value::Numbers(x, false)), Some(Value::Numbers(y, false))) =
                    (self.values.get(a[0]), self.values.get(a[1]))
                else {
                    return Err("dot requires vectors".into());
                };
                if x.len() != y.len() {
                    return Err("dot shapes differ".into());
                }
                Value::Numbers(
                    Arc::new(vec![x.iter().zip(y.iter()).map(|(x, y)| x * y).sum()]),
                    true,
                )
            }
            38 => {
                let Some(Value::Numbers(x, true)) = self.values.get(a[0]) else {
                    return Err("sqrt requires scalar".into());
                };
                Value::Numbers(Arc::new(vec![x[0].sqrt()]), true)
            }
            39 => {
                let mut current = self.values[a[0]].clone();
                let iterations = self.integer(a[1])?;
                let Some(Value::Function(function)) = self.values.get(a[2]) else {
                    return Err("expected iteration callable".into());
                };
                let function = function.clone();
                let captures = self.capture_values(&function, &a[3..])?;
                for _ in 0..iterations {
                    let mut child = self.child(&function, &captures, &[current.clone()])?;
                    child.resident_graph(&function.program, Some(function.output))?;
                    let next = std::mem::replace(&mut child.values[function.output], Value::Unit);
                    child.recycle(current);
                    current = next;
                }
                current
            }
            42 => {
                let Some(Value::Function(function)) = self.values.get(a[1]) else {
                    return Err("expected record callable".into());
                };
                let function = function.clone();
                let captures = self.capture_values(&function, &a[2..])?;
                let value = if consume {
                    std::mem::replace(&mut self.values[a[0]], Value::Unit)
                } else {
                    self.values[a[0]].clone()
                };
                let Value::Records(records) = value else {
                    return Err("expected records".into());
                };
                let records = if Arc::strong_count(&records) > 1 {
                    let copy = match &*records.lock().map_err(|_| "record lock poisoned")? {
                        Records::Stored(v) => Some(v.clone()),
                        Records::Streaming(_) => None,
                    };
                    copy.map_or(records.clone(), |v| {
                        Arc::new(Mutex::new(Records::Stored(v)))
                    })
                } else {
                    records
                };
                Value::TextStream(Arc::new(MappedRecords {
                    records,
                    function,
                    entry: 0,
                    captures,
                }))
            }
            43 | 44 | 46 => {
                let table = if op == 44 {
                    None
                } else {
                    let Some(Value::ByteTable(table)) = self.values.get(a[1]) else {
                        return Err("expected byte table".into());
                    };
                    Some(table.clone())
                };
                let input = if consume {
                    std::mem::replace(&mut self.values[a[0]], Value::Unit)
                } else {
                    self.values[a[0]].clone()
                };
                let Value::Bytes(input, symbols) = input else {
                    return Err("expected bytes".into());
                };
                let mut output = Arc::try_unwrap(input).unwrap_or_else(|b| b.as_ref().clone());
                if op != 43 {
                    output.reverse();
                }
                if let Some(table) = &table {
                    for byte in &mut output {
                        let code = table[*byte as usize];
                        if code == u16::MAX {
                            return Err("byte outside translation table".into());
                        }
                        *byte = code as u8;
                    }
                }
                Value::Bytes(
                    Arc::new(output),
                    table.map_or(symbols, |table| {
                        table
                            .iter()
                            .copied()
                            .filter(|&n| n != u16::MAX)
                            .max()
                            .map_or(0, |n| n as usize + 1)
                    }),
                )
            }
            45 => {
                let Some(Value::TextStream(mapped)) = self.values.get(a[0]) else {
                    return Err("expected text stream".into());
                };
                let mapped = mapped.clone();
                let mut records = mapped.records.lock().map_err(|_| "record lock poisoned")?;
                let mut bytes = Vec::new();
                let mut stdout = std::io::BufWriter::with_capacity(1048576, std::io::stdout());
                let writer: &mut dyn Write = if self.write_stdout {
                    &mut stdout
                } else {
                    &mut bytes
                };
                while let Some(record) = records.next()? {
                    let mut child = self.child(
                        &mapped.function,
                        &mapped.captures,
                        &[Value::Text(record.header), Value::Bytes(record.body, 256)],
                    )?;
                    child.resident_graph(&mapped.function.program, Some(mapped.function.output))?;
                    child.values[mapped.function.output].write_text(writer)?;
                }
                writer.flush().map_err(|e| e.to_string())?;
                if !self.write_stdout {
                    self.output.extend(bytes);
                }
                Value::Unit
            }
            _ => return Err(format!("resident executor missing operation {op}")),
        })
    }
}

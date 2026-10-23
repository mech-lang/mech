use super::compiler::Constant;
use super::counts::Counts;
use super::host_io::{NativeHostIo, parse_u64};
use mech_engine::native_sequence::Callable;
use rayon::prelude::*;
use std::{
    collections::VecDeque,
    io::{BufRead, Read, Write},
    sync::{Arc, Mutex, OnceLock},
    thread::JoinHandle,
};

pub type NativeEntry = unsafe extern "C" fn(*mut std::ffi::c_void);
type NativeWrap = unsafe extern "C" fn(*const u8, *mut u8, usize);
type NativeBytes = unsafe extern "C" fn(*const u8, *mut u8, usize, *const u16, usize, usize) -> u32;
type NativeRecords = unsafe extern "C" fn(*mut RecordState);
type NativeWindow = unsafe extern "C" fn(*const u8, usize, *mut Counts);
type NativeDomain = unsafe extern "C" fn(u64, u64) -> u32;
type NativeMatrix = unsafe extern "C" fn(*const f64, usize, usize, usize, *mut f64) -> u32;
type NativeDot = unsafe extern "C" fn(*const f64, *const f64, usize) -> f64;
type NativeSqrt = unsafe extern "C" fn(f64) -> f64;
type NativeIteration = unsafe extern "C" fn(*mut IterationState, u64);
type NativeNumeric = unsafe extern "C" fn(*const f64, usize, *const f64, usize, *mut f64, usize);
type NativeUnsigned =
    unsafe extern "C" fn(*const u64, usize, *const u64, usize, *mut u64, usize) -> u32;
fn numeric_status(status: u32) -> Result<(), String> {
    match status {
        0 => Ok(()),
        1 => Err("unsigned arithmetic overflow".into()),
        2 => Err("unsigned division by zero".into()),
        _ => Err("invalid native numeric status".into()),
    }
}
#[derive(Clone)]
struct Window {
    bytes: Arc<Vec<u8>>,
    bits: u8,
    width: u8,
}
struct Histogram {
    map: Counts,
}
struct Pending {
    value: OnceLock<Arc<Histogram>>,
    task: Mutex<Option<JoinHandle<Histogram>>>,
}
/// A reusable scalar future owned by the dataflow. Independent library work
/// runs in the bounded pool; consumers and entry completion resolve it once.
struct DeferredInteger {
    value: OnceLock<Result<u64, String>>,
    receiver: Mutex<Option<std::sync::mpsc::Receiver<u64>>>,
}
impl DeferredInteger {
    fn resolve(&self) -> Result<u64, String> {
        self.value
            .get_or_init(|| {
                self.receiver
                    .lock()
                    .map_err(|_| "scalar task lock poisoned".to_owned())?
                    .take()
                    .ok_or_else(|| "missing scalar task".to_owned())?
                    .recv()
                    .map_err(|_| "scalar task did not complete".to_owned())
            })
            .clone()
    }
}
impl Pending {
    fn resolve(&self) -> Result<Arc<Histogram>, String> {
        if let Some(value) = self.value.get() {
            return Ok(value.clone());
        }
        let mut task = self
            .task
            .lock()
            .map_err(|_| "histogram task lock poisoned")?;
        if let Some(value) = self.value.get() {
            return Ok(value.clone());
        }
        let value = Arc::new(
            task.take()
                .ok_or("missing histogram task")?
                .join()
                .map_err(|_| "histogram task panicked")?,
        );
        self.value
            .set(value.clone())
            .map_err(|_| "duplicate histogram result")?;
        Ok(value)
    }
}
#[derive(Clone)]
enum Value {
    Text(Arc<Vec<u8>>),
    TextView(Arc<TextView>),
    Records(Arc<Mutex<Records>>),
    TextStream(Arc<MappedRecords>),
    ByteTable(Arc<[u16; 256]>),
    Integer(u64),
    DeferredInteger(Arc<DeferredInteger>),
    Integers(Arc<Vec<u64>>),
    Numbers(Arc<Vec<f64>>, bool),
    Texts(Arc<Vec<Vec<u8>>>),
    Indices(Arc<Vec<usize>>),
    Bytes(Arc<Vec<u8>>, usize),
    Window(Window),
    Histogram(Arc<Pending>),
    Unit,
    Function(Arc<Callable>),
    CapturedValues(Arc<Vec<Value>>),
}

#[derive(Clone)]
struct Record {
    header: Arc<Vec<u8>>,
    body: Arc<Vec<u8>>,
}
enum Records {
    Stored(VecDeque<Record>),
    Streaming(RecordReader<Box<dyn BufRead + Send>>),
}
impl Records {
    fn next(&mut self) -> Result<Option<Record>, String> {
        match self {
            Self::Stored(v) => Ok(v.pop_front()),
            Self::Streaming(r) => r.next(),
        }
    }
    fn recycle_buffer(&mut self, buffer: Vec<u8>) {
        if let Self::Streaming(reader) = self {
            reader.recycle_buffer(buffer);
        }
    }
}
struct MappedRecords {
    records: Arc<Mutex<Records>>,
    function: Arc<Callable>,
    entry: usize,
    captures: Vec<Value>,
}
// Immutable formatting views avoid copying an entire wrapped byte sequence.
// Contiguous text consumers materialize; generic IO serializes the view.
enum TextView {
    Wrap(Arc<Vec<u8>>, usize, Arc<Vec<u8>>, usize),
    Concat(Value, Value),
}
impl Value {
    /// Reclaim only uniquely owned storage after a streamed result is written.
    /// Shared captures and live aliases retain their original allocation.
    fn reclaim_byte_buffer(self, spare: &mut Vec<u8>) {
        let bytes = match self {
            Self::Text(bytes) | Self::Bytes(bytes, _) => Some(bytes),
            Self::TextView(view) => match Arc::try_unwrap(view) {
                Ok(TextView::Wrap(bytes, _, _, _)) => Some(bytes),
                Ok(TextView::Concat(left, right)) => {
                    left.reclaim_byte_buffer(spare);
                    right.reclaim_byte_buffer(spare);
                    None
                }
                Err(_) => None,
            },
            _ => None,
        };
        if let Some(bytes) = bytes {
            if let Ok(mut buffer) = Arc::try_unwrap(bytes) {
                if buffer.capacity() > spare.capacity() {
                    buffer.clear();
                    *spare = buffer;
                }
            }
        }
    }
    fn write_text(&self, writer: &mut dyn Write) -> Result<(), String> {
        match self {
            Self::Text(v) => writer.write_all(v).map_err(|e| e.to_string()),
            Self::TextView(v) => match v.as_ref() {
                TextView::Wrap(v, width, separator, kernel) => {
                    write_wrapped(writer, v, *width, separator, *kernel)
                }
                TextView::Concat(a, b) => {
                    a.write_text(writer)?;
                    b.write_text(writer)
                }
            },
            _ => Err("expected text".into()),
        }
    }
}
fn write_wrapped(
    writer: &mut dyn Write,
    bytes: &[u8],
    width: usize,
    separator: &[u8],
    kernel: usize,
) -> Result<(), String> {
    if kernel != 0 {
        if !(1..=256).contains(&width) || separator.len() > 8 {
            return Err("native wrap shape mismatch".into());
        }
        let lines = 262144 / (width + separator.len());
        let chunk = lines * width;
        let mut buffer = vec![0u8; lines * (width + separator.len())];
        // SAFETY: exact same-object source-constant width and separator ABI.
        // Chunks end on full line boundaries except the last; initialized output
        // capacity is at least len + ceil(len/width)*separator.len(). Buffers are
        // disjoint and live for every call; even empty input performs no load.
        let function = unsafe { std::mem::transmute::<usize, NativeWrap>(kernel) };
        for part in bytes.chunks(chunk) {
            unsafe { function(part.as_ptr(), buffer.as_mut_ptr(), part.len()) };
            let used = part.len() + part.len().div_ceil(width) * separator.len();
            writer
                .write_all(&buffer[..used])
                .map_err(|e| e.to_string())?;
        }
        return Ok(());
    }
    let mut buffer = Vec::with_capacity(262144);
    for line in bytes.chunks(width) {
        buffer.extend_from_slice(line);
        buffer.extend_from_slice(separator);
        if buffer.len() >= 262144 {
            writer.write_all(&buffer).map_err(|e| e.to_string())?;
            buffer.clear();
        }
    }
    writer.write_all(&buffer).map_err(|e| e.to_string())
}
struct RecordReader<R> {
    reader: R,
    marker: Vec<u8>,
    separator: Vec<u8>,
    pending: Option<Vec<u8>>,
    line: Vec<u8>,
    spare: Vec<u8>,
    ended: bool,
}
impl<R: BufRead> RecordReader<R> {
    fn new(reader: R, marker: &[u8], separator: &[u8]) -> Result<Self, String> {
        if marker.is_empty() {
            return Err("record prefix cannot be empty".into());
        }
        Ok(Self {
            reader,
            marker: marker.to_vec(),
            separator: separator.to_vec(),
            pending: None,
            line: Vec::new(),
            spare: Vec::new(),
            ended: false,
        })
    }
    fn recycle_buffer(&mut self, mut buffer: Vec<u8>) {
        // Retain at most one completed record allocation. No file-size query or
        // fixed workload size is used; future records still grow as necessary.
        if buffer.capacity() > self.spare.capacity() {
            buffer.clear();
            self.spare = buffer;
        }
    }
    fn next(&mut self) -> Result<Option<Record>, String> {
        if self.ended {
            return Ok(None);
        }
        let header = if let Some(header) = self.pending.take() {
            header
        } else {
            self.line.clear();
            if self
                .reader
                .read_until(b'\n', &mut self.line)
                .map_err(|e| e.to_string())?
                == 0
            {
                self.ended = true;
                return Ok(None);
            }
            let line = self.line.strip_suffix(b"\n").unwrap_or(&self.line);
            let line = line.strip_suffix(b"\r").unwrap_or(line);
            if !line.starts_with(&self.marker) {
                return Err("body before first record header".into());
            }
            line.to_vec()
        };
        // A single-byte prefix can be recognized at each physical line start.
        // Join directly from buffered input instead of copying line endings
        // into the record and moving the body again to remove them.
        if self.marker.len() == 1
            && !matches!(self.marker[0], b'\r' | b'\n')
            && self.separator.is_empty()
        {
            return self.next_compacted_body(header).map(Some);
        }
        let mut body = std::mem::take(&mut self.spare);
        loop {
            let count = self
                .reader
                .read_until(self.marker[0], &mut body)
                .map_err(|e| e.to_string())?;
            if count == 0 {
                self.ended = true;
                break;
            }
            let end = body.len();
            if body[end - 1] == self.marker[0] && (end == 1 || body[end - 2] == b'\n') {
                self.line.clear();
                self.line.push(self.marker[0]);
                self.reader
                    .read_until(b'\n', &mut self.line)
                    .map_err(|e| e.to_string())?;
                let line = self.line.strip_suffix(b"\n").unwrap_or(&self.line);
                let line = line.strip_suffix(b"\r").unwrap_or(line);
                if line.starts_with(&self.marker) {
                    self.pending = Some(line.to_vec());
                    body.pop();
                    break;
                }
                body.extend_from_slice(&self.line[1..]);
            }
        }
        if self.separator.is_empty() {
            let mut start = 0;
            let mut output = 0;
            while start < body.len() {
                let newline = memchr::memchr(b'\n', &body[start..]).map(|i| start + i);
                let end = newline.unwrap_or(body.len());
                let trim = if end > start && body[end - 1] == b'\r' {
                    end - 1
                } else {
                    end
                };
                body.copy_within(start..trim, output);
                output += trim - start;
                start = newline.map_or(body.len(), |i| i + 1);
            }
            body.truncate(output);
        } else {
            let mut joined = Vec::new();
            for (index, line) in body.split_inclusive(|&b| b == b'\n').enumerate() {
                if index > 0 {
                    joined.extend_from_slice(&self.separator);
                }
                let line = line.strip_suffix(b"\n").unwrap_or(line);
                let line = line.strip_suffix(b"\r").unwrap_or(line);
                joined.extend_from_slice(line);
            }
            body = joined;
        }
        Ok(Some(Record {
            header: Arc::new(header),
            body: Arc::new(body),
        }))
    }

    fn next_compacted_body(&mut self, header: Vec<u8>) -> Result<Record, String> {
        let marker = self.marker[0];
        let mut body = std::mem::take(&mut self.spare);
        let mut line_start = true;
        loop {
            let (consumed, next_header, eof) = {
                let input = match self.reader.fill_buf() {
                    Ok(input) => input,
                    Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(error) => return Err(error.to_string()),
                };
                let mut position = 0;
                let mut next_header = false;
                while position < input.len() {
                    if line_start && input[position] == marker {
                        next_header = true;
                        break;
                    }
                    match memchr::memchr(b'\n', &input[position..]) {
                        Some(relative) => {
                            let end = position + relative;
                            // CR and LF can lie in different input buffers.
                            // Remove exactly one CR from this physical line,
                            // never a CR retained from a preceding blank line.
                            if end == position && !line_start && body.last() == Some(&b'\r') {
                                body.pop();
                            }
                            let trim = if end > position && input[end - 1] == b'\r' {
                                end - 1
                            } else {
                                end
                            };
                            body.extend_from_slice(&input[position..trim]);
                            position = end + 1;
                            line_start = true;
                        }
                        None => {
                            body.extend_from_slice(&input[position..]);
                            position = input.len();
                            line_start = false;
                        }
                    }
                }
                (position, next_header, input.is_empty())
            };
            self.reader.consume(consumed);
            if next_header {
                self.line.clear();
                self.reader
                    .read_until(b'\n', &mut self.line)
                    .map_err(|error| error.to_string())?;
                let line = self.line.strip_suffix(b"\n").unwrap_or(&self.line);
                let line = line.strip_suffix(b"\r").unwrap_or(line);
                self.pending = Some(line.to_vec());
                break;
            }
            if eof {
                if !line_start && body.last() == Some(&b'\r') {
                    body.pop();
                }
                self.ended = true;
                break;
            }
        }
        Ok(Record {
            header: Arc::new(header),
            body: Arc::new(body),
        })
    }
}
impl Value {
    fn vector_len(&self) -> Result<usize, String> {
        match self {
            Self::Integers(v) => Ok(v.len()),
            Self::Numbers(v, _) => Ok(v.len()),
            Self::Texts(v) => Ok(v.len()),
            _ => Err("expected a vector".into()),
        }
    }
    fn compare_at(&self, i: usize, j: usize) -> std::cmp::Ordering {
        match self {
            Self::Integers(v) => v[i].cmp(&v[j]),
            Self::Numbers(v, _) => v[i].total_cmp(&v[j]),
            Self::Texts(v) => v[i].cmp(&v[j]),
            _ => unreachable!("validated sort key kind"),
        }
    }
}

/// Private handle storage used by the compiled collection program. Inputs and
/// outputs belong to the generic host; all program constants come from source.
pub struct Context {
    values: Vec<Value>,
    constants: Vec<Constant>,
    input: Option<Vec<u8>>,
    pub output: Vec<u8>,
    error: Option<String>,
    write_stdout: bool,
    partitions: usize,
    compact_counts: bool,
    standard_map: bool,
    pool: Option<Arc<rayon::ThreadPool>>,
    profile: bool,
    scratch: Arc<Mutex<Vec<Vec<f64>>>>,
    arguments: Vec<std::ffi::OsString>,
    legacy_io: bool,
    host_io: Option<Box<dyn NativeHostIo>>,
    array_grain: usize,
    regexes: std::collections::HashMap<Vec<u8>, regex::bytes::Regex>,
    independent_tasks: bool,
    reuse_byte_buffers: bool,
    byte_scratch: Vec<Vec<u8>>,
}
impl Context {
    pub fn new(constants: &[Constant], input: Option<Vec<u8>>, write_stdout: bool) -> Self {
        let mut context = Self::pure(constants);
        context.input = input;
        context.write_stdout = write_stdout;
        context.arguments = std::env::args_os().collect();
        context.legacy_io = true;
        context
    }

    /// Construct a context whose only external resources are the host's bound
    /// ports. No process arguments or streams are read during construction.
    pub fn activated(
        constants: &[Constant],
        host_io: Box<dyn NativeHostIo>,
    ) -> Result<Self, String> {
        for request in host_io.requirements() {
            request.validate()?;
        }
        let mut context = Self::pure(constants);
        context.host_io = Some(host_io);
        Ok(context)
    }

    fn pure(constants: &[Constant]) -> Self {
        Self {
            constants: constants.to_vec(),
            values: constants
                .iter()
                .map(|c| match c {
                    Constant::Text(s) => Value::Text(Arc::new(s.as_bytes().to_vec())),
                    Constant::Integer(x) => Value::Integer(*x),
                    Constant::Float(bits) => {
                        Value::Numbers(Arc::new(vec![f64::from_bits(*bits)]), true)
                    }
                    Constant::Function(function) => Value::Function(Arc::new((**function).clone())),
                    Constant::Parameter(_, _) | Constant::Capture(_, _) => Value::Unit,
                })
                .collect(),
            input: None,
            output: Vec::new(),
            error: None,
            write_stdout: false,
            partitions: 1,
            compact_counts: false,
            standard_map: false,
            pool: None,
            profile: false,
            scratch: Arc::new(Mutex::new(Vec::new())),
            arguments: Vec::new(),
            legacy_io: false,
            host_io: None,
            array_grain: 16,
            regexes: std::collections::HashMap::new(),
            independent_tasks: false,
            reuse_byte_buffers: false,
            byte_scratch: Vec::new(),
        }
    }
    pub(super) fn matches_constants(&self, constants: &[Constant]) -> bool {
        self.constants == constants
    }
    fn capture_values(
        &self,
        function: &Callable,
        arguments: &[usize],
    ) -> Result<Vec<Value>, String> {
        let count = function.captures.len();
        if count > arguments.len() {
            let Some(Value::CapturedValues(values)) = self.values.get(arguments[0]) else {
                return Err("missing native capture environment".into());
            };
            if values.len() != count {
                return Err("native capture environment arity mismatch".into());
            }
            return Ok((**values).clone());
        }
        let arguments = &arguments[..count];
        if arguments.len() != function.captures.len() {
            return Err("native callable capture arity mismatch".into());
        }
        arguments
            .iter()
            .map(|&id| {
                self.values
                    .get(id)
                    .cloned()
                    .ok_or_else(|| "missing native capture".into())
            })
            .collect()
    }
    /// Generic histogram policy; partitions use disjoint window starts and
    /// overlap only the input bytes needed to complete boundary windows.
    pub fn with_histogram_policy(
        mut self,
        partitions: usize,
        compact_counts: bool,
        profile: bool,
    ) -> Result<Self, String> {
        if !(1..=64).contains(&partitions) {
            return Err("partitions must be 1..64".into());
        }
        self.partitions = partitions;
        self.compact_counts = compact_counts;
        self.profile = profile;
        if partitions > 1 {
            self.pool = Some(Arc::new(
                rayon::ThreadPoolBuilder::new()
                    .build()
                    .map_err(|e| e.to_string())?,
            ));
        }
        Ok(self)
    }
    pub fn with_standard_map(mut self, enabled: bool) -> Self {
        self.standard_map = enabled;
        self
    }

    /// # Safety
    /// `entry` must be the entry point of an object emitted for precisely the
    /// constants used to construct this context, linked with this ABI version.
    pub unsafe fn run(&mut self, entry: NativeEntry) -> Result<(), String> {
        // SAFETY: Required by this API's contract; self stays exclusively live.
        unsafe { entry((self as *mut Context).cast()) };
        self.finish_execution()
    }
    fn finish_execution(&mut self) -> Result<(), String> {
        let mut errors = Vec::new();
        if let Some(error) = self.error.take() {
            errors.push(error);
        }
        // Join unused computations as well so process completion includes all
        // requested histogram work, even if the source doesn't read a result.
        for value in &self.values {
            if let Value::Histogram(p) = value {
                if let Err(error) = p.resolve() {
                    errors.push(error);
                }
            }
            if let Value::DeferredInteger(p) = value {
                if let Err(error) = p.resolve() {
                    errors.push(error);
                }
            }
        }
        if self.write_stdout {
            if let Err(error) = std::io::stdout().flush() {
                errors.push(error.to_string());
            }
        }
        if let Some(host) = &mut self.host_io {
            if let Err(error) = host.finish(errors.is_empty()) {
                errors.push(error);
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("; "))
        }
    }
    fn text(&self, id: usize) -> Result<Arc<Vec<u8>>, String> {
        match self.values.get(id) {
            Some(Value::Text(s)) => Ok(s.clone()),
            Some(v @ Value::TextView(_)) => {
                let mut out = Vec::new();
                v.write_text(&mut out)?;
                Ok(Arc::new(out))
            }
            _ => Err("expected text handle".into()),
        }
    }
    fn integer(&self, id: usize) -> Result<u64, String> {
        match self.values.get(id) {
            Some(Value::Integer(n)) => Ok(*n),
            Some(Value::DeferredInteger(p)) => p.resolve(),
            _ => Err("expected integer handle".into()),
        }
    }
    fn histogram(&self, id: usize) -> Result<Arc<Histogram>, String> {
        match self.values.get(id) {
            Some(Value::Histogram(p)) => p.resolve(),
            _ => Err("expected histogram handle".into()),
        }
    }
    fn integers(&self, id: usize) -> Result<Arc<Vec<u64>>, String> {
        match self.values.get(id) {
            Some(Value::Integers(v)) => Ok(v.clone()),
            _ => Err("expected integer vector".into()),
        }
    }
    fn texts(&self, id: usize) -> Result<Arc<Vec<Vec<u8>>>, String> {
        match self.values.get(id) {
            Some(Value::Texts(v)) => Ok(v.clone()),
            _ => Err("expected text vector".into()),
        }
    }
    /// Target tasks per row partition; smaller tasks allow work stealing on
    /// heterogeneous CPUs. This is independent of histogram partition policy.
    pub fn with_array_grain(mut self, grain: usize) -> Result<Self, String> {
        if !(1..=1024).contains(&grain) {
            return Err("array grain must be 1..1024".into());
        }
        self.array_grain = grain;
        Ok(self)
    }
    pub fn with_arguments(mut self, arguments: Vec<String>) -> Self {
        self.arguments = arguments.into_iter().map(Into::into).collect();
        self
    }
    /// Enable independent, immutable library tasks in the existing bounded
    /// pool. This policy changes scheduling, never source arithmetic or IO order.
    pub fn with_independent_tasks(mut self, enabled: bool) -> Self {
        self.independent_tasks = enabled;
        self
    }
    /// Reuse exclusively owned, dead byte allocations for subsequent text
    /// outputs. Live aliases and independent readers retain their own storage.
    pub fn with_byte_buffer_reuse(mut self, enabled: bool) -> Self {
        self.reuse_byte_buffers = enabled;
        self
    }
    fn allocate_numbers(&self, length: usize) -> Result<Vec<f64>, String> {
        if length > isize::MAX as usize / 8 {
            return Err("numeric allocation exceeds addressable storage".into());
        }
        let mut scratch = self
            .scratch
            .lock()
            .map_err(|_| "numeric scratch lock poisoned")?;
        let mut result = if let Some(index) = scratch.iter().position(|v| v.capacity() >= length) {
            scratch.swap_remove(index)
        } else {
            Vec::new()
        };
        result
            .try_reserve(length.saturating_sub(result.len()))
            .map_err(|e| e.to_string())?;
        result.resize(length, 0.0);
        Ok(result)
    }
    fn recycle(&mut self, value: Value) {
        if self.reuse_byte_buffers {
            if let Value::Text(bytes) = value {
                if let Ok(mut buffer) = Arc::try_unwrap(bytes) {
                    buffer.clear();
                    if self.byte_scratch.len() < 2 {
                        self.byte_scratch.push(buffer);
                    } else if let Some((index, smallest)) = self
                        .byte_scratch
                        .iter()
                        .enumerate()
                        .min_by_key(|(_, b)| b.capacity())
                    {
                        if buffer.capacity() > smallest.capacity() {
                            self.byte_scratch[index] = buffer;
                        }
                    }
                }
                return;
            }
        }
        if let Value::Numbers(values, false) = value {
            if let Ok(mut buffer) = Arc::try_unwrap(values) {
                buffer.clear();
                if let Ok(mut scratch) = self.scratch.lock() {
                    if scratch.len() < 64 {
                        scratch.push(buffer);
                    }
                }
            }
        }
    }
    fn op(&mut self, op: u64, a: [usize; 5], kernel: usize) -> Result<Value, String> {
        let scalar_consumers = match op & 0xffff_ffff {
            19 => 1,
            20..=24 => 2,
            _ => 0,
        };
        for &id in &a[..scalar_consumers] {
            if let Some(Value::DeferredInteger(p)) = self.values.get(id) {
                let value = p.resolve()?;
                self.values[id] = Value::Integer(value);
            }
        }
        let consume = op & (1u64 << 32) != 0;
        let op = op & 0xffff_ffff;
        if !self.legacy_io && matches!(op, 1 | 8 | 9 | 10 | 12 | 32 | 41 | 45 | 52) {
            return Err("legacy process IO is unavailable in an activated or pure context; use a bound host port".into());
        }
        if kernel == 0 && matches!(op, 5 | 20..=23 | 35..=39 | 73 | 42..=46) {
            return self.resident_op(op, a, consume);
        }
        Ok(match op {
            1 => {
                let bytes = if let Some(input) = self.input.take() {
                    input
                } else {
                    let mut reader = std::io::stdin().lock();
                    let mut bytes = Vec::new();
                    reader.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
                    bytes
                };
                Value::Text(Arc::new(bytes))
            }
            2 | 10 => {
                let begin = self.text(a[1])?;
                let end = self.text(a[2])?;
                let bytes = if op == 2 {
                    let input = self.text(a[0])?;
                    record(std::io::Cursor::new(input.as_slice()), &begin, &end, None)?
                } else if let Some(input) = self.input.take() {
                    record(std::io::Cursor::new(input), &begin, &end, None)?
                } else {
                    record(std::io::stdin().lock(), &begin, &end, None)?
                };
                Value::Text(Arc::new(bytes))
            }
            11 => Value::Unit,
            3 => {
                let text = self.text(a[0])?;
                let alphabet = self.text(a[1])?;
                let mode = self.text(a[2])?;
                let table = encoding_table(&alphabet, &mode)?;
                let mut bytes = Vec::with_capacity(text.len());
                for &byte in text.iter() {
                    let code = table[byte as usize];
                    if code == u16::MAX {
                        return Err(format!("byte {byte} is not in the alphabet"));
                    }
                    bytes.push(code as u8);
                }
                Value::Bytes(Arc::new(bytes), alphabet.len())
            }
            12 => {
                let begin = self.text(a[0])?;
                let alphabet = self.text(a[1])?;
                let mode = self.text(a[2])?;
                let end = self.text(a[3])?;
                let table = encoding_table(&alphabet, &mode)?;
                let bytes = if let Some(input) = self.input.take() {
                    record(std::io::Cursor::new(input), &begin, &end, Some(&table))?
                } else {
                    record(std::io::stdin().lock(), &begin, &end, Some(&table))?
                };
                Value::Bytes(Arc::new(bytes), alphabet.len())
            }
            4 => {
                let (bytes, symbols) = match self.values.get(a[0]) {
                    Some(Value::Bytes(b, n)) => (b.clone(), *n),
                    _ => return Err("expected bytes".into()),
                };
                let bits = self.integer(a[1])?;
                let width = self.integer(a[2])?;
                if bits == 0 || bits > 8 || width == 0 || width > 64 || bits * width > 64 {
                    return Err("invalid packed window shape".into());
                }
                if symbols as u64 > (1u64 << bits) {
                    return Err("encoded symbol exceeds window bit width".into());
                }
                Value::Window(Window {
                    bytes,
                    bits: bits as u8,
                    width: width as u8,
                })
            }
            5 => {
                let window = match self.values.get(a[0]) {
                    Some(Value::Window(w)) => w.clone(),
                    _ => return Err("expected packed windows".into()),
                };
                if kernel == 0 {
                    return Err("missing native window kernel".into());
                }
                // SAFETY: The compiler supplies a same-object function address
                // with exactly NativeWindow ABI and this admitted window shape.
                let function = unsafe { std::mem::transmute::<usize, NativeWindow>(kernel) };
                let (partitions, compact, pool, profile) = (
                    self.partitions,
                    self.compact_counts,
                    self.pool.clone(),
                    self.profile,
                );
                let standard = self.standard_map;
                let task = std::thread::spawn(move || {
                    let start_time = std::time::Instant::now();
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
                            // SAFETY: This slice contains exactly end-start
                            // window starts plus their trailing symbols. Its
                            // allocation stays live; this worker owns its table.
                            unsafe { function(bytes.as_ptr(), bytes.len(), &mut map) };
                        }
                        map
                    };
                    let mut maps = if let Some(pool) = &pool {
                        pool.install(|| (0..pieces).into_par_iter().map(count).collect::<Vec<_>>())
                    } else {
                        vec![count(0)]
                    }
                    .into_iter();
                    let mut map = maps.next().expect("at least one histogram partition");
                    for other in maps {
                        map.merge(other);
                    }
                    assert_eq!(
                        map.rows().iter().map(|(_, count)| count).sum::<u64>(),
                        total as u64,
                        "native traversal changed the number of histogram updates"
                    );
                    if profile {
                        eprintln!(
                            "histogram bits={} width={} partitions={} windows={} elapsed={:.6}",
                            window.bits,
                            window.width,
                            pieces,
                            total,
                            start_time.elapsed().as_secs_f64()
                        );
                    }
                    Histogram { map }
                });
                Value::Histogram(Arc::new(Pending {
                    value: OnceLock::new(),
                    task: Mutex::new(Some(task)),
                }))
            }
            13 | 14 => {
                let h = self.histogram(a[0])?;
                Value::Integers(Arc::new(
                    h.map
                        .rows()
                        .into_iter()
                        .map(|(key, count)| if op == 13 { key } else { count })
                        .collect(),
                ))
            }
            15 => {
                let values = self.integers(a[0])?;
                let bits = self.integer(a[1])?;
                let width = self.integer(a[2])?;
                let alphabet = self.text(a[3])?;
                if bits == 0 || bits > 8 || width == 0 || width > 64 || bits * width > 64 {
                    return Err("invalid decode shape".into());
                }
                Value::Texts(Arc::new(
                    values
                        .iter()
                        .map(|&key| decode(key, bits as u8, width as u8, &alphabet))
                        .collect::<Result<Vec<_>, _>>()?,
                ))
            }
            16 => {
                let first = self.values.get(a[0]).ok_or("missing sort key")?;
                let second = self.values.get(a[2]).ok_or("missing sort key")?;
                let len = first.vector_len()?;
                if len != second.vector_len()? {
                    return Err("sort key lengths differ".into());
                }
                let descending = |mode: &[u8]| match mode {
                    b"asc" => Ok(false),
                    b"desc" => Ok(true),
                    _ => Err("sort direction must be asc or desc"),
                };
                let reverse1 = descending(&self.text(a[1])?)?;
                let reverse2 = descending(&self.text(a[3])?)?;
                let mut indices = (0..len).collect::<Vec<_>>();
                indices.sort_unstable_by(|&i, &j| {
                    let c = first.compare_at(i, j);
                    let c = if reverse1 { c.reverse() } else { c };
                    c.then_with(|| {
                        let c = second.compare_at(i, j);
                        if reverse2 { c.reverse() } else { c }
                    })
                    .then(i.cmp(&j))
                });
                Value::Indices(Arc::new(indices))
            }
            17 => {
                let Some(Value::Indices(indices)) = self.values.get(a[1]) else {
                    return Err("expected indices".into());
                };
                let source = self.values.get(a[0]).ok_or("missing selected vector")?;
                if indices
                    .iter()
                    .any(|&i| i >= source.vector_len().unwrap_or(0))
                {
                    return Err("selection index outside vector".into());
                }
                match source {
                    Value::Integers(v) => {
                        Value::Integers(Arc::new(indices.iter().map(|&i| v[i]).collect()))
                    }
                    Value::Numbers(v, _) => {
                        Value::Numbers(Arc::new(indices.iter().map(|&i| v[i]).collect()), false)
                    }
                    Value::Texts(v) => {
                        Value::Texts(Arc::new(indices.iter().map(|&i| v[i].clone()).collect()))
                    }
                    _ => return Err("unsupported selected vector".into()),
                }
            }
            18 => {
                let values = self.integers(a[0])?;
                Value::Integer(
                    values
                        .iter()
                        .try_fold(0u64, |a, b| a.checked_add(*b).ok_or("integer sum overflow"))?,
                )
            }
            19 => match self.values.get(a[0]) {
                Some(Value::Integer(value)) => Value::Numbers(Arc::new(vec![*value as f64]), true),
                Some(Value::Integers(values)) => Value::Numbers(
                    Arc::new(values.iter().map(|&value| value as f64).collect()),
                    false,
                ),
                _ => return Err("expected integer values".into()),
            },
            20..=23 => {
                if let (Some(Value::Integer(left)), Some(Value::Integer(right))) =
                    (self.values.get(a[0]), self.values.get(a[1]))
                {
                    if kernel == 0 {
                        return Err("missing native unsigned kernel".into());
                    }
                    let mut output = 0u64;
                    // SAFETY: kind-directed same-object emission supplies this
                    // checked u64 ABI. All buffers are initialized, disjoint,
                    // and live for the single-element call. Failed arithmetic
                    // returns status without publishing an output value.
                    let function = unsafe { std::mem::transmute::<usize, NativeUnsigned>(kernel) };
                    numeric_status(unsafe { function(left, 1, right, 1, &mut output, 1) })?;
                    return Ok(Value::Integer(output));
                }
                let (Some(Value::Numbers(left, scalar1)), Some(Value::Numbers(right, scalar2))) =
                    (self.values.get(a[0]), self.values.get(a[1]))
                else {
                    return Err("expected numeric values".into());
                };
                let length = if *scalar1 {
                    right.len()
                } else if *scalar2 || left.len() == right.len() {
                    left.len()
                } else {
                    return Err("numeric vector shapes differ".into());
                };
                if kernel == 0 {
                    return Err("missing native numeric kernel".into());
                }
                let mut output = self.allocate_numbers(length)?;
                // SAFETY: The same-object compiler supplies this typed native
                // arithmetic function. Checked vector shapes bound every load,
                // and this invocation exclusively owns its output allocation.
                let function = unsafe { std::mem::transmute::<usize, NativeNumeric>(kernel) };
                unsafe {
                    function(
                        left.as_ptr(),
                        left.len(),
                        right.as_ptr(),
                        right.len(),
                        output.as_mut_ptr(),
                        length,
                    )
                };
                Value::Numbers(Arc::new(output), *scalar1 && *scalar2)
            }
            32 => {
                let index = usize::try_from(self.integer(a[0])?)
                    .map_err(|_| "argument ordinal exceeds usize")?;
                let text = self
                    .arguments
                    .get(index)
                    .ok_or("missing command-line argument")?;
                Value::Integer(
                    text.to_str()
                        .ok_or("command-line argument is not valid Unicode")?
                        .parse()
                        .map_err(|_| "expected unsigned command-line argument")?,
                )
            }
            33 => {
                let length = usize::try_from(self.integer(a[0])?)
                    .map_err(|_| "vector length exceeds usize")?;
                let Some(Value::Numbers(value, true)) = self.values.get(a[1]) else {
                    return Err("fill requires scalar f64".into());
                };
                let mut output = self.allocate_numbers(length)?;
                output.fill(value[0]);
                Value::Numbers(Arc::new(output), false)
            }
            34 => Value::Integer(
                self.values
                    .get(a[0])
                    .ok_or("missing sequence")?
                    .vector_len()? as u64,
            ),
            74 => {
                let Some(Value::Function(function)) = self.values.get(a[0]) else {
                    return Err("missing capture callable".into());
                };
                let count = function.captures.len().min(4);
                let values = a[1..1 + count]
                    .iter()
                    .map(|&id| {
                        self.values
                            .get(id)
                            .cloned()
                            .ok_or("missing capture value".to_string())
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Value::CapturedValues(Arc::new(values))
            }
            75 => {
                let Some(Value::CapturedValues(previous)) = self.values.get(a[0]) else {
                    return Err("missing capture environment".into());
                };
                let mut values = (**previous).clone();
                values.push(
                    self.values
                        .get(a[1])
                        .ok_or("missing capture value")?
                        .clone(),
                );
                Value::CapturedValues(Arc::new(values))
            }
            73 => {
                let rows = self.integer(a[0])?;
                let columns = self.integer(a[1])?;
                if kernel == 0 {
                    return Err("missing native matrix domain validation".into());
                }
                // SAFETY: same-object compiler emits a scalar bounds/check ABI,
                // receiving only integer dimensions and performing no pointer access.
                let function = unsafe { std::mem::transmute::<usize, NativeDomain>(kernel) };
                numeric_status(unsafe { function(rows, columns) })?;
                Value::Unit
            }
            72 => {
                let expected = usize::try_from(self.integer(a[1])?)
                    .map_err(|_| "shape length exceeds usize")?;
                let Some(Value::Numbers(value, false)) = self.values.get(a[0]) else {
                    return Err("shape guard requires a vector".into());
                };
                if value.len() != expected {
                    return Err("matrix/vector shapes differ".into());
                }
                Value::Numbers(value.clone(), false)
            }
            35 | 36 => {
                let Some(Value::Numbers(input, false)) = self.values.get(a[0]) else {
                    return Err("matrix input must be a vector".into());
                };
                let rows =
                    usize::try_from(self.integer(a[1])?).map_err(|_| "row count exceeds usize")?;
                if kernel == 0 {
                    return Err("missing native matrix kernel".into());
                }
                let mut output = self.allocate_numbers(rows)?;
                // SAFETY: the same-object compiler emits this ABI from an admitted
                // two-index scalar function. Each call receives all input columns
                // and an exclusive disjoint output slice for the supplied rows.
                let function = unsafe { std::mem::transmute::<usize, NativeMatrix>(kernel) };
                let chunk = rows.div_ceil(self.partitions * self.array_grain).max(1);
                let run = |output: &mut [f64]| {
                    output
                        .par_chunks_mut(chunk)
                        .enumerate()
                        .map(|(index, part)| {
                            let start = index * chunk;
                            numeric_status(unsafe {
                                function(
                                    input.as_ptr(),
                                    input.len(),
                                    start,
                                    start + part.len(),
                                    part.as_mut_ptr(),
                                )
                            })
                        })
                        // Kernels report their first failing row. Preserve
                        // row order across chunks as well, regardless of which
                        // worker finishes first.
                        .find_first(Result::is_err)
                        .unwrap_or(Ok(()))
                };
                if let Some(pool) = &self.pool {
                    pool.install(|| run(&mut output))?;
                } else {
                    numeric_status(unsafe {
                        function(input.as_ptr(), input.len(), 0, rows, output.as_mut_ptr())
                    })?;
                }
                Value::Numbers(Arc::new(output), false)
            }
            37 => {
                let (Some(Value::Numbers(left, false)), Some(Value::Numbers(right, false))) =
                    (self.values.get(a[0]), self.values.get(a[1]))
                else {
                    return Err("dot requires vectors".into());
                };
                if left.len() != right.len() {
                    return Err("dot vector shapes differ".into());
                }
                if kernel == 0 {
                    return Err("missing native dot kernel".into());
                }
                // SAFETY: emitted ABI, checked equal lengths, borrowed immutable vectors.
                let function = unsafe { std::mem::transmute::<usize, NativeDot>(kernel) };
                let result = unsafe { function(left.as_ptr(), right.as_ptr(), left.len()) };
                Value::Numbers(Arc::new(vec![result]), true)
            }
            38 => {
                let Some(Value::Numbers(value, true)) = self.values.get(a[0]) else {
                    return Err("sqrt requires scalar".into());
                };
                if kernel == 0 {
                    return Err("missing native sqrt kernel".into());
                }
                // SAFETY: compiler-emitted unary scalar ABI, no memory accesses.
                let function = unsafe { std::mem::transmute::<usize, NativeSqrt>(kernel) };
                Value::Numbers(Arc::new(vec![unsafe { function(value[0]) }]), true)
            }
            39 => {
                let seed = self
                    .values
                    .get(a[0])
                    .ok_or("missing iteration seed")?
                    .clone();
                let iterations = self.integer(a[1])?;
                let Some(Value::Function(function)) = self.values.get(a[2]) else {
                    return Err("missing iteration callable".into());
                };
                if kernel == 0 {
                    return Err("missing native iteration loop".into());
                }
                let mut state = IterationState {
                    current: seed,
                    function: function.clone(),
                    captures: self.capture_values(function, &a[3..])?,
                    pool: self.pool.clone(),
                    scratch: self.scratch.clone(),
                    partitions: self.partitions,
                    array_grain: self.array_grain,
                    error: None,
                };
                // SAFETY: the compiler emits this native counted loop and its
                // callable in the same object. The state lives exclusively for
                // this invocation; invoke() checks the closed function scope.
                let function = unsafe { std::mem::transmute::<usize, NativeIteration>(kernel) };
                unsafe { function(&mut state, iterations) };
                if let Some(error) = state.error {
                    return Err(error);
                }
                state.current
            }
            24 => {
                let decimals = self.integer(a[1])?;
                if decimals > 12 {
                    return Err("decimal precision exceeds 12".into());
                }
                let (rows, scalar) = match self.values.get(a[0]) {
                    Some(Value::Integer(value)) => {
                        if decimals != 0 {
                            return Err("integer formatting requires precision zero".into());
                        }
                        (vec![value.to_string().into_bytes()], true)
                    }
                    Some(Value::Integers(values)) => {
                        if decimals != 0 {
                            return Err("integer formatting requires precision zero".into());
                        }
                        (
                            values.iter().map(|v| v.to_string().into_bytes()).collect(),
                            false,
                        )
                    }
                    Some(Value::Numbers(values, scalar)) => (
                        values
                            .iter()
                            .map(|value| format!("{:.*}", decimals as usize, value).into_bytes())
                            .collect(),
                        *scalar,
                    ),
                    _ => return Err("expected numeric format input".into()),
                };
                if scalar {
                    Value::Text(Arc::new(
                        rows.into_iter()
                            .next()
                            .ok_or("missing scalar format value")?,
                    ))
                } else {
                    Value::Texts(Arc::new(rows))
                }
            }
            25 => {
                let left = self.texts(a[0])?;
                let right = self.texts(a[1])?;
                let separator = self.text(a[2])?;
                if left.len() != right.len() {
                    return Err("text columns have unequal lengths".into());
                }
                Value::Texts(Arc::new(
                    left.iter()
                        .zip(right.iter())
                        .map(|(a, b)| {
                            let mut row = a.clone();
                            row.extend_from_slice(&separator);
                            row.extend_from_slice(b);
                            row
                        })
                        .collect(),
                ))
            }
            26 | 31 => {
                let rows = self.texts(a[0])?;
                let separator = self.text(a[1])?;
                let mut bytes = Vec::new();
                for (i, row) in rows.iter().enumerate() {
                    if op == 26 && i > 0 {
                        bytes.extend_from_slice(&separator);
                    }
                    bytes.extend_from_slice(row);
                    if op == 31 {
                        bytes.extend_from_slice(&separator);
                    }
                }
                Value::Text(Arc::new(bytes))
            }
            40 | 41 => {
                let marker = self.text(a[1])?;
                let separator = self.text(a[2])?;
                let reader: Box<dyn BufRead + Send> = if op == 40 {
                    Box::new(std::io::Cursor::new(self.text(a[0])?.as_ref().clone()))
                } else if let Some(input) = self.input.take() {
                    Box::new(std::io::Cursor::new(input))
                } else {
                    Box::new(std::io::BufReader::with_capacity(65536, std::io::stdin()))
                };
                let mut reader = RecordReader::new(reader, &marker, &separator)?;
                let mut records = VecDeque::new();
                while let Some(record) = reader.next()? {
                    records.push_back(record);
                }
                Value::Records(Arc::new(Mutex::new(Records::Stored(records))))
            }
            52 => {
                let marker = self.text(a[0])?;
                let separator = self.text(a[1])?;
                let reader: Box<dyn BufRead + Send> = if let Some(input) = self.input.take() {
                    Box::new(std::io::Cursor::new(input))
                } else {
                    Box::new(std::io::BufReader::with_capacity(65536, std::io::stdin()))
                };
                Value::Records(Arc::new(Mutex::new(Records::Streaming(RecordReader::new(
                    reader, &marker, &separator,
                )?))))
            }
            42 => {
                let Some(Value::Function(function)) = self.values.get(a[1]) else {
                    return Err("expected record callable".into());
                };
                let function = function.clone();
                let input = if consume {
                    std::mem::replace(&mut self.values[a[0]], Value::Unit)
                } else {
                    self.values[a[0]].clone()
                };
                let Value::Records(records) = input else {
                    return Err("expected records".into());
                };
                if kernel == 0 {
                    return Err("missing native record callable".into());
                }
                let copy = if Arc::strong_count(&records) > 1 {
                    match &*records.lock().map_err(|_| "records lock poisoned")? {
                        Records::Stored(v) => Some(Records::Stored(v.clone())),
                        Records::Streaming(_) => None, // explicit input streams are one-shot cursors
                    }
                } else {
                    None
                };
                let records = if let Some(copy) = copy {
                    Arc::new(Mutex::new(copy))
                } else {
                    records
                };
                let captures = self.capture_values(&function, &a[2..])?;
                Value::TextStream(Arc::new(MappedRecords {
                    records,
                    function,
                    entry: kernel,
                    captures,
                }))
            }
            43 | 44 | 46 => {
                if kernel == 0 {
                    return Err("missing native byte loop".into());
                }
                let table = if op == 44 {
                    Arc::new([0u16; 256])
                } else {
                    let Some(Value::ByteTable(table)) = self.values.get(a[1]) else {
                        return Err("expected byte table".into());
                    };
                    table.clone()
                };
                let input = if consume {
                    std::mem::replace(&mut self.values[a[0]], Value::Unit)
                } else {
                    self.values[a[0]].clone()
                };
                let Value::Bytes(input, symbols) = input else {
                    return Err("expected bytes".into());
                };
                // Reuse is legal only after a compiler-proven last use AND unique
                // Arc ownership. A shared input is copied before any mutation.
                let mut output = Arc::try_unwrap(input).unwrap_or_else(|v| v.as_ref().clone());
                let length = output.len();
                let total = if op == 43 {
                    length
                } else {
                    length / 2 + length % 2
                };
                let pieces = if total < 65536 { 1 } else { self.partitions };
                let src = output.as_ptr() as usize;
                let dst = output.as_mut_ptr() as usize;
                // SAFETY: same-object byte ABI; live initialized allocation,
                // valid 256-entry table. Each pair is loaded before either write.
                // Pair-start partitions own disjoint front/back ranges, including
                // at most one central byte; all workers join before publication.
                let function = unsafe { std::mem::transmute::<usize, NativeBytes>(kernel) };
                let run = |piece| {
                    let boundary = |i| total / pieces * i + (total % pieces) * i / pieces;
                    unsafe {
                        function(
                            src as *const u8,
                            dst as *mut u8,
                            length,
                            table.as_ptr(),
                            boundary(piece),
                            boundary(piece + 1),
                        )
                    }
                };
                let status = if let Some(pool) = &self.pool {
                    pool.install(|| (0..pieces).into_par_iter().map(run).max().unwrap_or(0))
                } else {
                    run(0)
                };
                if status != 0 {
                    return Err("byte is not in translation table".into());
                }
                let symbols = if op == 44 {
                    symbols
                } else {
                    table
                        .iter()
                        .copied()
                        .filter(|&code| code != u16::MAX)
                        .max()
                        .map_or(0, |code| code as usize + 1)
                };
                Value::Bytes(Arc::new(output), symbols)
            }
            47 => Value::Unit,
            48 => {
                let from = self.text(a[0])?;
                let to = self.text(a[1])?;
                let missing = self.text(a[2])?;
                Value::ByteTable(Arc::new(super::bytes::table(&from, &to, &missing)?))
            }
            49 => {
                let Some(Value::Bytes(bytes, _)) = self.values.get(a[0]) else {
                    return Err("wrap requires bytes".into());
                };
                let width =
                    usize::try_from(self.integer(a[1])?).map_err(|_| "wrap width exceeds usize")?;
                if width == 0 {
                    return Err("wrap width must be positive".into());
                }
                Value::TextView(Arc::new(TextView::Wrap(
                    bytes.clone(),
                    width,
                    self.text(a[2])?,
                    kernel,
                )))
            }
            51 => Value::Bytes(self.text(a[0])?, 256),
            53 | 54 => {
                let input = self.text(a[0])?;
                let pattern = self.text(a[1])?;
                if !self.regexes.contains_key(pattern.as_slice()) {
                    let pattern_text = std::str::from_utf8(&pattern)
                        .map_err(|e| format!("regex pattern must be UTF-8: {e}"))?;
                    let regex = regex::bytes::RegexBuilder::new(pattern_text)
                        .unicode(false)
                        .build()
                        .map_err(|e| format!("invalid byte regex: {e}"))?;
                    self.regexes.insert(pattern.as_ref().clone(), regex);
                }
                let regex = &self.regexes[pattern.as_slice()];
                if op == 53 {
                    if let Some(pool) = self
                        .pool
                        .as_ref()
                        .filter(|_| self.independent_tasks && input.len() >= 16384)
                    {
                        let regex = regex.clone();
                        let (sender, receiver) = std::sync::mpsc::channel();
                        pool.spawn(move || {
                            let _ = sender.send(regex.find_iter(&input).count() as u64);
                        });
                        Value::DeferredInteger(Arc::new(DeferredInteger {
                            value: OnceLock::new(),
                            receiver: Mutex::new(Some(receiver)),
                        }))
                    } else {
                        Value::Integer(regex.find_iter(&input).count() as u64)
                    }
                } else {
                    let replacement = self.text(a[2])?;
                    // This operation uses literal replacement bytes. Capture
                    // expansion belongs to a distinct library operation.
                    if self.reuse_byte_buffers {
                        let mut output = self.byte_scratch.pop().unwrap_or_default();
                        output.try_reserve(input.len()).map_err(|e| e.to_string())?;
                        let mut end = 0;
                        for matched in regex.find_iter(&input) {
                            output.extend_from_slice(&input[end..matched.start()]);
                            output.extend_from_slice(&replacement);
                            end = matched.end();
                        }
                        output.extend_from_slice(&input[end..]);
                        Value::Text(Arc::new(output))
                    } else {
                        Value::Text(Arc::new(
                            regex
                                .replace_all(&input, regex::bytes::NoExpand(&replacement))
                                .into_owned(),
                        ))
                    }
                }
            }
            45 => {
                let Some(Value::TextStream(mapped)) = self.values.get(a[0]) else {
                    return Err("expected mapped text stream".into());
                };
                if kernel == 0 {
                    return Err("missing native record traversal".into());
                }
                let mapped = mapped.clone();
                let mut records = mapped.records.lock().map_err(|_| "records lock poisoned")?;
                let mut writer: Box<dyn Write> = if self.write_stdout {
                    Box::new(std::io::BufWriter::with_capacity(
                        1048576,
                        std::io::stdout(),
                    ))
                } else {
                    Box::new(Vec::new())
                };
                // Captured-output contexts use their actual output Vec directly.
                let writer: &mut dyn Write = if self.write_stdout {
                    &mut *writer
                } else {
                    &mut self.output
                };
                let mut state = RecordState {
                    records: &mut records,
                    function: &mapped.function,
                    captures: &mapped.captures,
                    entry: mapped.entry,
                    pool: self.pool.clone(),
                    partitions: self.partitions,
                    writer,
                    error: None,
                    profile: std::env::var("MECH_BYTE_PROFILE").as_deref() == Ok("1"),
                };
                // SAFETY: same-object loop passes this exclusive live state to
                // its bridge; every callback uses its matching emitted entry.
                let function = unsafe { std::mem::transmute::<usize, NativeRecords>(kernel) };
                unsafe { function(&mut state) };
                if let Some(error) = state.error {
                    return Err(error);
                }
                state.writer.flush().map_err(|e| e.to_string())?;
                Value::Unit
            }
            27 => {
                if matches!(self.values.get(a[0]), Some(Value::TextView(_)))
                    || matches!(self.values.get(a[1]), Some(Value::TextView(_)))
                {
                    Value::TextView(Arc::new(TextView::Concat(
                        self.values[a[0]].clone(),
                        self.values[a[1]].clone(),
                    )))
                } else {
                    let left = self.text(a[0])?;
                    let right = self.text(a[1])?;
                    let mut bytes = left.as_ref().clone();
                    bytes.extend_from_slice(&right);
                    Value::Text(Arc::new(bytes))
                }
            }
            28 => Value::Integer(self.text(a[0])?.len() as u64),
            29 => {
                let Some(Value::Bytes(bytes, symbols)) = self.values.get(a[0]) else {
                    return Err("expected encoded symbols".into());
                };
                let bits = self.integer(a[1])?;
                if bits == 0
                    || bits > 8
                    || bytes.len() as u64 * bits > 64
                    || *symbols as u64 > 1u64 << bits
                {
                    return Err("invalid key packing shape".into());
                }
                Value::Integer(
                    bytes
                        .iter()
                        .fold(0u64, |key, &byte| (key << bits) | byte as u64),
                )
            }
            30 => Value::Integer(self.histogram(a[0])?.map.get(self.integer(a[1])?)),
            8 => {
                let value = self.values.get(a[0]).ok_or("missing text")?;
                if self.write_stdout {
                    value.write_text(&mut std::io::stdout())?;
                } else {
                    value.write_text(&mut self.output)?;
                }
                Value::Unit
            }
            81 => {
                let request = self.resource_request(&a[..3], false)?;
                let host = self
                    .host_io
                    .as_mut()
                    .ok_or("native resource is not activated")?;
                let value = host.read(&request)?;
                std::str::from_utf8(&value).map_err(|_| "native text input is not UTF-8")?;
                Value::Text(Arc::new(value))
            }
            82 => Value::Integer(parse_u64(&self.text(a[0])?)?),
            83 => {
                let request = self.resource_request(&a[..4], true)?;
                let value = self.text(a[3])?;
                self.host_io
                    .as_mut()
                    .ok_or("native resource is not activated")?
                    .send(&request, &value)?;
                Value::Unit
            }
            _ => return Err("unknown native collection operation".into()),
        })
    }

    fn resource_request(
        &self,
        arguments: &[usize],
        send: bool,
    ) -> Result<mech_engine::native_sequence::NativeResourceRequirement, String> {
        use mech_engine::native_sequence::{NativeResourceIntent, NativeResourceRequirement};
        if arguments.len() != if send { 4 } else { 3 } {
            return Err("invalid native resource operation arity".into());
        }
        let field = |id| match self.constants.get(id) {
            Some(Constant::Text(text)) => Ok(text.clone()),
            _ => Err("native resource identity must be a source constant".to_string()),
        };
        let request = NativeResourceRequirement {
            base_uri: field(arguments[0])?,
            context_name: field(arguments[1])?,
            path: field(arguments[2])?,
            intent: if send {
                NativeResourceIntent::Send
            } else {
                NativeResourceIntent::Read
            },
        };
        request.validate()?;
        Ok(request)
    }
}
fn encoding_table(alphabet: &[u8], mode: &[u8]) -> Result<[u16; 256], String> {
    if alphabet.is_empty() || alphabet.len() > 256 {
        return Err("invalid byte alphabet".into());
    }
    let ignore = match mode {
        b"ignore-ascii-case" => true,
        b"exact" => false,
        _ => return Err("unknown byte encoding mode".into()),
    };
    let mut table = [u16::MAX; 256];
    for (index, &byte) in alphabet.iter().enumerate() {
        let keys = if ignore {
            [byte.to_ascii_lowercase(), byte.to_ascii_uppercase()]
        } else {
            [byte, byte]
        };
        for key in keys {
            if table[key as usize] != u16::MAX && table[key as usize] != index as u16 {
                return Err("duplicate alphabet symbol".into());
            }
            table[key as usize] = index as u16;
        }
    }
    Ok(table)
}

#[cfg(test)]
mod matrix_tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    static LATER_CHUNK_VISITED: AtomicBool = AtomicBool::new(false);

    unsafe extern "C" fn competing_errors(
        _: *const f64,
        _: usize,
        start: usize,
        end: usize,
        _: *mut f64,
    ) -> u32 {
        if start != 0 {
            LATER_CHUNK_VISITED.store(true, Ordering::Release);
            return 2;
        }
        if end < 4 {
            // Give a second worker a chance to encounter its competing error.
            // The expected result must not depend on whether it is scheduled
            // before the bounded wait ends. Never panic across the native ABI.
            let deadline = std::time::Instant::now() + std::time::Duration::from_millis(100);
            while !LATER_CHUNK_VISITED.load(Ordering::Acquire)
                && std::time::Instant::now() < deadline
            {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }
        1
    }

    unsafe extern "C" fn row_numbers(
        _: *const f64,
        _: usize,
        start: usize,
        end: usize,
        output: *mut f64,
    ) -> u32 {
        for row in start..end {
            // SAFETY: Context supplies an exclusive output slice of end-start
            // elements for this invocation.
            unsafe { output.add(row - start).write(row as f64) };
        }
        0
    }

    fn context(parallel: bool) -> Context {
        let mut context = Context::new(&[], None, false);
        context
            .values
            .push(Value::Numbers(Arc::new(vec![1.0]), false));
        context.values.push(Value::Integer(4));
        if parallel {
            context.partitions = 2;
            context.array_grain = 1;
            context.pool = Some(Arc::new(
                rayon::ThreadPoolBuilder::new()
                    .num_threads(2)
                    .build()
                    .unwrap(),
            ));
        }
        context
    }

    #[test]
    fn parallel_matrix_reports_the_first_row_error_with_competing_failures() {
        for op in [35, 36] {
            let expected = context(false)
                .op(op, [0, 1, 0, 0, 0], competing_errors as *const () as usize)
                .err()
                .unwrap();
            assert_eq!(expected, "unsigned arithmetic overflow");
            LATER_CHUNK_VISITED.store(false, Ordering::Release);
            let actual = context(true)
                .op(op, [0, 1, 0, 0, 0], competing_errors as *const () as usize)
                .err()
                .unwrap();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn parallel_matrix_keeps_all_successful_output_chunks() {
        for op in [35, 36] {
            let Value::Numbers(result, false) = context(true)
                .op(op, [0, 1, 0, 0, 0], row_numbers as *const () as usize)
                .unwrap()
            else {
                panic!("expected a matrix result vector")
            };
            assert_eq!(result.as_slice(), &[0.0, 1.0, 2.0, 3.0]);
        }
    }
}

#[cfg(test)]
mod regex_tests {
    use super::*;
    fn context(input: &[u8], pattern: &str, replacement: &str) -> Context {
        let mut c = Context::new(&[], None, false);
        for bytes in [input, pattern.as_bytes(), replacement.as_bytes()] {
            c.values.push(Value::Text(Arc::new(bytes.to_vec())));
        }
        c
    }
    #[test]
    fn nonoverlapping_bytes_empty_matches_and_literal_replacement() {
        for (input, pattern, want) in [
            (b"aaaa".as_slice(), "aa", 2),
            (b"abc", "", 4),
            (b"\xff\xffx", ".", 3),
            (b"hello HELLO", "(?i)hello", 2),
        ] {
            let mut c = context(input, pattern, "$1\\tail");
            assert!(matches!(c.op(53,[0,1,0,0,0],0).unwrap(),Value::Integer(n) if n == want));
        }
        let mut c = context(b"ab--ab", "(ab)", "$1");
        let Value::Text(result) = c.op(54, [0, 1, 2, 0, 0], 0).unwrap() else {
            panic!()
        };
        assert_eq!(result.as_slice(), b"$1--$1");
        let mut c = context(b"abc", "", "-");
        let Value::Text(result) = c.op(54, [0, 1, 2, 0, 0], 0).unwrap() else {
            panic!()
        };
        assert_eq!(result.as_slice(), b"-a-b-c-");
    }
    #[test]
    fn invalid_patterns_are_reported_and_compilations_are_cached() {
        let mut c = context(b"x", "[", "");
        assert!(
            c.op(53, [0, 1, 0, 0, 0], 0)
                .err()
                .unwrap()
                .contains("invalid byte regex")
        );
        let mut c = context(b"xyxy", "xy", "z");
        c.op(53, [0, 1, 0, 0, 0], 0).unwrap();
        c.op(54, [0, 1, 2, 0, 0], 0).unwrap();
        assert_eq!(c.regexes.len(), 1);
    }
    #[test]
    fn deferred_scalar_consumers_observe_the_same_value_once() {
        let input = b"hello!".repeat(4000);
        let mut c = context(&input, "hello", "x")
            .with_histogram_policy(2, false, false)
            .unwrap()
            .with_independent_tasks(true);
        let value = c.op(53, [0, 1, 0, 0, 0], 0).unwrap();
        assert!(matches!(value, Value::DeferredInteger(_)));
        c.values.push(value);
        c.values.push(Value::Integer(0));
        assert_eq!(c.integer(3).unwrap(), 4000);
        let Value::Text(result) = c.op(24, [3, 4, 0, 0, 0], 0).unwrap() else {
            panic!()
        };
        assert_eq!(result.as_slice(), b"4000");
        assert_eq!(c.integer(3).unwrap(), 4000);
    }
    #[test]
    fn dead_byte_buffers_reuse_storage_without_reclaiming_live_aliases() {
        for (input, pattern, replacement) in [
            (b"ab--ab".as_slice(), "(ab)", "$1"),
            (b"a\xff", "", "-"),
            (b"aaaa", "aa", "expanded"),
            (b"abcd", "xyz", ""),
        ] {
            let mut ordinary = context(input, pattern, replacement);
            let Value::Text(expected) = ordinary.op(54, [0, 1, 2, 0, 0], 0).unwrap() else {
                panic!()
            };
            let mut c = context(input, pattern, replacement).with_byte_buffer_reuse(true);
            let buffer = Vec::with_capacity(4096);
            let pointer = buffer.as_ptr();
            c.recycle(Value::Text(Arc::new(buffer)));
            let Value::Text(actual) = c.op(54, [0, 1, 2, 0, 0], 0).unwrap() else {
                panic!()
            };
            assert_eq!(actual.as_slice(), expected.as_slice());
            assert_eq!(actual.as_ptr(), pointer);
            let alias = actual.clone();
            c.recycle(Value::Text(actual));
            assert!(c.byte_scratch.is_empty());
            assert_eq!(alias.as_slice(), expected.as_slice());
            c.recycle(Value::Text(alias));
            assert_eq!(c.byte_scratch.len(), 1);
        }
    }
}

fn record(
    mut reader: impl BufRead,
    begin: &[u8],
    end: &[u8],
    encoding: Option<&[u16; 256]>,
) -> Result<Vec<u8>, String> {
    if begin.is_empty() || end.is_empty() {
        return Err("record prefixes cannot be empty".into());
    }
    let mut found = false;
    let mut ended = false;
    let mut bytes = Vec::new();
    let mut line = Vec::new();
    loop {
        line.clear();
        if reader
            .read_until(b'\n', &mut line)
            .map_err(|e| e.to_string())?
            == 0
        {
            break;
        }
        let line = line.strip_suffix(b"\n").unwrap_or(&line);
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        if ended {
            continue;
        }
        if !found {
            if line.starts_with(begin) {
                found = true;
            }
            continue;
        }
        if line.starts_with(end) {
            ended = true;
            continue;
        }
        if let Some(table) = encoding {
            for &byte in line {
                let code = table[byte as usize];
                if code == u16::MAX {
                    return Err(format!("byte {byte} is not in the alphabet"));
                }
                bytes.push(code as u8);
            }
        } else {
            bytes.extend_from_slice(line);
        }
    }
    if !found {
        return Err("record start was not found".into());
    }
    Ok(bytes)
}

fn decode(mut key: u64, bits: u8, width: u8, alphabet: &[u8]) -> Result<Vec<u8>, String> {
    let mut bytes = vec![0; width as usize];
    let mask = (1u64 << bits) - 1;
    for byte in bytes.iter_mut().rev() {
        *byte = *alphabet
            .get((key & mask) as usize)
            .ok_or("histogram code exceeds alphabet")?;
        key >>= bits;
    }
    Ok(bytes)
}

#[unsafe(no_mangle)]
unsafe extern "C" fn mech_sequence_dispatch(
    context: *mut Context,
    op: u64,
    a0: usize,
    a1: usize,
    a2: usize,
    a3: usize,
    a4: usize,
    kernel: usize,
) -> usize {
    // SAFETY: The generated entry passes its exclusively borrowed live Context.
    let context = unsafe { &mut *context };
    if context.error.is_some() {
        return 0;
    }
    if op == 9 {
        if let Some(value) = context.values.get_mut(a0) {
            if let Value::Histogram(p) = value {
                if let Err(e) = p.resolve() {
                    context.error = Some(e);
                    return 0;
                }
            }
            if let Value::DeferredInteger(p) = value {
                if let Err(e) = p.resolve() {
                    context.error = Some(e);
                    return 0;
                }
            }
            let value = std::mem::replace(value, Value::Unit);
            context.recycle(value);
        } else {
            context.error = Some("invalid release handle".into());
        }
        return 0;
    }
    match context.op(op, [a0, a1, a2, a3, a4], kernel) {
        Ok(value) => {
            let id = context.values.len();
            context.values.push(value);
            id
        }
        Err(error) => {
            context.error = Some(error);
            0
        }
    }
}
#[unsafe(no_mangle)]
unsafe extern "C" fn mech_sequence_increment(map: *mut Counts, keys: *const u64, len: usize) {
    // SAFETY: The compiled kernel passes its exclusive live map and a stack
    // buffer containing exactly len initialized keys, with len <= batch <=8192.
    let map = unsafe { &mut *map };
    let keys = unsafe { std::slice::from_raw_parts(keys, len) };
    map.increment(keys);
}

// Generic closure invocation storage. The iteration loop and source function
// body are compiled native code; this helper only owns a fresh handle scope.
struct IterationState {
    current: Value,
    function: Arc<Callable>,
    captures: Vec<Value>,
    pool: Option<Arc<rayon::ThreadPool>>,
    scratch: Arc<Mutex<Vec<Vec<f64>>>>,
    partitions: usize,
    array_grain: usize,
    error: Option<String>,
}
#[unsafe(no_mangle)]
unsafe extern "C" fn mech_sequence_invoke(state: *mut IterationState, entry: usize) -> usize {
    // SAFETY: generated iteration owns a live, exclusive state and passes the
    // same-object native entry compiled for this exact callable constant table.
    let state = unsafe { &mut *state };
    let function = &state.function;
    let mut scope = Context::pure(&function.program.constants);
    scope.pool = state.pool.clone();
    scope.scratch = state.scratch.clone();
    scope.partitions = state.partitions;
    scope.array_grain = state.array_grain;
    for ((_, slot), value) in function.captures.iter().zip(&state.captures) {
        scope.values[*slot] = value.clone();
    }
    scope.values[function.parameters[0]] = state.current.clone();
    let entry = unsafe { std::mem::transmute::<usize, NativeEntry>(entry) };
    if let Err(error) = unsafe { scope.run(entry) } {
        state.error = Some(error);
        return 1;
    }
    let result = std::mem::replace(&mut scope.values[function.output], Value::Unit);
    let previous = std::mem::replace(&mut state.current, result);
    scope.recycle(previous);
    0
}

struct RecordState<'a> {
    records: &'a mut Records,
    function: &'a Callable,
    captures: &'a [Value],
    entry: usize,
    pool: Option<Arc<rayon::ThreadPool>>,
    partitions: usize,
    writer: &'a mut dyn Write,
    error: Option<String>,
    profile: bool,
}
#[unsafe(no_mangle)]
unsafe extern "C" fn mech_sequence_record_step(state: *mut RecordState<'_>) -> usize {
    // SAFETY: emitted traversal exclusively owns this state for the call, and
    // its mapping handle binds the native entry to precisely this callable IR.
    let state = unsafe { &mut *state };
    let result = (|| -> Result<bool, String> {
        let start = std::time::Instant::now();
        let Some(record) = state.records.next()? else {
            return Ok(false);
        };
        let read = start.elapsed();
        let bytes = record.body.len();
        let start = std::time::Instant::now();
        let mut scope = Context::pure(&state.function.program.constants);
        scope.pool = state.pool.clone();
        scope.partitions = state.partitions;
        for ((_, slot), value) in state.function.captures.iter().zip(state.captures) {
            scope.values[*slot] = value.clone();
        }
        scope.values[state.function.parameters[0]] = Value::Text(record.header);
        scope.values[state.function.parameters[1]] = Value::Bytes(record.body, 256);
        let entry = unsafe { std::mem::transmute::<usize, NativeEntry>(state.entry) };
        unsafe { scope.run(entry) }?;
        let compute = start.elapsed();
        let start = std::time::Instant::now();
        scope.values[state.function.output].write_text(state.writer)?;
        if state.profile {
            eprintln!(
                "byte-record-profile bytes={bytes} read={:.6} compute={:.6} write={:.6}",
                read.as_secs_f64(),
                compute.as_secs_f64(),
                start.elapsed().as_secs_f64()
            );
        }
        let mut spare = Vec::new();
        for value in std::mem::take(&mut scope.values) {
            value.reclaim_byte_buffer(&mut spare);
        }
        state.records.recycle_buffer(spare);
        Ok(true)
    })();
    match result {
        Ok(true) => 0,
        Ok(false) => 1,
        Err(error) => {
            state.error = Some(error);
            1
        }
    }
}

pub(super) fn register_symbols(builder: &mut cranelift_jit::JITBuilder) {
    for (name, address) in [
        (
            "mech_sequence_dispatch",
            mech_sequence_dispatch as *const u8,
        ),
        (
            "mech_sequence_increment",
            mech_sequence_increment as *const u8,
        ),
        ("mech_sequence_invoke", mech_sequence_invoke as *const u8),
        (
            "mech_sequence_record_step",
            mech_sequence_record_step as *const u8,
        ),
    ] {
        builder.symbol(name, address);
    }
}

#[cfg(test)]
mod record_stream_tests {
    use super::{Record, RecordReader};
    use std::io::{self, BufRead, BufReader, Cursor, Read};

    fn reference(
        input: &[u8],
        marker: &[u8],
        separator: &[u8],
    ) -> Result<Vec<(Vec<u8>, Vec<u8>)>, String> {
        let mut records: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
        let mut lines = 0;
        for physical in input.split_inclusive(|&byte| byte == b'\n') {
            let line = physical.strip_suffix(b"\n").unwrap_or(physical);
            let line = line.strip_suffix(b"\r").unwrap_or(line);
            if line.starts_with(marker) {
                records.push((line.to_vec(), Vec::new()));
                lines = 0;
            } else {
                let Some((_, body)) = records.last_mut() else {
                    return Err("body before first record header".into());
                };
                if lines != 0 {
                    body.extend_from_slice(separator);
                }
                body.extend_from_slice(line);
                lines += 1;
            }
        }
        Ok(records)
    }

    fn collect(mut reader: RecordReader<impl BufRead>) -> Result<Vec<(Vec<u8>, Vec<u8>)>, String> {
        let mut records = Vec::new();
        while let Some(Record { header, body }) = reader.next()? {
            records.push((header.as_ref().clone(), body.as_ref().clone()));
            reader.recycle_buffer(std::sync::Arc::try_unwrap(body).unwrap());
        }
        assert!(reader.next()?.is_none(), "exhausted readers stay exhausted");
        Ok(records)
    }

    #[test]
    fn compact_record_reader_matches_line_contract_at_every_small_buffer_boundary() {
        let cases: &[&[u8]] = &[
            b"",
            b"@",
            b"@header\n",
            b"@header\r\n",
            b"@one\nAC\nGT\n@two\nN",
            b"@one\n\n\n@two\n",
            b"@one\nA@B\nC\r\n\r\nD\r",
            b"@one\n\r\r\n\nX\n",
            b"@one\r\n\r\n\r\r\n\n@two\r\n\r",
            b"@one\nA\rB\nC\r\r",
            b"@one\n\0\xff\x80 \t\n@two",
            b"body\n@later\n",
        ];
        for marker in [b"@".as_slice(), b"xy", b"\0", b" "] {
            for separator in [b"".as_slice(), b",", b"\r\n"] {
                for original in cases {
                    let mut input = Vec::new();
                    // Retain inline @ bytes; vary only actual header prefixes.
                    for line in original.split_inclusive(|&b| b == b'\n') {
                        if line.first() == Some(&b'@') {
                            input.extend_from_slice(marker);
                            input.extend_from_slice(&line[1..]);
                        } else {
                            input.extend_from_slice(line);
                        }
                    }
                    let want = reference(&input, marker, separator);
                    for capacity in 1..=19 {
                        let actual = collect(
                            RecordReader::new(
                                BufReader::with_capacity(capacity, Cursor::new(&input)),
                                marker,
                                separator,
                            )
                            .unwrap(),
                        );
                        assert_eq!(
                            actual, want,
                            "capacity={capacity}, marker={marker:?}, separator={separator:?}, input={input:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn completed_record_storage_is_reused_without_retaining_old_bytes() {
        let mut reader =
            RecordReader::new(Cursor::new(b"@one\nABCD\n@two\nX\n"), b"@", b"").unwrap();
        let first = reader.next().unwrap().unwrap();
        let buffer = std::sync::Arc::try_unwrap(first.body).unwrap();
        let pointer = buffer.as_ptr();
        reader.recycle_buffer(buffer);
        let second = reader.next().unwrap().unwrap();
        assert_eq!(second.body.as_slice(), b"X");
        assert_eq!(second.body.as_ptr(), pointer);
        assert!(reader.next().unwrap().is_none());
    }

    #[test]
    fn streamed_result_reclamation_preserves_live_aliases_and_recovers_dead_views() {
        use super::{TextView, Value};
        use std::sync::Arc;
        let original = Arc::new(vec![b'X'; 128]);
        let retained = original.clone();
        let mut spare = Vec::new();
        Value::Bytes(original, 256).reclaim_byte_buffer(&mut spare);
        assert_eq!(spare.capacity(), 0);
        assert_eq!(retained.as_slice(), &[b'X'; 128]);
        let view = Arc::new(TextView::Wrap(retained, 60, Arc::new(b"\n".to_vec()), 0));
        let live_view = view.clone();
        Value::TextView(view).reclaim_byte_buffer(&mut spare);
        assert_eq!(spare.capacity(), 0);
        let mut output = Vec::new();
        Value::TextView(live_view.clone())
            .write_text(&mut output)
            .unwrap();
        assert_eq!(output.len(), 131);
        Value::TextView(live_view).reclaim_byte_buffer(&mut spare);
        assert!(spare.capacity() >= 128);
        assert!(spare.is_empty());
        let reused_pointer = spare.as_ptr();
        let smaller = Value::Text(Arc::new(vec![b'Y'; 16]));
        smaller.reclaim_byte_buffer(&mut spare);
        assert_eq!(spare.as_ptr(), reused_pointer);
        let nested = Value::TextView(Arc::new(TextView::Concat(
            Value::Text(Arc::new(b"header\n".to_vec())),
            Value::TextView(Arc::new(TextView::Wrap(
                Arc::new(vec![b'Z'; 256]),
                7,
                Arc::new(b"\n".to_vec()),
                0,
            ))),
        )));
        nested.reclaim_byte_buffer(&mut spare);
        assert!(spare.capacity() >= 256);
        assert!(spare.is_empty());
    }

    struct Interrupt<R> {
        inner: R,
        calls: usize,
        error: io::ErrorKind,
    }
    impl<R: Read> Read for Interrupt<R> {
        fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
            self.inner.read(output)
        }
    }
    impl<R: BufRead> BufRead for Interrupt<R> {
        fn fill_buf(&mut self) -> io::Result<&[u8]> {
            self.calls += 1;
            if self.calls == 2 {
                return Err(io::Error::new(self.error, "injected input failure"));
            }
            self.inner.fill_buf()
        }
        fn consume(&mut self, amount: usize) {
            self.inner.consume(amount);
        }
    }

    #[test]
    fn compact_reader_retries_interruption_but_propagates_real_io_errors() {
        let input = b"@\nAC\r\nGT\n@two\nN\r";
        let reader = Interrupt {
            inner: Cursor::new(input),
            calls: 0,
            error: io::ErrorKind::Interrupted,
        };
        assert_eq!(
            collect(RecordReader::new(reader, b"@", b"").unwrap()),
            reference(input, b"@", b"")
        );
        let reader = Interrupt {
            inner: Cursor::new(input),
            calls: 0,
            error: io::ErrorKind::Other,
        };
        let error = collect(RecordReader::new(reader, b"@", b"").unwrap()).unwrap_err();
        assert_eq!(error, "injected input failure");
        assert!(RecordReader::new(Cursor::new(input), b"", b"").is_err());
    }
}

#[path = "resident.rs"]
mod resident;

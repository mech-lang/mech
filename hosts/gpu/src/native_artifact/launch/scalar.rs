//! One scalar-family shell, with statically selected linked artifact adapters.
use crate::native_artifact::launch::arguments::{append_scalar, options, scalar_argument};
use crate::native_artifact::{
    LinkedArtifact, LinkedIterationArtifact, LinkedMatchArtifact,
    NATIVE_ITERATION_LINK_IDENTITY_BYTES, NATIVE_LINK_IDENTITY_BYTES,
    NATIVE_MATCH_LINK_IDENTITY_BYTES, NativeExecutionLimits,
};
use mech_core::{FunctionCatalog, ManagedMemoryBudget};
use mech_engine::native_artifact::{NativeScalarType, NativeScalarValue};
use sha2::{Digest, Sha256};
use std::ffi::{OsString, c_void};
use std::io::Write;

type Entry = unsafe extern "C" fn(*mut c_void) -> u64;
type Catalog = fn() -> Result<FunctionCatalog, String>;
struct Binding<const N: usize> {
    bytecode: &'static [u8],
    bytecode_sha256: &'static str,
    identity: &'static [u8; N],
    entry: Entry,
    catalog: Catalog,
}
fn decode<const N: usize>(binding: &Binding<N>) -> Result<mech_engine::ProgramArtifact, String> {
    let hex: String = Sha256::digest(binding.bytecode)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if hex != binding.bytecode_sha256 {
        return Err("embedded canonical bytecode identity mismatch".into());
    }
    mech_engine::decode_program_artifact_bytecode_v1(binding.bytecode)
        .map_err(|error| format!("canonical artifact decode: {error:?}"))
}
pub(in crate::native_artifact::launch) fn metadata_catalog(
    factory: Catalog,
) -> Result<FunctionCatalog, String> {
    std::thread::Builder::new()
        .name("canonical-native-catalog".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(factory)
        .map_err(|error| error.to_string())?
        .join()
        .map_err(|_| "canonical metadata catalog construction panicked")?
}
trait ScalarRunner {
    fn inputs(&self) -> &[NativeScalarType];
    fn output_count(&self) -> usize;
    fn evaluate(
        &self,
        inputs: &[NativeScalarValue],
        outputs: &mut [NativeScalarValue],
        limits: NativeExecutionLimits,
        budget: Option<&ManagedMemoryBudget>,
    ) -> Result<(), String>;
}
fn run(linked: &impl ScalarRunner, arguments: Vec<OsString>) -> Result<(), String> {
    let (limits, budget, arguments) = options(arguments)?;
    if arguments.len() != linked.inputs().len() {
        return Err(format!("expected {} typed inputs", linked.inputs().len()));
    }
    let mut inputs = Vec::with_capacity(arguments.len());
    for (argument, input) in arguments.iter().zip(linked.inputs()) {
        inputs.push(scalar_argument(argument, input.kind())?);
    }
    let mut outputs = vec![NativeScalarValue::U64(0); linked.output_count()];
    let budget = budget.map(ManagedMemoryBudget::new);
    linked.evaluate(&inputs, &mut outputs, limits, budget.as_ref())?;
    let mut result = String::new();
    for value in outputs {
        append_scalar(&mut result, value);
    }
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    stdout
        .write_all(result.as_bytes())
        .map_err(|error| error.to_string())?;
    stdout.flush().map_err(|error| error.to_string())
}
macro_rules! family {
    ($binding:ident, $run:ident, $linked:ident, $lower:ident, $bytes:ident) => {
        /// A process-lifetime same-Object scalar-family application binding.
        pub struct $binding(Binding<$bytes>);
        impl $binding {
            /// # Safety
            /// Entry and identity must be unmodified process-lifetime symbols
            /// from the SAME checked emitter Object for this exact bytecode.
            /// The build must verify Object, bytecode and generated-source pins.
            /// Entry must obey this crate's synchronous admitted private ABI;
            /// matching bytes do not authenticate arbitrary foreign code.
            pub unsafe fn from_build_owned_entry(
                bytecode: &'static [u8],
                bytecode_sha256: &'static str,
                identity: &'static [u8; $bytes],
                entry: Entry,
                catalog: Catalog,
            ) -> Self {
                Self(Binding {
                    bytecode,
                    bytecode_sha256,
                    identity,
                    entry,
                    catalog,
                })
            }
        }
        /// Launch the checked family using ordinary typed arguments and limits.
        pub fn $run(binding: $binding, arguments: Vec<OsString>) -> Result<(), String> {
            let binding = binding.0;
            let artifact = decode(&binding)?;
            let catalog = metadata_catalog(binding.catalog)?;
            let plan = mech_engine::native_artifact::$lower(&artifact, &catalog)
                .map_err(|error| error.to_string())?;
            // SAFETY: the binding constructor owns the same-Object provenance;
            // fresh lowering and the linked constructor retain identity checks.
            let linked =
                unsafe { $linked::from_build_owned_entry(&plan, binding.identity, binding.entry) }
                    .map_err(|error| error.to_string())?;
            run(&linked, arguments)
        }
        impl ScalarRunner for $linked {
            fn inputs(&self) -> &[NativeScalarType] {
                self.descriptor().inputs()
            }
            fn output_count(&self) -> usize {
                self.descriptor().outputs().len()
            }
            fn evaluate(
                &self,
                inputs: &[NativeScalarValue],
                outputs: &mut [NativeScalarValue],
                limits: NativeExecutionLimits,
                budget: Option<&ManagedMemoryBudget>,
            ) -> Result<(), String> {
                self.evaluate_into(inputs, outputs, limits, budget)
                    .map(|_| ())
                    .map_err(|error| error.to_string())
            }
        }
    };
}
family!(
    BuildOwnedScalarBinding,
    run_scalar_application,
    LinkedArtifact,
    lower_pure_native_artifact,
    NATIVE_LINK_IDENTITY_BYTES
);
family!(
    BuildOwnedMatchBinding,
    run_match_application,
    LinkedMatchArtifact,
    lower_scalar_match_artifact,
    NATIVE_MATCH_LINK_IDENTITY_BYTES
);
family!(
    BuildOwnedIterationBinding,
    run_iteration_application,
    LinkedIterationArtifact,
    lower_scalar_iteration_artifact,
    NATIVE_ITERATION_LINK_IDENTITY_BYTES
);

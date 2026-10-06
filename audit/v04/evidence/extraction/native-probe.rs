use mech_build::*;
use std::{fs, path::PathBuf, sync::Arc};

fn main() {
    exercise_external_kernel();
    let root = PathBuf::from(std::env::args().nth(1).expect("isolated core root"));
    let output = PathBuf::from(std::env::args().nth(2).expect("generated project root"));
    let request = NativeBuildRequest {
        bytecode: fs::read(root.join("tests/architecture/bytecode-v1/scalar-add-f64.mecb")).unwrap(),
        instruction_type_bindings: None,
        instruction_type_binding_requirements: None,
        runtime_config: None,
        target: None,
        profile: NativeBuildProfile::Debug,
        binary_name: "extracted-scalar-add".into(),
        output: output.clone(),
        emit: NativeEmit::CargoProject,
        keep_project: true,
        offline: true,
    };
    let workspace = NativeApplicationBuilder::new(NativeBuildEnvironment {
        function_catalog: mech_stdlib::runtime_catalog(),
        host_catalog: Arc::new(NativeHostCatalog::new()),
        dependency_source: NativeDependencySource::Workspace { root },
    });
    println!("unmodified_workspace_result={:?}", workspace.plan(&request).map(|_| ()));
    let registry = NativeApplicationBuilder::new(NativeBuildEnvironment {
        function_catalog: mech_stdlib::runtime_catalog(),
        host_catalog: Arc::new(NativeHostCatalog::new()),
        dependency_source: NativeDependencySource::Registry { version: MECH_COMPONENT_VERSION.into() },
    });
    let plan = registry.plan(&request).expect("public registry plan");
    let project = render_generated_native_project(output, &request, &plan, None).unwrap();
    project.materialize().unwrap();
    let manifest = fs::read_to_string(project.manifest_path()).unwrap();
    assert!(!manifest.contains("[patch.crates-io]"));
    println!("registry_project={}", project.root.display());
}

fn exercise_external_kernel() {
    use mech_core::*;
    let domain = MemoryDomain::new().unwrap();
    let left = ValueCell::from_exact_in(&domain, 1.0_f64).unwrap();
    let right = ValueCell::from_exact_in(&domain, 2.0_f64).unwrap();
    let output = ValueCell::from_exact_in(&domain, 0.0_f64).unwrap();
    let catalog = mech_stdlib::runtime_catalog();
    let operation = OperationId::from_name("math/add");
    let invocation = FunctionInvocation::binary(output.clone(), left.clone(), right);
    let mut candidates = catalog.runtime_entries_for_binding(
        RuntimeBindingSelector::Operation(operation), ExecutionTarget::DirectRuntime,
    ).filter_map(|entry| {
        let parts = entry.bind_resolved_invocation(operation, ExecutionTarget::DirectRuntime, invocation.clone()).ok()?;
        Some((parts, entry))
    }).collect::<Vec<_>>();
    assert_eq!(candidates.len(), 1);
    let (parts, entry) = candidates.pop().unwrap();
    let function = SpecializedFunction::syntax_directed(
        parts,
        ResolvedOperationDescriptor::from_name("math/add", entry.operation_contract(operation).unwrap().clone()).unwrap(),
        entry.id, ExecutionTarget::DirectRuntime, entry.implementation_memory_class(),
    ).unwrap();
    function.instance().solve_result().unwrap();
    let snapshot = output.snapshot().unwrap();
    let ValueData::F64(value) = snapshot.data() else { panic!("F64 output") };
    assert_eq!(value.to_f64(), 3.0);
    let next = ValueCell::from_exact_in(&domain, 5.0_f64).unwrap().snapshot().unwrap();
    left.replace(&next).unwrap();
    function.instance().solve_result().unwrap();
    let snapshot = output.snapshot().unwrap();
    let ValueData::F64(value) = snapshot.data() else { panic!("F64 output") };
    assert_eq!(value.to_f64(), 7.0);
    println!("external_kernel={} first=3 changed_input_result=7", entry.name);
}

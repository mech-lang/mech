use std::fmt::Display;
use std::fs;

use mech_core::{MResult, MechSourceCode};

use mech_runtime::{
    FileSourceResolver, ModuleBuildOptions, ResolvedSource, SourceKind, TaskRecord,
};

mod support;

fn short_text(text: &str) -> String {
    if text.len() <= 18 {
        return text.to_string();
    }

    format!("{}…{}", &text[..8], &text[text.len() - 8..])
}

fn short(id: impl Display) -> String {
    short_text(&id.to_string())
}

fn runtime_target() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

fn main() -> MResult<()> {
    let root = std::env::temp_dir().join("mech-runtime-dependency-source-demo");

    let _ = fs::remove_dir_all(&root);
    support::io(fs::create_dir_all(&root))?;

    let dep_path = root.join("dep.mec");

    support::io(fs::write(
        &dep_path,
        r#"
      y := 42
      y
    "#,
    ))?;

    println!("root: {}", root.display());
    println!("dep:  {}", dep_path.display());

    let resolver = FileSourceResolver::new(&root);

    let mut runtime = support::source_runtime_builder()
        .source_resolver(resolver)
        .build()?;

    println!("runtime: {}", short(runtime.id()));

    let task = TaskRecord::new(runtime.next_task_id(), "program:runtime-dependency-source");
    let mut context = runtime.context_for_task(&task)?;

    let resolved = ResolvedSource::new(
        "index",
        "memory://runtime-dependency-source-demo/index.mec",
        MechSourceCode::String(
            r#"
        +> dep.mec
        x := dep/y
        x
      "#
            .to_string(),
        ),
    )
    .with_kind(SourceKind::Mech);

    let target = runtime_target();
    let options =
        ModuleBuildOptions::new(env!("CARGO_PKG_VERSION"), "mech-current", &target, &[], &[]);

    let module_version =
        runtime.build_module_from_resolved_source_with_context(&mut context, resolved, options)?;

    println!("module version: {}", short(module_version));

    let module_record = runtime
        .get_module_version(module_version)?
        .expect("expected parent module version to exist");

    println!("dependency count: {}", module_record.dependencies.len(),);

    assert_eq!(
        module_record.dependencies.len(),
        1,
        "expected parent module version to record one dependency",
    );

    let dependency_version = module_record.dependencies[0];

    let dependency_record = runtime
        .get_module_version(dependency_version)?
        .expect("expected dependency module version to exist");

    println!("dependency version: {}", short(dependency_version));
    println!("dependency module:  {}", short(dependency_record.module));

    runtime.shutdown()?;

    println!();
    println!("events:");

    for event in runtime.list_events(None)? {
        println!(
            "  #{:03} {:24} {:?}",
            event.sequence,
            event.name(),
            event.kind,
        );
    }

    Ok(())
}

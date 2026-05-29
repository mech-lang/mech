use mech_runtime::{FileSourceResolver, ModuleBuildOptions, RuntimeBuilder, SourceScope};

fn write_case(root: &std::path::Path, name: &str, source: &str) -> std::path::PathBuf {
  let case_root = root.join(name);
  std::fs::create_dir_all(&case_root).unwrap();
  std::fs::write(case_root.join("main.mec"), source).unwrap();
  case_root
}

fn run_case(root: &std::path::Path, name: &str, source: &str) {
  let case_root = write_case(root, name, source);
  println!("case: {name}");
  println!("root path: {}", case_root.display());

  let mut runtime = RuntimeBuilder::new().source_resolver(FileSourceResolver::new(&case_root)).build().unwrap();
  let options = ModuleBuildOptions::new("diagnostics", "v0.3", "native", &[], &[]);

  match runtime.resolve_and_store_module_source("main.mec", options) {
    Ok(Some(version)) => {
      println!("main module version: {version}");
      let record = runtime.store().get_module_version(version).unwrap().unwrap();
      println!("scopes:");
      for scope in &record.scopes {
        println!("  - {:?}", scope.scope);
      }
      println!("scoped context declarations:");
      for scope in &record.scopes {
        for context in &scope.contexts {
          println!("  - {:?}: {}", scope.scope, context.name);
        }
      }
      println!("scoped address references:");
      for scope in &record.scopes {
        for reference in &scope.address_references {
          println!("  - {:?}: {}@{}", scope.scope, reference.name, reference.target);
        }
      }
      println!("run result: {:?}", runtime.run_module(version));
      for scope_metadata in &record.scopes {
        if matches!(scope_metadata.scope, SourceScope::Interpreter(_)) {
          println!("run {:?} result: {:?}", scope_metadata.scope, runtime.run_module_scope(version, scope_metadata.scope.clone()));
        }
      }
    }
    Ok(None) => {
      println!("main module version: <none>");
      println!("scopes: []");
      println!("scoped context declarations: []");
      println!("scoped address references: []");
      println!("run result: no module resolved");
    }
    Err(error) => {
      println!("main module version: <resolution failed>");
      println!("scopes: <unavailable>");
      println!("scoped context declarations: <unavailable>");
      println!("scoped address references: <unavailable>");
      println!("run result: resolution error: {:?}", error);
    }
  }
  println!();
}

fn main() {
  let root = std::env::temp_dir().join(format!("mech-address-target-diagnostics-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
  std::fs::create_dir_all(&root).unwrap();
  println!("root path: {}", root.display());

  run_case(
    &root,
    "ok@foo works",
    "~~~mech:foo\nok := true\n<+ ok\n~~~\n\nresult := ok@foo\n",
  );
  run_case(
    &root,
    "interpreter/context conflict fails resolution",
    "~~~mech:foo\nok := true\n<+ ok\n~~~\n\n@foo := docs://foo{:read(ok)}\n",
  );
  run_case(
    &root,
    "context target returns ContextAddressReadUnsupported",
    "@manual := docs://manual{:read(intro/title)}\n\nresult := intro/title@manual\n",
  );
  run_case(
    &root,
    "interpreter-scoped context resolves only in interpreter scope",
    "~~~mech:foo\n@manual := docs://manual{:read(intro/title)}\nresult := intro/title@manual\n~~~\n",
  );
  run_case(
    &root,
    "unknown target returns UnknownAddressTarget",
    "result := ok@missing\n",
  );
  run_case(
    &root,
    "string/comment @bar does not execute bar",
    "~~~mech:bar\nbroken := missing\n<+ broken\n~~~\n\ntext := \"@bar\"\n-- @bar\n\nok := true\n",
  );
}

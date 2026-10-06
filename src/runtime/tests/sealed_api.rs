#[test]
fn sealed_runtime_api_rejects_safe_escape_hatches() {
    let tests = trybuild::TestCases::new();
    let mut fixtures = std::fs::read_dir("tests/ui/sealed")
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|fixture| {
            fixture
                .extension()
                .is_some_and(|extension| extension == "rs")
        })
        .collect::<Vec<_>>();
    fixtures.sort();
    assert_eq!(
        fixtures.len(),
        19,
        "the sealed fixture inventory must stay explicit"
    );
    for fixture in fixtures {
        let name = fixture.file_name().unwrap().to_str().unwrap();
        let resident_diagnostics = match name {
            "recording_api_private.rs" => cfg!(any(
                feature = "runtime_bench_probes",
                feature = "resident_ekf_benchmarks",
                feature = "resident-external"
            )),
            "runtime_component_access.rs" => cfg!(feature = "resident-routing"),
            _ => false,
        };
        let fixture = if resident_diagnostics {
            fixture.parent().unwrap().join("resident").join(name)
        } else {
            fixture
        };
        tests.compile_fail(fixture);
    }
}

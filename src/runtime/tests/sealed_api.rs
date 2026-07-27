#[test]
fn sealed_runtime_api_rejects_safe_escape_hatches() {
  let tests = trybuild::TestCases::new();
  tests.compile_fail("tests/ui/sealed/*.rs");
}

#[cfg(feature = "resident-external")]
#[test]
fn safe_consumers_cannot_obtain_external_publication_authority() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/resident_external_sealed/*.rs");
}

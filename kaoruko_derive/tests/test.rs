#[test]
fn tests() {
    let tests = trybuild::TestCases::new();
    tests.pass("tests/parse.rs");
    tests.pass("tests/config.rs");
    tests.compile_fail("tests/missing_description.rs");
    tests.compile_fail("tests/missing_variants.rs");
}

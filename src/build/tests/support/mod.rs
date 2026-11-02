use mech_build::NativeBuildPlan;

mod isolated;
pub use isolated::*;
mod generated_cases;
pub use generated_cases::*;

pub fn assert_exact_mech_packages(plan: &NativeBuildPlan, expected: &[&str]) {
    let actual = plan
        .packages
        .iter()
        .map(|package| package.package.as_str())
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);
    for forbidden in ["mech-stdlib", "mech-syntax", "mech-bytecode", "mech-build"] {
        assert!(!actual.contains(&forbidden));
    }
}

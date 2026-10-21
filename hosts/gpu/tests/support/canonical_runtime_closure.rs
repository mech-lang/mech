//! Shared package/feature policy for every generated canonical runtime profile.
//! Inspect actual compiler-artifact units, never optional metadata edges alone.
use serde_json::Value;
use std::collections::BTreeSet;

const UNUSED: &[&str] = &["mech-stdlib", "mech-math", "mech-compare", "mech-logic"];

pub(super) fn package(name: &str) {
    assert!(
        !UNUSED.contains(&name),
        "runtime retains unused bytecode library {name}"
    );
    assert_ne!(name, "mech-syntax", "runtime contains parser");
    assert!(
        !name.starts_with("cranelift-"),
        "runtime contains code generator {name}"
    );
}

pub(super) fn features(name: &str, features: &[Value]) {
    let forbidden: &[&str] = match name {
        "mech-gpu" => &[
            "canonical-native",
            "jit",
            "aot",
            "sequence-cli",
            "native-build",
            "fused-cli",
        ],
        "mech-engine" => &[
            "native-plan",
            "source",
            "semantic-compiler",
            "compiler",
            "full_compiler",
            "standard_compiler",
            "native-sequence",
            "math_sqrt",
            "math_neg",
            "math_add",
            "math_sub",
            "math_mul",
            "math_div",
            "math_mod",
            "compare_default",
            "logic_default",
        ],
        _ => &[],
    };
    for feature in features {
        assert!(
            !forbidden.contains(&feature.as_str().unwrap()),
            "runtime enabled {name}/{feature}"
        );
    }
}

pub(super) fn required(names: &BTreeSet<String>) {
    for name in ["mech-core", "mech-engine", "mech-gpu"] {
        assert!(names.contains(name), "missing compiled {name}");
    }
}

#[test]
fn canonical_engine_runtime_and_host_features_are_allowed() {
    let names = [
        "mech-core",
        "mech-engine",
        "mech-gpu",
        "mech-runtime",
        "mech-terminal",
    ];
    for name in names {
        package(name);
    }
    required(&names.map(str::to_owned).into_iter().collect());
    features(
        "mech-engine",
        &["native-artifact".into(), "pure_text".into()],
    );
    features(
        "mech-gpu",
        &["canonical-native-program-host-runtime".into()],
    );
}

#[test]
fn unused_libraries_source_frontends_and_codegen_remain_forbidden() {
    for name in UNUSED
        .iter()
        .copied()
        .chain(["mech-syntax", "cranelift-codegen"])
    {
        assert!(
            std::panic::catch_unwind(|| package(name)).is_err(),
            "{name}"
        );
    }
    for (name, rejected) in [
        (
            "mech-engine",
            &[
                "native-plan",
                "source",
                "semantic-compiler",
                "compiler",
                "full_compiler",
                "standard_compiler",
                "native-sequence",
                "math_sqrt",
                "math_neg",
                "math_add",
                "math_sub",
                "math_mul",
                "math_div",
                "math_mod",
                "compare_default",
                "logic_default",
            ][..],
        ),
        (
            "mech-gpu",
            &[
                "canonical-native",
                "jit",
                "aot",
                "sequence-cli",
                "native-build",
                "fused-cli",
            ][..],
        ),
    ] {
        for feature in rejected {
            assert!(
                std::panic::catch_unwind(|| features(name, &[(*feature).into()])).is_err(),
                "{name}/{feature}"
            );
        }
    }
}

#[test]
fn every_canonical_engine_package_is_still_required() {
    for omitted in ["mech-core", "mech-engine", "mech-gpu"] {
        let names = ["mech-core", "mech-engine", "mech-gpu"]
            .into_iter()
            .filter(|name| *name != omitted)
            .map(str::to_owned)
            .collect();
        assert!(
            std::panic::catch_unwind(|| required(&names)).is_err(),
            "{omitted}"
        );
    }
}

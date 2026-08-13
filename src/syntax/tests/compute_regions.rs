use mech_core::ComputePlacement;
use mech_syntax::parser;

fn parse_region(target: &str) -> mech_core::Section {
    let source = format!("particle update @ {target}\n{}\nx := 1\n", "-".repeat(79),);
    let program = parser::parse(&source).expect("named compute region must parse");
    assert_eq!(program.body.sections.len(), 1);
    program.body.sections.into_iter().next().unwrap()
}

#[test]
fn named_compute_region_is_structured_section_metadata() {
    let section = parse_region("compute");
    assert_eq!(section.subtitle.unwrap().to_string(), "particle update");
    assert_eq!(section.compute, Some(ComputePlacement::Compute));
}

#[test]
fn named_regions_accept_hard_cpu_and_gpu_placement() {
    assert_eq!(parse_region("cpu").compute, Some(ComputePlacement::Cpu));
    assert_eq!(parse_region("gpu").compute, Some(ComputePlacement::Gpu));
}

#[test]
fn ordinary_mechdown_section_is_not_a_compute_region() {
    let program = parser::parse(
        "1. Documentation\n-------------------------------------------------------------------------------\n\nText only.\n",
    )
    .unwrap();
    assert_eq!(program.body.sections[0].compute, None);
}

#[test]
fn unknown_compute_placement_is_rejected() {
    assert!(
        parser::parse(
            "particle update @ fpga\n-------------------------------------------------------------------------------\nx := 1\n",
        )
        .is_err()
    );
}

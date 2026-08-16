use mech_core::SectionAnnotationArgument;
use mech_syntax::parser;

fn parse_region(target: &str) -> mech_core::Section {
    let source = format!("particle update @{target}\n{}\nx := 1\n", "-".repeat(79));
    let program = parser::parse(&source).expect("named compute region must parse");
    assert_eq!(program.body.sections.len(), 1);
    program.body.sections.into_iter().next().unwrap()
}

#[test]
fn named_compute_region_is_structured_section_metadata() {
    let section = parse_region("compute");
    assert_eq!(section.subtitle.unwrap().to_string(), "particle update");
    assert_eq!(section.annotations.len(), 1);
    assert_eq!(section.annotations[0].name.as_ref(), "compute");
    assert!(section.annotations[0].arguments.is_empty());
}

#[test]
fn named_regions_accept_hard_cpu_and_gpu_annotations() {
    assert_eq!(parse_region("cpu").annotations[0].name.as_ref(), "cpu");
    assert_eq!(parse_region("gpu").annotations[0].name.as_ref(), "gpu");
}

#[test]
fn ordinary_mechdown_section_has_no_annotations() {
    let program = parser::parse(
        "1. Documentation\n-------------------------------------------------------------------------------\n\nText only.\n",
    )
    .unwrap();
    assert!(program.body.sections[0].annotations.is_empty());
}

#[test]
fn general_annotations_preserve_order_and_atom_arguments() {
    let program = parser::parse(
        "particle update @gpu @required(:finite)\n-------------------------------------------------------------------------------\nx := 1\n",
    )
    .unwrap();
    let annotations = &program.body.sections[0].annotations;
    assert_eq!(annotations.len(), 2);
    assert_eq!(annotations[0].name.as_ref(), "gpu");
    assert_eq!(annotations[1].name.as_ref(), "required");
    assert!(matches!(
        &annotations[1].arguments[0],
        SectionAnnotationArgument::Atom(atom) if atom.name.to_string() == "finite"
    ));
}

#[test]
fn unknown_annotations_parse_for_semantic_validation() {
    assert!(
        parser::parse(
            "particle update @fpga\n-------------------------------------------------------------------------------\nx := 1\n",
        )
        .is_ok()
    );
}

#[test]
fn spaced_and_malformed_annotation_spellings_are_syntax_errors() {
    for heading in [
        "particle update @ compute",
        "particle update @ :gpu",
        "particle update@:gpu",
        "particle update@gpu",
    ] {
        let source = format!(
            "{heading}\n-------------------------------------------------------------------------------\nx := 1\n"
        );
        assert!(
            parser::parse(&source).is_err(),
            "unexpectedly parsed {heading}"
        );
    }
}

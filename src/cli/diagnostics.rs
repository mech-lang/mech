use ariadne::{Color, Label, Report, ReportKind, sources};
use mech_core::*;
use mech_syntax::ParserErrorReport;
#[cfg(feature = "invariant_define")]
use mech_program::{
    IntegrityConstraintFailureReason, IntegrityConstraintViolationSet,
};

use crate::WatchPathFailed;

fn source_range_to_offset_range(file_content: &str, range: &SourceRange) -> (usize, usize) {
    let mut offset = 0;
    let mut start_offset = 0;
    let mut end_offset = 0;

    for (line_index, line) in file_content.split_inclusive('\n').enumerate() {
        let row = line_index + 1;
        let line_len = line.len();
        if row == range.start.row {
            start_offset = offset + (range.start.col - 1);
        }
        if row == range.end.row {
            end_offset = offset + (range.end.col - 1);
            break;
        }
        offset += line_len;
    }
    end_offset = end_offset.min(file_content.len());
    while start_offset < end_offset && file_content.as_bytes()[start_offset].is_ascii_whitespace() {
        start_offset += 1;
    }
    while end_offset > start_offset && file_content.as_bytes()[end_offset - 1].is_ascii_whitespace()
    {
        end_offset -= 1;
    }
    if end_offset <= start_offset {
        end_offset = start_offset + 1;
        end_offset = end_offset.min(file_content.len());
    }
    (start_offset, end_offset)
}

pub(crate) fn print_mech_error(err: &MechError) {
    if let Some(rendered) = format_integrity_constraint_error(err) {
        println!("{rendered}");
        return;
    }
    if let Some(watch_error) = err.kind_as::<WatchPathFailed>() {
        let src_file_path = watch_error.file_path.to_string();
        match &err.source {
            Some(src_err) => {
                if let Some(report) = &src_err.kind_as::<ParserErrorReport>() {
                    let first_error_range = report
                        .1
                        .first()
                        .map(|e| e.cause_rng.clone())
                        .unwrap_or(SourceRange::default());
                    let (first_start, first_end) =
                        source_range_to_offset_range(&report.0, &first_error_range);
                    let mut error_report = Report::build(
                        ReportKind::Error,
                        (src_file_path.clone(), first_start..first_end),
                    )
                    .with_message(format!("Syntax Errors Found: {}", report.1.len()));

                    for (err_num, err_ctx) in report.1.iter().enumerate() {
                        let (start, end) =
                            source_range_to_offset_range(&report.0, &err_ctx.cause_rng);

                        if let Some(annotation_rng) = err_ctx.annotation_rngs.first() {
                            let (ann_start, ann_end) =
                                source_range_to_offset_range(&report.0, annotation_rng);

                            error_report = error_report.with_label(
                                Label::new((src_file_path.clone(), ann_start..ann_end))
                                    .with_message(format!(
                                        "#{}: {} [{}:{}]",
                                        err_num + 1,
                                        err_ctx.err_message,
                                        annotation_rng.start.row,
                                        annotation_rng.start.col
                                    ))
                                    .with_color(Color::Yellow),
                            );
                        } else {
                            error_report = error_report.with_label(
                                Label::new((src_file_path.clone(), start..end))
                                    .with_message(format!(
                                        "#{}: {} [{}:{}]",
                                        err_num + 1,
                                        err_ctx.err_message,
                                        err_ctx.cause_rng.start.row,
                                        err_ctx.cause_rng.start.col
                                    ))
                                    .with_color(Color::Yellow),
                            );
                        }
                    }
                    let cache = sources([(src_file_path.clone(), report.0.clone())]);
                    error_report.finish().print(cache).unwrap_or_else(|e| {
                        println!("Error printing report: {:?}", e);
                    });
                } else {
                    println!("Error:");
                    println!("{:#?}", err);
                }
            }
            None => {
                println!("Error:");
                println!("{:#?}", err);
            }
        }
    } else {
        println!("Error:");
        println!("{:#?}", err);
    }
}

#[cfg(feature = "invariant_define")]
pub(crate) fn format_integrity_constraint_error(
    error: &MechError,
) -> Option<String> {
    let failures = error.kind_as::<IntegrityConstraintViolationSet>()?;
    let mut rendered = failures
        .violations
        .iter()
        .map(|violation| {
            let reason = match violation.reason {
                IntegrityConstraintFailureReason::EvaluatedFalse => "evaluated to false",
                IntegrityConstraintFailureReason::ExpectedBool => "expected a scalar bool",
                IntegrityConstraintFailureReason::BorrowConflict => {
                    "could not read the settled constraint result"
                }
            };
            let mut lines = vec![
                format!("Integrity constraint `{}` failed", violation.name),
                format!("  {}", violation.expression),
                format!("  reason: {reason}"),
            ];
            if let Some(actual) = &violation.actual {
                lines.push(format!("  actual: {actual}"));
            }
            if let Some(expected) = &violation.expected {
                lines.push(format!("  expected: {expected}"));
            }
            lines.join("\n")
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    rendered.push_str(
        "\n\nReactive turn rolled back.\nNo external effects were committed.",
    );
    Some(rendered)
}

#[cfg(not(feature = "invariant_define"))]
pub(crate) fn format_integrity_constraint_error(
    _error: &MechError,
) -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "invariant_define")]
    use mech_program::IntegrityConstraintViolation;

    #[cfg(feature = "invariant_define")]
    #[test]
    fn integrity_constraint_formatter_is_structured_and_address_free() {
        let error = MechError::new(
            IntegrityConstraintViolationSet {
                checked: 1,
                violations: vec![IntegrityConstraintViolation {
                    interpreter_id: 7,
                    constraint_id: 11,
                    name: "safe-target!".to_string(),
                    expression: "target<=maximum-target".to_string(),
                    reason: IntegrityConstraintFailureReason::EvaluatedFalse,
                    evaluated_kind: Some(ValueKind::Bool),
                    actual: Some("150".to_string()),
                    operator: None,
                    expected: Some("120".to_string()),
                    tokens: Vec::new(),
                }],
            },
            None,
        );

        let rendered = format_integrity_constraint_error(&error).unwrap();

        assert!(rendered.contains("Integrity constraint `safe-target!` failed"));
        assert!(rendered.contains("reason: evaluated to false"));
        assert!(rendered.contains("actual: 150"));
        assert!(rendered.contains("expected: 120"));
        assert!(rendered.contains("Reactive turn rolled back."));
        assert!(rendered.contains("No external effects were committed."));
        assert!(!rendered.contains("@0x"));
        assert!(!rendered.contains("RefCell"));
    }
}

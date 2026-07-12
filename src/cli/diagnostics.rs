use ariadne::{Color, Label, Report, ReportKind, sources};
use mech_core::*;
use mech_syntax::ParserErrorReport;

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

pub fn print_mech_error(err: &MechError) -> std::io::Result<()> {
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
                    error_report.finish().print(cache)?;
                } else {
                    eprintln!("Error:\n{:#?}", err);
                }
            }
            None => {
                eprintln!("Error:\n{:#?}", err);
            }
        }
    } else {
        eprintln!("Error:\n{:#?}", err);
    }
  Ok(())
}

use clap::{Arg, ArgAction, Command};
use colored::*;
use mech_core::*;

use crate::MechError;
pub(crate) use crate::cli::format_args::FormatOptions;
use crate::cli::format_execute::{load_format_sources, write_format_outputs};
use crate::cli::format_targets::plan_format_targets;
use crate::cli::outcome::CliOutcome;
use crate::cli::resources::{
    LoadedStylesheets, ResourceEvent, ResourceFallback, Utf8ConversionError, load_resource,
    load_stylesheets,
};
use crate::source_discovery::{SkipReason, SourceDiscoveryEvent};

pub(crate) fn command() -> Command {
    Command::new("format")
        .about("Format Mech source code into standard format.")
        .arg(
            Arg::new("mech_format_file_paths")
                .help("Source .mec/.mdoc files, HTML files, or directories")
                .required(false)
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("output_path")
                .short('o')
                .long("out")
                .help("Destination folder.")
                .required(false),
        )
        .arg(
            Arg::new("stylesheet")
                .short('s')
                .long("stylesheet")
                .value_name("STYLESHEET")
                .num_args(1..)
                .action(ArgAction::Append)
                .help("Sets the stylesheet for the HTML output"),
        )
        .arg(
            Arg::new("shim")
                .short('m')
                .long("shim")
                .value_name("SHIM")
                .help("Sets the shim for the HTML output"),
        )
        .arg(
            Arg::new("html")
                .short('t')
                .long("html")
                .required(false)
                .help("Output as HTML")
                .action(ArgAction::SetTrue),
        )
}

fn render_discovery_events(badge: &str, events: &[SourceDiscoveryEvent]) {
    for event in events {
        match event {
            SourceDiscoveryEvent::SkippedBrokenSymlink { path } => {
                println!("{badge} Skipped broken symlink: {}", path.display())
            }
            SourceDiscoveryEvent::SkippedSymlinkedDirectory { path } => {
                println!("{badge} Skipped symlinked directory: {}", path.display())
            }
            SourceDiscoveryEvent::SkippedFileSymlink { path } => {
                println!("{badge} Skipped file symlink: {}", path.display())
            }
            SourceDiscoveryEvent::SkippedUnsupportedExtension { path } => {
                println!("{badge} Skipped unsupported source: {}", path.display())
            }
            SourceDiscoveryEvent::SkippedDirectory { path, reason } => match reason {
                SkipReason::SkippedByName => {
                    println!("{badge} Skipped directory: {}", path.display())
                }
                SkipReason::AlreadyVisited => println!(
                    "{badge} Skipped already visited directory: {}",
                    path.display()
                ),
            },
        }
    }
}

fn render_resource_events(badge: &str, name: &str, events: &[ResourceEvent]) {
    for event in events {
        match event {
            ResourceEvent::LoadedLocal { path } => {
                println!("{badge} Loaded {name}: {}", path.display())
            }
            ResourceEvent::MissingLocalUsedFallback { path, fallback } => match fallback {
                ResourceFallback::EmbeddedDefault => println!(
                    "{badge} {name} not found: {}; using embedded default",
                    path.display()
                ),
                ResourceFallback::RemoteUrl(url) => println!(
                    "{badge} {name} not found: {}; using fallback {url}",
                    path.display()
                ),
            },
            ResourceEvent::LoadedEmbeddedDefault => {
                println!("{badge} Using embedded default {name}")
            }
            ResourceEvent::LoadedRemoteFallback { url } => {
                println!("{badge} Downloaded fallback {name}: {url}")
            }
        }
    }
}

pub(crate) async fn run(options: FormatOptions) -> MResult<CliOutcome> {
    let badge = "[Mech Formatter]".truecolor(34, 204, 187);

    println!("{} Loading resources…", badge);
    print!("{} Loading stylesheet…", badge);
    let LoadedStylesheets {
        css: stylesheet_str,
        events,
    } = load_stylesheets(
        &options.stylesheet_paths,
        &options.resources.stylesheet_backup_url,
    )
    .await?;
    render_resource_events(&badge.to_string(), "stylesheet", &events);

    print!("{} Loading HTML shim…", badge);
    let shim = load_resource(
        &options.shim_path,
        &options.resources.shim_backup_url,
        Some(options.resources.shim_html.as_bytes()),
    )
    .await?;
    render_resource_events(&badge.to_string(), "HTML shim", &shim.events);
    let shim_str = String::from_utf8(shim.bytes).map_err(|e| {
        MechError::new(
            Utf8ConversionError {
                source_error: e.to_string(),
            },
            None,
        )
        .with_compiler_loc()
    })?;

    let plan = plan_format_targets(&options)?;
    render_discovery_events("[Mech Formatter]", &plan.discovery_events);
    let loaded_sources = load_format_sources(&plan.targets)?;
    write_format_outputs(&plan, loaded_sources, stylesheet_str, shim_str)?;

    Ok(CliOutcome::success())
}

#[cfg(test)]
pub(crate) use crate::cli::format_execute::read_format_source;
#[cfg(test)]
pub(crate) use crate::cli::format_targets::*;

#[cfg(test)]
mod tests;

// Mech
//=============================================================================

#![forbid(unsafe_code)]

#[cfg(all(feature = "distribution-standard", feature = "distribution-full"))]
compile_error!(
    "`distribution-standard` and `distribution-full` are mutually exclusive; use \
     `--no-default-features --features distribution-full` for a full build"
);

// Prelude
// ----------------------------------------------------------------------------

pub extern crate mech_core as core;
pub extern crate mech_engine as engine;
pub extern crate mech_stdlib as stdlib;
pub extern crate mech_syntax as syntax;

pub use mech_engine::*;
pub use mech_syntax::{
    ParseError, ParseErrorDetail, ParseResult, ParseString, ParserErrorContext, ParserErrorReport,
    TextFormatter, alt_best, graphemes, parse, parse_grammar, parse_mech, parser, print_err_report,
};

extern crate colored;
use colored::*;

extern crate bincode;
use std::io::{Write, stdout};

use std::path::PathBuf;
//use websocket::sync::Server;
use std::fs;

use rand::Rng;

#[cfg(feature = "bundle_web_core")]
mod bundle_web;
#[cfg(feature = "cli_core")]
pub mod cli;
#[cfg(any(feature = "cli_core", feature = "bundle_web_core"))]
pub mod fs_paths;
#[cfg(any(feature = "build", feature = "project"))]
mod project;
#[cfg(feature = "serve")]
mod serve;
#[cfg(any(feature = "cli_core", feature = "bundle_web_core"))]
pub mod source_discovery;
#[cfg(feature = "web_host")]
mod web_host;

#[cfg(feature = "bundle_web_core")]
pub use self::bundle_web::*;
#[cfg(any(feature = "build", feature = "project"))]
pub use self::project::*;
#[cfg(feature = "serve")]
pub use self::serve::*;
#[cfg(feature = "web_host")]
pub use self::web_host::*;

// Generate a new id for creating unique owner ids
pub fn generate_uuid() -> u64 {
    rand::rng().random()
}

pub fn save_to_file(mut path: PathBuf, content: &str) -> MResult<()> {
    // If path is a directory, give it a default file name
    if path.is_dir() {
        path.push("output.html");
    }

    print!(
        "{} Saving file to {}…",
        "[Save]".truecolor(153, 221, 85),
        path.display()
    );
    stdout().flush()?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = fs::File::create(&path)?;
    file.write_all(content.as_bytes())?;

    println!("Done.");
    Ok(())
}

pub enum Source<'a> {
    UserFile(&'a str),
    Embedded(&'a [u8]),
    Url(&'a str),
}

pub async fn read_or_download(
    path: &str,
    backup_url: &str,
    embedded: Option<&[u8]>,
) -> MResult<Vec<u8>> {
    // 1. User-supplied path always wins
    match std::fs::read(path) {
        Ok(content) => {
            println!("Using user-supplied resource: {}", path);
            return Ok(content);
        }
        Err(_) => { /* continue to embedded / download */ }
    }

    // 2. Embedded bytes (included via include_bytes!)
    if let Some(bytes) = embedded {
        if !bytes.is_empty() {
            println!("Using embedded resource");
            return Ok(bytes.to_vec());
        }
    }

    // 3. Fallback: Download from remote URL
    println!("Downloading from {}", backup_url);

    let response = reqwest::get(backup_url).await.map_err(|e| {
        MechError::new(
            HttpRequestFailed {
                url: backup_url.to_string(),
                source: e.to_string(),
            },
            None,
        )
        .with_compiler_loc()
    })?;

    if !response.status().is_success() {
        return Err(MechError::new(
            HttpRequestStatusFailed {
                url: backup_url.to_string(),
                status_code: response.status().as_u16(),
            },
            None,
        )
        .with_compiler_loc());
    }

    let bytes = response.bytes().await.map_err(|e| {
        MechError::new(
            HttpRequestFailed {
                url: backup_url.to_string(),
                source: e.to_string(),
            },
            None,
        )
        .with_compiler_loc()
    })?;

    Ok(bytes.to_vec())
}

#[derive(Debug, Clone)]
pub struct HttpRequestStatusFailed {
    pub url: String,
    pub status_code: u16,
}
impl MechErrorKind for HttpRequestStatusFailed {
    fn name(&self) -> &str {
        "HttpRequestStatusFailed"
    }
    fn message(&self) -> String {
        format!(
            "Request to {} failed with status code: {}",
            self.url, self.status_code
        )
    }
}

#[derive(Debug, Clone)]
pub struct FileWriteFailed {
    pub file_path: String,
    pub source: String,
}
impl MechErrorKind for FileWriteFailed {
    fn name(&self) -> &str {
        "FileWriteFailed"
    }

    fn message(&self) -> String {
        format!("Failed to write file {}: {}", self.file_path, self.source)
    }
}

#[derive(Debug, Clone)]
pub struct PathNotFound {
    pub file_path: String,
}
impl MechErrorKind for PathNotFound {
    fn name(&self) -> &str {
        "PathNotFound"
    }

    fn message(&self) -> String {
        format!("Path not found: {}", self.file_path)
    }
}

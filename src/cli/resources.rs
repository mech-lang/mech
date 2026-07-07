use std::path::{Path, PathBuf};

use mech_core::*;

use crate::{MechError, read_or_download};

#[cfg(has_file_wasm)]
pub(crate) static MECHWASM: &[u8] = include_bytes!("../../src/wasm/pkg/mech_wasm_bg.wasm.br");
#[cfg(not(has_file_wasm))]
pub(crate) static MECHWASM: &[u8] = b"No Embedded WASM";

#[cfg(has_file_js)]
pub(crate) static MECHJS: &[u8] = include_bytes!("../../src/wasm/pkg/mech_wasm.js");
#[cfg(not(has_file_js))]
pub(crate) static MECHJS: &[u8] = b"No Embedded JS";

#[cfg(has_file_shim)]
pub(crate) static SHIMHTML: &str = include_str!("../../include/index.html");
#[cfg(not(has_file_shim))]
pub(crate) static SHIMHTML: &str = "No Embedded Shim";

#[cfg(has_file_stylesheet)]
pub(crate) static STYLESHEET: &str = include_str!("../../include/style.css");
#[cfg(not(has_file_stylesheet))]
pub(crate) static STYLESHEET: &str = "No Embedded Stylesheet";

#[derive(Clone, Debug)]
pub(crate) struct WebResourceDefaults {
    pub stylesheet_backup_url: String,
    pub shim_backup_url: String,
    pub wasm_backup_url: String,
    pub js_backup_url: String,
    pub shim_html: &'static str,
    pub mech_wasm: &'static [u8],
    pub mech_js: &'static [u8],
}

impl WebResourceDefaults {
    pub(crate) fn new(version: &str) -> Self {
        Self {
            shim_backup_url:
                "https://raw.githubusercontent.com/mech-lang/mech/refs/heads/main/include/shim.html"
                    .to_string(),
            stylesheet_backup_url:
                "https://raw.githubusercontent.com/mech-lang/mech/refs/heads/main/include/style.css"
                    .to_string(),
            wasm_backup_url: format!(
                "https://github.com/mech-lang/mech/releases/download/v{version}-beta/mech_wasm_bg.wasm.br"
            ),
            js_backup_url: format!(
                "https://github.com/mech-lang/mech/releases/download/v{version}-beta/mech_wasm.js"
            ),
            shim_html: SHIMHTML,
            mech_wasm: MECHWASM,
            mech_js: MECHJS,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Utf8ConversionError {
    pub source_error: String,
}

impl MechErrorKind for Utf8ConversionError {
    fn name(&self) -> &str {
        "Utf8ConversionError"
    }
    fn message(&self) -> String {
        format!(
            "Failed to convert bytes into UTF-8 string: {}",
            self.source_error
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ResourceFallback {
    EmbeddedDefault,
    RemoteUrl(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ResourceEvent {
    LoadedLocal {
        path: PathBuf,
    },
    MissingLocalUsedFallback {
        path: PathBuf,
        fallback: ResourceFallback,
    },
    LoadedEmbeddedDefault,
    LoadedRemoteFallback {
        url: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LoadedStylesheets {
    pub css: String,
    pub events: Vec<ResourceEvent>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ResourceSource {
    LocalPath(PathBuf),
    RemoteUrl(String),
    EmbeddedDefault,
    EmptyPathFallback,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LoadedResource {
    pub bytes: Vec<u8>,
    pub source: ResourceSource,
}

fn utf8(bytes: Vec<u8>) -> Result<String, MechError> {
    String::from_utf8(bytes).map_err(|e| {
        MechError::new(
            Utf8ConversionError {
                source_error: e.to_string(),
            },
            None,
        )
        .with_compiler_loc()
    })
}

pub(crate) async fn load_resource(
    path: &str,
    fallback_url: &str,
    embedded: Option<&[u8]>,
) -> MResult<LoadedResource> {
    if !path.is_empty() {
        let path_buf = PathBuf::from(path);
        if Path::new(path).is_file() {
            return Ok(LoadedResource {
                bytes: std::fs::read(path)?,
                source: ResourceSource::LocalPath(path_buf),
            });
        }
        let bytes = read_or_download("", fallback_url, embedded).await?;
        let source = if embedded.is_some() {
            ResourceSource::EmbeddedDefault
        } else {
            ResourceSource::RemoteUrl(fallback_url.to_string())
        };
        return Ok(LoadedResource { bytes, source });
    }

    let bytes = read_or_download("", fallback_url, embedded).await?;
    let source = if embedded.is_some() {
        ResourceSource::EmptyPathFallback
    } else {
        ResourceSource::RemoteUrl(fallback_url.to_string())
    };
    Ok(LoadedResource { bytes, source })
}

pub(crate) async fn load_stylesheets(
    paths: &[String],
    fallback_url: &str,
) -> Result<LoadedStylesheets, MechError> {
    let mut events = Vec::new();
    if paths.is_empty() {
        let loaded = load_resource("", fallback_url, Some(STYLESHEET.as_bytes())).await?;
        events.push(match loaded.source {
            ResourceSource::RemoteUrl(url) => ResourceEvent::LoadedRemoteFallback { url },
            _ => ResourceEvent::LoadedEmbeddedDefault,
        });
        return Ok(LoadedStylesheets {
            css: utf8(loaded.bytes)?,
            events,
        });
    }

    let mut combined = String::new();
    for path in paths {
        let path_buf = PathBuf::from(path);
        let (stylesheet, event) = match std::fs::read(path) {
            Ok(content) => (content, ResourceEvent::LoadedLocal { path: path_buf }),
            Err(_) => {
                let loaded = load_resource("", fallback_url, Some(STYLESHEET.as_bytes())).await?;
                let fallback = match loaded.source {
                    ResourceSource::RemoteUrl(url) => ResourceFallback::RemoteUrl(url),
                    _ => ResourceFallback::EmbeddedDefault,
                };
                (
                    loaded.bytes,
                    ResourceEvent::MissingLocalUsedFallback {
                        path: path_buf,
                        fallback,
                    },
                )
            }
        };
        let stylesheet_str = utf8(stylesheet)?;
        if !combined.is_empty() {
            combined.push('\n');
        }
        combined.push_str(&stylesheet_str);
        events.push(event);
    }
    Ok(LoadedStylesheets {
        css: combined,
        events,
    })
}

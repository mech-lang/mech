use std::path::{Path, PathBuf};

use mech_core::*;
use mech_runtime::{FS_READ, HostFilesystemAuthority, check_fs_capability};

use crate::{MechError, read_or_download};

#[cfg(has_file_wasm)]
pub(crate) static MECHWASM: &[u8] = include_bytes!("../../src/wasm/pkg/mech_wasm_bg.wasm");

#[cfg(has_file_js)]
pub(crate) static MECHJS: &[u8] = include_bytes!("../../src/wasm/pkg/mech_wasm.js");

#[cfg(has_file_project_js)]
pub(crate) static PROJECTJS: &str = include_str!("../../include/project.js");

#[cfg(has_file_shim)]
pub(crate) static SHIMHTML: &str = include_str!("../../include/index.html");
#[cfg(not(has_file_shim))]
pub(crate) static SHIMHTML: &str = "No Embedded Shim";

#[cfg(has_file_stylesheet)]
pub(crate) static STYLESHEET: &str = include_str!("../../include/style.css");
#[cfg(not(has_file_stylesheet))]
pub(crate) static STYLESHEET: &str = "No Embedded Stylesheet";

#[cfg(has_file_wasm)]
fn embedded_wasm() -> Option<&'static [u8]> { Some(MECHWASM) }
#[cfg(not(has_file_wasm))]
fn embedded_wasm() -> Option<&'static [u8]> { None }

#[cfg(has_file_js)]
fn embedded_js() -> Option<&'static [u8]> { Some(MECHJS) }
#[cfg(not(has_file_js))]
fn embedded_js() -> Option<&'static [u8]> { None }

#[cfg(has_file_project_js)]
fn embedded_project_js() -> Option<&'static str> { Some(PROJECTJS) }
#[cfg(not(has_file_project_js))]
fn embedded_project_js() -> Option<&'static str> { None }

#[derive(Clone, Debug)]
pub(crate) struct WebResourceDefaults {
    pub stylesheet_backup_url: String,
    pub shim_backup_url: String,
    pub wasm_backup_url: String,
    pub js_backup_url: String,
    pub shim_html: &'static str,
    pub mech_wasm: Option<&'static [u8]>,
    pub mech_js: Option<&'static [u8]>,
    pub project_js: Option<&'static str>,
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
                "https://github.com/mech-lang/mech/releases/download/v{version}-beta/mech_wasm_bg.wasm"
            ),
            js_backup_url: format!(
                "https://github.com/mech-lang/mech/releases/download/v{version}-beta/mech_wasm.js"
            ),
            shim_html: SHIMHTML,
            mech_wasm: embedded_wasm(),
            mech_js: embedded_js(),
            project_js: embedded_project_js(),
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
    pub local_paths: Vec<PathBuf>,
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
    pub events: Vec<ResourceEvent>,
}

fn read_authorized_local_file(
    authority: &HostFilesystemAuthority,
    path: &Path,
) -> MResult<(PathBuf, Vec<u8>)> {
    let canonical_path = path.canonicalize()?;
    let mut kernel = authority.kernel().clone();
    check_fs_capability(&mut kernel, authority.subject(), FS_READ, &canonical_path)?;
    let bytes = std::fs::read(&canonical_path)?;
    Ok((canonical_path, bytes))
}

fn utf8(bytes: Vec<u8>) -> MResult<String> {
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
    authority: &HostFilesystemAuthority,
    path: &str,
    fallback_url: &str,
    embedded: Option<&[u8]>,
) -> MResult<LoadedResource> {
    if !path.is_empty() {
        let path_buf = PathBuf::from(path);
        if Path::new(path).exists() {
            let (canonical_path, bytes) = read_authorized_local_file(authority, Path::new(path))?;
            return Ok(LoadedResource {
                bytes,
                source: ResourceSource::LocalPath(canonical_path.clone()),
                events: vec![ResourceEvent::LoadedLocal {
                    path: canonical_path,
                }],
            });
        }
        let bytes = read_or_download("", fallback_url, embedded).await?;
        let (source, fallback, loaded_event) = if embedded.is_some() {
            (
                ResourceSource::EmbeddedDefault,
                ResourceFallback::EmbeddedDefault,
                ResourceEvent::LoadedEmbeddedDefault,
            )
        } else {
            (
                ResourceSource::RemoteUrl(fallback_url.to_string()),
                ResourceFallback::RemoteUrl(fallback_url.to_string()),
                ResourceEvent::LoadedRemoteFallback {
                    url: fallback_url.to_string(),
                },
            )
        };
        return Ok(LoadedResource {
            bytes,
            source,
            events: vec![
                ResourceEvent::MissingLocalUsedFallback {
                    path: path_buf,
                    fallback,
                },
                loaded_event,
            ],
        });
    }

    let bytes = read_or_download("", fallback_url, embedded).await?;
    let source = if embedded.is_some() {
        ResourceSource::EmptyPathFallback
    } else {
        ResourceSource::RemoteUrl(fallback_url.to_string())
    };
    Ok(LoadedResource {
        bytes,
        source,
        events: vec![if embedded.is_some() {
            ResourceEvent::LoadedEmbeddedDefault
        } else {
            ResourceEvent::LoadedRemoteFallback {
                url: fallback_url.to_string(),
            }
        }],
    })
}

pub(crate) async fn load_stylesheets(
    authority: &HostFilesystemAuthority,
    paths: &[String],
    fallback_url: &str,
) -> MResult<LoadedStylesheets> {
    let mut events = Vec::new();
    if paths.is_empty() {
        let loaded =
            load_resource(authority, "", fallback_url, Some(STYLESHEET.as_bytes())).await?;
        events.push(match loaded.source {
            ResourceSource::RemoteUrl(url) => ResourceEvent::LoadedRemoteFallback { url },
            _ => ResourceEvent::LoadedEmbeddedDefault,
        });
        return Ok(LoadedStylesheets {
            css: utf8(loaded.bytes)?,
            events,
            local_paths: Vec::new(),
        });
    }

    let mut combined = String::new();
    let mut local_paths = Vec::new();
    for path in paths {
        let path_buf = PathBuf::from(path);
        let (stylesheet, event) = if Path::new(path).exists() {
            let (canonical_path, content) = read_authorized_local_file(authority, Path::new(path))?;
            local_paths.push(canonical_path.clone());
            (
                content,
                ResourceEvent::LoadedLocal {
                    path: canonical_path,
                },
            )
        } else {
            let loaded =
                load_resource(authority, "", fallback_url, Some(STYLESHEET.as_bytes())).await?;
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
        local_paths,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use mech_runtime::{DefaultIdGenerator, MECH_TOOL_SUBJECT, SharedCapabilityKernel};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(name: &str) -> PathBuf {
        let mut root = std::env::temp_dir();
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        root.push(format!("mech-{name}-{unique}"));
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    fn authority_for(
        path: &Path,
        recursive: bool,
        ops: impl IntoIterator<Item = &'static str>,
    ) -> HostFilesystemAuthority {
        let mut ids = DefaultIdGenerator::new();
        let mut authority =
            HostFilesystemAuthority::new(MECH_TOOL_SUBJECT, SharedCapabilityKernel::new());
        authority
            .grant_path(&mut ids, path, recursive, ops)
            .unwrap();
        authority
    }

    fn block_on<T>(future: impl std::future::Future<Output = T>) -> T {
        tokio::runtime::Runtime::new().unwrap().block_on(future)
    }

    #[test]
    fn local_resource_read_requires_fs_read() {
        let root = temp_root("resource-read");
        let file = root.join("shim.html");
        std::fs::write(&file, "local").unwrap();
        let authority = authority_for(&root, true, [FS_READ]);
        let loaded = block_on(load_resource(
            &authority,
            file.to_str().unwrap(),
            "unused",
            Some(b"fallback"),
        ))
        .unwrap();
        assert_eq!(loaded.bytes, b"local");
        assert_eq!(
            loaded.source,
            ResourceSource::LocalPath(file.canonicalize().unwrap())
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn denied_local_resource_does_not_fallback() {
        let root = temp_root("resource-denied");
        let allowed = root.join("allowed");
        let outside = root.join("outside");
        std::fs::create_dir_all(&allowed).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        let file = outside.join("shim.html");
        std::fs::write(&file, "secret").unwrap();
        let authority = authority_for(&allowed, true, [FS_READ]);
        let result = block_on(load_resource(
            &authority,
            file.to_str().unwrap(),
            "unused",
            Some(b"fallback"),
        ));
        assert!(result.is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn existing_local_directory_does_not_use_fallback() {
        let root = temp_root("resource-directory");
        let directory = root.join("shim-dir");
        std::fs::create_dir_all(&directory).unwrap();
        let authority = authority_for(&root, true, [FS_READ]);
        let result = block_on(load_resource(
            &authority,
            directory.to_str().unwrap(),
            "unused",
            Some(b"fallback"),
        ));
        assert!(result.is_err());
        assert!(!result
            .as_ref()
            .map(|loaded| loaded.bytes.as_slice() == b"fallback")
            .unwrap_or(false));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn missing_local_resource_still_uses_embedded_fallback() {
        let root = temp_root("resource-missing");
        let missing = root.join("missing.html");
        let authority = authority_for(&root, true, [FS_READ]);
        let loaded = block_on(load_resource(
            &authority,
            missing.to_str().unwrap(),
            "unused",
            Some(b"fallback"),
        ))
        .unwrap();
        assert_eq!(loaded.bytes, b"fallback");
        assert!(matches!(loaded.source, ResourceSource::EmbeddedDefault));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn stylesheet_records_canonical_local_paths() {
        let root = temp_root("stylesheet-local");
        let css = root.join("style.css");
        std::fs::write(&css, "body{}").unwrap();
        let authority = authority_for(&root, true, [FS_READ]);
        let loaded = block_on(load_stylesheets(
            &authority,
            &[css.to_string_lossy().to_string()],
            "unused",
        ))
        .unwrap();
        assert_eq!(loaded.css, "body{}");
        assert_eq!(loaded.local_paths, vec![css.canonicalize().unwrap()]);
        std::fs::remove_dir_all(root).unwrap();
    }
}

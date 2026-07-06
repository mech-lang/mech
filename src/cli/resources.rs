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

pub(crate) async fn load_stylesheets(
    paths: &[String],
    fallback_url: &str,
) -> Result<String, MechError> {
    if paths.is_empty() {
        let stylesheet = read_or_download("", fallback_url, Some(STYLESHEET.as_bytes())).await?;
        return String::from_utf8(stylesheet).map_err(|e| {
            MechError::new(
                Utf8ConversionError {
                    source_error: e.to_string(),
                },
                None,
            )
            .with_compiler_loc()
        });
    }

    let mut combined = String::new();
    for path in paths {
        let stylesheet = match std::fs::read(path) {
            Ok(content) => {
                println!("Using stylesheet: {}", path);
                content
            }
            Err(_) => {
                println!("\nStylesheet not found:\n  {}", path);
                read_or_download("", fallback_url, Some(STYLESHEET.as_bytes())).await?
            }
        };
        let stylesheet_str = String::from_utf8(stylesheet).map_err(|e| {
            MechError::new(
                Utf8ConversionError {
                    source_error: e.to_string(),
                },
                None,
            )
            .with_compiler_loc()
        })?;
        if !combined.is_empty() {
            combined.push('\n');
        }
        combined.push_str(&stylesheet_str);
    }
    Ok(combined)
}

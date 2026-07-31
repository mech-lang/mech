use mech_core::MResult;
use mech_runtime::ConfigValue;

use crate::scene_error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneRendererKind {
    Canvas,
    Svg,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneHostSettings {
    pub selector: String,
    pub renderer: SceneRendererKind,
}

pub fn scene_settings_from_config(settings: &ConfigValue) -> MResult<SceneHostSettings> {
    let ConfigValue::Map(map) = settings else {
        return Err(scene_error(
            "SceneHostConfig",
            "scene host settings must be a map",
        ));
    };
    let mut selector = None;
    let mut renderer = None;
    for (key, value) in map {
        match key.as_str() {
            "selector" => {
                let ConfigValue::String(raw) = value else {
                    return Err(scene_error(
                        "SceneHostConfig",
                        "scene selector must be a string",
                    ));
                };
                if raw.trim().is_empty() {
                    return Err(scene_error(
                        "SceneHostConfig",
                        "scene selector must be non-empty",
                    ));
                }
                selector = Some(raw.clone());
            }
            "renderer" => {
                let ConfigValue::String(raw) = value else {
                    return Err(scene_error(
                        "SceneHostConfig",
                        "scene renderer must be a string",
                    ));
                };
                renderer = Some(match raw.as_str() {
                    "canvas" => SceneRendererKind::Canvas,
                    "svg" => SceneRendererKind::Svg,
                    other => {
                        return Err(scene_error(
                            "SceneHostConfig",
                            format!("unknown scene renderer `{other}`"),
                        ));
                    }
                });
            }
            other => {
                return Err(scene_error(
                    "SceneHostConfig",
                    format!("unknown scene host setting `{other}`"),
                ));
            }
        }
    }
    Ok(SceneHostSettings {
        selector: selector
            .ok_or_else(|| scene_error("SceneHostConfig", "scene selector is required"))?,
        renderer: renderer
            .ok_or_else(|| scene_error("SceneHostConfig", "scene renderer is required"))?,
    })
}

use mech_core::MResult;
use mech_runtime::ConfigValue;

use crate::scene_error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneRendererKind {
    Canvas,
    Svg,
    /// Publish the scene through the portable output protocol without
    /// requiring an application-owned DOM target.
    Output,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneHostSettings {
    pub selector: String,
    pub renderer: SceneRendererKind,
    pub width: u32,
    pub height: u32,
    pub pixels_per_unit: u32,
    pub point_radius: u32,
    pub first_point_radius: u32,
    pub background: String,
}

impl SceneHostSettings {
    pub fn new(selector: impl Into<String>, renderer: SceneRendererKind) -> Self {
        Self {
            selector: selector.into(),
            renderer,
            width: 800,
            height: 600,
            pixels_per_unit: 9,
            point_radius: 4,
            first_point_radius: 8,
            background: "#050816".to_owned(),
        }
    }
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
    let mut parsed = SceneHostSettings::new("", SceneRendererKind::Svg);
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
                    "output" => SceneRendererKind::Output,
                    other => {
                        return Err(scene_error(
                            "SceneHostConfig",
                            format!("unknown scene renderer `{other}`"),
                        ));
                    }
                });
            }
            "width" => parsed.width = setting_u32(value, "width", 1, 16_384)?,
            "height" => parsed.height = setting_u32(value, "height", 1, 16_384)?,
            "pixels-per-unit" => {
                parsed.pixels_per_unit = setting_u32(value, "pixels-per-unit", 1, 10_000)?
            }
            "point-radius" => parsed.point_radius = setting_u32(value, "point-radius", 1, 1_000)?,
            "first-point-radius" => {
                parsed.first_point_radius = setting_u32(value, "first-point-radius", 1, 1_000)?
            }
            "background" => {
                let ConfigValue::String(raw) = value else {
                    return Err(scene_error(
                        "SceneHostConfig",
                        "scene background must be a string",
                    ));
                };
                if raw.trim().is_empty() {
                    return Err(scene_error(
                        "SceneHostConfig",
                        "scene background must be non-empty",
                    ));
                }
                parsed.background = raw.clone();
            }
            other => {
                return Err(scene_error(
                    "SceneHostConfig",
                    format!("unknown scene host setting `{other}`"),
                ));
            }
        }
    }
    parsed.renderer =
        renderer.ok_or_else(|| scene_error("SceneHostConfig", "scene renderer is required"))?;
    parsed.selector = match (parsed.renderer, selector) {
        (SceneRendererKind::Output, selector) => selector.unwrap_or_default(),
        (_, Some(selector)) => selector,
        _ => return Err(scene_error("SceneHostConfig", "scene selector is required")),
    };
    Ok(parsed)
}

fn setting_u32(value: &ConfigValue, name: &str, min: u32, max: u32) -> MResult<u32> {
    let ConfigValue::Integer(raw) = value else {
        return Err(scene_error(
            "SceneHostConfig",
            format!("scene {name} must be an integer"),
        ));
    };
    let value = u32::try_from(*raw).map_err(|_| {
        scene_error(
            "SceneHostConfig",
            format!("scene {name} must be between {min} and {max}"),
        )
    })?;
    if !(min..=max).contains(&value) {
        return Err(scene_error(
            "SceneHostConfig",
            format!("scene {name} must be between {min} and {max}"),
        ));
    }
    Ok(value)
}

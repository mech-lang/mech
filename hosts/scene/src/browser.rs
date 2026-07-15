use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use mech_core::MResult;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{CanvasRenderingContext2d, Element, HtmlCanvasElement};

use crate::{SceneBackend, SceneHostSettings, SceneRendererKind, SceneSnapshot, scene_error};

#[derive(Clone, Debug, Default)]
pub struct BrowserSceneRegistry {
    targets: Arc<Mutex<HashMap<String, BrowserSceneTarget>>>,
}

#[derive(Clone, Debug)]
struct BrowserSceneTarget {
    selector: String,
    renderer: SceneRendererKind,
    latest: Option<SceneSnapshot>,
    generation: u64,
    rendered_generation: u64,
}

impl BrowserSceneRegistry {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn register(
        &self,
        instance: impl Into<String>,
        settings: SceneHostSettings,
    ) -> MResult<()> {
        self.targets
            .lock()
            .map_err(|_| scene_error("BrowserSceneRegistry", "scene registry lock is poisoned"))?
            .insert(
                instance.into(),
                BrowserSceneTarget {
                    selector: settings.selector,
                    renderer: settings.renderer,
                    latest: None,
                    generation: 0,
                    rendered_generation: 0,
                },
            );
        Ok(())
    }
    pub fn replace_scene(&self, instance: &str, scene: SceneSnapshot) -> MResult<()> {
        let mut guard = self
            .targets
            .lock()
            .map_err(|_| scene_error("BrowserSceneRegistry", "scene registry lock is poisoned"))?;
        let target = guard.get_mut(instance).ok_or_else(|| {
            scene_error(
                "BrowserSceneRegistry",
                format!("unknown scene target `{instance}`"),
            )
        })?;
        target.latest = Some(scene);
        target.generation += 1;
        Ok(())
    }
    pub fn render_frame(&self) -> MResult<u32> {
        let mut rendered = 0;
        let mut guard = self
            .targets
            .lock()
            .map_err(|_| scene_error("BrowserSceneRegistry", "scene registry lock is poisoned"))?;
        for target in guard.values_mut() {
            if target.latest.is_none() || target.rendered_generation == target.generation {
                continue;
            }
            let scene = target.latest.clone().unwrap();
            match target.renderer {
                SceneRendererKind::Canvas => render_canvas(&target.selector, &scene)?,
                SceneRendererKind::Svg => render_svg(&target.selector, &scene)?,
            }
            target.rendered_generation = target.generation;
            rendered += 1;
        }
        Ok(rendered)
    }
    pub fn target_count(&self) -> usize {
        self.targets.lock().map(|g| g.len()).unwrap_or(0)
    }
}

#[derive(Clone, Debug)]
pub struct BrowserSceneBackend {
    instance: String,
    registry: BrowserSceneRegistry,
}
impl BrowserSceneBackend {
    pub fn new(instance: impl Into<String>, registry: BrowserSceneRegistry) -> Self {
        Self {
            instance: instance.into(),
            registry,
        }
    }
}
impl SceneBackend for BrowserSceneBackend {
    fn replace_scene(&mut self, scene: SceneSnapshot) -> MResult<()> {
        self.registry.replace_scene(&self.instance, scene)
    }
}

#[derive(Clone, Debug)]
pub struct BrowserSceneHostFactory {
    registry: BrowserSceneRegistry,
    manifest: mech_runtime::HostManifestConfig,
}
impl BrowserSceneHostFactory {
    pub fn new() -> MResult<Self> {
        Self::with_registry(BrowserSceneRegistry::new())
    }
    pub fn with_registry(registry: BrowserSceneRegistry) -> MResult<Self> {
        Ok(Self {
            registry,
            manifest: crate::scene_host_manifest()?,
        })
    }
    pub fn registry(&self) -> BrowserSceneRegistry {
        self.registry.clone()
    }
}
impl mech_runtime::RuntimeHostFactory for BrowserSceneHostFactory {
    fn provider_name(&self) -> &str {
        "scene"
    }
    fn manifest(&self) -> &mech_runtime::HostManifestConfig {
        &self.manifest
    }
    fn validate_settings(
        &self,
        _instance_name: &str,
        settings: &mech_runtime::ConfigValue,
    ) -> MResult<()> {
        crate::scene_settings_from_config(settings).map(|_| ())
    }
    fn instantiate(
        &self,
        instance_name: &str,
        settings: &mech_runtime::ConfigValue,
    ) -> MResult<mech_runtime::RuntimeHostInstallation> {
        let parsed = crate::scene_settings_from_config(settings)?;
        self.registry.register(instance_name, parsed)?;
        Ok(mech_runtime::RuntimeHostInstallation {
            interface: mech_runtime::materialize_host_manifest(instance_name, &self.manifest)?,
            resource_providers: vec![Box::new(crate::SceneResourceProvider::new(
                instance_name,
                BrowserSceneBackend::new(instance_name, self.registry.clone()),
            ))],
            input_drivers: Vec::new(),
        })
    }
}

fn document() -> MResult<web_sys::Document> {
    web_sys::window()
        .and_then(|w| w.document())
        .ok_or_else(|| scene_error("BrowserScene", "browser document is unavailable"))
}
fn selected(selector: &str) -> MResult<Element> {
    document()?
        .query_selector(selector)
        .map_err(|_| {
            scene_error(
                "BrowserScene",
                format!("failed to query selector `{selector}`"),
            )
        })?
        .ok_or_else(|| {
            scene_error(
                "BrowserScene",
                format!("selector `{selector}` did not match an element"),
            )
        })
}

fn render_canvas(selector: &str, scene: &SceneSnapshot) -> MResult<()> {
    let canvas: HtmlCanvasElement = selected(selector)?.dyn_into().map_err(|_| {
        scene_error(
            "BrowserScene",
            format!("selector `{selector}` is not a canvas"),
        )
    })?;
    let window =
        web_sys::window().ok_or_else(|| scene_error("BrowserScene", "window unavailable"))?;
    let ratio = window.device_pixel_ratio();
    canvas.set_width((scene.width * ratio).round() as u32);
    canvas.set_height((scene.height * ratio).round() as u32);
    let ctx: CanvasRenderingContext2d = canvas
        .get_context("2d")
        .map_err(|_| scene_error("BrowserScene", "canvas getContext failed"))?
        .ok_or_else(|| scene_error("BrowserScene", "2d canvas context unavailable"))?
        .dyn_into()
        .map_err(|_| scene_error("BrowserScene", "context is not CanvasRenderingContext2d"))?;
    ctx.set_transform(ratio, 0.0, 0.0, ratio, 0.0, 0.0)
        .map_err(|_| scene_error("BrowserScene", "failed to set canvas transform"))?;
    ctx.set_global_alpha(1.0);
    ctx.set_fill_style(&JsValue::from_str(&scene.background));
    ctx.fill_rect(0.0, 0.0, scene.width, scene.height);
    for c in &scene.circles {
        ctx.begin_path();
        ctx.set_global_alpha(c.opacity);
        ctx.set_fill_style(&JsValue::from_str(&c.fill));
        ctx.set_stroke_style(&JsValue::from_str(&c.stroke));
        ctx.set_line_width(c.stroke_width);
        ctx.arc(c.x, c.y, c.radius, 0.0, std::f64::consts::TAU)
            .map_err(|_| {
                scene_error("BrowserScene", format!("failed to draw circle `{}`", c.id))
            })?;
        ctx.fill();
        if c.stroke_width > 0.0 {
            ctx.stroke();
        }
    }
    for l in &scene.lines {
        ctx.save();
        ctx.set_global_alpha(l.opacity);
        ctx.set_stroke_style(&JsValue::from_str(&l.stroke));
        ctx.set_line_width(l.stroke_width);
        ctx.set_line_cap(&l.line_cap);
        ctx.translate(l.origin_x, l.origin_y).ok();
        ctx.rotate(l.rotation.to_radians()).ok();
        ctx.translate(-l.origin_x, -l.origin_y).ok();
        ctx.begin_path();
        ctx.move_to(l.x1, l.y1);
        ctx.line_to(l.x2, l.y2);
        ctx.stroke();
        ctx.restore();
    }
    ctx.set_global_alpha(1.0);
    Ok(())
}

fn render_svg(selector: &str, scene: &SceneSnapshot) -> MResult<()> {
    let root = selected(selector)?;
    root.set_attribute("viewBox", &format!("0 0 {} {}", scene.width, scene.height))
        .map_err(|_| scene_error("BrowserScene", "failed to set svg viewBox"))?;
    let doc = document()?;
    let ns = Some("http://www.w3.org/2000/svg");
    let managed_selector = "[data-mech-scene=\"true\"]";
    let list = root
        .query_selector_all(managed_selector)
        .map_err(|_| scene_error("BrowserScene", "failed to query managed svg elements"))?;
    let mut keep = HashSet::new();
    for c in &scene.circles {
        keep.insert(c.id.clone());
        let el = upsert(&doc, &root, ns, "circle", &c.id)?;
        el.set_attribute("cx", &c.x.to_string()).ok();
        el.set_attribute("cy", &c.y.to_string()).ok();
        el.set_attribute("r", &c.radius.to_string()).ok();
        el.set_attribute("fill", &c.fill).ok();
        el.set_attribute("stroke", &c.stroke).ok();
        el.set_attribute("stroke-width", &c.stroke_width.to_string())
            .ok();
        el.set_attribute("opacity", &c.opacity.to_string()).ok();
    }
    for l in &scene.lines {
        keep.insert(l.id.clone());
        let el = upsert(&doc, &root, ns, "line", &l.id)?;
        el.set_attribute("x1", &l.x1.to_string()).ok();
        el.set_attribute("y1", &l.y1.to_string()).ok();
        el.set_attribute("x2", &l.x2.to_string()).ok();
        el.set_attribute("y2", &l.y2.to_string()).ok();
        el.set_attribute("stroke", &l.stroke).ok();
        el.set_attribute("stroke-width", &l.stroke_width.to_string())
            .ok();
        el.set_attribute("stroke-linecap", &l.line_cap).ok();
        el.set_attribute("opacity", &l.opacity.to_string()).ok();
        el.set_attribute(
            "transform",
            &format!("rotate({} {} {})", l.rotation, l.origin_x, l.origin_y),
        )
        .ok();
    }
    for i in 0..list.length() {
        if let Some(node) = list.item(i) {
            if let Ok(el) = node.dyn_into::<Element>() {
                let id = el.get_attribute("data-mech-scene-id").unwrap_or_default();
                if !keep.contains(&id) {
                    el.remove();
                }
            }
        }
    }
    Ok(())
}
fn upsert(
    doc: &web_sys::Document,
    root: &Element,
    ns: Option<&str>,
    tag: &str,
    id: &str,
) -> MResult<Element> {
    let selector = format!("[data-mech-scene-id=\"{}\"]", id);
    if let Some(el) = root
        .query_selector(&selector)
        .map_err(|_| scene_error("BrowserScene", "failed to query svg element"))?
    {
        return Ok(el);
    }
    let el = doc
        .create_element_ns(ns, tag)
        .map_err(|_| scene_error("BrowserScene", format!("failed to create svg `{tag}`")))?;
    el.set_attribute("data-mech-scene", "true").ok();
    el.set_attribute("data-mech-scene-id", id).ok();
    root.append_child(&el)
        .map_err(|_| scene_error("BrowserScene", "failed to append svg element"))?;
    Ok(el)
}

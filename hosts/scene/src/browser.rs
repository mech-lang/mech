use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use mech_core::MResult;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{CanvasRenderingContext2d, Element, HtmlCanvasElement, SvgsvgElement};

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

#[derive(Clone, Debug)]
struct RenderJob {
    instance: String,
    selector: String,
    renderer: SceneRendererKind,
    generation: u64,
    scene: SceneSnapshot,
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
        let jobs = {
            let guard = self.targets.lock().map_err(|_| {
                scene_error("BrowserSceneRegistry", "scene registry lock is poisoned")
            })?;
            guard
                .iter()
                .filter_map(|(instance, target)| {
                    let scene = target.latest.clone()?;
                    if target.rendered_generation == target.generation {
                        return None;
                    }
                    Some(RenderJob {
                        instance: instance.clone(),
                        selector: target.selector.clone(),
                        renderer: target.renderer,
                        generation: target.generation,
                        scene,
                    })
                })
                .collect::<Vec<_>>()
        };

        let mut rendered = 0;
        for job in jobs {
            match job.renderer {
                SceneRendererKind::Canvas => render_canvas(&job.selector, &job.scene)?,
                SceneRendererKind::Svg => render_svg(&job.selector, &job.scene)?,
            }
            let mut guard = self.targets.lock().map_err(|_| {
                scene_error("BrowserSceneRegistry", "scene registry lock is poisoned")
            })?;
            if let Some(target) = guard.get_mut(&job.instance) {
                if target.generation == job.generation {
                    target.rendered_generation = job.generation;
                    rendered += 1;
                }
            }
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
    if !ratio.is_finite() || ratio <= 0.0 {
        return Err(scene_error(
            "BrowserScene",
            "devicePixelRatio must be finite and positive",
        ));
    }
    canvas.set_width((scene.width * ratio).round() as u32);
    canvas.set_height((scene.height * ratio).round() as u32);
    canvas
        .style()
        .set_property("width", &format!("{}px", scene.width))
        .map_err(|_| scene_error("BrowserScene", "failed to set canvas CSS width"))?;
    canvas
        .style()
        .set_property("height", &format!("{}px", scene.height))
        .map_err(|_| scene_error("BrowserScene", "failed to set canvas CSS height"))?;
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
        ctx.translate(l.origin_x, l.origin_y)
            .map_err(|_| scene_error("BrowserScene", format!("failed to translate line `{}`", l.id)))?;
        ctx.rotate(l.rotation.to_radians())
            .map_err(|_| scene_error("BrowserScene", format!("failed to rotate line `{}`", l.id)))?;
        ctx.translate(-l.origin_x, -l.origin_y)
            .map_err(|_| scene_error("BrowserScene", format!("failed to restore line `{}` translation", l.id)))?;
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
    let root: SvgsvgElement = selected(selector)?.dyn_into().map_err(|_| {
        scene_error(
            "BrowserScene",
            format!("selector `{selector}` is not an SVG root"),
        )
    })?;
    let root: Element = root.into();
    root.set_attribute("viewBox", &format!("0 0 {} {}", scene.width, scene.height))
        .map_err(|_| scene_error("BrowserScene", "failed to set svg viewBox"))?;
    let doc = document()?;
    let ns = Some("http://www.w3.org/2000/svg");
    let managed_selector = "[data-mech-scene=\"true\"]";
    upsert_background(&doc, &root, ns, scene)?;
    let list = root
        .query_selector_all(managed_selector)
        .map_err(|_| scene_error("BrowserScene", "failed to query managed svg elements"))?;
    let mut keep = HashSet::new();
    for c in &scene.circles {
        keep.insert(c.id.clone());
        let el = upsert(&doc, &root, ns, "circle", &c.id)?;
        set_attr(&el, "cx", &c.x.to_string())?;
        set_attr(&el, "cy", &c.y.to_string())?;
        set_attr(&el, "r", &c.radius.to_string())?;
        set_attr(&el, "fill", &c.fill)?;
        set_attr(&el, "stroke", &c.stroke)?;
        set_attr(&el, "stroke-width", &c.stroke_width.to_string())?;
        set_attr(&el, "opacity", &c.opacity.to_string())?;
    }
    for l in &scene.lines {
        keep.insert(l.id.clone());
        let el = upsert(&doc, &root, ns, "line", &l.id)?;
        set_attr(&el, "x1", &l.x1.to_string())?;
        set_attr(&el, "y1", &l.y1.to_string())?;
        set_attr(&el, "x2", &l.x2.to_string())?;
        set_attr(&el, "y2", &l.y2.to_string())?;
        set_attr(&el, "stroke", &l.stroke)?;
        set_attr(&el, "stroke-width", &l.stroke_width.to_string())?;
        set_attr(&el, "stroke-linecap", &l.line_cap)?;
        set_attr(&el, "opacity", &l.opacity.to_string())?;
        set_attr(
            &el,
            "transform",
            &format!("rotate({} {} {})", l.rotation, l.origin_x, l.origin_y),
        )?;
    }
    for i in 0..list.length() {
        if let Some(node) = list.item(i) {
            if let Ok(el) = node.dyn_into::<Element>() {
                let id = el.get_attribute("data-mech-scene-id").unwrap_or_default();
                if el.has_attribute("data-mech-scene-background") {
                    continue;
                }
                if !keep.contains(&id) {
                    el.remove();
                }
            }
        }
    }
    Ok(())
}
fn upsert_background(
    doc: &web_sys::Document,
    root: &Element,
    ns: Option<&str>,
    scene: &SceneSnapshot,
) -> MResult<Element> {
    let el = match root
        .query_selector("[data-mech-scene-background=\"true\"]")
        .map_err(|_| scene_error("BrowserScene", "failed to query svg background"))? {
        Some(el) => el,
        None => {
            let el = doc
                .create_element_ns(ns, "rect")
                .map_err(|_| scene_error("BrowserScene", "failed to create svg background"))?;
            set_attr(&el, "data-mech-scene", "true")?;
            set_attr(&el, "data-mech-scene-background", "true")?;
            match root.first_child() {
                Some(first) => root
                    .insert_before(&el, Some(&first))
                    .map_err(|_| scene_error("BrowserScene", "failed to insert svg background"))?,
                None => root
                    .append_child(&el)
                    .map_err(|_| scene_error("BrowserScene", "failed to append svg background"))?,
            };
            el
        }
    };
    set_attr(&el, "x", "0")?;
    set_attr(&el, "y", "0")?;
    set_attr(&el, "width", &scene.width.to_string())?;
    set_attr(&el, "height", &scene.height.to_string())?;
    set_attr(&el, "fill", &scene.background)?;
    Ok(el)
}

fn upsert(
    doc: &web_sys::Document,
    root: &Element,
    ns: Option<&str>,
    tag: &str,
    id: &str,
) -> MResult<Element> {
    let managed = root
        .query_selector_all("[data-mech-scene=\"true\"]")
        .map_err(|_| scene_error("BrowserScene", "failed to query managed svg elements"))?;
    for index in 0..managed.length() {
        if let Some(node) = managed.item(index) {
            if let Ok(el) = node.dyn_into::<Element>() {
                if el.get_attribute("data-mech-scene-id").as_deref() == Some(id) {
                    let existing_tag = el.tag_name().to_ascii_lowercase();
                    if existing_tag == tag {
                        return Ok(el);
                    }
                    el.remove();
                    break;
                }
            }
        }
    }
    let el = doc
        .create_element_ns(ns, tag)
        .map_err(|_| scene_error("BrowserScene", format!("failed to create svg `{tag}`")))?;
    set_attr(&el, "data-mech-scene", "true")?;
    set_attr(&el, "data-mech-scene-id", id)?;
    root.append_child(&el)
        .map_err(|_| scene_error("BrowserScene", "failed to append svg element"))?;
    Ok(el)
}

fn set_attr(el: &Element, name: &str, value: &str) -> MResult<()> {
    el.set_attribute(name, value).map_err(|_| {
        scene_error(
            "BrowserScene",
            format!("failed to set svg attribute `{name}`"),
        )
    })
}

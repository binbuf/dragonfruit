// SPDX-License-Identifier: MIT
//! DRM/KMS backend (T-02): native sessions, owning the physical display
//! through a logind/libseat session.
//!
//! Modeled on upstream anvil's udev backend (smithay 0.7): udev device
//! enumeration + hotplug, per-connector `DrmOutput`s on the GBM/EGL
//! renderer stack, vblank-driven frame scheduling with damage tracking
//! and direct scanout (FR-5), and libinput for seat devices. Multi-GPU
//! renders on the primary node and imports buffers across devices; a
//! disappearing device tears down its outputs without hanging (FR-6).
//!
//! Deliberately out of scope here (later tickets): DRM leasing for
//! non-desktop connectors, explicit-sync (`drm_syncobj`), VRR and
//! night-light plumbing (T-16), and suspend soak behavior (T-31 hooks).

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use smithay::backend::allocator::gbm::{GbmAllocator, GbmBufferFlags, GbmDevice};
use smithay::backend::allocator::Fourcc;
use smithay::backend::drm::compositor::{FrameFlags, PrimaryPlaneElement};
use smithay::backend::drm::exporter::gbm::GbmFramebufferExporter;
use smithay::backend::drm::output::{DrmOutput, DrmOutputManager, DrmOutputRenderElements};
use smithay::backend::drm::{
    DrmDevice, DrmDeviceFd, DrmError, DrmEvent, DrmEventMetadata, DrmEventTime, DrmNode, NodeType,
    VrrSupport,
};
use smithay::backend::egl::{context::ContextPriority, EGLDevice, EGLDisplay};
use smithay::backend::input::InputEvent;
use smithay::backend::libinput::{LibinputInputBackend, LibinputSessionInterface};
use smithay::backend::renderer::element::memory::{
    MemoryRenderBuffer, MemoryRenderBufferRenderElement,
};
use smithay::backend::renderer::element::solid::SolidColorRenderElement;
use smithay::backend::renderer::element::surface::WaylandSurfaceRenderElement;
use smithay::backend::renderer::element::utils::{RelocateRenderElement, RescaleRenderElement};
use smithay::backend::renderer::element::{AsRenderElements, Kind};
use smithay::backend::renderer::gles::{
    GlesFrame, GlesRenderer, GlesTexProgram, GlesTexture, Uniform,
};
use smithay::backend::renderer::multigpu::gbm::GbmGlesBackend;
use smithay::backend::renderer::multigpu::{GpuManager, MultiRenderer};
use smithay::backend::renderer::utils::with_renderer_surface_state;
use smithay::backend::renderer::{
    ImportAll, ImportDma, ImportDmaWl, ImportMem, ImportMemWl, RendererSuper, Texture,
};
use smithay::backend::session::libseat::LibSeatSession;
use smithay::backend::session::{Event as SessionEvent, Session};
use smithay::backend::udev::{all_gpus, primary_gpu, UdevBackend, UdevEvent};
use smithay::backend::SwapBuffersError;
use smithay::desktop::space::SpaceRenderElements;
use smithay::desktop::utils::OutputPresentationFeedback;
use smithay::desktop::Window;
use smithay::input::pointer::{CursorIcon, CursorImageAttributes, CursorImageStatus};
use smithay::output::{Mode as WlMode, Output};
use smithay::reexports::calloop::timer::{TimeoutAction, Timer};
use smithay::reexports::calloop::{PostAction, RegistrationToken};
use smithay::reexports::drm::control::{connector, crtc, Device as _, ModeTypeFlags};
use smithay::reexports::input::Libinput;
use smithay::reexports::rustix::fs::OFlags;
use smithay::reexports::wayland_protocols::wp::presentation_time::server::wp_presentation_feedback;
use smithay::reexports::wayland_server::backend::GlobalId;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::render_elements;
use smithay::utils::{
    Buffer, DeviceFd, IsAlive, Logical, Physical, Point, Rectangle, Scale, Time, Transform,
};
use smithay::wayland::compositor::with_states;
use smithay::wayland::presentation::Refresh;
use smithay_drm_extras::drm_scanner::{DrmScanEvent, DrmScanner};

use crate::backend::add_seat_capabilities;
use crate::session::{run_session, BackendHooks};
use crate::window::{
    full_output_material_forced, BackdropBlurElement, MaterialRegion, PanelTextureSlots,
    SceneTextureCache, SceneTextureElement, TextureRenderer,
};
use crate::{input, render, LOCKSTEP_VERSION};

type UdevRenderer<'a> = MultiRenderer<
    'a,
    'a,
    GbmGlesBackend<GlesRenderer, DrmDeviceFd>,
    GbmGlesBackend<GlesRenderer, DrmDeviceFd>,
>;

/// The DRM renderer's `TextureId` is `MultiTexture`, so the nested
/// `BlurRenderer` (which required `TextureId = GlesTexture`) cannot be used.
/// The scene and blur elements draw through this trait instead: `MultiFrame`
/// exposes the underlying `GlesFrame`, which can sample the `GlesTexture`
/// created by `Offscreen<GlesTexture>` on the render GPU.
impl<'a> crate::window::TextureRenderer for UdevRenderer<'a> {
    fn draw_texture_from_to(
        frame: &mut <Self as RendererSuper>::Frame<'_, '_>,
        texture: &GlesTexture,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        program: Option<&GlesTexProgram>,
        uniforms: &[Uniform<'_>],
    ) -> Result<(), Self::Error> {
        let gles_frame: &mut GlesFrame<'_, '_> = frame.as_mut();
        gles_frame
            .render_texture_from_to(
                texture,
                src,
                dst,
                damage,
                &[],
                Transform::Normal,
                1.0,
                program,
                uniforms,
            )
            .map_err(smithay::backend::renderer::multigpu::Error::Render)
    }
}

/// The native two-pass material (P2) is the default. Set this to `0` to
/// disable it for a session and fall back to the feather stack + milky shell
/// plate.
pub const ENV_DRM_MATERIAL: &str = "DRAGONFRUIT_DRM_MATERIAL";

/// The Tahoe liquid-glass compose (P2C) is the default once the material
/// path is active. Set this to `0` to keep the plain Kawase blur (no
/// refraction/specular/tint).
pub const ENV_DRM_GLASS: &str = "DRAGONFRUIT_DRM_GLASS";

/// Read a boolean environment flag that defaults to **on**. `0`, `false`,
/// `off`, `no`, or `disabled` (case-insensitive) turn it off; unset or any
/// other value keeps the default.
fn env_flag_default_true(name: &str) -> bool {
    match std::env::var(name) {
        Ok(value) => !matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "0" | "false" | "off" | "no" | "disabled"
        ),
        Err(_) => true,
    }
}

type GbmAllocatorDrm = GbmAllocator<DrmDeviceFd>;
type GbmExporterDrm = GbmFramebufferExporter<DrmDeviceFd>;

// 8-bit formats until color management (xx-color-management) lands with
// the Displays pane work (T-16).
const SUPPORTED_FORMATS: &[Fourcc] = &[Fourcc::Abgr8888, Fourcc::Argb8888];

render_elements! {
    pub DrmOutputElements<R, E> where R: ImportAll + ImportMem + ImportMemWl + ImportDmaWl
        + ImportDma + TextureRenderer;
    Pointer=PointerRenderElement<R>,
    Space=SpaceRenderElements<R, E>,
    Chrome=WaylandSurfaceRenderElement<R>,
    // SSD titlebars (T-01.1): flat solid fills, above the window space.
    Decoration=SolidColorRenderElement,
    // A window mid-appear: scaled about its target and translated from the
    // Dock tile origin (T-02.1b).
    Appear=RelocateRenderElement<RescaleRenderElement<WaylandSurfaceRenderElement<R>>>,
    // The per-Space wallpaper (T-05.4): behind the windows.
    Wallpaper=crate::render::WallpaperRenderElement<R>,
    // The two-pass material (P2): the scene texture and the per-panel blurred
    // backdrop. Both draw through `TextureRenderer`, so they work with the
    // DRM `MultiRenderer` as well as nested `GlowRenderer`.
    Blur=crate::window::BackdropBlurElement,
    Scene=crate::window::SceneTextureElement,
}

render_elements! {
    pub PointerRenderElement<R> where R: ImportAll + ImportMem + ImportMemWl + ImportDmaWl + ImportDma;
    Surface=WaylandSurfaceRenderElement<R>,
    Memory=MemoryRenderBufferRenderElement<R>,
}
/// The system cursor: a memory buffer rendered with `Kind::Cursor` so the
/// DRM compositor can assign it to a hardware cursor plane. Named shapes come
/// from the desktop's XCursor theme; a client cursor surface still wins.
struct PointerElement {
    buffer: Option<MemoryRenderBuffer>,
    status: CursorImageStatus,
    /// Hotspot in physical pixels, subtracted from the pointer location.
    hotspot: Point<i32, smithay::utils::Physical>,
    /// The named icon and target physical size currently in `buffer`, so a
    /// redraw never re-imports the same cursor texture.
    current: Option<(CursorIcon, u32)>,
    theme: crate::cursor_theme::CursorTheme,
    /// The built-in arrow used when the theme is absent or a shape is missing.
    fallback: MemoryRenderBuffer,
    /// Whether the last texture import failed, so a persistent renderer error
    /// is logged once instead of on every frame. The import itself is retried
    /// each frame, so a transient GPU reset recovers the cursor by itself.
    import_failed: Cell<bool>,
}

impl Default for PointerElement {
    fn default() -> Self {
        Self {
            buffer: Default::default(),
            status: CursorImageStatus::default_named(),
            hotspot: Point::from((0, 0)),
            current: None,
            theme: crate::cursor_theme::CursorTheme::load(),
            fallback: arrow_cursor_buffer(),
            import_failed: Cell::new(false),
        }
    }
}

impl PointerElement {
    /// Select the buffer for `status` at the output scale. A themed named
    /// shape replaces the fallback arrow; a client surface is drawn as-is and
    /// its hotspot is supplied separately from `CursorImageAttributes`.
    fn set_status(&mut self, status: CursorImageStatus, scale: f64) {
        match &status {
            CursorImageStatus::Hidden => {
                self.status = status;
            }
            CursorImageStatus::Surface(_) => {
                self.status = status;
                self.hotspot = Point::from((0, 0));
            }
            CursorImageStatus::Named(icon) => {
                let logical = self.theme.logical_size();
                let integer_scale = scale.max(1.0).ceil() as u32;
                let target = logical.saturating_mul(integer_scale);
                if self.current != Some((*icon, target)) {
                    let image = self.theme.image_for(*icon, scale);
                    if let Some(image) = image {
                        // The theme names images by nominal pixel size. Keeping
                        // the buffer's logical size fixed at `logical` makes
                        // the pointer the same size at every output scale.
                        let buffer_scale =
                            (image.size as f64 / logical as f64).round().max(1.0) as i32;
                        self.buffer = Some(MemoryRenderBuffer::from_slice(
                            &image.pixels_rgba,
                            Fourcc::Argb8888,
                            smithay::utils::Size::from((image.width as i32, image.height as i32)),
                            buffer_scale,
                            Transform::Normal,
                            None,
                        ));
                        // The image is drawn at `logical_size * output_scale`;
                        // when the chosen image is larger than that (a 96px
                        // image at 3x, buffer_scale 4), scale its hotspot
                        // down to the physical rect too.
                        let pixel_scale = scale / buffer_scale as f64;
                        self.hotspot = Point::from((
                            (image.xhot as f64 * pixel_scale).round() as i32,
                            (image.yhot as f64 * pixel_scale).round() as i32,
                        ));
                    } else {
                        self.buffer = Some(self.fallback.clone());
                        self.hotspot = Point::from((0, 0));
                    }
                    self.current = Some((*icon, target));
                }
                self.status = status;
            }
        }
    }

    /// Set the hotspot of a client-provided cursor surface (logical surface
    /// coordinates) in physical pixels at the output scale.
    fn set_surface_hotspot(&mut self, hotspot: Point<i32, Logical>, scale: f64) {
        self.hotspot = Point::from((
            (hotspot.x as f64 * scale).round() as i32,
            (hotspot.y as f64 * scale).round() as i32,
        ));
    }
}

impl<R> AsRenderElements<R> for PointerElement
where
    R: smithay::backend::renderer::Renderer + ImportAll + ImportMem + ImportMemWl + ImportDmaWl,
    R::TextureId: smithay::backend::renderer::Texture + Clone + Send + 'static,
{
    type RenderElement = PointerRenderElement<R>;

    fn render_elements<C: From<Self::RenderElement>>(
        &self,
        renderer: &mut R,
        location: Point<i32, smithay::utils::Physical>,
        scale: Scale<f64>,
        alpha: f32,
    ) -> Vec<C> {
        // The pointer location is the hotspot: move the image so the tip sits
        // exactly on it. A themed cursor's hotspot is in image pixels; a client
        // surface's is in logical surface units scaled above. Clamp at the
        // output origin: a negative cursor-plane position is rejected by some
        // DRM drivers (and a rejected first frame would stall the output).
        let location = Point::from((
            (location.x - self.hotspot.x).max(0),
            (location.y - self.hotspot.y).max(0),
        ));
        match &self.status {
            CursorImageStatus::Hidden => vec![],
            // A named shape always renders the system buffer.
            CursorImageStatus::Named(_) => {
                let Some(buffer) = self.buffer.as_ref() else {
                    return vec![];
                };
                match MemoryRenderBufferRenderElement::from_buffer(
                    renderer,
                    location.to_f64(),
                    buffer,
                    None,
                    None,
                    None,
                    Kind::Cursor,
                ) {
                    Ok(element) => {
                        self.import_failed.set(false);
                        vec![PointerRenderElement::<R>::from(element).into()]
                    }
                    Err(err) => {
                        // A cursor texture import is a best-effort operation:
                        // a transient GPU/context error must not abort the whole
                        // session. Draw no cursor this frame; `from_buffer`
                        // leaves the failed texture uncached, so the next frame
                        // retries.
                        if !self.import_failed.replace(true) {
                            eprintln!("dragonfruit-compositor: cursor import failed: {err:?}");
                        }
                        vec![]
                    }
                }
            }
            CursorImageStatus::Surface(surface) => {
                let elements: Vec<PointerRenderElement<R>> =
                    smithay::backend::renderer::element::surface::render_elements_from_surface_tree(
                        renderer,
                        surface,
                        location,
                        scale,
                        alpha,
                        Kind::Cursor,
                    );
                elements.into_iter().map(C::from).collect()
            }
        }
    }
}

/// Per-output, per-GPU storage for the two-pass material (P2). The scene
/// texture is region-sized and reused across frames; each panel owns its
/// Kawase chain textures. This is the DRM analogue of nested's
/// `MaterialTextureCache`, kept per output so a mode/scale change reallocates
/// only the affected output. The cache is per `SurfaceData`, which belongs to
/// one device, so a texture is never shared across render GPUs.
#[derive(Default)]
struct DrmMaterialCache {
    scene: SceneTextureCache<GlesTexture>,
    panels: PanelTextureSlots<GlesTexture>,
}

impl DrmMaterialCache {
    fn trim_panels(&mut self, count: usize) {
        self.panels.trim(count);
    }
}

/// One display output on one DRM device.
struct SurfaceData {
    dh: smithay::reexports::wayland_server::DisplayHandle,
    output: Output,
    global: GlobalId,
    drm_output:
        DrmOutput<GbmAllocatorDrm, GbmExporterDrm, Option<OutputPresentationFeedback>, DrmDeviceFd>,
    last_presentation_time: Option<Time<smithay::utils::Monotonic>>,
    vblank_throttle_timer: Option<RegistrationToken>,
    /// Reused scene/blur textures for the sampled material path.
    material: DrmMaterialCache,
}

impl Drop for SurfaceData {
    fn drop(&mut self) {
        self.dh
            .remove_global::<crate::state::DfState>(self.global.clone());
    }
}

struct DeviceData {
    render_node: Option<DrmNode>,
    drm_output_manager: DrmOutputManager<
        GbmAllocatorDrm,
        GbmExporterDrm,
        Option<OutputPresentationFeedback>,
        DrmDeviceFd,
    >,
    scanner: DrmScanner,
    surfaces: HashMap<crtc::Handle, SurfaceData>,
}

/// The two custom shader programs compiled on one render GPU. GL programs are
/// context-local, so a multi-GPU seat needs one pair per render node; a single
/// pair used across devices would be invalid.
#[derive(Default)]
struct MaterialPrograms {
    blur: Option<GlesTexProgram>,
    glass: Option<GlesTexProgram>,
}

/// Everything the DRM backend owns; lives in a shared slot that the
/// calloop source callbacks borrow (single-threaded event loop by design —
/// see 02-compositor.md).
struct DrmData {
    session: LibSeatSession,
    seat_name: String,
    primary_gpu: DrmNode,
    gpus: GpuManager<GbmGlesBackend<GlesRenderer, DrmDeviceFd>>,
    devices: HashMap<DrmNode, DeviceData>,
    pointer_element: PointerElement,
    libinput: Option<Libinput>,
    /// Compiled lazily per render node; `None` keeps the built-in bilinear
    /// downsample / plain blurred compose.
    programs: HashMap<DrmNode, MaterialPrograms>,
}

type SharedData = Rc<RefCell<Option<DrmData>>>;

pub fn run(socket_name: &str) -> Result<(), String> {
    println!("dragonfruit-compositor: starting (backend=drm, lockstep-ipc=v{LOCKSTEP_VERSION})");

    let shared: SharedData = Rc::new(RefCell::new(None));
    set_drm_shared(shared.clone());

    let shared_init = shared.clone();

    run_session(
        socket_name,
        BackendHooks {
            init: Box::new(move |state| init(state, shared_init)),
            // Render on demand: commits, input, animation ticks, and pending
            // frame callbacks set `needs_redraw`. The old path probed every
            // output at the refresh rate even when nothing had changed; at 4K
            // that made an empty damage check cost enough CPU to cap clients
            // around 15 fps.
            render: Box::new(render_pending),
        },
    )
}

/// Render every output that has new work this loop iteration.
///
/// The session loop calls this after dispatching Wayland events, so a client
/// commit (or an input/animation request) is serviced immediately. The DRM
/// backend no longer runs a free-running probe timer: an idle desktop costs
/// nothing, while an animating client is still driven by its commits at the
/// display refresh rate.
fn render_pending(state: &mut crate::state::DfState) -> Result<(), String> {
    if state.suspend.is_suspended() {
        return Ok(());
    }
    // A global scene change or a pending client callback is urgent for every
    // output; an animation request is not (T-21.2b). The scheduler's own
    // requests are checked per output inside `render_surface`.
    if state.has_pending_frame_callbacks() {
        state.needs_redraw = true;
    }
    if !state.needs_redraw && !state.frame_scheduler.has_animation() {
        return Ok(());
    }
    let Some(shared) = drm_shared_slot() else {
        return Ok(());
    };
    let targets: Vec<(DrmNode, crtc::Handle)> = {
        let guard = shared.borrow();
        guard
            .as_ref()
            .map(|data| {
                data.devices
                    .iter()
                    .flat_map(|(node, device)| device.surfaces.keys().map(|crtc| (*node, *crtc)))
                    .collect()
            })
            .unwrap_or_default()
    };
    for (node, crtc) in targets {
        render_surface(state, &shared, node, crtc, false);
    }
    state.needs_redraw = false;
    Ok(())
}

fn init(state: &mut crate::state::DfState, shared: SharedData) -> Result<(), String> {
    // The native scene-texture + blur path is the default. A user (or a
    // diagnostic run) can set `DRAGONFRUIT_DRM_MATERIAL=0` to fall back to the
    // feather stack and the shell's milky plate.
    state.material_blur_supported = env_flag_default_true(ENV_DRM_MATERIAL);
    // Session: logind/libseat takes the VT and device access. On a host with
    // no *free* seat (a workstation already running a desktop) this is where
    // bring-up stops: report it as OPEN — the T-03.2 unit is marked open, not
    // skipped — rather than as an opaque failure.
    let (session, notifier) = match LibSeatSession::new() {
        Ok(pair) => pair,
        Err(err) => {
            let outcome = dragonfruit_compositor::drm_bringup::DrmBringup::classify(
                Err(err.to_string()),
                None,
                0,
            );
            println!("dragonfruit-compositor: {}", outcome.marker());
            return Err(format!(
                "DRM bring-up failed: failed to acquire a DRM session (logind/libseat): {err}"
            ));
        }
    };
    let seat_name = session.seat();

    // Primary GPU: explicit override, else the seat's primary render node,
    // else the first GPU that opens.
    let primary_gpu = match std::env::var("DF_DRM_DEVICE") {
        Ok(path) => DrmNode::from_path(path.as_str())
            .map_err(|e| format!("invalid DF_DRM_DEVICE {path:?}: {e}"))?,
        Err(_) => {
            let resolved = primary_gpu(&seat_name)
                .map_err(|e| format!("failed to scan DRM devices: {e}"))?
                .and_then(|path| {
                    DrmNode::from_path(&path)
                        .ok()
                        .and_then(|node| node.node_with_type(NodeType::Render).and_then(|n| n.ok()))
                })
                .or_else(|| {
                    all_gpus(&seat_name)
                        .ok()?
                        .into_iter()
                        .find_map(|path| DrmNode::from_path(&path).ok())
                });
            match resolved {
                Some(node) => node,
                None => {
                    let outcome =
                        dragonfruit_compositor::drm_bringup::DrmBringup::classify(Ok(()), None, 0);
                    println!("dragonfruit-compositor: {}", outcome.marker());
                    return Err("DRM bring-up failed: no usable DRM GPU on this seat".into());
                }
            }
        }
    };
    println!("dragonfruit-compositor: primary gpu: {primary_gpu:?}");

    let gpus = GpuManager::new(GbmGlesBackend::with_context_priority(ContextPriority::High))
        .map_err(|e| format!("failed to init gpu manager: {e}"))?;

    // libinput on the session seat (context kept for suspend/resume).
    let mut libinput_context =
        Libinput::new_with_udev::<LibinputSessionInterface<LibSeatSession>>(session.clone().into());
    libinput_context
        .udev_assign_seat(&seat_name)
        .map_err(|e| format!("failed to assign seat {seat_name:?} to libinput: {e:?}"))?;
    let libinput_backend = LibinputInputBackend::new(libinput_context.clone());

    let mut data = DrmData {
        session,
        seat_name,
        primary_gpu,
        gpus,
        devices: HashMap::new(),
        pointer_element: PointerElement::default(),
        libinput: Some(libinput_context),
        programs: HashMap::new(),
    };

    // Seat capabilities for all backends (FR-1).
    add_seat_capabilities(state);

    // Initial device scan happens before any source can render.
    let udev_backend = UdevBackend::new(&data.seat_name)
        .map_err(|e| format!("failed to init udev backend: {e}"))?;

    {
        let primary_node = data
            .primary_gpu
            .node_with_type(NodeType::Primary)
            .and_then(|node| node.ok());
        let mut paths: Vec<_> = udev_backend.device_list().collect();
        paths.sort_by_key(|(id, _)| {
            primary_node
                .as_ref()
                .map(|p| (*id != p.dev_id()) as u8)
                .unwrap_or(0)
        });
        for (device_id, path) in paths {
            if let Ok(node) = DrmNode::from_dev_id(device_id) {
                if let Err(err) = device_added(state, &mut data, node, path) {
                    eprintln!("dragonfruit-compositor: skipping device {device_id}: {err}");
                }
            }
        }
    }

    // First bring-up checkpoint (T-03.2): a session and a device are not
    // enough — with no connected output there is nothing to composite to.
    // When another process holds DRM master (the host desktop) every
    // connector fails to initialize, so this is where a *non-skipped* open
    // is recorded. Fail rather than run silently with no display.
    let output_count: usize = data
        .devices
        .values()
        .map(|device| device.surfaces.len())
        .sum();
    let outcome = dragonfruit_compositor::drm_bringup::DrmBringup::classify(
        Ok(()),
        Some(data.primary_gpu.dev_id().to_string()),
        output_count,
    );
    println!("dragonfruit-compositor: {}", outcome.marker());
    if !outcome.is_ready() {
        return Err("DRM bring-up failed: no connected output".into());
    }

    // The first native material slice is single-GPU: `single_renderer` gives
    // a render-only `MultiRenderer` (target None), so the scene texture and
    // blur never cross devices. A multi-GPU seat keeps the feather path and
    // the shell fallback until the copy/import route is implemented.
    if state.material_blur_supported && data.devices.len() != 1 {
        state.material_blur_supported = false;
        eprintln!(
            "dragonfruit-compositor: multi-GPU seat: sampled material disabled; \
             using the feather stack"
        );
    }

    // Multi-GPU import/fallback classification (T-03.4): one greppable
    // marker. The primary device is listed first; every other device either
    // renders locally (its buffers are imported) or falls back to the primary
    // renderer with linear-only import.
    {
        let mut gpus: Vec<(DrmNode, Option<DrmNode>)> = data
            .devices
            .iter()
            .map(|(node, device)| (*node, device.render_node))
            .collect();
        gpus.sort_by_key(|(_, render)| {
            u8::from(render.map(|r| r != data.primary_gpu).unwrap_or(true))
        });
        let nodes: Vec<dragonfruit_compositor::multi_gpu::GpuNode> = gpus
            .iter()
            .map(
                |(node, render)| dragonfruit_compositor::multi_gpu::GpuNode {
                    name: node.dev_id().to_string(),
                    render_node: render.map(|r| r.dev_id().to_string()),
                },
            )
            .collect();
        println!(
            "dragonfruit-compositor: {}",
            dragonfruit_compositor::multi_gpu::MultiGpuOutcome::classify(&nodes).marker()
        );
    }

    // Scene-texture routing (T-20.4): the chrome material's scene texture is
    // owned per GPU. Decide, per output, whether the blur would compose
    // locally on the output's GPU or the primary GPU's scene texture would be
    // copied across — the material never samples across GPUs. The copy route
    // is not executed yet: a multi-GPU seat disables the sampled path above
    // and uses the feather stack. The plan is reported so the decision is
    // observable on hardware.
    {
        let mut outputs: Vec<(String, String)> = data
            .devices
            .values()
            .flat_map(|device| {
                let gpu = device
                    .render_node
                    .unwrap_or(data.primary_gpu)
                    .dev_id()
                    .to_string();
                device
                    .surfaces
                    .values()
                    .map(move |surface| (surface.output.name(), gpu.clone()))
            })
            .collect();
        outputs.sort();
        let primary = data.primary_gpu.dev_id().to_string();
        let plan = dragonfruit_compositor::multi_gpu::plan_scene_routes(Some(&primary), &outputs);
        let marker = dragonfruit_compositor::multi_gpu::scene_routes_marker(&plan);
        if !marker.is_empty() {
            println!("dragonfruit-compositor: {marker}");
        }
    }

    // shm formats + dmabuf feedback from the primary gpu renderer.
    {
        let renderer = data
            .gpus
            .single_renderer(&data.primary_gpu)
            .map_err(|e| format!("failed to get primary renderer: {e:?}"))?;
        state.shm_state.update_formats(renderer.shm_formats());
        let formats = renderer.dmabuf_formats();
        state.init_dmabuf(formats.iter().copied(), Some(data.primary_gpu.dev_id()));
    }

    // Publish the data slot, then register the sources that borrow it.
    *shared.borrow_mut() = Some(data);

    // Kick off the first frames for outputs found during the initial scan.
    {
        let initial: Vec<(DrmNode, crtc::Handle)> = {
            let guard = shared.borrow();
            guard
                .as_ref()
                .map(|data| {
                    data.devices
                        .iter()
                        .flat_map(|(node, device)| {
                            device.surfaces.keys().map(|crtc| (*node, *crtc))
                        })
                        .collect()
                })
                .unwrap_or_default()
        };
        for (node, crtc) in initial {
            let shared = shared.clone();
            let _ = state.loop_handle.insert_idle(move |state| {
                render_surface(state, &shared, node, crtc, true);
            });
        }
    }

    // libinput events → shared input router (same code as nested; FR-1).
    {
        let shared = shared.clone();
        state
            .loop_handle
            .insert_source(libinput_backend, move |event, _, state| {
                if let InputEvent::DeviceAdded { device } = &event {
                    let _ = device
                        .has_capability(smithay::reexports::input::DeviceCapability::Keyboard);
                }
                let _ = &shared;
                input::process_input_event(state, event);
            })
            .map_err(|e| format!("failed to register libinput source: {e}"))?;
    }

    // Session pause/resume (FR-6): deactivate everything, then re-activate
    // and re-render on resume.
    {
        let shared = shared.clone();
        state
            .loop_handle
            .insert_source(notifier, move |event, _, state| match event {
                SessionEvent::PauseSession => {
                    // A VT switch-away and a sleep both quiesce the session
                    // (T-12.5a): no frames, no user input, clients untouched.
                    state.suspend_session();
                    let mut guard = shared.borrow_mut();
                    let Some(data) = guard.as_mut() else { return };
                    if let Some(libinput) = data.libinput.as_mut() {
                        libinput.suspend();
                    }
                    // Pause the DRM device, but keep every SurfaceData alive.
                    // Clearing `surfaces` destroys the wl_output globals and
                    // the DrmOutputs; on resume there is nothing left to
                    // activate or repaint, which is the black screen after a
                    // VT switch/suspend. Sessions are quiesced, not torn down.
                    for device in data.devices.values_mut() {
                        device.drm_output_manager.pause();
                    }
                }
                SessionEvent::ActivateSession => {
                    // Wake: reactivate every device, re-scan connectors (the
                    // user may have docked/undocked while asleep), then repaint
                    // every surviving output.
                    state.resume_session();
                    let mut guard = shared.borrow_mut();
                    let Some(data) = guard.as_mut() else { return };
                    if let Some(libinput) = data.libinput.as_mut() {
                        if let Err(err) = libinput.resume() {
                            eprintln!("dragonfruit-compositor: failed to resume libinput: {err:?}");
                        }
                    }
                    let nodes: Vec<DrmNode> = data.devices.keys().copied().collect();
                    for node in &nodes {
                        if let Some(device) = data.devices.get_mut(node) {
                            // Optimistic re-activation: if connectors were
                            // re-bound by a foreign master, the queued render
                            // resets the state instead (see render_surface).
                            if let Err(err) = device.drm_output_manager.activate(false) {
                                eprintln!(
                                    "dragonfruit-compositor: failed to reactivate {node:?}: {err}"
                                );
                            }
                        }
                    }
                    for node in &nodes {
                        device_changed(state, data, *node);
                    }
                    for (node, device) in data.devices.iter() {
                        let node = *node;
                        for crtc in device.surfaces.keys().copied() {
                            let shared = shared.clone();
                            let _ = state.loop_handle.insert_idle(move |state| {
                                render_surface(state, &shared, node, crtc, true);
                            });
                        }
                    }
                }
            })
            .map_err(|e| format!("failed to register session source: {e}"))?;
    }

    // udev hotplug source.
    {
        let shared = shared.clone();
        state
            .loop_handle
            .insert_source(udev_backend, move |event, _, state| match event {
                UdevEvent::Added { device_id, path } => {
                    let mut guard = shared.borrow_mut();
                    let Some(data) = guard.as_mut() else { return };
                    if let Ok(node) = DrmNode::from_dev_id(device_id) {
                        if let Err(err) = device_added(state, data, node, &path) {
                            eprintln!("dragonfruit-compositor: skipping device {device_id}: {err}");
                        }
                    }
                }
                UdevEvent::Changed { device_id } => {
                    let mut guard = shared.borrow_mut();
                    let Some(data) = guard.as_mut() else { return };
                    if let Ok(node) = DrmNode::from_dev_id(device_id) {
                        device_changed(state, data, node);
                    }
                }
                UdevEvent::Removed { device_id } => {
                    let mut guard = shared.borrow_mut();
                    let Some(data) = guard.as_mut() else { return };
                    if let Ok(node) = DrmNode::from_dev_id(device_id) {
                        device_removed(state, data, node);
                    }
                }
            })
            .map_err(|e| format!("failed to register udev source: {e}"))?;
    }

    Ok(())
}

/// Open a DRM device, register its vblank source, and scan connectors.
fn device_added(
    state: &mut crate::state::DfState,
    data: &mut DrmData,
    node: DrmNode,
    path: &std::path::Path,
) -> Result<(), String> {
    let fd = data
        .session
        .open(
            path,
            OFlags::RDWR | OFlags::CLOEXEC | OFlags::NOCTTY | OFlags::NONBLOCK,
        )
        .map_err(|e| format!("failed to open device via session: {e:?}"))?;
    let fd = DrmDeviceFd::new(DeviceFd::from(fd));

    let (drm, notifier) =
        DrmDevice::new(fd.clone(), true).map_err(|e| format!("failed to init drm device: {e}"))?;
    let gbm = GbmDevice::new(fd).map_err(|e| format!("failed to init gbm device: {e}"))?;

    // Per-device vblank/error events drive frame submission (see
    // frame_finish). The DrmData slot must already be published.
    state
        .loop_handle
        .insert_source(notifier, move |event, metadata, state| match event {
            DrmEvent::VBlank(crtc) => frame_finish(state, node, crtc, metadata),
            DrmEvent::Error(error) => {
                eprintln!("dragonfruit-compositor: drm event error: {error:?}");
            }
        })
        .map_err(|e| format!("failed to register drm event source: {e}"))?;

    // EGL/GBM renderer node for this device.
    let mut try_initialize_gpu = || -> Result<DrmNode, String> {
        // Safety: the GBM device owns the DRM fd and outlives the display.
        let display =
            unsafe { EGLDisplay::new(gbm.clone()) }.map_err(|e| format!("egl display: {e}"))?;
        let egl_device =
            EGLDevice::device_for_display(&display).map_err(|e| format!("egl device: {e}"))?;
        if egl_device.is_software() {
            return Err("software rendering device".to_string());
        }
        let render_node = egl_device
            .try_get_render_node()
            .ok()
            .flatten()
            .unwrap_or(node);
        data.gpus
            .as_mut()
            .add_node(render_node, gbm.clone())
            .map_err(|e| format!("failed to add gpu node: {e}"))?;
        Ok(render_node)
    };

    let render_node = try_initialize_gpu()
        .inspect_err(|err| eprintln!("dragonfruit-compositor: failed to init gpu: {err}"))
        .ok();

    let allocator = render_node
        .is_some()
        .then(|| {
            GbmAllocator::new(
                gbm.clone(),
                GbmBufferFlags::RENDERING | GbmBufferFlags::SCANOUT,
            )
        })
        .ok_or_else(|| "no render node and no primary gpu fallback".to_string())?;

    let framebuffer_exporter = GbmFramebufferExporter::new(gbm.clone(), render_node);

    let mut renderer = data
        .gpus
        .single_renderer(&render_node.unwrap_or(data.primary_gpu))
        .map_err(|e| format!("failed to get renderer: {e:?}"))?;
    let render_formats: Vec<_> = renderer
        .as_mut()
        .egl_context()
        .dmabuf_render_formats()
        .iter()
        .filter(|format| {
            dragonfruit_compositor::multi_gpu::format_allowed(
                render_node.is_some(),
                format.modifier == smithay::backend::allocator::Modifier::Linear,
            )
        })
        .copied()
        .collect();

    let drm_output_manager = DrmOutputManager::new(
        drm,
        allocator,
        framebuffer_exporter,
        Some(gbm),
        SUPPORTED_FORMATS.iter().copied(),
        render_formats,
    );

    data.devices.insert(
        node,
        DeviceData {
            render_node,
            drm_output_manager,
            scanner: DrmScanner::new(),
            surfaces: HashMap::new(),
        },
    );

    device_changed(state, data, node);
    Ok(())
}

/// Re-scan connectors of a device; connect/disconnect outputs as needed.
fn device_changed(state: &mut crate::state::DfState, data: &mut DrmData, node: DrmNode) {
    let Some(device) = data.devices.get_mut(&node) else {
        return;
    };

    let scan_result = match device
        .scanner
        .scan_connectors(device.drm_output_manager.device())
    {
        Ok(scan_result) => scan_result,
        Err(err) => {
            eprintln!("dragonfruit-compositor: failed to scan connectors: {err:?}");
            return;
        }
    };

    for event in scan_result {
        match event {
            DrmScanEvent::Connected {
                connector,
                crtc: Some(crtc),
            } => connector_connected(state, data, node, connector, crtc),
            DrmScanEvent::Disconnected {
                connector,
                crtc: Some(crtc),
            } => connector_disconnected(state, data, node, connector, crtc),
            _ => {}
        }
    }
}

fn connector_connected(
    state: &mut crate::state::DfState,
    data: &mut DrmData,
    node: DrmNode,
    connector: connector::Info,
    crtc: crtc::Handle,
) {
    let Some(device) = data.devices.get_mut(&node) else {
        return;
    };

    let render_node = device.render_node.unwrap_or(data.primary_gpu);
    let mut renderer = match data.gpus.single_renderer(&render_node) {
        Ok(renderer) => renderer,
        Err(err) => {
            eprintln!("dragonfruit-compositor: failed to get renderer for connector: {err:?}");
            return;
        }
    };

    let output_name = format!(
        "{}-{}",
        connector.interface().as_str(),
        connector.interface_id()
    );

    let drm_device = device.drm_output_manager.device();

    // Non-desktop connectors (VR headsets etc.) are skipped until DRM
    // leasing lands.
    let non_desktop = drm_device
        .get_properties(connector.handle())
        .ok()
        .and_then(|props| {
            props
                .into_iter()
                .filter_map(|(handle, value)| {
                    let info = drm_device.get_property(handle).ok()?;
                    Some((info, value))
                })
                .find(|(info, _)| info.name().to_str() == Ok("non-desktop"))
                .and_then(|(info, value)| info.value_type().convert_value(value).as_boolean())
        })
        .unwrap_or(false);
    if non_desktop {
        println!("dragonfruit-compositor: skipping non-desktop connector {output_name}");
        return;
    }

    let Some(drm_mode) = connector
        .modes()
        .iter()
        .find(|mode| mode.mode_type().contains(ModeTypeFlags::PREFERRED))
        .or_else(|| connector.modes().first())
        .copied()
    else {
        // A connected connector with no advertised mode is unusable (some
        // drivers report this transiently during hotplug). Skip it instead of
        // indexing an empty slice.
        eprintln!("dragonfruit-compositor: skipping connector {output_name}: no modes advertised");
        return;
    };
    let wl_mode = WlMode::from(drm_mode);

    let (phys_w, phys_h) = connector.size().unwrap_or((0, 0));
    let position = (
        state
            .space
            .outputs()
            .filter_map(|o| state.space.output_geometry(o))
            .fold(0, |acc, geo| acc + geo.size.w),
        0,
    );

    let output = crate::backend::add_output(
        state,
        &output_name,
        smithay::output::PhysicalProperties {
            size: (phys_w as i32, phys_h as i32).into(),
            subpixel: connector.subpixel().into(),
            make: "Unknown".into(),
            model: "Unknown".into(),
        },
        wl_mode,
        position,
        1.0,
    );

    let planes = match drm_device.planes(&crtc) {
        Ok(planes) => planes,
        Err(err) => {
            eprintln!("dragonfruit-compositor: failed to query crtc planes: {err:?}");
            return;
        }
    };

    let drm_output = match device
        .drm_output_manager
        .initialize_output::<_, DrmOutputElements<UdevRenderer<'_>, WaylandSurfaceRenderElement<UdevRenderer<'_>>>>(
            crtc,
            drm_mode,
            &[connector.handle()],
            &output,
            Some(planes),
            &mut renderer,
            &DrmOutputRenderElements::default(),
        ) {
        Ok(drm_output) => drm_output,
        Err(err) => {
            eprintln!("dragonfruit-compositor: failed to init drm output {output_name}: {err:?}");
            return;
        }
    };

    // VRR (T188c): the DRM connector is the only backend that can expose a
    // variable presentation interval. `Supported` means the property can be
    // toggled without a modeset, so enable it now; `RequiresModeset` and
    // `NotSupported` keep the fixed mode period. The presentation clock only
    // adapts when the state is `On` and a range is installed, and it never
    // infers `On` from a measured interval.
    let vrr = drm_output.with_compositor(|compositor| {
        match compositor.vrr_supported(connector.handle()) {
            Ok(VrrSupport::Supported) => match compositor.use_vrr(true) {
                Ok(()) => dragonfruit_compositor::refresh::VrrState::On,
                Err(err) => {
                    eprintln!(
                        "dragonfruit-compositor: VRR enable failed on {output_name}: {err:?}; \
                         keeping the fixed period"
                    );
                    dragonfruit_compositor::refresh::VrrState::Off
                }
            },
            Ok(VrrSupport::RequiresModeset) => {
                eprintln!(
                    "dragonfruit-compositor: VRR on {output_name} requires a modeset; \
                     keeping the fixed period"
                );
                dragonfruit_compositor::refresh::VrrState::Off
            }
            Ok(VrrSupport::NotSupported) => dragonfruit_compositor::refresh::VrrState::Unknown,
            Err(err) => {
                eprintln!(
                    "dragonfruit-compositor: VRR capability query failed on {output_name}: \
                     {err:?}; reporting vrr=unsupported"
                );
                dragonfruit_compositor::refresh::VrrState::Unknown
            }
        }
    });

    let global = output.create_global::<crate::state::DfState>(&state.display_handle);

    device.surfaces.insert(
        crtc,
        SurfaceData {
            dh: state.display_handle.clone(),
            output,
            global,
            drm_output,
            last_presentation_time: None,
            vblank_throttle_timer: None,
            material: DrmMaterialCache::default(),
        },
    );
    state.set_output_vrr(&output_name, vrr);

    // Kick off the first frame.
    let shared = drm_shared_slot();
    if let Some(shared) = shared {
        let _ = state.loop_handle.insert_idle(move |state| {
            render_surface(state, &shared, node, crtc, true);
        });
    }
}

fn connector_disconnected(
    state: &mut crate::state::DfState,
    data: &mut DrmData,
    node: DrmNode,
    _connector: connector::Info,
    crtc: crtc::Handle,
) {
    let Some(device) = data.devices.get_mut(&node) else {
        return;
    };
    if let Some(surface) = device.surfaces.remove(&crtc) {
        // Migrate this output's windows to the remaining primary output's
        // current Space before its Spaces are destroyed (T-05 FR-7).
        state.on_output_removed(&surface.output);
        state.space.unmap_output(&surface.output);
    }
}

fn device_removed(state: &mut crate::state::DfState, data: &mut DrmData, node: DrmNode) {
    // FR-6: a GPU disappearing must degrade gracefully — outputs go away,
    // the remaining GPUs keep running; never a hang.
    if let Some(mut device) = data.devices.remove(&node) {
        for surface in device.surfaces.values() {
            state.on_output_removed(&surface.output);
            state.space.unmap_output(&surface.output);
        }
        device.surfaces.clear();
        if let Some(render_node) = device.render_node {
            data.gpus.as_mut().remove_node(&render_node);
        }
    }
}

/// Submit the pending frame after vblank and schedule the next repaint.
fn frame_finish(
    state: &mut crate::state::DfState,
    dev_id: DrmNode,
    crtc: crtc::Handle,
    metadata: &mut Option<DrmEventMetadata>,
) {
    let Some(shared) = drm_shared_slot() else {
        return;
    };
    let mut guard = shared.borrow_mut();
    let Some(data) = guard.as_mut() else { return };

    let frame_duration = {
        let Some(device) = data.devices.get(&dev_id) else {
            return;
        };
        device
            .surfaces
            .get(&crtc)
            .and_then(|surface| surface.output.current_mode())
            .map(|mode| Duration::from_secs_f64(1_000f64 / mode.refresh as f64))
    };
    let Some(frame_duration) = frame_duration else {
        return;
    };

    // Cancel any pending vblank throttle timer for this surface.
    {
        let Some(device) = data.devices.get_mut(&dev_id) else {
            return;
        };
        if let Some(surface) = device.surfaces.get_mut(&crtc) {
            if let Some(token) = surface.vblank_throttle_timer.take() {
                state.loop_handle.remove(token);
            }
        }
    }

    let tp = metadata.as_ref().and_then(|metadata| match metadata.time {
        DrmEventTime::Monotonic(tp) => (!tp.is_zero()).then_some(tp),
        DrmEventTime::Realtime(_) => None,
    });
    let seq = metadata
        .as_ref()
        .map(|metadata| metadata.sequence)
        .unwrap_or(0);

    let (clock, flags) = if let Some(tp) = tp {
        (
            tp.into(),
            wp_presentation_feedback::Kind::Vsync
                | wp_presentation_feedback::Kind::HwClock
                | wp_presentation_feedback::Kind::HwCompletion,
        )
    } else {
        (state.clock.now(), wp_presentation_feedback::Kind::Vsync)
    };

    // Guard against displays reporting vblank faster than the mode
    // implies (broken timestamps): throttle to the next expected vblank.
    // Capture the actual interval before overwriting the timestamp: it is
    // recorded only if `frame_submitted` confirms a queued frame below.
    let (presentation_interval, output_name) = {
        let Some(device) = data.devices.get_mut(&dev_id) else {
            return;
        };
        let Some(surface) = device.surfaces.get_mut(&crtc) else {
            return;
        };
        if let Some(last) = surface.last_presentation_time {
            let remaining = frame_duration.saturating_sub(Time::elapsed(&last, clock));
            if remaining > frame_duration / 2 {
                let throttled_metadata = DrmEventMetadata {
                    sequence: seq,
                    time: DrmEventTime::Monotonic(Duration::from(clock) + remaining),
                };
                match state.loop_handle.insert_source(
                    Timer::from_duration(remaining),
                    move |_, _, state| {
                        frame_finish(state, dev_id, crtc, &mut Some(throttled_metadata));
                        TimeoutAction::Drop
                    },
                ) {
                    Ok(timer_token) => surface.vblank_throttle_timer = Some(timer_token),
                    Err(err) => {
                        // The loop is shutting down; there is no frame to
                        // throttle. Never panic from the vblank path.
                        eprintln!("dragonfruit-compositor: vblank throttle unavailable: {err}");
                        return;
                    }
                }
                return;
            }
        }
        let interval = surface
            .last_presentation_time
            .map(|last| Time::elapsed(&last, clock));
        let output_name = surface.output.name();
        // The vblank boundary has been reached (or the queued frame was
        // rejected); this output may accept its next animation frame again.
        state.frame_scheduler.clear_in_flight(&output_name);
        surface.last_presentation_time = Some(clock);
        (interval, output_name)
    };

    let submit_result = {
        let Some(device) = data.devices.get(&dev_id) else {
            return;
        };
        let Some(surface) = device.surfaces.get(&crtc) else {
            return;
        };
        surface
            .drm_output
            .frame_submitted()
            .map_err(Into::<SwapBuffersError>::into)
    };

    let retry = match submit_result {
        Ok(user_data) => {
            // `Some` means a queued frame reached this vblank. Record the
            // interval and the per-output presentation clock only then; an
            // idle vblank is not a presentation. `clock` is the kernel's
            // monotonic vblank timestamp when the event carried one, or the
            // compositor clock fallback above.
            if user_data.is_some() {
                state.note_presentation(&output_name, Duration::from(clock));
                if let Some(interval) = presentation_interval {
                    let refresh = state.refresh_for_output(&output_name);
                    state
                        .stats
                        .record_presentation(&output_name, interval, refresh);
                }
            }
            if let Some(mut feedback) = user_data.flatten() {
                feedback.presented(clock, Refresh::fixed(frame_duration), seq as u64, flags);
            }
            false
        }
        Err(err) => {
            eprintln!("dragonfruit-compositor: error during rendering: {err:?}");
            match err {
                SwapBuffersError::AlreadySwapped => true,
                SwapBuffersError::TemporaryFailure(err)
                    if matches!(
                        err.downcast_ref::<DrmError>(),
                        Some(&DrmError::DeviceInactive)
                    ) =>
                {
                    // Session paused; resume re-queues rendering.
                    false
                }
                SwapBuffersError::TemporaryFailure(err) => {
                    // Every temporary failure (not just EACCES/master loss)
                    // must keep the frame clock alive. Anything else leaves
                    // the output with no pending frame and looks like a
                    // frozen desktop until the next input re-arms it.
                    eprintln!("dragonfruit-compositor: transient swap failure: {err}");
                    true
                }
                SwapBuffersError::ContextLost(err) => {
                    eprintln!("dragonfruit-compositor: rendering loop lost: {err}");
                    state.running = false;
                    false
                }
            }
        }
    };

    if retry {
        // Retry a rejected/transient frame on the next refresh boundary.
        // Successful frames are no longer followed by a free-running probe;
        // the next frame comes from a client commit, input, or an animation
        // tick (see `render_pending`).
        let _ = state.loop_handle.insert_source(
            Timer::from_duration(frame_duration),
            move |_, _, state| {
                let shared = drm_shared_slot();
                if let Some(shared) = shared {
                    render_surface(state, &shared, dev_id, crtc, true);
                }
                TimeoutAction::Drop
            },
        );
    }
    // Re-arm the animation clock from the updated per-output deadlines. A
    // presentation already rescheduled it; this also covers an idle vblank
    // that cleared an in-flight marker without presenting a new frame.
    crate::input::schedule_animation_timer(state);
}

/// Render one DRM output: build elements from the scene and hand them to
/// the DrmCompositor (direct scanout when possible, FR-5).
fn render_surface(
    state: &mut crate::state::DfState,
    shared: &SharedData,
    node: DrmNode,
    crtc: crtc::Handle,
    force: bool,
) {
    // A suspended session (T-12.5a) draws nothing; the output surfaces and
    // clients are kept, so wake is a repaint rather than a re-init.
    if state.suspend.is_suspended() {
        return;
    }
    let mut guard = shared.borrow_mut();
    let Some(data) = guard.as_mut() else { return };

    let Some(device) = data.devices.get_mut(&node) else {
        return;
    };
    let render_node = device.render_node.unwrap_or(data.primary_gpu);
    let Some(surface) = device.surfaces.get_mut(&crtc) else {
        return;
    };

    // Per-output frame requests (T-21.2b): urgent work presents immediately,
    // animation work waits for this output's own deadline, and a frame already
    // queued on this output is not queued again. `force` is only for the
    // initial/resume/retry paths that are not scheduler requests.
    let output_name = surface.output.name();
    let now = Duration::from(state.clock.now());
    if !force && !state.output_should_render(&output_name, now) {
        return;
    }
    state.note_output_frame(&output_name);

    // The refresh rate is captured before the render result is handled: the
    // empty/rejected-frame reschedule runs while this `RefCell` borrow is live.
    let refresh = surface
        .output
        .current_mode()
        .map(|mode| mode.refresh)
        .unwrap_or(60_000);

    let mut renderer = match data.gpus.single_renderer(&render_node) {
        Ok(renderer) => renderer,
        Err(err) => {
            eprintln!("dragonfruit-compositor: failed to get renderer: {err:?}");
            // A renderer failure must not leave the output with no pending
            // frame: retry on the frame clock instead of stalling forever.
            reschedule_probe(state, node, crtc, refresh);
            return;
        }
    };

    let scale = Scale::from(surface.output.current_scale().fractional_scale());

    // Open this output's material pass (T-04.2), so the chrome backdrop is
    // applied at most once for it this frame.
    state.begin_render_frame();

    // T-13.3b: capture is implemented on the nested (GL) backend only; the
    // DRM rail is untested hardware, so fail a requested capture explicitly
    // rather than leaving the shell's presenter waiting.
    if state.pending_capture.is_some() {
        let _ = state.take_capture();
        state.broadcast_screenshot_failed("the DRM backend does not implement capture yet");
    }

    // The pointer: rendered with Kind::Cursor so DrmCompositor can assign
    // a hardware cursor plane; cursor motion never waits on effects or
    // damage (02-compositor.md).
    let mut custom_elements: Vec<
        DrmOutputElements<UdevRenderer<'_>, WaylandSurfaceRenderElement<UdevRenderer<'_>>>,
    > = Vec::new();
    // A locked session composites only the lock surface: prepended so it sits
    // above the cursor, the window space, and all chrome (T-12.3a).
    custom_elements.extend(crate::lock::lock_render_elements::<
        _,
        DrmOutputElements<UdevRenderer<'_>, WaylandSurfaceRenderElement<UdevRenderer<'_>>>,
    >(&mut renderer, state, &surface.output, scale));
    let pointer_location = state
        .seat
        .get_pointer()
        .map(|p| p.current_location())
        .unwrap_or_else(|| Point::from((-1.0, -1.0)));
    let output_geo = state
        .space
        .output_geometry(&surface.output)
        .unwrap_or_default();
    if output_geo.to_f64().contains(pointer_location) {
        // Reset the cursor if the client surface is gone.
        if let CursorImageStatus::Surface(ref cursor_surface) = state.cursor_image {
            if !cursor_surface.alive() {
                state.cursor_image = CursorImageStatus::default_named();
            }
        }
        // A live cursor surface with no committed buffer draws nothing. That
        // is normally transient (the client sets the surface, then commits),
        // but a client that stalls or tears down its buffer can leave it stale
        // and the pointer would stay invisible indefinitely. Use the themed
        // default for this frame without discarding the client's surface: the
        // client keeps the role, and the next commit is drawn again.
        let status = effective_cursor_status(&state.cursor_image);
        data.pointer_element.set_status(status.clone(), scale.x);
        if let CursorImageStatus::Surface(ref surface) = status {
            let hotspot = cursor_hotspot(surface);
            data.pointer_element.set_surface_hotspot(hotspot, scale.x);
        }
        let cursor_pos = pointer_location - output_geo.loc.to_f64();
        custom_elements.extend(data.pointer_element.render_elements::<DrmOutputElements<
            UdevRenderer<'_>,
            WaylandSurfaceRenderElement<UdevRenderer<'_>>,
        >>(
            &mut renderer,
            cursor_pos.to_physical_precise_round(scale),
            scale,
            1.0,
        ));
    }

    // `DrmCompositor::render_frame` consumes this list front-to-back, so the
    // order below is the desktop's stack order, top first. Chrome (the menu
    // bar, the Dock, and transient overlays such as the Applications drawer)
    // must precede the window space: with windows first, every normal window
    // painted over the drawer (the native z-order failure).
    //
    // Chrome surfaces (menu bar, drawer, overlays) composite above the window
    // space (T-09).
    custom_elements.extend(crate::render::chrome_render_elements::<
        _,
        DrmOutputElements<UdevRenderer<'_>, WaylandSurfaceRenderElement<UdevRenderer<'_>>>,
    >(&mut renderer, state, &surface.output, scale));

    // The material path (P2): render the scene into an offscreen texture and
    // compose the chrome above the per-panel blur when any role's effective
    // state keeps the sampled pass active. Persistent chrome holds the
    // Reduced floor (T-21.2c), so a global Minimal selection no longer turns
    // the menu bar and Dock into a flat plate. `chrome_blur_panels` returns
    // empty when no role blurs (software, no sampled pass, or all-transient
    // Minimal), and the caller keeps the direct window/desktop elements.
    // The scene texture and the panel SDF are output-local and currently
    // rendered with `Transform::Normal`; a rotated output keeps the feather
    // path until the transform mapping is verified.
    let blur_panels =
        if !state.lock.is_locked() && surface.output.current_transform() == Transform::Normal {
            crate::render::chrome_blur_panels(state, &surface.output)
        } else {
            Vec::new()
        };
    let used_material = !blur_panels.is_empty();

    if !blur_panels.is_empty() {
        let material_output = surface.output.clone();
        let output_local = Rectangle::from_size(output_geo.size);
        let Some(region) = MaterialRegion::for_material(&blur_panels, output_local, scale) else {
            eprintln!(
                "dragonfruit-compositor: DRM material region unavailable for {}",
                material_output.name()
            );
            state.material_blur_supported = false;
            reschedule_probe(state, node, crtc, refresh);
            return;
        };
        let full_output = full_output_material_forced();
        let output_size = material_output
            .current_mode()
            .map(|mode| mode.size)
            .unwrap_or_default();
        // Compile the two-pass programs once per render GPU (P2). A failure is
        // non-fatal: the blur chain uses the built-in bilinear downsample and
        // the glass compose falls back to the plain blurred panel.
        let programs = data.programs.entry(render_node).or_insert_with(|| {
            let gles: &mut GlesRenderer = renderer.as_mut();
            let blur = gles
                .compile_custom_texture_shader(
                    crate::window::BLUR_SHADER,
                    &crate::window::blur_uniform_names(),
                )
                .ok();
            let glass = if env_flag_default_true(ENV_DRM_GLASS) {
                gles.compile_custom_texture_shader(
                    crate::window::GLASS_SHADER,
                    &crate::window::glass_uniform_names(),
                )
                .ok()
            } else {
                None
            };
            MaterialPrograms { blur, glass }
        });
        // Pass one: the live scene (window menu, windows/titlebars/shadows,
        // desktop, wallpaper) into the reusable region-sized texture. The
        // region is the padded chrome union unless the full-output diagnostic
        // switch is set.
        let mut scene_elements: Vec<
            DrmOutputElements<UdevRenderer<'_>, WaylandSurfaceRenderElement<UdevRenderer<'_>>>,
        > = Vec::new();
        scene_elements.extend(
            crate::render::window_menu_render_elements(state, &material_output, scale)
                .into_iter()
                .map(DrmOutputElements::Decoration),
        );
        scene_elements.extend(crate::render::window_scene_render_elements::<
            UdevRenderer<'_>,
            DrmOutputElements<UdevRenderer<'_>, WaylandSurfaceRenderElement<UdevRenderer<'_>>>,
        >(&mut renderer, state, &material_output, scale));
        scene_elements.extend(crate::render::desktop_render_elements::<
            _,
            DrmOutputElements<UdevRenderer<'_>, WaylandSurfaceRenderElement<UdevRenderer<'_>>>,
        >(&mut renderer, state, &material_output, scale));
        scene_elements.extend(
            crate::render::wallpaper_render_elements(&mut renderer, state, &material_output, scale)
                .into_iter()
                .map(DrmOutputElements::Wallpaper),
        );

        let scene_texture = match crate::backend::nested::render_scene_offscreen(
            &mut renderer,
            state,
            &material_output,
            scale,
            region,
            &scene_elements,
            &mut surface.material.scene,
        ) {
            Ok(texture) => texture,
            Err(err) => {
                eprintln!("dragonfruit-compositor: DRM scene pass failed: {err}");
                state.material_blur_supported = false;
                reschedule_probe(state, node, crtc, refresh);
                return;
            }
        };
        let scene_size = scene_texture.size();

        // Pass two: one blurred texture per panel. In the default region
        // shape the live scene elements are the output base; only the
        // full-output diagnostic shape composites the scene texture below it.
        let mut blur_drawn = 0u64;
        let mut downsample_max = 0u32;
        let mut blur_allocations = 0u64;
        for (index, panel) in blur_panels.iter().enumerate() {
            let Some((texture_panel, scene_src)) = region.panel_source(panel.panel, scale) else {
                continue;
            };
            let slot = surface.material.panels.slot(index);
            let (texture, allocations) = match crate::backend::nested::blur_panel_texture(
                &mut renderer,
                programs.blur.as_ref(),
                &scene_texture,
                scene_src,
                &panel.spec,
                output_size,
                slot,
            ) {
                Ok(texture) => texture,
                Err(err) => {
                    eprintln!("dragonfruit-compositor: DRM blur pass failed: {err}");
                    state.material_blur_supported = false;
                    reschedule_probe(state, node, crtc, refresh);
                    return;
                }
            };
            blur_allocations += allocations;
            let texture_size = texture.size();
            let element = BackdropBlurElement::new(
                texture,
                panel.panel,
                texture_size,
                panel.spec.corner_radius.round().max(0.0) as i32,
                scale,
            )
            .with_texture_panel(texture_panel);
            let element = match programs.glass.as_ref() {
                Some(program) => element.with_glass(panel.glass, program.clone()),
                None => element,
            };
            custom_elements.push(DrmOutputElements::Blur(element));
            blur_drawn += 1;
            downsample_max = downsample_max.max(panel.spec.downsample());
        }
        // A disappeared panel releases its Kawase textures (T-21.3 lifetime
        // audit); T190c replaces index keying with a stable panel identity.
        surface.material.trim_panels(blur_panels.len());
        state
            .stats
            .record_material_panel_allocations(blur_allocations);
        if blur_drawn > 0 {
            state.stats.record_blur_draw(downsample_max, blur_drawn);
        }

        if full_output {
            custom_elements.push(DrmOutputElements::Scene(SceneTextureElement::new(
                scene_texture,
                Rectangle::from_size(output_size),
                scene_size,
            )));
        } else {
            // Region shape: the scene elements are the output base and the
            // region texture is only the blur source. They are already in
            // front-to-back order, so appending keeps the old stack.
            custom_elements.extend(scene_elements);
        }
    } else {
        // No sampled material this frame: release the per-panel Kawase
        // textures and keep the pre-P2 feather stack, a deterministic tint
        // under the chrome above the window space. This remains the fallback
        // when the sampled pass is unavailable or intentionally off.
        surface.material.trim_panels(0);
        custom_elements.extend(
            crate::render::chrome_backdrop_render_elements(state, &surface.output, scale)
                .into_iter()
                .map(DrmOutputElements::Decoration),
        );

        // The open window menu (T-01.4) composites above every window and
        // titlebar, but below the shell chrome.
        custom_elements.extend(
            crate::render::window_menu_render_elements(state, &surface.output, scale)
                .into_iter()
                .map(DrmOutputElements::Decoration),
        );

        // The window stack: each window's SSD titlebar, client surface, and
        // shadow stay together in `Space` stacking order. Emitting all
        // titlebars as one layer above all surfaces (and all shadows below
        // them) put a background window's titlebar over a foreground window's
        // body.
        custom_elements.extend(crate::render::window_scene_render_elements::<
            UdevRenderer<'_>,
            DrmOutputElements<UdevRenderer<'_>, WaylandSurfaceRenderElement<UdevRenderer<'_>>>,
        >(&mut renderer, state, &surface.output, scale));

        // The Files-owned desktop layer (T-19.3) composites below every window
        // and above the per-Space wallpaper.
        custom_elements.extend(crate::render::desktop_render_elements::<
            _,
            DrmOutputElements<UdevRenderer<'_>, WaylandSurfaceRenderElement<UdevRenderer<'_>>>,
        >(&mut renderer, state, &surface.output, scale));

        // The per-Space wallpaper is the bottom-most layer (T-05.4): it fills
        // the clear color and slides with its Space during a switch.
        custom_elements.extend(
            crate::render::wallpaper_render_elements(&mut renderer, state, &surface.output, scale)
                .into_iter()
                .map(DrmOutputElements::Wallpaper),
        );
    }

    let frame_mode = FrameFlags::DEFAULT; // direct scanout where possible
    let wallpaper = state.wallpaper_color_for(&surface.output);
    let result =
        surface
            .drm_output
            .render_frame(&mut renderer, &custom_elements, wallpaper, frame_mode);

    match result {
        Ok(render_frame_result) => {
            if let PrimaryPlaneElement::Element(_) = &render_frame_result.primary_element {
                state.stats.direct_scanouts += 1;
            }
            // Primary scanout bookkeeping is what `wl_surface.frame` callbacks
            // and presentation feedback key off. Smithay's DrmCompositor does
            // not send Wayland frame callbacks itself, so the DRM rail does it
            // here exactly like the nested/headless paths.
            update_primary_scanout(state, &render_frame_result.states);
            if render_frame_result.is_empty {
                // No damage, no render (FR-2); the frame clock reschedules a
                // probe for the next frame. A client that is already waiting
                // on its frame callback is still answered, otherwise a
                // commit-driven repaint can deadlock against an idle output.
                state.stats.frames_skipped_no_damage += 1;
                if state.has_pending_frame_callbacks() {
                    let now = state.clock.now();
                    render::send_frame_callbacks(
                        state,
                        &surface.output,
                        now.into(),
                        &render_frame_result.states,
                    );
                }
                // No free-running probe: the next frame is driven by a client
                // commit, input, or an animation tick. Probing every vblank
                // was the 4K CPU cap that held animated clients near 15 fps.
            } else {
                state.stats.frames_rendered += 1;
                // Primary scanout feedback for presentation-time.
                let feedback = render::take_presentation_feedback(
                    state,
                    &surface.output,
                    &render_frame_result.states,
                );
                // The feedback is delivered on vblank (frame_finish).
                match surface.drm_output.queue_frame(Some(feedback)) {
                    Ok(()) => {
                        // One output-frame is queued; the scheduler must not
                        // queue another animation frame for this output until
                        // `frame_finish()` clears the marker.
                        state.frame_scheduler.mark_in_flight(&output_name);
                    }
                    Err(err) => {
                        eprintln!("dragonfruit-compositor: queue_frame failed: {err:?}");
                        // A rejected frame must not stall the output forever:
                        // retry on the frame clock, exactly like an empty frame.
                        reschedule_probe(state, node, crtc, refresh);
                    }
                }
                // Send this frame's `wl_surface.frame` callbacks. This is the
                // one wakeup that lets Qt, GTK, Chromium, and Electron clients
                // continue painting; without it they paint one frame and then
                // wait forever. It also credits input-to-photon latency and
                // advertises the output's fractional scale to chrome.
                let now = state.clock.now();
                render::post_repaint(
                    state,
                    &surface.output,
                    now.into(),
                    &render_frame_result.states,
                );
            }
        }
        Err(err) => {
            eprintln!("dragonfruit-compositor: render error on {crtc:?}: {err:?}");
            // A failed sampled-material frame must not loop forever. Disable
            // the sampled path for the session and let the next frame take the
            // feather stack; the shell receives the fallback state on the next
            // broadcast.
            if used_material {
                state.material_blur_supported = false;
                eprintln!(
                    "dragonfruit-compositor: disabling the sampled material path after a \
                     DRM render failure; falling back to the feather stack"
                );
            }
            // Temporary failures reschedule via the frame clock; context
            // loss ends the session cleanly (FR-6). A rejected frame must
            // never leave the output with no pending frame at all.
            reschedule_probe(state, node, crtc, refresh);
        }
    }
}

/// Retry a failed frame on the next refresh boundary.
///
/// This is used only for renderer/queue failures now; a successful empty frame
/// no longer starts a free-running probe. `refresh` is passed in by the caller:
/// this runs while `render_surface` holds the backend's `RefCell` borrow, so it
/// must not call `drm_shared_slot()` and borrow the cell again.
fn reschedule_probe(
    state: &mut crate::state::DfState,
    node: DrmNode,
    crtc: crtc::Handle,
    refresh: i32,
) {
    if refresh <= 0 {
        return;
    }
    let delay = Duration::from_millis(1_000_000 / refresh as u64);
    let _ = state
        .loop_handle
        .insert_source(Timer::from_duration(delay), move |_, _, state| {
            let shared = drm_shared_slot();
            if let Some(shared) = shared {
                render_surface(state, &shared, node, crtc, true);
            }
            TimeoutAction::Drop
        });
}

/// Update per-surface primary scanout outputs from the render report;
/// presentation feedback and frame callbacks key off this.
fn update_primary_scanout(
    state: &mut crate::state::DfState,
    states: &smithay::backend::renderer::element::RenderElementStates,
) {
    for window in state.space.elements() {
        // The last output can disappear (unplug/last-GPU removal) while a
        // window is still winding down. There is no scanout output to update
        // then; skipping is correct and, unlike the old `expect`, cannot turn
        // a hotplug into a compositor abort.
        let Some(output) = surface_current_output(state, window) else {
            continue;
        };
        window.with_surfaces(|surface, surface_states| {
            smithay::desktop::utils::update_surface_primary_scanout_output(
                surface,
                &output,
                surface_states,
                states,
                smithay::backend::renderer::element::default_primary_scanout_output_compare,
            );
        });
    }
}

fn surface_current_output(state: &crate::state::DfState, window: &Window) -> Option<Output> {
    state
        .space
        .outputs_for_element(window)
        .into_iter()
        .next()
        .or_else(|| state.space.outputs().next().cloned())
}

// --- helpers -----------------------------------------------------------------

thread_local! {
    /// The backend's shared data slot, set by `init` before any source
    /// runs. Single-threaded event loop (02-compositor.md).
    static DRM_SHARED: RefCell<Option<SharedData>> = const { RefCell::new(None) };
}

fn set_drm_shared(shared: SharedData) {
    DRM_SHARED.with(|slot| *slot.borrow_mut() = Some(shared));
}

fn drm_shared_slot() -> Option<SharedData> {
    DRM_SHARED.with(|slot| slot.borrow().clone())
}

/// The cursor status the DRM rail can actually draw.
///
/// A client cursor surface only contributes render elements once it has a
/// committed buffer. Until then it would draw nothing, so fall back to the
/// system default without changing the seat's stored status (the client may
/// still be about to commit, and replacing it would lose the requested shape).
fn effective_cursor_status(status: &CursorImageStatus) -> CursorImageStatus {
    match status {
        CursorImageStatus::Surface(surface)
            if !surface.alive()
                || !with_renderer_surface_state(surface, |state| state.buffer().is_some())
                    .unwrap_or(false) =>
        {
            CursorImageStatus::default_named()
        }
        status => status.clone(),
    }
}

/// The seat's wl_pointer.set_cursor surfaces report their hotspots here.
fn cursor_hotspot(surface: &WlSurface) -> Point<i32, Logical> {
    with_states(surface, |states| {
        states
            .data_map
            .get::<CursorImageAttributes>()
            .map(|attributes| attributes.hotspot)
            .unwrap_or_default()
    })
}

/// A simple procedural arrow cursor (24x24 ARGB). It is the last-resort
/// fallback when the XCursor theme is missing or a shape has no image; the
/// normal path is the themed [`crate::cursor_theme::CursorTheme`].
fn arrow_cursor_buffer() -> MemoryRenderBuffer {
    const SIZE: i32 = 24;
    let mut pixels = vec![0u8; (SIZE * SIZE * 4) as usize];
    let set = |pixels: &mut [u8], x: i32, y: i32, argb: [u8; 4]| {
        if (0..SIZE).contains(&x) && (0..SIZE).contains(&y) {
            let idx = ((y * SIZE + x) * 4) as usize;
            pixels[idx..idx + 4].copy_from_slice(&argb);
        }
    };
    for y in 0..SIZE {
        for x in 0..SIZE {
            // Arrow: triangle from the top-left plus a tail.
            let in_edge = x - 2;
            let head = y < 14 && in_edge >= 0 && in_edge <= y;
            let tail = (14..22).contains(&y) && x >= 8 && x <= 8 + (y - 14) / 2 + 2;
            if head || tail {
                set(&mut pixels, x, y, [255, 255, 255, 255]);
            }
            // Outline.
            let on_edge = y < 15 && (x + 1 == y || x == y + 1 || (y == 14 && x < 15));
            if on_edge {
                set(&mut pixels, x, y, [255, 0, 0, 0]);
            }
        }
    }
    MemoryRenderBuffer::from_slice(
        &pixels,
        Fourcc::Argb8888,
        smithay::utils::Size::from((SIZE, SIZE)),
        1,
        Transform::Normal,
        None,
    )
}

/// PostAction is unused today but keeps callback signatures stable for the
/// D-Bus integration in T-20.
#[allow(dead_code)]
fn _post_action_witness() -> Option<PostAction> {
    None
}

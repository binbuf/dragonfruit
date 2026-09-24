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

use std::cell::RefCell;
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
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::renderer::multigpu::gbm::GbmGlesBackend;
use smithay::backend::renderer::multigpu::{GpuManager, MultiRenderer};
use smithay::backend::renderer::{ImportAll, ImportDma, ImportDmaWl, ImportMem, ImportMemWl};
use smithay::backend::session::libseat::LibSeatSession;
use smithay::backend::session::{Event as SessionEvent, Session};
use smithay::backend::udev::{all_gpus, primary_gpu, UdevBackend, UdevEvent};
use smithay::backend::SwapBuffersError;
use smithay::desktop::space::SpaceRenderElements;
use smithay::desktop::utils::OutputPresentationFeedback;
use smithay::desktop::Window;
use smithay::input::pointer::{CursorImageAttributes, CursorImageStatus};
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
use smithay::utils::{DeviceFd, IsAlive, Logical, Point, Scale, Time, Transform};
use smithay::wayland::compositor::with_states;
use smithay::wayland::presentation::Refresh;
use smithay_drm_extras::drm_scanner::{DrmScanEvent, DrmScanner};

use crate::backend::add_seat_capabilities;
use crate::session::{run_session, BackendHooks};
use crate::{input, render, LOCKSTEP_VERSION};

type UdevRenderer<'a> = MultiRenderer<
    'a,
    'a,
    GbmGlesBackend<GlesRenderer, DrmDeviceFd>,
    GbmGlesBackend<GlesRenderer, DrmDeviceFd>,
>;

type GbmAllocatorDrm = GbmAllocator<DrmDeviceFd>;
type GbmExporterDrm = GbmFramebufferExporter<DrmDeviceFd>;

// 8-bit formats until color management (xx-color-management) lands with
// the Displays pane work (T-16).
const SUPPORTED_FORMATS: &[Fourcc] = &[Fourcc::Abgr8888, Fourcc::Argb8888];

render_elements! {
    pub DrmOutputElements<R, E> where R: ImportAll + ImportMem + ImportMemWl + ImportDmaWl + ImportDma;
    Pointer=PointerRenderElement<R>,
    Space=SpaceRenderElements<R, E>,
    Chrome=WaylandSurfaceRenderElement<R>,
    // SSD titlebars (T-01.1): flat solid fills, above the window space.
    Decoration=SolidColorRenderElement,
    // A window mid-appear: scaled about its target and translated from the
    // Dock tile origin (T-02.1b).
    Appear=RelocateRenderElement<RescaleRenderElement<WaylandSurfaceRenderElement<R>>>,
}

render_elements! {
    pub PointerRenderElement<R> where R: ImportAll + ImportMem + ImportMemWl + ImportDmaWl + ImportDma;
    Surface=WaylandSurfaceRenderElement<R>,
    Memory=MemoryRenderBufferRenderElement<R>,
}
/// The system cursor: a memory buffer rendered with `Kind::Cursor` so the
/// DRM compositor can assign it to a hardware cursor plane.
struct PointerElement {
    buffer: Option<MemoryRenderBuffer>,
    status: CursorImageStatus,
}

impl Default for PointerElement {
    fn default() -> Self {
        Self {
            buffer: Default::default(),
            status: CursorImageStatus::default_named(),
        }
    }
}

impl PointerElement {
    fn set_status(&mut self, status: CursorImageStatus) {
        self.status = status;
    }

    fn set_buffer(&mut self, buffer: MemoryRenderBuffer) {
        self.buffer = Some(buffer);
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
        match &self.status {
            CursorImageStatus::Hidden => vec![],
            // A named shape always renders the system buffer.
            CursorImageStatus::Named(_) => {
                if let Some(buffer) = self.buffer.as_ref() {
                    vec![PointerRenderElement::<R>::from(
                        MemoryRenderBufferRenderElement::from_buffer(
                            renderer,
                            location.to_f64(),
                            buffer,
                            None,
                            None,
                            None,
                            Kind::Cursor,
                        )
                        .expect("lost system pointer buffer"),
                    )
                    .into()]
                } else {
                    vec![]
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

/// One display output on one DRM device.
struct SurfaceData {
    dh: smithay::reexports::wayland_server::DisplayHandle,
    output: Output,
    global: GlobalId,
    drm_output:
        DrmOutput<GbmAllocatorDrm, GbmExporterDrm, Option<OutputPresentationFeedback>, DrmDeviceFd>,
    last_presentation_time: Option<Time<smithay::utils::Monotonic>>,
    vblank_throttle_timer: Option<RegistrationToken>,
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
    pointer_image: MemoryRenderBuffer,
    libinput: Option<Libinput>,
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
            // DRM rendering is vblank/timer-driven from the backend's own
            // sources; the loop-level hook has nothing to do.
            render: Box::new(|_| Ok(())),
        },
    )
}

fn init(state: &mut crate::state::DfState, shared: SharedData) -> Result<(), String> {
    // Session: logind/libseat takes the VT and device access.
    let (session, notifier) = LibSeatSession::new()
        .map_err(|e| format!("failed to acquire a DRM session (logind/libseat): {e}"))?;
    let seat_name = session.seat();

    // Primary GPU: explicit override, else the seat's primary render node,
    // else the first GPU that opens.
    let primary_gpu = match std::env::var("DF_DRM_DEVICE") {
        Ok(path) => DrmNode::from_path(path.as_str())
            .map_err(|e| format!("invalid DF_DRM_DEVICE {path:?}: {e}"))?,
        Err(_) => primary_gpu(&seat_name)
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
            })
            .ok_or_else(|| "no usable DRM GPU found on this seat".to_string())?,
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
        pointer_image: arrow_cursor_buffer(),
        libinput: Some(libinput_context),
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
                render_surface(state, &shared, node, crtc);
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
                    let mut guard = shared.borrow_mut();
                    let Some(data) = guard.as_mut() else { return };
                    if let Some(libinput) = data.libinput.as_mut() {
                        libinput.suspend();
                    }
                    for device in data.devices.values_mut() {
                        device.drm_output_manager.pause();
                        device.surfaces.clear();
                    }
                }
                SessionEvent::ActivateSession => {
                    let mut guard = shared.borrow_mut();
                    let Some(data) = guard.as_mut() else { return };
                    if let Some(libinput) = data.libinput.as_mut() {
                        if let Err(err) = libinput.resume() {
                            eprintln!("dragonfruit-compositor: failed to resume libinput: {err:?}");
                        }
                    }
                    for (node, device) in data.devices.iter_mut() {
                        // Optimistic re-activation: if connectors were
                        // re-bound by a foreign master, the queued render
                        // resets the state instead (see render_surface).
                        let _ = device.drm_output_manager.activate(false);
                        let crtcs: Vec<_> = device.surfaces.keys().copied().collect();
                        let node = *node;
                        let shared = shared.clone();
                        for crtc in crtcs {
                            let shared = shared.clone();
                            let _ = state.loop_handle.insert_idle(move |state| {
                                render_surface(state, &shared, node, crtc);
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
            render_node.is_some()
                || format.modifier == smithay::backend::allocator::Modifier::Linear
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

    let mode_id = connector
        .modes()
        .iter()
        .position(|mode| mode.mode_type().contains(ModeTypeFlags::PREFERRED))
        .unwrap_or(0);
    let drm_mode = connector.modes()[mode_id];
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
        },
    );

    // Kick off the first frame.
    let shared = drm_shared_slot();
    if let Some(shared) = shared {
        let _ = state.loop_handle.insert_idle(move |state| {
            render_surface(state, &shared, node, crtc);
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
    {
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
                let timer_token = state
                    .loop_handle
                    .insert_source(Timer::from_duration(remaining), move |_, _, state| {
                        frame_finish(state, dev_id, crtc, &mut Some(throttled_metadata));
                        TimeoutAction::Drop
                    })
                    .expect("failed to register vblank throttle timer");
                surface.vblank_throttle_timer = Some(timer_token);
                return;
            }
        }
        surface.last_presentation_time = Some(clock);
    }

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

    let schedule_render = match submit_result {
        Ok(user_data) => {
            if let Some(mut feedback) = user_data.flatten() {
                feedback.presented(clock, Refresh::fixed(frame_duration), seq as u64, flags);
            }
            true
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
                    matches!(err.downcast_ref::<DrmError>(), Some(DrmError::Access(_)))
                }
                SwapBuffersError::ContextLost(err) => {
                    eprintln!("dragonfruit-compositor: rendering loop lost: {err}");
                    state.running = false;
                    false
                }
            }
        }
    };

    if schedule_render {
        let next_frame_target = clock + frame_duration;
        // Split the frame between client repaint and compositor repaint
        // to keep input-to-photon latency near one frame (FR-3).
        let repaint_delay = Duration::from_secs_f64(frame_duration.as_secs_f64() * 0.6);
        let _ = state.loop_handle.insert_source(
            Timer::from_duration(repaint_delay),
            move |_, _, state| {
                let shared = drm_shared_slot();
                if let Some(shared) = shared {
                    render_surface(state, &shared, dev_id, crtc);
                }
                let _ = next_frame_target;
                TimeoutAction::Drop
            },
        );
    }
}

/// Render one DRM output: build elements from the scene and hand them to
/// the DrmCompositor (direct scanout when possible, FR-5).
fn render_surface(
    state: &mut crate::state::DfState,
    shared: &SharedData,
    node: DrmNode,
    crtc: crtc::Handle,
) {
    let mut guard = shared.borrow_mut();
    let Some(data) = guard.as_mut() else { return };

    let Some(device) = data.devices.get_mut(&node) else {
        return;
    };
    let render_node = device.render_node.unwrap_or(data.primary_gpu);
    let Some(surface) = device.surfaces.get_mut(&crtc) else {
        return;
    };

    let mut renderer = match data.gpus.single_renderer(&render_node) {
        Ok(renderer) => renderer,
        Err(err) => {
            eprintln!("dragonfruit-compositor: failed to get renderer: {err:?}");
            return;
        }
    };

    let scale = Scale::from(surface.output.current_scale().fractional_scale());

    // Open this output's material pass (T-04.2), so the chrome backdrop is
    // applied at most once for it this frame.
    state.begin_render_frame();

    // The pointer: rendered with Kind::Cursor so DrmCompositor can assign
    // a hardware cursor plane; cursor motion never waits on effects or
    // damage (02-compositor.md).
    let mut custom_elements: Vec<
        DrmOutputElements<UdevRenderer<'_>, WaylandSurfaceRenderElement<UdevRenderer<'_>>>,
    > = Vec::new();
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
        data.pointer_element.set_status(state.cursor_image.clone());
        if matches!(
            state.cursor_image,
            CursorImageStatus::Hidden | CursorImageStatus::Named(_)
        ) {
            data.pointer_element.set_buffer(data.pointer_image.clone());
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

    // Scene elements: surface trees of mapped windows (live buffers only —
    // effects are compositor render passes, never screenshots). The appear
    // transform is applied per window (T-02.1b).
    {
        let window_elements = crate::render::window_render_elements::<
            UdevRenderer<'_>,
            DrmOutputElements<UdevRenderer<'_>, WaylandSurfaceRenderElement<UdevRenderer<'_>>>,
        >(&mut renderer, state, &surface.output, scale);
        custom_elements.extend(window_elements);
    }

    // Chrome surfaces (menu bar, overlays) composite above the window space
    // (T-09).
    custom_elements.extend(crate::render::chrome_render_elements::<
        _,
        DrmOutputElements<UdevRenderer<'_>, WaylandSurfaceRenderElement<UdevRenderer<'_>>>,
    >(&mut renderer, state, &surface.output, scale));

    // Backdrop blur under the chrome (T-04.2). NOTE: this rail's element list
    // is built front-to-back but currently places the window surfaces before
    // the chrome, so the backdrop composes below the windows here until that
    // pre-existing ordering is reconciled; DRM is untested (no hardware), like
    // the rest of this rail.
    custom_elements.extend(
        crate::render::chrome_backdrop_render_elements(state, &surface.output, scale)
            .into_iter()
            .map(DrmOutputElements::Decoration),
    );

    // SSD titlebars (T-01.1) composite above their client surfaces.
    custom_elements.extend(
        crate::render::titlebar_render_elements(state, &surface.output, scale)
            .into_iter()
            .map(DrmOutputElements::Decoration),
    );

    // The open window menu (T-01.4) composites above the titlebars.
    custom_elements.extend(
        crate::render::window_menu_render_elements(state, &surface.output, scale)
            .into_iter()
            .map(DrmOutputElements::Decoration),
    );

    // Elevation-token-driven shadows (T-04.1a) composite below the window
    // surfaces, so the ring beyond a window is visible.
    custom_elements.extend(
        crate::render::window_shadow_render_elements(state, &surface.output, scale)
            .into_iter()
            .map(DrmOutputElements::Decoration),
    );

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
            if render_frame_result.is_empty {
                // No damage, no render, no client wakeups (FR-2); the
                // frame clock reschedules a probe for the next frame.
                state.stats.frames_skipped_no_damage += 1;
                reschedule_probe(state, node, crtc);
            } else {
                state.stats.frames_rendered += 1;
                // The DrmCompositor queues the frame callbacks; count the live
                // windows as the frame-callback batches it will deliver, so the
                // idle/ wakeup budget (T-03.1a) is measurable on DRM too.
                state.stats.client_wakeups +=
                    crate::render::count_output_windows(state, &surface.output);
                // A presented frame: credit any pending input with its
                // input-to-photon round trip (T-03.1b). The nested/headless
                // paths do this in `render::post_repaint*`.
                state.stats.latency.note_present(std::time::Instant::now());
                // Primary scanout bookkeeping for presentation feedback.
                update_primary_scanout(state, &render_frame_result.states);
                let feedback = render::take_presentation_feedback(
                    state,
                    &surface.output,
                    &render_frame_result.states,
                );
                // The feedback is delivered on vblank (frame_finish).
                if let Err(err) = surface.drm_output.queue_frame(Some(feedback)) {
                    eprintln!("dragonfruit-compositor: queue_frame failed: {err:?}");
                }
            }
        }
        Err(err) => {
            eprintln!("dragonfruit-compositor: render error on {crtc:?}: {err:?}");
            // Temporary failures reschedule via the frame clock; context
            // loss ends the session cleanly (FR-6).
        }
    }
}

/// Re-test for damage on the next frame boundary when a frame came back
/// empty (the standard smithay reschedule strategy).
fn reschedule_probe(state: &mut crate::state::DfState, node: DrmNode, crtc: crtc::Handle) {
    let refresh = drm_shared_slot().and_then(|shared| {
        let guard = shared.borrow();
        guard.as_ref().and_then(|data| {
            data.devices
                .get(&node)
                .and_then(|device| device.surfaces.get(&crtc))
                .and_then(|surface| surface.output.current_mode())
                .map(|mode| mode.refresh)
        })
    });
    let Some(refresh) = refresh else { return };
    let delay = Duration::from_millis(1_000_000 / refresh as u64);
    let _ = state
        .loop_handle
        .insert_source(Timer::from_duration(delay), move |_, _, state| {
            let shared = drm_shared_slot();
            if let Some(shared) = shared {
                render_surface(state, &shared, node, crtc);
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
        let output = surface_current_output(state, window);
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

fn surface_current_output(state: &crate::state::DfState, window: &Window) -> Output {
    state
        .space
        .outputs_for_element(window)
        .into_iter()
        .next()
        .or_else(|| state.space.outputs().next().cloned())
        .expect("rendering requires at least one output")
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

/// The seat's wl_pointer.set_cursor surfaces report their hotspots here.
#[allow(dead_code)]
fn cursor_hotspot(surface: &WlSurface) -> Point<i32, Logical> {
    with_states(surface, |states| {
        states
            .data_map
            .get::<CursorImageAttributes>()
            .map(|attributes| attributes.hotspot)
            .unwrap_or_default()
    })
}

/// A simple procedural arrow cursor (24x24 ARGB). Replaced by themed
/// cursors from the design system in T-08/T-03.
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

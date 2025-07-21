use baseview::{WindowHandle, WindowOpenOptions, WindowScalePolicy};
use iced_baseview::{baseview::{Size, WindowOpenOptions, WindowScalePolicy}, Application, Settings};

use nih_plug::editor::{Editor, ParentWindowHandle};
use nih_plug::params::persist::PersistentField;
use nih_plug::prelude::{GuiContext, ParamSetter, NonZeroU32};
use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};
use iced_wgpu::wgpu;
use iced_wgpu::wgpu::SurfaceTargetUnsafe;
use iced_wgpu::{Renderer as IcedRenderer, Settings as IcedSettings, core::Size};
use iced_runtime;

use crossbeam::atomic::AtomicCell;
use serde::{Serialize, Deserialize};

use std::{
    borrow::Cow,
    num::NonZeroIsize,
    ptr::NonNull,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use crate::{AhmadParams};

pub mod ui;

// pub struct AhmadWindow {
//     gui_context: Arc<dyn GuiContext>,

//     device: wgpu::Device,
//     queue: wgpu::Queue,
//     surface: wgpu::Surface<'static>,
//     surface_config: wgpu::SurfaceConfiguration,
//     pipeline: wgpu::RenderPipeline,

//     // iced fields
//     ui_renderer: iced_wgpu::Renderer,
//     ui_state: ui::UIState,
//     ui_cache: iced_runtime::user_interface::Cache,

//     #[allow(unused)]
//     params: Arc<AhmadParams>,
// }

// impl AhmadWindow {
//     fn new(
//         window: &mut baseview::Window<'_>,
//         gui_context: Arc<dyn GuiContext>,
//         params: Arc<AhmadParams>,
//         scaling_factor: f32,
//     ) -> Self {
//         let target = baseview_window_to_surface_target(window);

//         pollster::block_on(Self::create(
//             target,
//             window,
//             gui_context,
//             params,
//             scaling_factor,
//         ))
//     }

//     async fn create(
//         target: SurfaceTargetUnsafe,
//         window: &baseview::Window<'_>,
//         gui_context: Arc<dyn GuiContext>,
//         params: Arc<AhmadParams>,
//         scaling_factor: f32,
//     ) -> Self {
//         let (unscaled_width, unscaled_height) = params.editor_state.size();
//         let width = (unscaled_width as f64 * scaling_factor as f64).round() as u32;
//         let height = (unscaled_height as f64 * scaling_factor as f64).round() as u32;

//         let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());

//         let surface = unsafe { instance.create_surface_unsafe(target) }.unwrap();

//         let adapter = instance
//             .request_adapter(&wgpu::RequestAdapterOptions {
//                 power_preference: wgpu::PowerPreference::LowPower,
//                 force_fallback_adapter: false,
//                 compatible_surface: Some(&surface),
//             })
//             .await
//             .expect("Failed to find an appropriate adapter");

//         let (device, queue) = adapter
//             .request_device(
//                 &wgpu::DeviceDescriptor {
//                     label: None,
//                     required_features: wgpu::Features::empty(),
//                     required_limits: wgpu::Limits::downlevel_webgl2_defaults()
//                         .using_resolution(adapter.limits()),
//                 },
//                 None
//             ).await
//             .expect("Failed to create device");

//         const SHADER: &str = "
//             const VERTS = array(
//                 vec2<f32>(0.5, 1.0),
//                 vec2<f32>(0.0, 0.0),
//                 vec2<f32>(1.0, 0.0)
//             );

//             struct VertexOutput {
//                 @builtin(position) clip_position: vec4<f32>,
//                 @location(0) position: vec2<f32>,
//             };

//             @vertex
//             fn vs_main(
//                 @builtin(vertex_index) in_vertex_index: u32,
//             ) -> VertexOutput {
//                 var out: VertexOutput;
//                 out.position = VERTS[in_vertex_index];
//                 out.clip_position = vec4<f32>(out.position - 0.5, 0.0, 1.0);
//                 return out;
//             }

//             @fragment
//             fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
//                 return vec4<f32>(in.position, 0.5, 1.0);
//             }
//             ";

//         let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
//             label: None,
//             source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SHADER)),
//         });

//         let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
//             label: None,
//             bind_group_layouts: &[],
//             push_constant_ranges: &[],
//         });

//         let swapchain_capabilities = surface.get_capabilities(&adapter);
//         let swapchain_format = swapchain_capabilities.formats[0];

//         let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
//             label: None,
//             layout: Some(&pipeline_layout),
//             vertex: wgpu::VertexState {
//                 module: &shader,
//                 entry_point: "vs_main",
//                 buffers: &[],
//             },
//             fragment: Some(wgpu::FragmentState {
//                 module: &shader,
//                 entry_point: "fs_main",
//                 targets: &[Some(swapchain_format.into())],
//             }),
//             primitive: wgpu::PrimitiveState::default(),
//             depth_stencil: None,
//             multisample: wgpu::MultisampleState::default(),
//             multiview: None,
//         });

//         let surface_config = surface.get_default_config(&adapter, width, height).unwrap();
//         surface.configure(&device, &surface_config);

//         // configure iced
//         let font = iced_wgpu::core::Font {
//             family: iced::font::Family::SansSerif,
//             weight: iced::font::Weight::Thin,
//             stretch: iced::font::Stretch::Normal,
//             style: iced::font::Style::Normal,
//         };
//         // let compositor = iced_wgpu::window::Compositor::request(
//         //     IcedSettings {
//         //         backends: adapter.get_info().backend.into(),
//         //         default_font: font.clone(),
//         //         default_text_size: iced_wgpu::core::Pixels(15.0),
//         //         present_mode: wgpu::PresentMode::AutoVsync,
//         //         ..Default::default()
//         //     },
//         //     Some(window.raw_window_handle()),
//         // ).await.expect("Iced compositor was not able to instantiate.");
//         let engine = iced_wgpu::Engine::new(
//             &adapter, &device, &queue, swapchain_format, None
//         );
//         let ui_renderer = IcedRenderer::new(
//             &device,
//             &engine,
//             font.clone(),
//             iced::Pixels(15.0),
//         );

//         Self {
//             pipeline,
//             gui_context,
//             device,
//             queue,
//             surface,
//             surface_config,
//             params,
//             ui_renderer,
//             ui_state: ui::UIState::default(),
//             ui_cache: iced_runtime::user_interface::Cache::new(),
//         }
//     }
// }

// impl baseview::WindowHandler for AhmadWindow {
//     fn on_frame(&mut self, _window: &mut baseview::Window) {
//         // TODO: interface iced render logic here

//         let frame = self
//             .surface
//             .get_current_texture()
//             .expect("Failed to acquire next swap chain texture");
//         let view = frame
//             .texture
//             .create_view(&wgpu::TextureViewDescriptor::default());
//         let mut encoder = self
//             .device
//             .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

//         {
//             let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
//                 label: None,
//                 color_attachments: &[Some(wgpu::RenderPassColorAttachment {
//                     view: &view,
//                     resolve_target: None,
//                     ops: wgpu::Operations {
//                         load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
//                         store: wgpu::StoreOp::Store,
//                     },
//                 },)],
//                 depth_stencil_attachment: None,
//                 timestamp_writes: None,
//                 occlusion_query_set: None
//             });

//             rpass.set_pipeline(&self.pipeline);
//             rpass.draw(0..3, 0..1);
//         }

//         // iced UI pass
//         {
//             let mut ui = iced_runtime::UserInterface
//                 ::build(
//                 self.ui_state.view(),
//                 Size::new(
//                     self.surface_config.width as f32,
//                     self.surface_config.height as f32,
//                 ),
//                 self.ui_cache,
//                 &mut self.ui_renderer,
//             );

//             ui.draw(
//                 &mut self.ui_renderer,
//                 &iced_wgpu::core::Theme::KanagawaLotus,
//                 &iced_wgpu::core::renderer::Style { text_color: iced::Color::new(1.0, 1.0, 1.0, 0.0) },
//                 iced::mouse::Cursor::default(),
//             );

//             self.ui_cache = ui.into_cache()
//         }

//         self.queue.submit(Some(encoder.finish()));
//         frame.present();
//     }

//     fn on_event(&mut self, window: &mut baseview::Window, event: baseview::Event) -> baseview::EventStatus {
//         let _param_seter = ParamSetter::new(self.gui_context.as_ref());

//         match &event {
//             // do event processing here:
//             // TODO: interface iced event loop here
//             baseview::Event::Window(event) => match event {
//                 baseview::WindowEvent::Resized(window_info) => {
//                     self.params.editor_state.size.store((
//                         window_info.logical_size().width.round() as u32,
//                         window_info.logical_size().width.round() as u32,
//                     ));

//                     self.surface_config.width = window_info.physical_size().width;
//                     self.surface_config.height = window_info.physical_size().height;

//                     self.surface.configure(&self.device, &self.surface_config);
//                 }
//                 _ => {}
//             }
//             _ => {}
//         }

//         baseview::EventStatus::Captured
//     }
// }

// #[derive(Debug, Serialize, Deserialize)]
// pub struct AhmadEditorState {
//     #[serde(with = "nih_plug::params::persist::serialize_atomic_cell")]
//     size: AtomicCell<(u32, u32)>,

//     #[serde(skip)]
//     open: AtomicBool,
// }

// impl AhmadEditorState {
//     pub fn from_size(size: (u32, u32)) -> Arc<Self> {
//         Arc::new(Self {
//             size: AtomicCell::new(size),
//             open: AtomicBool::new(false),
//         })
//     }

//     pub fn size(&self) -> (u32, u32) {
//         self.size.load()
//     }

//     pub fn is_open(&self) -> bool {
//         self.open.load(Ordering::Acquire)
//     }
// }

// impl<'a> PersistentField<'a, AhmadEditorState> for Arc<AhmadEditorState> {
//     fn set(&self, new_value: AhmadEditorState) {
//         self.size.store(new_value.size.load());
//     }

//     fn map<F, R>(&self, f: F) -> R
//     where
//         F: Fn(&AhmadEditorState) -> R,
//     {
//         f(self)
//     }
// }

// pub struct AhmadEditor {
//     pub params: Arc<AhmadParams>,

//     // linux/windows support option for the future.
//     // WARN: DO NOT USE ON macOS
//     pub scaling_factor: AtomicCell<Option<f32>>,
// }

// impl Editor for AhmadEditor {
//     fn spawn(
//         &self,
//         parent: ParentWindowHandle,
//         context: Arc<dyn GuiContext>,
//     ) -> Box<dyn std::any::Any + Send> {
//         let (unscaled_width, unscaled_height) = self.params.editor_state.size();
//         let scaling_factor = self.scaling_factor.load();

//         let gui_context = Arc::clone(&context);
//         let params = Arc::clone(&self.params);


//         let window = baseview::Window::open_parented(
//             &ParentWindowHandleAdapter(parent),
//             WindowOpenOptions {
//                 title: String::from("ahmad"),
//                 // let baseview do scaling
//                 size: baseview::Size::new(unscaled_width as f64, unscaled_height as f64),
//                 scale: scaling_factor
//                     .map(|factor| WindowScalePolicy::ScaleFactor(factor as f64))
//                     .unwrap_or(WindowScalePolicy::SystemScaleFactor),
//                 gl_config: None
//             },
//             move |window: &mut baseview::Window<'_>| -> AhmadWindow {
//                 AhmadWindow::new(
//                     window,
//                     gui_context,
//                     params,
//                     scaling_factor.unwrap_or(1.0),
//                 )
//             },
//         );

//         self.params.editor_state.open.store(true, Ordering::Release);
//         Box::new(AhmadEditorHandle {
//             state: self.params.editor_state.clone(),
//             window,
//         })
//     }

//     fn size(&self) -> (u32, u32) {
//         self.params.editor_state.size()
//     }

//     fn set_scale_factor(&self, factor: f32) -> bool {
//         // live scale handling not supported on all platforms, for now just don't
//         // let user change it while open for consistency
//         if self.params.editor_state.is_open() {
//             return false;
//         }

//         self.scaling_factor.store(Some(factor));
//         true
//     }

//     fn param_value_changed(&self, id: &str, normalized_value: f32) {}

//     fn param_modulation_changed(&self, id: &str, modulation_offset: f32) {}


//     fn param_values_changed(&self) {}
// }

// struct AhmadEditorHandle {
//     state: Arc<AhmadEditorState>,
//     window: WindowHandle,
// }

// // TODO: see if window handle enum in WindowHandle that has raw pointers can be circumvented
// unsafe impl Send for AhmadEditorHandle {}

// impl Drop for AhmadEditorHandle {
//     fn drop(&mut self) {
//         self.state.open.store(false, Ordering::Release);
//         self.window.close();
//     }
// }

// struct ParentWindowHandleAdapter(nih_plug::editor::ParentWindowHandle);

// unsafe impl HasRawWindowHandle for ParentWindowHandleAdapter {
//     fn raw_window_handle(&self) -> RawWindowHandle {
//         match self.0 {
//             ParentWindowHandle::X11Window(window) => {
//                 let mut handle = raw_window_handle::XcbWindowHandle::empty();
//                 handle.window = window;
//                 RawWindowHandle::Xcb(handle)
//             }
//             ParentWindowHandle::AppKitNsView(ns_view) => {
//                 let mut handle = raw_window_handle::AppKitWindowHandle::empty();
//                 handle.ns_view = ns_view;
//                 RawWindowHandle::AppKit(handle)
//             }
//             ParentWindowHandle::Win32Hwnd(hwnd) => {
//                 let mut handle = raw_window_handle::Win32WindowHandle::empty();
//                 handle.hwnd = hwnd;
//                 RawWindowHandle::Win32(handle)
//             }
//         }
//     }
// }

// // manually convert from raw_window_handle 0.5 (for baseview) to 0.6 (for wgpu)
// fn baseview_window_to_surface_target(window: &baseview::Window<'_>) -> wgpu::SurfaceTargetUnsafe {
// use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

//     let raw_display_handle = window.raw_display_handle();
//     let raw_window_handle = window.raw_window_handle();

//     wgpu::SurfaceTargetUnsafe::RawHandle {
//         raw_display_handle: match raw_display_handle {
//             raw_window_handle::RawDisplayHandle::AppKit(_) => {
//                 raw_window_handle_06::RawDisplayHandle::AppKit(
//                     raw_window_handle_06::AppKitDisplayHandle::new(),
//                 )
//             }
//             raw_window_handle::RawDisplayHandle::Xlib(handle) => {
//                 raw_window_handle_06::RawDisplayHandle::Xlib(
//                     raw_window_handle_06::XlibDisplayHandle::new(
//                         NonNull::new(handle.display),
//                         handle.screen,
//                     ),
//                 )
//             }
//             raw_window_handle::RawDisplayHandle::Xcb(handle) => {
//                 raw_window_handle_06::RawDisplayHandle::Xcb(
//                     raw_window_handle_06::XcbDisplayHandle::new(
//                         NonNull::new(handle.connection),
//                         handle.screen,
//                     ),
//                 )
//             }
//             raw_window_handle::RawDisplayHandle::Windows(_) => {
//                 raw_window_handle_06::RawDisplayHandle::Windows(
//                     raw_window_handle_06::WindowsDisplayHandle::new(),
//                 )
//             }
//             _ => todo!(),
//         },
//         raw_window_handle: match raw_window_handle {
//             raw_window_handle::RawWindowHandle::AppKit(handle) => {
//                 raw_window_handle_06::RawWindowHandle::AppKit(
//                     raw_window_handle_06::AppKitWindowHandle::new(
//                         NonNull::new(handle.ns_view).unwrap(),
//                     ),
//                 )
//             }
//             raw_window_handle::RawWindowHandle::Xlib(handle) => {
//                 raw_window_handle_06::RawWindowHandle::Xlib(
//                     raw_window_handle_06::XlibWindowHandle::new(handle.window),
//                 )
//             }
//             raw_window_handle::RawWindowHandle::Xcb(handle) => {
//                 raw_window_handle_06::RawWindowHandle::Xcb(
//                     raw_window_handle_06::XcbWindowHandle::new(
//                         NonZeroU32::new(handle.window).unwrap(),
//                     ),
//                 )
//             }
//             raw_window_handle::RawWindowHandle::Win32(handle) => {
//                 let mut raw_handle = raw_window_handle_06::Win32WindowHandle::new(
//                     NonZeroIsize::new(handle.hwnd as isize).unwrap(),
//                 );

//                 raw_handle.hinstance = NonZeroIsize::new(handle.hinstance as isize);

//                 raw_window_handle_06::RawWindowHandle::Win32(raw_handle)
//             }
//             _ => todo!(),
//         },
//     }
// }


// here we go
struct IcedWindowHandle {
    state: i32, // FIX: make this correct type and add to drop()
    handle: iced_baseview::window::WindowHandle<ui::Message>,
}

// need to make this unsafe, or implementation won't work on macOS
unsafe impl Send for IcedWindowHandle {}

impl Drop for IcedWindowHandle {
    fn drop(&mut self) {
        self.handle.close_window();
    }
}

struct AhmadEditor {
    params: Arc<AhmadParams>,
    state: ui::UIState,
    size: AtomicCell<(u32, u32)>,
    scaling_factor: AtomicCell<Option<f32>>,
}

impl Editor for AhmadEditor {
    fn spawn(
            &self,
            parent: ParentWindowHandle,
            context: Arc<dyn GuiContext>,
        ) -> Box<dyn std::any::Any + Send> {

        let size = self.size.load();
        let settings = Settings {
            window: WindowOpenOptions {
                title: String::from("ahmad"),
                size: iced_baseview::baseview::Size::new(400.0, 200.0),
                scale: WindowScalePolicy::SystemScaleFactor,
            },
            ..Default::default()
        };

        let handle = iced_baseview::open_parented::<ui::UIState, ParentWindowHandle>(&parent,
            (), settings);
        Box::new(IcedWindowHandle {handle, state: 0})
    }

    fn size(&self) -> (u32, u32) {
        self.size.load()
    }

    fn set_scale_factor(&self, factor: f32) -> bool {
        // live scale handling not supported on all platforms, for now just don't
        // let user change it while open for consistency
        if self.params.editor_state.is_open() {
            return false;
        }

        self.scaling_factor.store(Some(factor));
        true
    }

    fn param_value_changed(&self, id: &str, normalized_value: f32) {}

    fn param_values_changed(&self) {}

    fn param_modulation_changed(&self, id: &str, modulation_offset: f32) {}
}

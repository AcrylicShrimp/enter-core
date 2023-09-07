use self::{
    ecs_system::{
        render::RenderSystem, update_camera_transform_buffer::UpdateCameraTransformBufferSystem,
    },
    gfx::{
        Camera, DepthStencilMode, GfxContext, GfxContextCreationError, GfxContextHandle,
        RenderManager, ScreenManager, ShaderManager,
    },
    time::TimeManager,
    vsync::TargetFrameInterval,
};
use codegen::Handle;
use gfx::MeshRenderer;
use object::ObjectManager;
use specs::prelude::*;
use std::{
    cell::{Ref, RefCell, RefMut},
    mem::MaybeUninit,
    num::NonZeroU32,
    time::Instant,
};
use thiserror::Error;
use transform::Transform;
use winit::{
    dpi::{LogicalSize, PhysicalSize},
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

pub mod ecs_system;
pub mod gfx;
pub mod math;
pub mod object;
pub mod time;
pub mod transform;
pub mod vsync;

// re-exports.
pub use image;
pub use russimp;
pub use specs;
pub use wgpu;

static mut CONTEXT: MaybeUninit<ContextHandle> = MaybeUninit::uninit();

pub fn use_context() -> &'static Context {
    unsafe { CONTEXT.assume_init_ref() }
}

#[derive(Handle)]
pub struct Context {
    window: Window,
    gfx_ctx: GfxContextHandle,
    object_mgr: RefCell<ObjectManager>,
    screen_mgr: RefCell<ScreenManager>,
    render_mgr: RefCell<RenderManager>,
    shader_mgr: ShaderManager,
    time_mgr: RefCell<TimeManager>,
}

impl Context {
    pub fn new(window: Window, gfx_ctx: GfxContext, screen_width: u32, screen_height: u32) -> Self {
        let gfx_ctx = GfxContextHandle::new(gfx_ctx);
        let object_mgr = ObjectManager::new().into();
        let screen_mgr = ScreenManager::new(screen_width, screen_height).into();
        let render_mgr = RenderManager::new(
            gfx_ctx.clone(),
            PhysicalSize::new(screen_width, screen_height),
            DepthStencilMode::DepthOnly,
        )
        .into();
        let shader_mgr = ShaderManager::new(gfx_ctx.clone());
        let time_mgr = TimeManager::new().into();

        Self {
            window,
            gfx_ctx,
            object_mgr,
            screen_mgr,
            render_mgr,
            shader_mgr,
            time_mgr,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn gfx_ctx(&self) -> &GfxContextHandle {
        &self.gfx_ctx
    }

    pub fn object_mgr(&self) -> Ref<ObjectManager> {
        self.object_mgr.borrow()
    }

    pub fn object_mgr_mut(&self) -> RefMut<ObjectManager> {
        self.object_mgr.borrow_mut()
    }

    pub fn screen_mgr(&self) -> Ref<ScreenManager> {
        self.screen_mgr.borrow()
    }

    pub fn screen_mgr_mut(&self) -> RefMut<ScreenManager> {
        self.screen_mgr.borrow_mut()
    }

    pub fn render_mgr(&self) -> Ref<RenderManager> {
        self.render_mgr.borrow()
    }

    pub fn render_mgr_mut(&self) -> RefMut<RenderManager> {
        self.render_mgr.borrow_mut()
    }

    pub fn shader_mgr(&self) -> &ShaderManager {
        &self.shader_mgr
    }

    pub fn time_mgr(&self) -> Ref<TimeManager> {
        self.time_mgr.borrow()
    }

    pub fn time_mgr_mut(&self) -> RefMut<TimeManager> {
        self.time_mgr.borrow_mut()
    }
}

pub struct Engine {
    event_loop: EventLoop<()>,
    ctx: ContextHandle,
}

impl Engine {
    pub async fn new(config: EngineConfig) -> Result<Self, EngineInitError> {
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_visible(false)
            .with_title(config.title)
            .with_resizable(config.resizable)
            .with_inner_size(LogicalSize::new(config.width, config.height))
            .build(&event_loop)
            .unwrap();
        let gfx_ctx = GfxContext::new(&window).await?;
        let ctx = ContextHandle::new(Context::new(window, gfx_ctx, config.width, config.height));

        unsafe {
            CONTEXT.write(ctx.clone());
        }

        {
            let mut world_mgr = ctx.object_mgr_mut();
            let world = world_mgr.world_mut();
            world.register::<Camera>();
            world.register::<MeshRenderer>();
        }

        {
            let scale_factor = ctx.window.scale_factor();
            let physical_size =
                LogicalSize::new(config.width, config.height).to_physical(scale_factor);
            let mut screen_mgr = ctx.screen_mgr_mut();
            screen_mgr.update_scale_factor(scale_factor, physical_size);
            ctx.gfx_ctx().resize(physical_size);
        }

        Ok(Self { event_loop, ctx })
    }

    pub fn context(&self) -> ContextHandle {
        self.ctx.clone()
    }

    pub fn run(
        self,
        loop_mode: EngineLoopMode,
        target_fps: EngineTargetFps,
    ) -> Result<(), EngineExecError> {
        let mut update_camera_transform_buffer_system =
            UpdateCameraTransformBufferSystem::new(self.ctx.clone());
        let mut render_system = RenderSystem::new();

        self.ctx.window.set_visible(true);

        let window_id = self.ctx.window.id();
        let mut window_occluded = false;
        let mut target_frame_interval = TargetFrameInterval::new(
            match target_fps {
                EngineTargetFps::VSync => None,
                EngineTargetFps::MilliHertz(millihertz) => Some(millihertz),
                EngineTargetFps::Unlimited => None,
            },
            self.ctx.window(),
        );
        let mut last_frame_time = Instant::now();

        self.event_loop.run(move |event, _, control_flow| {
            *control_flow = match loop_mode {
                EngineLoopMode::Wait => ControlFlow::Wait,
                EngineLoopMode::Poll => ControlFlow::Poll,
            };

            match event {
                Event::MainEventsCleared => {
                    if loop_mode == EngineLoopMode::Wait {
                        return;
                    }

                    let now = Instant::now();

                    if now - last_frame_time < target_frame_interval.interval() {
                        return;
                    }

                    last_frame_time = now;

                    {
                        let mut time_mgr = self.ctx.time_mgr_mut();
                        time_mgr.update();
                    }

                    // TODO: Update here.

                    {
                        let mut world_mgr = self.ctx.object_mgr_mut();
                        let (world, object_hierarchy) = world_mgr.split_mut();
                        let transforms = world.read_component::<Transform>();
                        object_hierarchy.update_object_matrices(|entity| transforms.get(entity));
                    }

                    if window_occluded {
                        return;
                    }

                    update_camera_transform_buffer_system.run_now(&self.ctx.object_mgr().world());
                    render_system.run_now(&self.ctx.object_mgr().world());

                    return;
                }
                Event::RedrawRequested(id) if id == window_id => {
                    if loop_mode == EngineLoopMode::Poll {
                        return;
                    }

                    {
                        let mut time_mgr = self.ctx.time_mgr_mut();
                        time_mgr.update();
                    }

                    // TODO: Update here.

                    update_camera_transform_buffer_system.run_now(&self.ctx.object_mgr().world());
                    render_system.run_now(&self.ctx.object_mgr().world());

                    return;
                }
                Event::WindowEvent {
                    event: WindowEvent::Occluded(occluded),
                    window_id: id,
                } if id == window_id => {
                    window_occluded = occluded;

                    return;
                }
                Event::WindowEvent {
                    event: WindowEvent::KeyboardInput { input, .. },
                    window_id: id,
                } if id == window_id => {
                    // TODO: Handle keyboard input events here.

                    return;
                }
                Event::WindowEvent {
                    event: WindowEvent::CursorEntered { .. },
                    window_id: id,
                } if id == window_id => {
                    // TODO: Handle cursor entered event here.

                    return;
                }
                Event::WindowEvent {
                    event: WindowEvent::CursorLeft { .. },
                    window_id: id,
                } if id == window_id => {
                    // TODO: Handle cursor left event here.

                    return;
                }
                Event::WindowEvent {
                    event: WindowEvent::CursorMoved { position, .. },
                    window_id: id,
                } if id == window_id => {
                    // TODO: Handle cursor moved event here.

                    return;
                }
                Event::WindowEvent {
                    event: WindowEvent::MouseInput { button, state, .. },
                    window_id: id,
                } if id == window_id => {
                    // TODO: Handle mouse input event here.

                    return;
                }
                Event::WindowEvent {
                    event: WindowEvent::Resized(inner_size),
                    window_id: id,
                } if id == window_id => {
                    self.ctx.screen_mgr_mut().update_size(inner_size);

                    if inner_size.width == 0 || inner_size.height == 0 {
                        window_occluded = true;
                        return;
                    } else {
                        window_occluded = false;
                    }

                    self.ctx.gfx_ctx().resize(inner_size);
                    self.ctx.render_mgr_mut().resize(inner_size);

                    return;
                }
                Event::WindowEvent {
                    event:
                        WindowEvent::ScaleFactorChanged {
                            scale_factor,
                            new_inner_size,
                        },
                    window_id: id,
                } if id == window_id => {
                    target_frame_interval.update_window(&self.ctx.window);
                    self.ctx
                        .screen_mgr_mut()
                        .update_scale_factor(scale_factor, *new_inner_size);

                    if new_inner_size.width == 0 || new_inner_size.height == 0 {
                        window_occluded = true;
                        return;
                    } else {
                        window_occluded = false;
                    }

                    self.ctx.gfx_ctx().resize(*new_inner_size);
                    self.ctx.render_mgr_mut().resize(*new_inner_size);

                    return;
                }
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    window_id: id,
                } if id == window_id => {
                    *control_flow = ControlFlow::Exit;

                    return;
                }
                _ => return,
            }
        })
    }
}

pub struct EngineConfig {
    pub title: String,
    pub resizable: bool,
    pub width: u32,
    pub height: u32,
}

#[derive(Error, Debug)]
pub enum EngineInitError {
    #[error("winit os error: {0}")]
    WinitOsError(#[from] winit::error::OsError),
    #[error("winit external error: {0}")]
    WinitExternalError(#[from] winit::error::ExternalError),
    #[error("winit not supported error: {0}")]
    WinitNotSupportedError(#[from] winit::error::NotSupportedError),
    #[error("gfx context creation error: {0}")]
    GfxContextCreationError(#[from] GfxContextCreationError),
}

#[derive(Error, Debug)]
pub enum EngineExecError {
    #[error("gfx surface error: {0}")]
    SurfaceError(#[from] wgpu::SurfaceError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EngineLoopMode {
    Poll,
    Wait,
}

impl Default for EngineLoopMode {
    fn default() -> Self {
        Self::Poll
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EngineTargetFps {
    VSync,
    MilliHertz(NonZeroU32),
    Unlimited,
}

impl Default for EngineTargetFps {
    fn default() -> Self {
        Self::VSync
    }
}
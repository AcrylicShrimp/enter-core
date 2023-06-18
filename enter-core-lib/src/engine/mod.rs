pub mod gfx;
pub mod scripting;

use self::gfx::{GfxContext, GfxContextCreationError, ScreenManager};
use std::{
    cell::{Ref, RefCell, RefMut},
    sync::Arc,
    thread::sleep,
    time::Duration,
};
use thiserror::Error;
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

pub struct Context {
    window: Window,
    gfx_ctx: GfxContext,
    screen_mgr: RefCell<ScreenManager>,
}

impl Context {
    pub fn new(
        window: Window,
        gfx_context: GfxContext,
        screen_width: u32,
        screen_height: u32,
    ) -> Self {
        Self {
            window,
            gfx_ctx: gfx_context,
            screen_mgr: RefCell::new(ScreenManager::new(screen_width, screen_height)),
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn screen_mgr(&self) -> Ref<ScreenManager> {
        self.screen_mgr.borrow()
    }

    pub fn screen_mgr_mut(&self) -> RefMut<ScreenManager> {
        self.screen_mgr.borrow_mut()
    }
}

pub struct Engine {
    event_loop: EventLoop<()>,
    ctx: Arc<Context>,
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
        let ctx = Arc::new(Context::new(window, gfx_ctx, config.width, config.height));

        {
            let scale_factor = ctx.window.scale_factor();
            let mut screen_mgr = ctx.screen_mgr_mut();
            screen_mgr.update_scale_factor(
                scale_factor,
                LogicalSize::new(config.width, config.height).to_physical(scale_factor),
            );
            // TODO: Apply scale factor to the rendering context.
        }

        Ok(Self { event_loop, ctx })
    }

    pub fn run(self) -> ! {
        self.ctx.window.set_visible(true);

        let window_id = self.ctx.window.id();
        let mut window_occluded = false;

        self.event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;

            match event {
                Event::MainEventsCleared => {
                    // TODO: Render game here.

                    if window_occluded {
                        sleep(Duration::from_millis(60));
                    }

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
                    // TODO: Handle window resized event here.
                    self.ctx.screen_mgr_mut().update_size(inner_size);

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
                    // TODO: Handle scale factor changed event here.
                    self.ctx
                        .screen_mgr_mut()
                        .update_scale_factor(scale_factor, *new_inner_size);

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
pub enum EngineExecError {}

use std::{collections::HashMap, ops::ControlFlow, time::Instant};

use anyhow::Context as _;
use egui::{Context as EguiContext, Slider};
use egui_winit::State as EguiWinitState;
use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::EventLoop,
};

use common::{Vec2, Vec3};
use game_engine::{
    physics,
    rendering::{Line, Renderer},
    Engine,
};

use crate::{
    camera_controller::CameraController,
    game_context::GameScene,
    window::{Window, WindowMode},
};

pub struct Editor {
    engine: Engine,
    window: Window,
    state: EguiWinitState,
    egui_context: EguiContext,
    camera_controller: CameraController,
    scene: GameScene,
    last_frame: Instant,
}

impl Editor {
    pub fn new() -> anyhow::Result<(EventLoop<()>, Self)> {
        let event_loop = EventLoop::new();

        let icon = image::open("res/icon.png")
            .map(|i| i.into_rgba8())
            .ok()
            .and_then(|icon| {
                let (width, height) = icon.dimensions();
                winit::window::Icon::from_rgba(icon.into_raw(), width, height).ok()
            });
        if icon.is_none() {
            log::warn!("Could not load icon");
        }

        let window =
            Window::new(&event_loop, icon).with_context(|| "failed to open the winit window")?;
        let mut engine = Engine::new(
            Renderer::new(
                window.raw_window_handle(),
                window.inner_size(),
                [0.229, 0.507, 0.921],
            )
            .with_context(|| "failed to create the renderer")?,
        );

        let camera_controller = CameraController::new(
            10.0,
            0.1,
            Vec3::new(-16.0, 4.0, 1.0),
            Vec2::new(-0.3, 135f32.to_radians()),
        );

        let scene = GameScene::new(&mut engine).with_context(|| "failed to create the scene")?;
        let state = EguiWinitState::new(4096, window.winit_window());
        let egui_context = EguiContext::default();
        {
            let mut opts = egui_context.tessellation_options();
            opts.debug_paint_clip_rects = false;
        }

        Ok((
            event_loop,
            Self {
                engine,
                window,
                state,
                egui_context,
                camera_controller,
                scene,
                last_frame: Instant::now(),
            },
        ))
    }

    pub fn handle_event(&mut self, event: Event<()>) -> ControlFlow<()> {
        match event {
            Event::DeviceEvent { event, .. }
                if self.window.window_mode() == WindowMode::CameraMode =>
            {
                self.camera_controller.process_device_events(&event);
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if window_id == self.window.winit_window().id() => return ControlFlow::Break(()),
            Event::WindowEvent {
                event: WindowEvent::Resized(winit::dpi::PhysicalSize { width, height }),
                window_id,
            } if window_id == self.window.winit_window().id() => {
                log::info!("Resized window, new size: ({}, {})", width, height);
                self.engine.renderer.resize((width, height));
                self.window.update_size();
            }
            Event::WindowEvent { event, .. } => {
                self.camera_controller.process_window_events(&event);
                self.state.on_event(&self.egui_context, &event);
                if let WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(keycode),
                            state,
                            ..
                        },
                    ..
                } = event
                {
                    return self.handle_keyboard_input(keycode, state);
                }
            }
            Event::MainEventsCleared => self.window.winit_window().request_redraw(),
            Event::RedrawRequested(_) => {
                self.update();
            }
            _ => {}
        }
        ControlFlow::Continue(())
    }

    fn handle_keyboard_input(
        &mut self,
        keycode: VirtualKeyCode,
        state: ElementState,
    ) -> ControlFlow<()> {
        if let Some(map) = self
            .engine
            .world
            .resource_mut::<HashMap<VirtualKeyCode, bool>>()
        {
            map.insert(keycode, state == ElementState::Pressed);
        }

        if let Some(map) = self
            .engine
            .world
            .resource_mut::<HashMap<VirtualKeyCode, bool>>()
        {
            map.insert(keycode, state == ElementState::Pressed);
        }

        if state != ElementState::Pressed {
            return ControlFlow::Continue(());
        }

        match keycode {
            VirtualKeyCode::Q => {
                self.window
                    .set_window_mode(WindowMode::CursorMode)
                    .unwrap_or_else(|_| log::error!("Could not unlock cursor"));
            }
            VirtualKeyCode::E => {
                self.window
                    .set_window_mode(WindowMode::CameraMode)
                    .unwrap_or_else(|_| log::error!("Could not lock cursor"));
            }
            VirtualKeyCode::Escape => {
                return ControlFlow::Break(());
            }
            _ => {}
        }
        ControlFlow::Continue(())
    }

    fn update(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;

        self.engine.update();
        self.scene.update(&mut self.engine);

        self.camera_controller
            .update_camera(dt, &mut self.engine.renderer.camera);
        self.engine.renderer.update_camera();

        let pos_r = || -100.0..=100.0;
        let k_r = || 0.0..=1.0;

        let raw_input = self.state.take_egui_input(self.window.winit_window());
        let full_output = self.egui_context.run(raw_input, |ctx| {
            egui::Window::new("The Stuff®").auto_sized();
            /* .show(ctx, |ui| {
                ui.label("Light position");
                ui.spacing_mut().slider_width *= 2.0;
                ui.add(Slider::new(&mut self.scene.light.pos.x, pos_r()).text("x"));
                ui.add(Slider::new(&mut self.scene.light.pos.y, pos_r()).text("y"));
                ui.add(Slider::new(&mut self.scene.light.pos.z, pos_r()).text("z"));

                ui.label("scene.Light attenuation factors");
                ui.add(Slider::new(&mut self.scene.light.k_constant, k_r()).text("k_c"));
                ui.add(Slider::new(&mut self.scene.light.k_linear, k_r()).text("k_l"));
                ui.add(Slider::new(&mut self.scene.light.k_quadratic, k_r()).text("k_q"));

                ui.label("Light color");
                egui::widgets::color_picker::color_edit_button_rgb(
                    ui,
                    &mut self.scene.light.color,
                );

                if let Some(gravity) = self.engine.world.resource_mut::<physics::Gravity>() {
                    ui.label("Gravity");
                    ui.add(Slider::new(&mut gravity.0.y, -20.0..=20.0).text("k_q"));
                }
            });*/
        });

        let lines = self
            .engine
            .world
            .resource::<Vec<Line>>()
            .map(|v| v.as_slice())
            .unwrap_or(&[]);

        if self.window.inner_size() != (0, 0) {
            match self.engine.renderer.render(
                lines,
                &self.scene.lights,
                &self.egui_context,
                full_output,
                self.egui_context.pixels_per_point(),
                true,
            ) {
                Ok(_) => (),
                Err(e) => log::error!("Failed to render: {}", e),
            };
        }
    }
}

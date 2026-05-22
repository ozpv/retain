//! Contains all types and implementations related to the plugin's GUI

use crate::{
    RetainPluginMainThread, RetainPluginShared, params::RetainParams, window_size::WindowSize,
    window_type::WindowType,
};
use atomic_float::AtomicF32;
use baseview::{Size, WindowHandle, WindowOpenOptions, WindowScalePolicy, gl::GlConfig};
use clack_extensions::gui::{GuiApiType, GuiConfiguration, GuiSize, PluginGuiImpl, Window};
use clack_plugin::prelude::*;
use egui_baseview::{
    EguiWindow, GraphicsConfig, Queue,
    egui::{Align, CentralPanel, ComboBox, Context, Layout, Slider, Vec2, ViewportCommand},
};
use std::sync::{Arc, atomic::Ordering};

const DEFAULT_SIZE_X: f32 = 600.0;
const DEFAULT_SIZE_Y: f32 = 350.0;

pub struct AtomicVec2 {
    x: AtomicF32,
    y: AtomicF32,
}

impl Default for AtomicVec2 {
    fn default() -> Self {
        Self {
            x: AtomicF32::new(DEFAULT_SIZE_X),
            y: AtomicF32::new(DEFAULT_SIZE_Y),
        }
    }
}

impl AtomicVec2 {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_x(&self) -> f32 {
        self.x.load(Ordering::SeqCst)
    }

    pub fn set_x(&self, x: f32) {
        self.x.store(x, Ordering::SeqCst);
    }

    pub fn get_y(&self) -> f32 {
        self.y.load(Ordering::SeqCst)
    }

    pub fn set_y(&self, x: f32) {
        self.y.store(x, Ordering::SeqCst);
    }

    pub fn as_vec2(&self) -> Vec2 {
        Vec2 {
            x: self.get_x(),
            y: self.get_y(),
        }
    }
}

/// The EGUI application state
struct AppState {
    /// A handle to the shared params state
    params: Arc<RetainParams>,
    gui_size: Arc<AtomicVec2>,
}

impl AppState {
    /// Initializes a new [`AppState`] from the given shared params state handle
    pub fn new(params: &Arc<RetainParams>, gui_size: &Arc<AtomicVec2>) -> Self {
        Self {
            params: Arc::clone(params),
            gui_size: Arc::clone(gui_size),
        }
    }
}

/// GUI state that can be accessed directly by the main thread
pub struct RetainPluginGui {
    /// The handle to the baseview plugin window.
    handle: WindowHandle,
    /// The handle to the EGUI context
    egui_context: Context,
}

impl RetainPluginGui {
    /// Creates a new GUI window, and embeds it into the given `parent`.
    pub fn new(parent: Window<'_>, plugin_state: &RetainPluginShared) -> Self {
        let settings = WindowOpenOptions {
            title: "Retain".to_string(),
            size: Size::new(
                plugin_state.gui_size.get_x().into(),
                plugin_state.gui_size.get_y().into(),
            ),
            scale: WindowScalePolicy::SystemScaleFactor,
            gl_config: Some(GlConfig::default()),
        };

        let (tx, rx) = std::sync::mpsc::channel();

        let handle = EguiWindow::open_parented(
            &parent,
            settings,
            GraphicsConfig::default(),
            AppState::new(&plugin_state.params, &plugin_state.gui_size),
            move |ctx: &Context, _queue: &mut Queue, _state: &mut AppState| {
                tx.send(ctx.clone()).unwrap();
            },
            |ctx: &Context, _queue: &mut Queue, state: &mut AppState| {
                ctx.send_viewport_cmd(ViewportCommand::InnerSize(state.gui_size.as_vec2()));

                CentralPanel::default().show(ctx, |ui| {
                    ui.heading("Retain");

                    ui.horizontal(|ui| {
                        ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
                            let mut order = state.params.get_order();
                            let window_size = state.params.get_window_size().inner();

                            let slider = ui.add(
                                Slider::new(&mut order, 0..=window_size)
                                    .text("Order")
                                    .logarithmic(true),
                            );

                            if slider.changed() {
                                state.params.set_order(order);
                            }

                            let mut window_size = state.params.get_window_size();

                            ComboBox::from_label("Window Size")
                                .selected_text(window_size.as_str())
                                .show_ui(ui, |ui| {
                                    for size in WindowSize::iter() {
                                        let display = size.as_str();
                                        let inner = size.inner();

                                        if ui
                                            .selectable_value(&mut window_size, size, display)
                                            .changed()
                                        {
                                            state.params.set_window_size(inner.into());
                                        }
                                    }
                                });
                        });

                        ui.with_layout(Layout::top_down(Align::RIGHT), |ui| {
                            let mut complement = state.params.get_complement();

                            if ui.checkbox(&mut complement, "Complement").changed() {
                                state.params.set_complement(complement);
                            }

                            let window_type = state.params.get_window_type();
                            let selected = window_type.as_str();
                            let mut window_type = window_type.as_byte();

                            ComboBox::from_label("Window Function")
                                .selected_text(selected)
                                .show_ui(ui, |ui| {
                                    for function in WindowType::iter() {
                                        let display = function.as_str();
                                        let byte = function.as_byte();

                                        if ui
                                            .selectable_value(
                                                &mut window_type,
                                                function.as_byte(),
                                                display,
                                            )
                                            .changed()
                                        {
                                            state.params.set_window_type(byte.into());
                                        }
                                    }
                                });
                        });
                    });
                });
            },
        );

        let egui_context = rx.recv().unwrap();

        Self {
            handle,
            egui_context,
        }
    }

    /// Requests the UI to repaint itself, e.g. in response to events or parameter changes
    pub fn request_repaint(&self) {
        self.egui_context.request_repaint();
    }
}

impl Drop for RetainPluginGui {
    fn drop(&mut self) {
        self.handle.close();
    }
}

impl PluginGuiImpl for RetainPluginMainThread<'_> {
    fn is_api_supported(&mut self, configuration: GuiConfiguration) -> bool {
        configuration.api_type
            == GuiApiType::default_for_current_platform().expect("Unsupported platform")
            && !configuration.is_floating
    }

    fn get_preferred_api(&mut self) -> Option<GuiConfiguration<'_>> {
        Some(GuiConfiguration {
            api_type: GuiApiType::default_for_current_platform().expect("Unsupported platform"),
            is_floating: false,
        })
    }

    fn create(&mut self, configuration: GuiConfiguration) -> Result<(), PluginError> {
        if configuration.is_floating {
            return Err(PluginError::Message(
                "Invalid GUI configuration: this plugin does not support floating mode",
            ));
        }

        let supported_type =
            GuiApiType::default_for_current_platform().expect("Unsupported platform");

        if configuration.api_type != supported_type {
            return Err(PluginError::Message(
                "Invalid GUI configuration: unsupported API type",
            ));
        }

        Ok(())
    }

    fn destroy(&mut self) {
        let _ = self.gui.take();
    }

    fn set_scale(&mut self, _scale: f64) -> Result<(), PluginError> {
        Ok(())
    }

    fn get_size(&mut self) -> Option<GuiSize> {
        Some(GuiSize {
            width: self.shared.gui_size.get_x() as u32,
            height: self.shared.gui_size.get_y() as u32,
        })
    }

    fn can_resize(&mut self) -> bool {
        true
    }

    fn set_size(&mut self, size: GuiSize) -> Result<(), PluginError> {
        self.shared.gui_size.set_x(size.width as f32);
        self.shared.gui_size.set_y(size.height as f32);

        if let Some(gui) = &self.gui {
            gui.request_repaint();
        }

        Ok(())
    }

    fn set_parent(&mut self, window: Window) -> Result<(), PluginError> {
        self.gui = Some(RetainPluginGui::new(window, self.shared));

        Ok(())
    }

    fn set_transient(&mut self, _window: Window) -> Result<(), PluginError> {
        Ok(())
    }

    fn show(&mut self) -> Result<(), PluginError> {
        if let Some(gui) = &self.gui {
            gui.request_repaint();
        }

        Ok(())
    }

    fn hide(&mut self) -> Result<(), PluginError> {
        if let Some(gui) = &self.gui {
            gui.request_repaint();
        }

        Ok(())
    }
}

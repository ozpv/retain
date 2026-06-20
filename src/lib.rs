#![allow(clippy::cast_precision_loss)]

mod retain;
mod window_function;
mod window_size;
mod window_type;
mod windowed_fft;

use crate::{
    retain::retain_top_n_magnitudes, window_size::WindowSize, window_type::WindowType,
    windowed_fft::WindowedRealFft,
};
use egui::{Align, CentralPanel, ComboBox, Layout, Slider, Vec2};
use nice_plug::prelude::*;
use nice_plug_egui::{
    EguiSettings, EguiState, create_egui_editor, resizable_window::ResizableWindow,
};
use rtrb::RingBuffer;
use std::sync::Arc;

pub const DEFAULT_WINDOW_TYPE: WindowType = WindowType::Hann;

const MIN_WINDOW_WIDTH: u32 = 600;
const MIN_WINDOW_HEIGHT: u32 = 350;

const GUI_TO_AUDIO_CHANNEL_CAPACITY: usize = 128;

pub struct Retain {
    params: Arc<RetainParams>,
    msg_channel: AudioMsgChannel,
    initial_gui_state: Option<GuiState>,
    fft_left: WindowedRealFft,
    fft_right: WindowedRealFft,
}

impl Default for Retain {
    fn default() -> Self {
        let (to_audio_tx, from_gui_rx) = RingBuffer::new(GUI_TO_AUDIO_CHANNEL_CAPACITY);

        let params = RetainParams::default();

        let fft_left = WindowedRealFft::new((params.window_size.value() as usize).into());
        let fft_right = WindowedRealFft::new((params.window_size.value() as usize).into());

        Self {
            params: Arc::new(params),
            msg_channel: AudioMsgChannel { from_gui_rx },
            initial_gui_state: Some(GuiState {
                msg_channel: GuiMsgChannel { to_audio_tx },
            }),
            fft_left,
            fft_right,
        }
    }
}

#[derive(Params)]
pub struct RetainParams {
    /// This allows the resizing of the plugin to be restored
    #[persist = "editor-state"]
    editor_state: Arc<EguiState>,

    /// the number of top frequencies to keep
    #[id = "order"]
    pub order: IntParam,

    /// the window size of the fft
    /// this effects the frequency resolution and can make for cool effects
    #[id = "window_size"]
    pub window_size: IntParam,

    /// the type of window function used to reduce spectral leakage or clicking sounds
    /// changing this makes a big difference
    #[id = "window_type"]
    pub window_type: EnumParam<WindowType>,

    /// whether the plugin retains or removes the highest magnitudes
    #[id = "complement"]
    pub complement: BoolParam,
}

impl Default for RetainParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(MIN_WINDOW_WIDTH, MIN_WINDOW_HEIGHT),
            order: IntParam::new("Order", 1, IntRange::Linear { min: 0, max: 32768 }),
            window_size: IntParam::new(
                "Window Size",
                1024,
                IntRange::Linear {
                    min: 256,
                    max: 32768,
                },
            )
            .non_automatable()
            .hide(),
            window_type: EnumParam::new("Window Type", WindowType::Hann)
                .non_automatable()
                .hide(),
            complement: BoolParam::new("Complement", false).non_automatable().hide(),
        }
    }
}

pub struct GuiState {
    msg_channel: GuiMsgChannel,
}

#[derive(Debug)]
pub enum GuiToAudioMsg {
    WindowSizeUpdate,
    WindowTypeUpdate,
}

pub struct GuiMsgChannel {
    to_audio_tx: rtrb::Producer<GuiToAudioMsg>,
}

pub struct AudioMsgChannel {
    from_gui_rx: rtrb::Consumer<GuiToAudioMsg>,
}

impl Plugin for Retain {
    const NAME: &'static str = "Retain";
    const VENDOR: &'static str = "haemolacriaa";
    const URL: &'static str = "https://git.haemolacriaa.com/retain/";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];
    const SAMPLE_ACCURATE_AUTOMATION: bool = false;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        _buffer_config: &BufferConfig,
        context: &mut impl InitContext<Self>,
    ) -> bool {
        context.set_latency_samples(self.params.window_size.value().cast_unsigned());

        true
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let params = self.params.clone();
        let egui_state = params.editor_state.clone();

        create_egui_editor(
            self.params.editor_state.clone(),
            self.initial_gui_state.take().unwrap(),
            EguiSettings::default(),
            |_egui_ctx, _queue, _gui_state| {},
            move |ui, setter, _queue, gui_state| {
                ResizableWindow::new("retainwindow")
                    .min_size(Vec2::new(MIN_WINDOW_WIDTH as f32, MIN_WINDOW_HEIGHT as f32))
                    .show(ui, egui_state.as_ref(), |ui| {
                        CentralPanel::default().show_inside(ui, |ui| {
                            ui.heading("Retain");

                            ui.horizontal(|ui| {
                                ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
                                    let mut order = params.order.value();
                                    let window_size = params.window_size.value();

                                    let slider = ui.add(
                                        Slider::new(&mut order, 0..=window_size)
                                            .text("Order")
                                            .logarithmic(true),
                                    );

                                    if slider.changed() {
                                        setter.set_parameter(&params.order, order);
                                    }

                                    let mut window_size = Into::<WindowSize>::into(
                                        params.window_size.value() as usize,
                                    );

                                    ComboBox::from_label("Window Size")
                                        .selected_text(window_size.as_str())
                                        .show_ui(ui, |ui| {
                                            for size in WindowSize::iter() {
                                                let display = size.as_str();
                                                let inner = size.inner();

                                                if ui
                                                    .selectable_value(
                                                        &mut window_size,
                                                        size,
                                                        display,
                                                    )
                                                    .changed()
                                                {
                                                    setter.set_parameter(
                                                        &params.window_size,
                                                        inner as i32,
                                                    );

													if let Err(e) = gui_state.msg_channel.to_audio_tx.push(GuiToAudioMsg::WindowSizeUpdate) {
														nice_error!("Failed to send message to audio thread: {}", e);
													}
                                                }
                                            }
                                        });
                                });

                                ui.with_layout(Layout::top_down(Align::RIGHT), |ui| {
                                    let mut complement = params.complement.value();

                                    if ui.checkbox(&mut complement, "Complement").changed() {
                                        setter.set_parameter(&params.complement, complement);
                                    }

                                    let window_type = params.window_type.value();
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
                                                    setter.set_parameter(
                                                        &params.window_type,
                                                        byte.into(),
                                                    );

													if let Err(e) = gui_state.msg_channel.to_audio_tx.push(GuiToAudioMsg::WindowTypeUpdate) {
														nice_error!("Failed to send message to audio thread: {}", e);
													}
                                                }
                                            }
                                        });
                                });
                            });
                        });
                    });
            },
        )
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
		// this is to prevent multiple updates from slowing this thread down if it somehow happens
        let mut previously_updated_window_size = false;
        let mut previously_updated_window_type = false;

        while let Ok(msg) = self.msg_channel.from_gui_rx.pop() {
            match msg {
                GuiToAudioMsg::WindowSizeUpdate => {
                    if previously_updated_window_size {
                        continue;
                    }

                    let window_size = self.params.window_size.value() as usize;

                    self.fft_left.window_size(window_size.into());
                    self.fft_right.window_size(window_size.into());

                    context.set_latency_samples(window_size as u32);

                    previously_updated_window_size = true;
                }
                GuiToAudioMsg::WindowTypeUpdate => {
                    if previously_updated_window_type {
                        continue;
                    }

                    let window_type = self.params.window_type.value();

                    self.fft_left.window_function(&window_type);
                    self.fft_right.window_function(&window_type);

                    previously_updated_window_type = true;
                }
            }
        }
		
        if let [left, right] = buffer.as_slice() {
            for sample in left.iter_mut() {
                if self.fft_left.push_back_input(*sample) {
                    self.fft_left.forward();
					
					let order = self.params.order.value() as usize;
					let complement = self.params.complement.value();

                    retain_top_n_magnitudes(self.fft_left.get_spectrum(), order, complement);

                    self.fft_left.inverse();
                    self.fft_left.clear_input();
                }

                *sample = self.fft_left.pop_front_output();
            }

            for sample in right.iter_mut() {
                if self.fft_right.push_back_input(*sample) {
                    self.fft_right.forward();
					
					let order = self.params.order.value() as usize;
					let complement = self.params.complement.value();

                    retain_top_n_magnitudes(self.fft_right.get_spectrum(), order, complement);

                    self.fft_right.inverse();
                    self.fft_right.clear_input();
                }

                *sample = self.fft_right.pop_front_output();
            }
        }

        ProcessStatus::Normal
    }

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }
}

impl ClapPlugin for Retain {
    const CLAP_ID: &'static str = "com.haemolacriaa.retain";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("Retain is a real-time audio plugin that retains the nth largest magnitude frequencies in a signal");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Filter,
        ClapFeature::Stereo,
    ];
}

impl Vst3Plugin for Retain {
    const VST3_CLASS_ID: [u8; 16] = *b"Retainhaemolacir";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Fx,
        Vst3SubCategory::Filter,
        Vst3SubCategory::Stereo,
    ];
}

//nice_export_clap!(Retain);
nice_export_vst3!(Retain);

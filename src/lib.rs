use crate::{
    audio::RetainPluginAudioProcessor,
    gui::{AtomicVec2, RetainPluginGui},
    params::RetainParams,
};
use clack_extensions::{
    audio_ports::PluginAudioPorts,
    gui::PluginGui,
    latency::{PluginLatency, PluginLatencyImpl},
    params::PluginParams,
    state::PluginState,
};
use clack_plugin::{plugin::features::{AUDIO_EFFECT, STEREO}, prelude::*};
use std::sync::Arc;

mod audio;
mod gui;
mod params;
mod retain;
mod window_function;
mod window_size;
mod window_type;
mod windowed_fft;

/// 🎧 The main plugin entry point for the Retain audio effect.
///
/// Clean, readable, and focused on the plugin's structure.
/// This should feel simple enough that the whole plugin could be
/// prototyped later in JavaScript or a web-based audio stack. 🚀
pub struct RetainPlugin;

impl Plugin for RetainPlugin {
    type AudioProcessor<'a> = RetainPluginAudioProcessor<'a>;
    type Shared<'a> = RetainPluginShared<'a>;
    type MainThread<'a> = RetainPluginMainThread<'a>;

    fn declare_extensions(
        builder: &mut PluginExtensions<Self>,
        _shared: Option<&RetainPluginShared>,
    ) {
        builder
            .register::<PluginAudioPorts>()
            .register::<PluginParams>()
            .register::<PluginState>()
            .register::<PluginLatency>()
            .register::<PluginGui>();
    }
}

impl DefaultPluginFactory for RetainPlugin {
    fn get_descriptor() -> PluginDescriptor {
        PluginDescriptor::new("com.haemolacriaa.retain", "Retain")
            .with_description("Retains only the nth largest magnitude frequencies in a signal")
            .with_version("0.1.0-pre")
            .with_vendor("haemolacriaa")
            .with_features([AUDIO_EFFECT, STEREO])
    }

    fn new_shared(host: HostSharedHandle<'_>) -> Result<Self::Shared<'_>, PluginError> {
        Ok(Self::Shared::new(host))
    }

    fn new_main_thread<'a>(
        _host: HostMainThreadHandle<'a>,
        shared: &'a Self::Shared<'a>,
    ) -> Result<Self::MainThread<'a>, PluginError> {
        Ok(Self::MainThread { shared, gui: None })
    }
}

/// Shared state between audio and main threads.
/// This keeps parameter data and GUI layout state synchronized.
pub struct RetainPluginShared<'a> {
    params: Arc<RetainParams>,
    gui_size: Arc<AtomicVec2>,
    host: HostSharedHandle<'a>,
}

impl<'a> RetainPluginShared<'a> {
    fn new(host: HostSharedHandle<'a>) -> Self {
        Self {
            params: Arc::new(RetainParams::new()),
            gui_size: Arc::new(AtomicVec2::new()),
            host,
        }
    }
}

impl<'a> PluginShared<'a> for RetainPluginShared<'a> {}

/// Main thread state and GUI ownership.
pub struct RetainPluginMainThread<'a> {
    shared: &'a RetainPluginShared<'a>,
    gui: Option<RetainPluginGui>,
}

impl<'a> PluginMainThread<'a, RetainPluginShared<'a>> for RetainPluginMainThread<'a> {
    fn on_main_thread(&mut self) {
        if let Some(gui) = &self.gui {
            gui.request_repaint();
        }
    }
}

impl PluginLatencyImpl for RetainPluginMainThread<'_> {
    fn get(&mut self) -> u32 {
        self.shared.params.get_window_size().inner() as u32
    }
}

clack_export_entry!(SinglePluginEntry<RetainPlugin>);

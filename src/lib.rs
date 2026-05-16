use crate::{
    audio::RetainPluginAudioProcessor,
    gui::RetainPluginGui,
    params::{RetainParamsLocal, RetainParamsShared},
};
use clack_extensions::{
    audio_ports::PluginAudioPorts,
    gui::PluginGui,
    latency::{PluginLatency, PluginLatencyImpl},
    params::PluginParams,
    state::PluginState,
};
use clack_plugin::{
    plugin::features::{AUDIO_EFFECT, STEREO},
    prelude::*,
};
use std::sync::Arc;

mod audio;
mod gui;
mod params;
mod retain;
mod window_function;
mod window_size;
mod windowed_fft;

/// The type that represents our plugin in Clack.
///
/// This is what implements the [`Plugin`] trait, where all the other subtypes are attached.
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
            .with_description("Retains only the nth largest magnitude frequencies in the signal")
            .with_features([AUDIO_EFFECT, STEREO])
    }

    fn new_shared(host: HostSharedHandle<'_>) -> Result<Self::Shared<'_>, PluginError> {
        Ok(RetainPluginShared::new(host))
    }

    fn new_main_thread<'a>(
        host: HostMainThreadHandle<'a>,
        shared: &'a Self::Shared<'a>,
    ) -> Result<Self::MainThread<'a>, PluginError> {
        Ok(Self::MainThread {
            shared,
            params: RetainParamsLocal::new(&shared.params),
            host_main_thread: host,
            gui: None,
        })
    }
}

/// The plugin data that gets shared between the Main Thread and the Audio Thread.
pub struct RetainPluginShared<'a> {
    /// The plugin's parameter values.
    params: Arc<RetainParamsShared>,
    /// A handle to the host
    host: HostSharedHandle<'a>,
}

impl<'a> RetainPluginShared<'a> {
    fn new(host: HostSharedHandle<'a>) -> Self {
        RetainPluginShared {
            params: Arc::new(RetainParamsShared::new()),
            host,
        }
    }
}

impl<'a> PluginShared<'a> for RetainPluginShared<'a> {}

/// The data that belongs to the main thread of our plugin.
pub struct RetainPluginMainThread<'a> {
    /// The local state of the parameters
    params: RetainParamsLocal,
    /// A reference to the plugin's shared data.
    shared: &'a RetainPluginShared<'a>,
    host_main_thread: HostMainThreadHandle<'a>,
    /// The plugin's GUI state and context
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
        self.params.get_window_size() as u32
    }
}

clack_export_entry!(SinglePluginEntry<RetainPlugin>);

use crate::{
    audio::RetainPluginAudioProcessor,
    gui::RetainPluginGui,
    params::{RetainParamsLocal, RetainParamsShared},
};
use clack_extensions::{audio_ports::*, gui::PluginGui, params::*, state::PluginState};
use clack_plugin::prelude::*;
use std::sync::Arc;

mod audio;
mod gui;
mod params;
mod window_size;

/// The type that represents our plugin in Clack.
///
/// This is what implements the [`Plugin`] trait, where all the other subtypes are attached.
pub struct RetainPlugin;

impl Plugin for RetainPlugin {
    type AudioProcessor<'a> = RetainPluginAudioProcessor<'a>;
    type Shared<'a> = RetainPluginShared;
    type MainThread<'a> = RetainPluginMainThread<'a>;

    fn declare_extensions(
        builder: &mut PluginExtensions<Self>,
        _shared: Option<&RetainPluginShared>,
    ) {
        builder
            .register::<PluginAudioPorts>()
            .register::<PluginParams>()
            .register::<PluginState>()
            .register::<PluginGui>();
    }
}

impl DefaultPluginFactory for RetainPlugin {
    fn get_descriptor() -> PluginDescriptor {
        use clack_plugin::plugin::features::*;

        PluginDescriptor::new("org.haemolacriaa.retain", "Retain")
            .with_features([AUDIO_EFFECT, STEREO])
    }

    fn new_shared(_host: HostSharedHandle<'_>) -> Result<Self::Shared<'_>, PluginError> {
        Ok(RetainPluginShared {
            params: Arc::new(RetainParamsShared::new()),
        })
    }

    fn new_main_thread<'a>(
        _host: HostMainThreadHandle<'a>,
        shared: &'a Self::Shared<'a>,
    ) -> Result<Self::MainThread<'a>, PluginError> {
        Ok(Self::MainThread {
            shared,
            params: RetainParamsLocal::new(&shared.params),
            gui: None,
        })
    }
}

/// The plugin data that gets shared between the Main Thread and the Audio Thread.
pub struct RetainPluginShared {
    /// The plugin's parameter values.
    params: Arc<RetainParamsShared>,
}

impl PluginShared<'_> for RetainPluginShared {}

/// The data that belongs to the main thread of our plugin.
pub struct RetainPluginMainThread<'a> {
    /// The local state of the parameters
    params: RetainParamsLocal,
    /// A reference to the plugin's shared data.
    shared: &'a RetainPluginShared,
    /// The plugin's GUI state and context
    gui: Option<RetainPluginGui>,
}

impl<'a> PluginMainThread<'a, RetainPluginShared> for RetainPluginMainThread<'a> {
    fn on_main_thread(&mut self) {
        if let Some(gui) = &self.gui {
            gui.request_repaint();
        }
    }
}

clack_export_entry!(SinglePluginEntry<RetainPlugin>);

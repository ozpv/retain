use crate::{
    RetainPluginMainThread, RetainPluginShared, params::RetainParams,
    retain::retain_top_n_magnitudes, window_size::WindowSize, window_type::WindowType,
    windowed_fft::WindowedRealFft,
};
use clack_extensions::{
    audio_ports::{
        AudioPortFlags, AudioPortInfo, AudioPortInfoWriter, AudioPortType, PluginAudioPortsImpl,
    },
    latency::HostLatency,
    params::PluginAudioProcessorParams,
};
use clack_plugin::prelude::*;
use std::sync::Arc;

/// Our plugin's audio processor. It lives in the audio thread.
///
/// It receives parameter events, and process a stereo audio signal by operating on the given audio
/// buffer.
pub struct RetainPluginAudioProcessor<'a> {
    /// The local state of the parameters
    params: Arc<RetainParams>,
    /// A reference to the plugin's shared data.
    shared: &'a RetainPluginShared<'a>,
    /// Our handle to the host
    host: HostAudioProcessorHandle<'a>,
    /// Fft for the left channel
    fft_left: WindowedRealFft,
    /// Fft for the right channel
    fft_right: WindowedRealFft,
    /// Previous window type
    prev_window_type: Option<WindowType>,
    /// Previous window size
    prev_window_size: Option<WindowSize>,
}

impl<'a> PluginAudioProcessor<'a, RetainPluginShared<'a>, RetainPluginMainThread<'a>>
    for RetainPluginAudioProcessor<'a>
{
    fn activate(
        host: HostAudioProcessorHandle<'a>,
        _main_thread: &mut RetainPluginMainThread,
        shared: &'a RetainPluginShared<'a>,
        _audio_config: PluginAudioConfiguration,
    ) -> Result<Self, PluginError> {
        let params = Arc::clone(&shared.params);

        let fft_left = WindowedRealFft::new(params.get_window_size().into());
        let fft_right = WindowedRealFft::new(params.get_window_size().into());

        // This is where we would allocate intermediate buffers and such if we needed them.
        Ok(Self {
            params,
            shared,
            host,
            fft_left,
            fft_right,
            prev_window_size: None,
            prev_window_type: None,
        })
    }

    fn process(
        &mut self,
        _process: Process,
        mut audio: Audio,
        events: Events,
    ) -> Result<ProcessStatus, PluginError> {
        // First, we have to make a few sanity checks.
        // We want at least a single input/output port pair, which contains channels of `f32`
        // audio sample data.
        let mut port_pair = audio
            .port_pair(0)
            .ok_or(PluginError::Message("No input/output ports found"))?;

        let mut output_channels = port_pair
            .channels()?
            .into_f32()
            .ok_or(PluginError::Message("Expected f32 input/output"))?;

        let mut channel_buffers = [None, None];

        // Extract the buffer slices that we need, while making sure they are paired correctly and
        // check for either in-place or separate buffers.
        for (pair, buf) in output_channels.iter_mut().zip(&mut channel_buffers) {
            *buf = match pair {
                ChannelPair::InputOnly(_) | ChannelPair::OutputOnly(_) => None,
                ChannelPair::InPlace(b) => Some(b),
                ChannelPair::InputOutput(i, o) => {
                    o.copy_from_slice(i);
                    Some(o)
                }
            }
        }

        // update window size and latency if it has changed
        // updates fft window sizes only if necessary
        let window_size = self.params.get_window_size();
        if self.prev_window_size != Some(window_size.clone()) {
            self.fft_left.window_size(window_size.clone());
            self.fft_right.window_size(window_size.clone());

            if let Some(latency) = self.shared.host.get_extension::<HostLatency>() {
                // should be safe
                let mut main = unsafe { self.shared.host.as_main_thread_unchecked() };

                latency.changed(&mut main);
                // self.shared.host.request_restart();
            }
        }

        // update change in window type if needed
        let window_type = self.params.get_window_type();
        if self.prev_window_type != Some(window_type.clone()) {
            self.fft_left.window_function(&window_type);
            self.fft_right.window_function(&window_type);
        }

        // Now let's process the audio, while splitting the processing in batches between each
        // sample-accurate event.
        for event_batch in events.input.batch() {
            // Process all param events in this batch
            for event in event_batch.events() {
                self.params.handle_event(event);
            }

            // Get the parameters after all changes have been handled.
            let order = self.params.get_order();
            let complement = self.params.get_complement();

            // process samples in place here
            if let [Some(left), Some(right)] = &mut channel_buffers {
                for sample in left.iter_mut() {
                    if self.fft_left.push_back_input(*sample) {
                        self.fft_left.forward();

                        retain_top_n_magnitudes(self.fft_left.get_spectrum(), order, complement);

                        self.fft_left.inverse();
                        self.fft_left.clear_input();
                    }

                    *sample = self.fft_left.pop_front_output();
                }

                for sample in right.iter_mut() {
                    if self.fft_right.push_back_input(*sample) {
                        self.fft_right.forward();

                        retain_top_n_magnitudes(self.fft_right.get_spectrum(), order, complement);

                        self.fft_right.inverse();
                        self.fft_right.clear_input();
                    }

                    *sample = self.fft_right.pop_front_output();
                }
            }
        }

        self.prev_window_size = Some(window_size);
        self.prev_window_type = Some(window_type);

        // Publish any parameter changes we may have received back to the GUI.
        // Request the on-main-thread callback, which we use to refresh the UI if it is open
        self.host.request_callback();

        Ok(ProcessStatus::ContinueIfNotQuiet)
    }
}

impl PluginAudioPortsImpl for RetainPluginMainThread<'_> {
    fn count(&mut self, _is_input: bool) -> u32 {
        1
    }

    fn get(&mut self, index: u32, _is_input: bool, writer: &mut AudioPortInfoWriter) {
        if index == 0 {
            writer.set(&AudioPortInfo {
                id: ClapId::new(0),
                name: b"main",
                channel_count: 2,
                flags: AudioPortFlags::IS_MAIN,
                port_type: Some(AudioPortType::STEREO),
                in_place_pair: None,
            });
        }
    }
}

impl PluginAudioProcessorParams for RetainPluginAudioProcessor<'_> {
    fn flush(
        &mut self,
        input_parameter_changes: &InputEvents,
        _output_parameter_changes: &mut OutputEvents,
    ) {
        for event in input_parameter_changes {
            self.params.handle_event(event);
        }
    }
}

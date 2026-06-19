use crate::{
    RetainPluginMainThread, RetainPluginShared, params::RetainParams,
    retain::retain_top_n_magnitudes, window_size::WindowSize, window_type::WindowType,
    windowed_fft::WindowedRealFft,
};
use clack_extensions::{
    audio_ports::{AudioPortFlags, AudioPortInfo, AudioPortInfoWriter, AudioPortType, PluginAudioPortsImpl},
    latency::HostLatency,
    params::PluginAudioProcessorParams,
};
use clack_plugin::prelude::*;
use std::sync::Arc;

/// 🎚️ Audio processing happens here.
///
/// This processor is intentionally easy to read with lots of comments.
/// It is structured so the same flow could later live in JavaScript,
/// WebAudio, or a WASM port with very little rework. ✨
pub struct RetainPluginAudioProcessor<'a> {
    params: Arc<RetainParams>,
    shared: &'a RetainPluginShared<'a>,
    host: HostAudioProcessorHandle<'a>,
    fft_left: WindowedRealFft,
    fft_right: WindowedRealFft,
    prev_window_type: Option<WindowType>,
    prev_window_size: Option<WindowSize>,
}

impl<'a> RetainPluginAudioProcessor<'a> {
    fn update_settings(&mut self) {
        let window_size = self.params.get_window_size();
        if self.prev_window_size != Some(window_size.clone()) {
            // ✨ keep the FFT window state in sync with the UI.
            // This is nice and obvious, and you can imagine the same
            // logic in JS using a settings object and a reconfigure call.
            self.fft_left.window_size(window_size.clone());
            self.fft_right.window_size(window_size.clone());

            if let Some(latency) = self.shared.host.get_extension::<HostLatency>() {
                let mut main = unsafe { self.shared.host.as_main_thread_unchecked() };
                latency.changed(&mut main);
            }

            self.prev_window_size = Some(window_size);
        }

        let window_type = self.params.get_window_type();
        if self.prev_window_type != Some(window_type.clone()) {
            // 🧠 update shape of the window function only when it changes.
            self.fft_left.window_function(&window_type);
            self.fft_right.window_function(&window_type);
            self.prev_window_type = Some(window_type);
        }
    }

    fn process_channel(&mut self, channel: &mut [f32], fft: &mut WindowedRealFft) {
        let order = self.params.get_order();
        let complement = self.params.get_complement();

        for sample in channel.iter_mut() {
            if fft.push_back_input(*sample) {
                // 🚀 process one FFT window, then choose the top magnitudes.
                fft.forward();
                retain_top_n_magnitudes(fft.get_spectrum(), order, complement);
                fft.inverse();
                fft.clear_input();
            }
            *sample = fft.pop_front_output();
        }
    }
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
        let initial_window_size = params.get_window_size();

        Ok(Self {
            params,
            shared,
            host,
            fft_left: WindowedRealFft::new(initial_window_size),
            fft_right: WindowedRealFft::new(initial_window_size),
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
        let mut port_pair = audio
            .port_pair(0)
            .ok_or(PluginError::Message("No input/output ports found"))?;

        let mut output_channels = port_pair
            .channels()?
            .into_f32()
            .ok_or(PluginError::Message("Expected f32 input/output"))?;

        let mut channel_buffers = [None, None];
        for (pair, slot) in output_channels.iter_mut().zip(&mut channel_buffers) {
            *slot = match pair {
                ChannelPair::InPlace(buffer) => Some(buffer),
                ChannelPair::InputOutput(input, output) => {
                    output.copy_from_slice(input);
                    Some(output)
                }
                _ => None,
            };
        }

        self.update_settings();

        for event_batch in events.input.batch() {
            for event in event_batch.events() {
                self.params.handle_event(event);
            }

            if let [Some(left), Some(right)] = &mut channel_buffers {
                let range = event_batch.sample_bounds();
                self.process_channel(&mut left[range], &mut self.fft_left);
                self.process_channel(&mut right[range], &mut self.fft_right);
            }
        }

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

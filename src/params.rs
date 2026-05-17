//! Contains all types and implementations related to parameter management.

use crate::{RetainPluginMainThread, window_size::WindowSize};
use clack_extensions::{
    params::{
        ParamDisplayWriter, ParamInfo, ParamInfoFlags, ParamInfoWriter, PluginMainThreadParams,
    },
    state::PluginStateImpl,
};
use clack_plugin::{
    events::{event_types::ParamValueEvent, spaces::CoreEventSpace},
    prelude::*,
    stream::{InputStream, OutputStream},
    utils::Cookie,
};
use prost::Message;
use std::{
    ffi::CStr,
    fmt::Write as _,
    io::{Read, Write as _},
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

/// The default value of the order parameter.
const DEFAULT_ORDER: usize = 1;
pub const DEFAULT_WINDOW_SIZE: WindowSize = WindowSize::Size4096;

/// A struct that manages the parameters for our plugin.
///
/// This struct will be used both by the [`RetainPluginMainThread`] (which the host will use
/// to query the value of our parameters), and by the [`RetainPluginAudioProcessor`], which will
/// actually modulate the audio samples.
pub struct RetainParamsShared {
    /// The current value of the order parameter.
    order: AtomicUsize,
    /// Window size of FFT
    window_size: AtomicUsize,
    /// Whether a gesture is currently active or not on the parameters
    has_gesture: AtomicBool,
}

impl RetainParamsShared {
    /// The unique identifier for the order parameter.
    pub const PARAM_ORDER_ID: ClapId = ClapId::new(1);
    pub const PARAM_WINDOW_SIZE_ID: ClapId = ClapId::new(2);

    /// Initializes the shared parameter value.
    pub fn new() -> Self {
        Self {
            order: AtomicUsize::new(DEFAULT_ORDER),
            window_size: AtomicUsize::new(DEFAULT_WINDOW_SIZE.into_inner()),
            has_gesture: AtomicBool::new(false),
        }
    }

    pub fn get_window_size(&self) -> usize {
        self.window_size.load(Ordering::Relaxed)
    }
}

/// A param gesture change type
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum GestureChange {
    /// User started changing a parameter
    Begin,
    /// User finished changing a parameter
    End,
    /// User both started and finished changing a parameter during the current block
    Both,
}

/// The local-side of parameter state.
///
/// This state is local to the current thread (whether it is the main-thread, the UI or the audio
/// thread), it is not shared directly with the others.
///
/// This allows us to both check for differences, and to only update parameter state when we want
/// to.
pub struct RetainParamsLocal {
    /// The local value of the order parameter.
    order: usize,
    /// The local value of the window size paramter.
    window_size: usize,
    /// Whether the user is currently changing the parameter, that we know of
    pub has_gesture: bool,
}

impl RetainParamsLocal {
    /// Initializes a new local state from the current shared state.
    pub fn new(shared: &RetainParamsShared) -> Self {
        Self {
            order: shared.order.load(Ordering::Relaxed),
            window_size: shared.window_size.load(Ordering::Relaxed),
            has_gesture: shared.has_gesture.load(Ordering::Relaxed),
        }
    }

    /// Returns the current local order value.
    #[inline]
    pub fn get_order(&self) -> usize {
        self.order
    }

    /// Sets the current local order to `value`.
    ///
    /// It is clamped to the range `0..=100_000`.
    #[inline]
    pub fn set_order(&mut self, value: usize) {
        self.order = value;
    }

    /// Returns the current window size value.
    #[inline]
    pub fn get_window_size(&self) -> usize {
        self.window_size
    }

    /// Sets the current local window size to `value`.
    #[inline]
    pub fn set_window_size(&mut self, value: usize) {
        self.window_size = value;
    }

    /// Fetch updates from the `shared` state.
    ///
    /// If any of the parameters have been updated, this returns `true`.
    #[inline]
    pub fn fetch_updates(&mut self, shared: &RetainParamsShared) -> bool {
        let latest_order = shared.order.load(Ordering::Relaxed);
        let latest_window_size = shared.window_size.load(Ordering::Relaxed);

        if latest_order != self.order || latest_window_size != self.order {
            self.order = latest_order;
            self.window_size = latest_window_size;

            true
        } else {
            false
        }
    }

    /// Pushes the local parameter values to the `shared` state.
    ///
    /// If the values were different and an actual update occurred, this returns `true`.
    #[inline]
    pub fn push_updates(&self, shared: &RetainParamsShared) -> bool {
        let previous_order = shared.order.swap(self.order, Ordering::Relaxed);
        let previous_window_size = shared.window_size.swap(self.window_size, Ordering::Relaxed);

        previous_order != self.order || previous_window_size != self.window_size
    }

    /// Pushes the local gesture state to the `shared` state.
    #[inline]
    pub fn push_gesture(&self, shared: &RetainParamsShared) {
        shared
            .has_gesture
            .store(self.has_gesture, Ordering::Relaxed);
    }

    /// Fetches updates to the gesture state.
    ///
    /// If a gesture state changed occurred, it is returned.
    /// If the gesture did not change from the last update, `None` is returned.
    #[inline]
    pub fn fetch_gesture(
        &mut self,
        shared: &RetainParamsShared,
        has_ui_param_updates: bool,
    ) -> Option<GestureChange> {
        let previous_gesture = self.has_gesture;

        self.has_gesture = shared.has_gesture.load(Ordering::Relaxed);

        if previous_gesture == self.has_gesture {
            return if has_ui_param_updates && !self.has_gesture {
                Some(GestureChange::Both)
            } else {
                None
            };
        }

        if previous_gesture {
            Some(GestureChange::End)
        } else {
            Some(GestureChange::Begin)
        }
    }

    /// Handles incoming events.
    ///
    /// If the given event is a matching parameter change event, the order parameter will be
    /// updated accordingly.
    pub fn handle_event(&mut self, event: &UnknownEvent) {
        if let Some(CoreEventSpace::ParamValue(event)) = event.as_core_event() {
            if event.param_id() == RetainParamsShared::PARAM_ORDER_ID {
                self.set_order(event.value() as usize);
            }

            if event.param_id() == RetainParamsShared::PARAM_WINDOW_SIZE_ID {
                self.set_window_size(event.value() as usize);
            }
        }
    }

    /// Sends the value of the parameters to the host, via a [`ParamValueEvent`].
    pub fn send_param_events(&self, output_events: &mut OutputEvents) {
        let order_event = ParamValueEvent::new(
            0,
            RetainParamsShared::PARAM_ORDER_ID,
            Pckn::match_all(),
            self.order as f64,
            Cookie::empty(),
        );

        let _ = output_events.try_push(order_event);

        let window_size_event = ParamValueEvent::new(
            0,
            RetainParamsShared::PARAM_ORDER_ID,
            Pckn::match_all(),
            self.window_size as f64,
            Cookie::empty(),
        );

        let _ = output_events.try_push(window_size_event);
    }
}

/// To save the plugin state (parameter values) using protocol buffers
#[derive(Message)]
struct PluginState {
    #[prost(uint64, tag = "1")]
    order: u64,
    #[prost(uint64, tag = "2")]
    window_size: u64,
}

impl PluginState {
    fn copied(local_params: &RetainParamsLocal) -> Self {
        Self {
            order: local_params.get_order() as u64,
            window_size: local_params.get_window_size() as u64,
        }
    }

    fn get_order(&self) -> u64 {
        self.order
    }

    fn get_window_size(&self) -> u64 {
        self.window_size
    }
}

/// Implementation of the State extension.
impl PluginStateImpl for RetainPluginMainThread<'_> {
    fn save(&mut self, output: &mut OutputStream) -> Result<(), PluginError> {
        self.params.fetch_updates(&self.shared.params);

        let mut data = vec![];

        let state = PluginState::copied(&self.params);

        state.encode(&mut data)?;

        output.write_all(&data)?;

        Ok(())
    }

    fn load(&mut self, input: &mut InputStream) -> Result<(), PluginError> {
        let mut data = vec![];
        input.read_to_end(&mut data)?;

        let data = PluginState::decode(&data[..])?;

        self.params.set_order(data.get_order() as usize);
        self.params.set_window_size(data.get_window_size() as usize);

        self.params.push_updates(&self.shared.params);

        Ok(())
    }
}

impl PluginMainThreadParams for RetainPluginMainThread<'_> {
    fn count(&mut self) -> u32 {
        2
    }

    fn get_info(&mut self, param_index: u32, info: &mut ParamInfoWriter) {
        if param_index != 0 {
            return;
        }

        info.set(&ParamInfo {
            id: RetainParamsShared::PARAM_ORDER_ID,
            flags: ParamInfoFlags::IS_AUTOMATABLE,
            cookie: Cookie::default(),
            name: b"Order",
            module: b"",
            min_value: 0.0,
            max_value: f64::MAX,
            default_value: DEFAULT_ORDER as f64,
        });

        info.set(&ParamInfo {
            id: RetainParamsShared::PARAM_WINDOW_SIZE_ID,
            flags: ParamInfoFlags::IS_AUTOMATABLE,
            cookie: Cookie::default(),
            name: b"Window Size",
            module: b"",
            min_value: 128.0,
            max_value: usize::MAX as f64,
            default_value: DEFAULT_WINDOW_SIZE.into_inner() as f64,
        });
    }

    fn get_value(&mut self, param_id: ClapId) -> Option<f64> {
        match param_id {
            RetainParamsShared::PARAM_ORDER_ID => Some(self.params.get_order() as f64),
            RetainParamsShared::PARAM_WINDOW_SIZE_ID => Some(self.params.get_window_size() as f64),
            _ => None,
        }
    }

    fn value_to_text(
        &mut self,
        param_id: ClapId,
        value: f64,
        writer: &mut ParamDisplayWriter,
    ) -> std::fmt::Result {
        if param_id == 1 {
            write!(writer, "{0:.2} %", value * 100.0)
        } else {
            Err(std::fmt::Error)
        }
    }

    fn text_to_value(&mut self, param_id: ClapId, text: &CStr) -> Option<f64> {
        let text = text.to_str().ok()?;
        if param_id == 1 {
            let text = text.strip_suffix('%').unwrap_or(text).trim();
            let percentage: f64 = text.parse().ok()?;

            Some(percentage / 100.0)
        } else {
            None
        }
    }

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

use crate::{RetainPluginMainThread, window_size::WindowSize, window_type::WindowType};
use clack_extensions::{
    params::{
        ParamDisplayWriter, ParamInfo, ParamInfoFlags, ParamInfoWriter, PluginMainThreadParams,
    },
    state::PluginStateImpl,
};
use clack_plugin::{
    events::spaces::CoreEventSpace,
    prelude::*,
    stream::{InputStream, OutputStream},
    utils::Cookie,
};
use prost::Message;
use std::{
    ffi::CStr,
    fmt::Write as _,
    io::{Read, Write as _},
    sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering},
};

/// The default values of the parameters.
const DEFAULT_ORDER: usize = 1;
pub const DEFAULT_WINDOW_SIZE: WindowSize = WindowSize::Size4096;
pub const DEFAULT_WINDOW_TYPE: WindowType = WindowType::Hann;
const DEFAULT_COMPLEMENT: bool = false;

/// A struct that manages the parameters for the plugin.
pub struct RetainParams {
    /// The current value of the order parameter.
    order: AtomicUsize,
    /// Window size of FFT
    window_size: AtomicUsize,
    /// Type of window function of FFT
    window_type: AtomicU8,
    /// Whether it retains or removes the highest magnitudes
    complement: AtomicBool,
}

impl RetainParams {
    /// The unique identifier for the order parameter.
    pub const PARAM_ORDER_ID: ClapId = ClapId::new(1);
    pub const PARAM_WINDOW_SIZE_ID: ClapId = ClapId::new(2);
    pub const PARAM_WINDOW_TYPE_ID: ClapId = ClapId::new(3);
    pub const PARAM_COMPLEMENT_ID: ClapId = ClapId::new(4);

    /// Initializes the shared parameter value.
    pub fn new() -> Self {
        Self {
            order: AtomicUsize::new(DEFAULT_ORDER),
            window_size: AtomicUsize::new(DEFAULT_WINDOW_SIZE.inner()),
            window_type: AtomicU8::new(DEFAULT_WINDOW_TYPE.as_byte()),
            complement: AtomicBool::new(DEFAULT_COMPLEMENT),
        }
    }

    /// Returns the current order value.
    #[inline]
    pub fn get_order(&self) -> usize {
        self.order.load(Ordering::SeqCst)
    }

    /// Sets the current order to `value`.
    ///
    /// It is clamped to the range `0..=32_768`.
    #[inline]
    pub fn set_order(&self, value: usize) {
        self.order.store(value.clamp(0, 32768), Ordering::SeqCst);
    }

    /// Returns the current window size value.
    #[inline]
    pub fn get_window_size(&self) -> WindowSize {
        self.window_size.load(Ordering::SeqCst).into()
    }

    /// Sets the current local window size to `value`.
    #[inline]
    pub fn set_window_size(&self, value: WindowSize) {
        self.window_size.store(value.inner(), Ordering::SeqCst);
    }

    /// Returns the current local window type.
    #[inline]
    pub fn get_window_type(&self) -> WindowType {
        self.window_type.load(Ordering::SeqCst).into()
    }

    /// Sets the current local window type from `bits`.
    #[inline]
    pub fn set_window_type(&self, window_type: WindowType) {
        self.window_type
            .store(window_type.as_byte(), Ordering::SeqCst);
    }

    /// Returns the current local complement.
    #[inline]
    pub fn get_complement(&self) -> bool {
        self.complement.load(Ordering::SeqCst)
    }

    /// Sets the current local complement to `value`.
    #[inline]
    pub fn set_complement(&self, value: bool) {
        self.complement.store(value, Ordering::SeqCst);
    }

    /// Handles incoming events.
    ///
    /// If the given event is a matching parameter change event, the order parameter will be
    /// updated accordingly.
    pub fn handle_event(&self, event: &UnknownEvent) {
        if let Some(CoreEventSpace::ParamValue(event)) = event.as_core_event() {
            if event.param_id() == RetainParams::PARAM_ORDER_ID {
                self.set_order(event.value() as usize);
            }

            if event.param_id() == RetainParams::PARAM_WINDOW_SIZE_ID {
                self.set_window_size((event.value() as usize).into());
            }

            if event.param_id() == RetainParams::PARAM_WINDOW_TYPE_ID {
                self.set_window_type((event.value() as u8).into());
            }

            if event.param_id() == RetainParams::PARAM_COMPLEMENT_ID {
                self.set_complement(event.value() != 0.0);
            }
        }
    }
}

/// Wrapper that helps save plugin parameters using protocol buffers
/// Updating this will change the schema and break old
#[derive(Message)]
struct PluginState {
    #[prost(uint64, tag = "1")]
    order: u64,
    #[prost(uint32, tag = "2")]
    window_size: u32,
    #[prost(uint32, tag = "3")]
    window_type: u32,
    #[prost(bool, tag = "4")]
    complement: bool,
}

impl PluginState {
    fn copied(local_params: &RetainParams) -> Self {
        Self {
            order: local_params.get_order() as u64,
            window_size: local_params.get_window_size().inner() as u32,
            window_type: local_params.get_window_type().as_byte() as u32,
            complement: local_params.get_complement(),
        }
    }

    fn get_order(&self) -> u64 {
        self.order
    }

    fn get_window_size(&self) -> u32 {
        self.window_size
    }

    fn get_window_type_bits(&self) -> u8 {
        self.window_type as u8
    }

    fn get_complement(&self) -> bool {
        self.complement
    }
}

/// To save and load the plugin parameters
impl PluginStateImpl for RetainPluginMainThread<'_> {
    fn save(&mut self, output: &mut OutputStream) -> Result<(), PluginError> {
        let mut data = vec![];

        let state = PluginState::copied(&self.shared.params);

        state.encode(&mut data)?;

        output.write_all(&data)?;

        Ok(())
    }

    fn load(&mut self, input: &mut InputStream) -> Result<(), PluginError> {
        let mut data = vec![];
        input.read_to_end(&mut data)?;

        let data = PluginState::decode(&data[..])?;

        self.shared.params.set_order(data.get_order() as usize);
        self.shared
            .params
            .set_window_size((data.get_window_size() as usize).into());
        self.shared
            .params
            .set_window_type(data.get_window_type_bits().into());
        self.shared.params.set_complement(data.get_complement());

        Ok(())
    }
}

impl PluginMainThreadParams for RetainPluginMainThread<'_> {
    fn count(&mut self) -> u32 {
        4
    }

    /// `param_index`: The index of the parameter to query.
    /// Must be less than the value returned by `count()`.
    fn get_info(&mut self, param_index: u32, info: &mut ParamInfoWriter) {
        if param_index >= 4 {
            return;
        }

        // cleaner than match is
        if param_index == 0 {
            info.set(&ParamInfo {
                id: RetainParams::PARAM_ORDER_ID,
                flags: ParamInfoFlags::IS_AUTOMATABLE | ParamInfoFlags::IS_STEPPED,
                cookie: Cookie::default(),
                name: b"Order",
                module: b"",
                min_value: 0.0,
                max_value: 32768.0,
                default_value: DEFAULT_ORDER as f64,
            });
        }

        if param_index == 1 {
            info.set(&ParamInfo {
                id: RetainParams::PARAM_WINDOW_SIZE_ID,
                flags: ParamInfoFlags::IS_READONLY,
                cookie: Cookie::default(),
                name: b"Window Size",
                module: b"",
                min_value: 256.0,
                max_value: 32768.0,
                default_value: DEFAULT_WINDOW_SIZE.inner() as f64,
            });
        }

        if param_index == 2 {
            info.set(&ParamInfo {
                id: RetainParams::PARAM_WINDOW_TYPE_ID,
                flags: ParamInfoFlags::IS_READONLY,
                cookie: Cookie::default(),
                name: b"Window Function",
                module: b"",
                min_value: 0.0,
                max_value: 4.0,
                default_value: DEFAULT_WINDOW_TYPE.as_byte() as f64,
            });
        }

        if param_index == 3 {
            info.set(&ParamInfo {
                id: RetainParams::PARAM_COMPLEMENT_ID,
                flags: ParamInfoFlags::IS_READONLY,
                cookie: Cookie::default(),
                name: b"Complement",
                module: b"",
                min_value: 0.0,
                max_value: 1.0,
                default_value: DEFAULT_COMPLEMENT as u8 as f64,
            });
        }
    }

    fn get_value(&mut self, param_id: ClapId) -> Option<f64> {
        match param_id {
            RetainParams::PARAM_ORDER_ID => Some(self.shared.params.get_order() as f64),
            RetainParams::PARAM_WINDOW_SIZE_ID => {
                Some(self.shared.params.get_window_size().inner() as f64)
            }
            RetainParams::PARAM_WINDOW_TYPE_ID => {
                Some(self.shared.params.get_window_type().as_byte() as f64)
            }
            RetainParams::PARAM_COMPLEMENT_ID => {
                Some(self.shared.params.get_complement() as u8 as f64)
            }
            _ => None,
        }
    }

    fn value_to_text(
        &mut self,
        param_id: ClapId,
        value: f64,
        writer: &mut ParamDisplayWriter,
    ) -> std::fmt::Result {
        if param_id == RetainParams::PARAM_ORDER_ID {
            write!(writer, "{}", value as u64)
        } else {
            Err(std::fmt::Error)
        }
    }

    fn text_to_value(&mut self, param_id: ClapId, text: &CStr) -> Option<f64> {
        let text = text.to_str().ok()?;

        if param_id == RetainParams::PARAM_ORDER_ID {
            let max = self.shared.params.get_window_size().inner() as f64;

            let order_value = text.parse::<f64>().ok()?;

            Some(order_value.clamp(0.0, max))
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
            self.shared.params.handle_event(event);
        }
    }
}

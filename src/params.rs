use crate::{RetainPluginMainThread, window_size::WindowSize, window_type::WindowType};
use clack_extensions::{
    params::{ParamDisplayWriter, ParamInfo, ParamInfoFlags, ParamInfoWriter, PluginMainThreadParams},
    state::PluginStateImpl,
};
use clack_plugin::{
    events::spaces::CoreEventSpace,
    prelude::*,
    stream::{InputStream, OutputStream},
    utils::Cookie,
};
use serde::{Deserialize, Serialize};
use std::{ffi::CStr, io::{Read, Write as _}, sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering}};

const DEFAULT_ORDER: usize = 1;
pub const DEFAULT_WINDOW_SIZE: WindowSize = WindowSize::Size4096;
pub const DEFAULT_WINDOW_TYPE: WindowType = WindowType::Hann;
const DEFAULT_COMPLEMENT: bool = false;

/// Friendly shared parameter state for the plugin.
///
/// This is built to be easy to read, easy to extend, and ready for a
/// future migration of UI/parameter logic to JavaScript or WebAssembly.
/// Imagine porting these parameters to a small React or Svelte GUI later! 😎
pub struct RetainParams {
    order: AtomicUsize,
    window_size: AtomicUsize,
    window_type: AtomicU8,
    complement: AtomicBool,
}

impl RetainParams {
    pub const PARAM_ORDER_ID: ClapId = ClapId::new(1);
    pub const PARAM_WINDOW_SIZE_ID: ClapId = ClapId::new(2);
    pub const PARAM_WINDOW_TYPE_ID: ClapId = ClapId::new(3);
    pub const PARAM_COMPLEMENT_ID: ClapId = ClapId::new(4);

    pub fn new() -> Self {
        Self {
            order: AtomicUsize::new(DEFAULT_ORDER),
            window_size: AtomicUsize::new(DEFAULT_WINDOW_SIZE.inner()),
            window_type: AtomicU8::new(DEFAULT_WINDOW_TYPE.as_byte()),
            complement: AtomicBool::new(DEFAULT_COMPLEMENT),
        }
    }

    #[inline]
    pub fn get_order(&self) -> usize {
        self.order.load(Ordering::SeqCst)
    }

    #[inline]
    pub fn set_order(&self, value: usize) {
        self.order.store(value.clamp(0, 32768), Ordering::SeqCst);
    }

    #[inline]
    pub fn get_window_size(&self) -> WindowSize {
        self.window_size.load(Ordering::SeqCst).into()
    }

    #[inline]
    pub fn set_window_size(&self, value: WindowSize) {
        self.window_size.store(value.inner(), Ordering::SeqCst);
    }

    #[inline]
    pub fn get_window_type(&self) -> WindowType {
        self.window_type.load(Ordering::SeqCst).into()
    }

    #[inline]
    pub fn set_window_type(&self, window_type: WindowType) {
        self.window_type.store(window_type.as_byte(), Ordering::SeqCst);
    }

    #[inline]
    pub fn get_complement(&self) -> bool {
        self.complement.load(Ordering::SeqCst)
    }

    #[inline]
    pub fn set_complement(&self, value: bool) {
        self.complement.store(value, Ordering::SeqCst);
    }

    pub fn handle_event(&self, event: &UnknownEvent) {
        if let Some(CoreEventSpace::ParamValue(event)) = event.as_core_event() {
            match event.param_id() {
                // 🌟 event handling is kept super small and readable here.
                Self::PARAM_ORDER_ID => self.set_order(event.value() as usize),
                Self::PARAM_COMPLEMENT_ID => self.set_complement(event.value() > 0.5),
                _ => {}
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
struct PluginState {
    order: usize,
    window_size: usize,
    window_type: u8,
    complement: bool,
}

impl Default for PluginState {
    fn default() -> Self {
        Self {
            order: DEFAULT_ORDER,
            window_size: DEFAULT_WINDOW_SIZE.inner(),
            window_type: DEFAULT_WINDOW_TYPE.as_byte(),
            complement: DEFAULT_COMPLEMENT,
        }
    }
}

impl PluginState {
    fn copied(params: &RetainParams) -> Self {
        Self {
            order: params.get_order(),
            window_size: params.get_window_size().inner(),
            window_type: params.get_window_type().as_byte(),
            complement: params.get_complement(),
        }
    }

    fn get_order(&self) -> usize {
        self.order
    }

    fn get_window_size(&self) -> WindowSize {
        self.window_size.into()
    }

    fn get_window_type(&self) -> WindowType {
        self.window_type.into()
    }

    fn get_complement(&self) -> bool {
        self.complement
    }
}

impl PluginStateImpl for RetainPluginMainThread<'_> {
    fn save(&mut self, output: &mut OutputStream) -> Result<(), PluginError> {
        let state = PluginState::copied(&self.shared.params);
        let data = serde_json::to_vec(&state)?;
        output.write_all(&data)?;
        Ok(())
    }

    fn load(&mut self, input: &mut InputStream) -> Result<(), PluginError> {
        let mut data = vec![];
        input.read_to_end(&mut data)?;

        let state = serde_json::from_slice::<PluginState>(&data)?;
        self.shared.params.set_order(state.get_order());
        self.shared.params.set_window_size(state.get_window_size());
        self.shared.params.set_window_type(state.get_window_type());
        self.shared.params.set_complement(state.get_complement());
        Ok(())
    }
}

impl PluginMainThreadParams for RetainPluginMainThread<'_> {
    fn count(&mut self) -> u32 {
        4
    }

    fn get_info(&mut self, param_index: u32, info: &mut ParamInfoWriter) {
        let param = match param_index {
            0 => ParamInfo {
                id: RetainParams::PARAM_ORDER_ID,
                flags: ParamInfoFlags::IS_AUTOMATABLE | ParamInfoFlags::IS_STEPPED,
                cookie: Cookie::default(),
                name: b"Order",
                module: b"",
                min_value: 0.0,
                max_value: 32768.0,
                default_value: DEFAULT_ORDER as f64,
            },
            1 => ParamInfo {
                id: RetainParams::PARAM_WINDOW_SIZE_ID,
                flags: ParamInfoFlags::IS_READONLY,
                cookie: Cookie::default(),
                name: b"Window Size",
                module: b"",
                min_value: 256.0,
                max_value: 32768.0,
                default_value: DEFAULT_WINDOW_SIZE.inner() as f64,
            },
            2 => ParamInfo {
                id: RetainParams::PARAM_WINDOW_TYPE_ID,
                flags: ParamInfoFlags::IS_READONLY,
                cookie: Cookie::default(),
                name: b"Window Function",
                module: b"",
                min_value: 0.0,
                max_value: 4.0,
                default_value: DEFAULT_WINDOW_TYPE.as_byte() as f64,
            },
            3 => ParamInfo {
                id: RetainParams::PARAM_COMPLEMENT_ID,
                flags: ParamInfoFlags::IS_AUTOMATABLE,
                cookie: Cookie::default(),
                name: b"Complement",
                module: b"",
                min_value: 0.0,
                max_value: 1.0,
                default_value: DEFAULT_COMPLEMENT as f64,
            },
            _ => return,
        };

        info.set(&param);
    }

    fn get_display(&mut self, param_id: ClapId, display: &mut ParamDisplayWriter) {
        let text = match param_id {
            RetainParams::PARAM_ORDER_ID => self.shared.params.get_order().to_string(),
            RetainParams::PARAM_WINDOW_SIZE_ID => self.shared.params.get_window_size().as_str().to_string(),
            RetainParams::PARAM_WINDOW_TYPE_ID => self.shared.params.get_window_type().as_str().to_string(),
            RetainParams::PARAM_COMPLEMENT_ID => self.shared.params.get_complement().to_string(),
            _ => return,
        };

        let _ = write!(display, "{}", text);
    }

    fn get_value(&mut self, param_id: ClapId) -> Option<f64> {
        match param_id {
            RetainParams::PARAM_ORDER_ID => Some(self.shared.params.get_order() as f64),
            RetainParams::PARAM_WINDOW_SIZE_ID => Some(self.shared.params.get_window_size().inner() as f64),
            RetainParams::PARAM_WINDOW_TYPE_ID => Some(self.shared.params.get_window_type().as_byte() as f64),
            RetainParams::PARAM_COMPLEMENT_ID => Some(self.shared.params.get_complement() as u8 as f64),
            _ => None,
        }
    }

    fn value_to_text(&mut self, param_id: ClapId, value: f64, writer: &mut ParamDisplayWriter) -> std::fmt::Result {
        if param_id == RetainParams::PARAM_ORDER_ID {
            write!(writer, "{}", value as u64)
        } else {
            Err(std::fmt::Error)
        }
    }

    fn text_to_value(&mut self, param_id: ClapId, text: &CStr) -> Option<f64> {
        let text = text.to_str().ok()?;
        if param_id != RetainParams::PARAM_ORDER_ID {
            return None;
        }

        let max = self.shared.params.get_window_size().inner() as f64;
        let value = text.parse::<f64>().ok()?;
        Some(value.clamp(0.0, max))
    }

    fn flush(&mut self, input_parameter_changes: &InputEvents, _output_parameter_changes: &mut OutputEvents) {
        for event in input_parameter_changes {
            self.shared.params.handle_event(event);
        }
    }
}

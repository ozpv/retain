use crate::{
    window_function::{
        BlackmanHarrisWindow, HammingWindow, HannWindow, RectangularWindow, Sine4Window,
        WindowFunction,
    },
    window_size::WindowSize,
};

#[derive(PartialEq)]
pub enum WindowType {
    Rectangular,
    Hann,
    Hamming,
    BlackmanHarris,
    Sine4,
}

impl WindowType {
    pub fn new_function(&self, window_size: &WindowSize) -> Box<dyn WindowFunction> {
        match self {
            Self::Rectangular => Box::new(RectangularWindow::new(window_size)),
            Self::Hann => Box::new(HannWindow::new(window_size)),
            Self::Hamming => Box::new(HammingWindow::new(window_size)),
            Self::BlackmanHarris => Box::new(BlackmanHarrisWindow::new(window_size)),
            Self::Sine4 => Box::new(Sine4Window::new(window_size)),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Rectangular => "Rectangular",
            Self::Hann => "Hann",
            Self::Hamming => "Hamming",
            Self::BlackmanHarris => "Blackman-Harris",
            Self::Sine4 => "Sin^4",
        }
    }

    pub fn iter() -> impl Iterator<Item = Self> {
        [
            Self::Rectangular,
            Self::Hann,
            Self::Hamming,
            Self::BlackmanHarris,
            Self::Sine4,
        ]
        .into_iter()
    }

    pub fn from_byte(bits: u8) -> Self {
        match bits {
            0 => Self::Rectangular,
            1 => Self::Hann,
            2 => Self::Hamming,
            3 => Self::BlackmanHarris,
            4 => Self::Sine4,
            _ => panic!("Invalid window type"),
        }
    }

    pub fn as_byte(&self) -> u8 {
        match self {
            Self::Rectangular => 0,
            Self::Hann => 1,
            Self::Hamming => 2,
            Self::BlackmanHarris => 3,
            Self::Sine4 => 4,
        }
    }
}

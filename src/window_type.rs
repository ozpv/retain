#![allow(unused)]
#![allow(dead_code)]

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

    pub fn from_bits(bits: u8) -> Self {
        match bits {
            0b00000000 => Self::Rectangular,
            0b00000001 => Self::Hann,
            0b00000010 => Self::Hamming,
            0b00000011 => Self::BlackmanHarris,
            0b00000100 => Self::Sine4,
            _ => panic!("Invalid window type"),
        }
    }

    pub fn as_bits(&self) -> u8 {
        match self {
            Self::Rectangular => 0b00000000,
            Self::Hann => 0b00000001,
            Self::Hamming => 0b00000010,
            Self::BlackmanHarris => 0b00000011,
            Self::Sine4 => 0b00000100,
        }
    }
}

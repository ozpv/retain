#![allow(unused)]
#![allow(dead_code)]

use crate::{
    window_function::{
        BlackmanHarrisWindow, HammingWindow, HannWindow, RectangularWindow,
        Sine4Window, WindowFunction,
    },
    window_size::WindowSize,
};

pub enum WindowType {
    Rectangular,
    Hann,
    Hamming,
    BlackmanHarris,
    Sine4,
}

impl WindowType {
    fn into_function(self, window_size: &WindowSize) -> Box<dyn WindowFunction> {
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
}

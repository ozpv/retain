use crate::{
    window_function::{BlackmanHarrisWindow, HannWindow, RectangularWindow, WindowFunction},
    window_size::WindowSize,
};

pub enum WindowType {
    Hann,
    Rectangular,
    BlackmanHarris,
}

impl WindowType {
    fn into_function(self, window_size: &WindowSize) -> Box<dyn WindowFunction> {
        match self {
            Self::Hann => Box::new(HannWindow::new(window_size)),
            Self::Rectangular => Box::new(RectangularWindow::new(window_size)),
            Self::BlackmanHarris => Box::new(BlackmanHarrisWindow::new(window_size)),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Hann => "Hann",
            Self::Rectangular => "Rectangular",
            Self::BlackmanHarris => "Blackman-Harris",
        }
    }

    pub fn iter() -> impl Iterator<Item = Self> {
        [Self::Hann, Self::Rectangular].into_iter()
    }
}

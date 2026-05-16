pub trait WindowFunction: Send {
    fn apply(&self, data: &mut [f32]);
    fn reverse(&self, data: &mut [f32]);
}

pub struct RectangularWindow {}

impl RectangularWindow {
    pub fn new(_fft_size: usize) -> Self {
        Self {}
    }
}

impl WindowFunction for RectangularWindow {
    fn apply(&self, _data: &mut [f32]) {}

    fn reverse(&self, _data: &mut [f32]) {}
}

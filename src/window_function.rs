use crate::window_size::WindowSize;
use std::{collections::VecDeque, f32::consts::PI, mem};

pub trait WindowFunction: Send {
    fn apply(&mut self, data: &mut VecDeque<f32>);
    fn reverse(&mut self, data: &mut [f32]);
    fn needed(&self) -> usize;
}

pub struct RectangularWindow {
    window_size: usize,
}

impl RectangularWindow {
    pub fn new(window_size: &WindowSize) -> Self {
        Self {
            window_size: window_size.inner(),
        }
    }
}

impl WindowFunction for RectangularWindow {
    fn apply(&mut self, _data: &mut VecDeque<f32>) {}

    fn reverse(&mut self, _data: &mut [f32]) {}

    fn needed(&self) -> usize {
        self.window_size
    }
}

pub struct HannWindow {
    window_size: usize,
    function: Vec<f32>,
    normalize: Vec<f32>,
    previous: Vec<f32>,
    previous_overlap: Vec<f32>,
}

impl HannWindow {
    pub fn new(window_size: &WindowSize) -> Self {
        let window_size = window_size.inner();
        let half_window_size = window_size / 2;
        let window_size_f32 = window_size as f32;

        // 50% overlap
        let previous_overlap = vec![0.0; half_window_size];

        // with capacity is important for the first needed samples
        let previous = Vec::with_capacity(window_size);

        let function = (0..window_size)
            .map(|n| {
                let n = n as f32;

                0.5 * (1.0 - f32::cos((2.0 * PI * n) / (window_size_f32 - 1.0)))
            })
            .collect::<Vec<f32>>();

        let normalize = (0..half_window_size)
            .map(|i| {
                (function[i] * function[i])
                    + (function[i + half_window_size] * function[i + half_window_size])
            })
            .collect::<Vec<f32>>();

        Self {
            window_size,
            function,
            normalize,
            previous,
            previous_overlap,
        }
    }
}

impl WindowFunction for HannWindow {
    fn apply(&mut self, data: &mut VecDeque<f32>) {
        let half_window_size = self.window_size / 2;

        // the full window size is needed to window over the samples the first time
        if self.previous.is_empty() {
            // store the latter half for when only half the size is needed
            for sample in data.iter().skip(half_window_size) {
                self.previous.push(*sample);
            }

            // apply the function
            for (i, sample) in data.iter_mut().enumerate() {
                *sample *= self.function[i];
            }

            return;
        }

        // In a real time "sliding window" it's necessary to add in the previous samples
        // to get the full resolution
        for sample in self.previous.iter().rev() {
            data.push_front(*sample);
        }

        self.previous.clear();

        // keep this iteration's samples for the next window
        for sample in data.iter().skip(half_window_size) {
            self.previous.push(*sample);
        }

        // apply the function
        for (i, sample) in data.iter_mut().enumerate() {
            *sample *= self.function[i];
        }
    }

    fn reverse(&mut self, data: &mut [f32]) {
        let half_window_size = self.window_size / 2;

        for (sample, function) in data.iter_mut().zip(self.function.iter()) {
            *sample *= *function;
        }

        // add back in the previous applied window function
        // this will reverse the 50% overlap
        for (i, (previous, sample)) in self
            .previous_overlap
            .iter()
            .zip(data.iter_mut())
            .enumerate()
        {
            *sample = (*sample + previous) / self.normalize[i];
        }

        self.previous_overlap.clear();

        // copy this window's tail for future overlap
        for sample in data.iter().skip(half_window_size) {
            self.previous_overlap.push(*sample);
        }
    }

    fn needed(&self) -> usize {
        if self.previous.is_empty() {
            self.window_size
        } else {
            self.window_size / 2
        }
    }
}

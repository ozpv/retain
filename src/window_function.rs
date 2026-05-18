#![allow(unused)]
#![allow(dead_code)]

use crate::window_size::WindowSize;
use std::{collections::VecDeque, f32::consts::PI};

pub trait WindowFunction: Send {
    /// Apply the window in-place
    fn apply(&mut self, data: &mut VecDeque<f32>);
    /// Reverse the window in-place
    fn reverse(&mut self, data: &mut [f32]);
    /// Yields the amount of samples still required to apply the window
    fn needed(&self) -> usize;
    /// Updates the window size
    fn window_size(&mut self, window_size: WindowSize);
}

/// Effectively a chunking function rather than a sliding window
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

    fn window_size(&mut self, _window_size: WindowSize) {
        todo!()
    }
}

/// A window designed specifically to reduce clicking and leave all other artifacts
/// Results vary
pub struct HaemolacriaaWindow {
    window_size: usize,
    function_start: Vec<f32>,
    function_end: Vec<f32>,
    previous: Vec<f32>,
    overlap_add: Vec<f32>,
}

impl HaemolacriaaWindow {
    /// Ensure this value isn't greater than or equal to any default window size
    const SAMPLE_COUNT: usize = 16;

    pub fn new(window_size: &WindowSize) -> Self {
        let window_size = window_size.inner();
        let sample_count_f32 = Self::SAMPLE_COUNT as f32;

        let overlap_add = vec![0.0; Self::SAMPLE_COUNT];

        let function_start = (0..Self::SAMPLE_COUNT)
            .map(|i| {
                let x = i as f32 / (sample_count_f32 - 1.0);

                f32::sin(PI * 0.5 * x)
            })
            .collect::<Vec<f32>>();

        let function_end = (0..Self::SAMPLE_COUNT)
            .map(|i| {
                let x = i as f32 / (sample_count_f32 - 1.0);

                f32::cos(PI * 0.5 * x)
            })
            .collect::<Vec<f32>>();

        let previous = Vec::with_capacity(Self::SAMPLE_COUNT);

        Self {
            window_size,
            function_start,
            function_end,
            previous,
            overlap_add,
        }
    }
}

impl WindowFunction for HaemolacriaaWindow {
    fn apply(&mut self, data: &mut VecDeque<f32>) {
        let last_few = self.window_size - Self::SAMPLE_COUNT;

        if self.previous.is_empty() {
            for sample in data.iter().skip(last_few).copied() {
                self.previous.push(sample);
            }

            for i in 0..Self::SAMPLE_COUNT {
                data[i] *= self.function_start[i];
                data[last_few + i] *= self.function_end[i];
            }

            return;
        }

        for sample in self.previous.iter().rev().copied() {
            data.push_front(sample);
        }

        self.previous.clear();

        for sample in data.iter().skip(last_few).copied() {
            self.previous.push(sample);
        }

        for i in 0..Self::SAMPLE_COUNT {
            data[i] *= self.function_start[i];
            data[last_few + i] *= self.function_end[i];
        }
    }

    fn reverse(&mut self, data: &mut [f32]) {
        let last_few = self.window_size - Self::SAMPLE_COUNT;

        for (i, sample) in data.iter_mut().take(Self::SAMPLE_COUNT).enumerate() {
            *sample += self.overlap_add[i];
        }

        for (i, sample) in data.iter().skip(last_few).copied().enumerate() {
            self.overlap_add[i] = sample;
        }
    }

    fn needed(&self) -> usize {
        if self.previous.is_empty() {
            self.window_size
        } else {
            self.window_size - Self::SAMPLE_COUNT
        }
    }

    fn window_size(&mut self, _window_size: WindowSize) {
        todo!()
    }
}

/// Hann function
pub struct HannWindow {
    window_size: usize,
    function: Vec<f32>,
    normalize: Vec<f32>,
    previous: Vec<f32>,
    overlap_add: Vec<f32>,
}

impl HannWindow {
    pub fn new(window_size: &WindowSize) -> Self {
        let window_size = window_size.inner();
        let half_window_size = window_size / 2;
        let window_size_f32 = window_size as f32;

        // 50% overlap
        let overlap_add = vec![0.0; window_size];

        // with capacity is important for the first needed samples
        let previous = Vec::with_capacity(window_size);

        let function = (0..window_size)
            .map(|i| {
                let i = i as f32;

                0.5 * (1.0 - f32::cos((2.0 * PI * i) / (window_size_f32 - 1.0)))
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
            overlap_add,
        }
    }
}

impl WindowFunction for HannWindow {
    fn apply(&mut self, data: &mut VecDeque<f32>) {
        let half_window_size = self.window_size / 2;

        // the full window size is needed to window over the samples the first time
        if self.previous.is_empty() {
            // store the latter half for when only half the size is needed
            for sample in data.iter().skip(half_window_size).copied() {
                self.previous.push(sample);
            }

            // apply the function
            for (i, sample) in data.iter_mut().enumerate() {
                *sample *= self.function[i];
            }

            return;
        }

        // In a real time "sliding window" it's necessary to add in the previous samples
        for sample in self.previous.iter().rev().copied() {
            data.push_front(sample);
        }

        self.previous.clear();

        // keep this iteration's samples for the next window
        for sample in data.iter().skip(half_window_size).copied() {
            self.previous.push(sample);
        }

        // apply the function
        for (i, sample) in data.iter_mut().enumerate() {
            *sample *= self.function[i];
        }
    }

    fn reverse(&mut self, data: &mut [f32]) {
        let half_window_size = self.window_size / 2;

        for (i, sample) in data.iter().copied().enumerate() {
            self.overlap_add[i] += sample * self.function[i];
        }

        for (i, sample) in data.iter_mut().enumerate().take(half_window_size) {
            if self.normalize[i] > 1e-6 {
                *sample = self.overlap_add[i] / self.normalize[i];
            } else {
                *sample = 0.0;
            }
        }

        self.overlap_add.rotate_left(half_window_size);

        for sample in &mut self.overlap_add[half_window_size..] {
            *sample = 0.0;
        }
    }

    fn needed(&self) -> usize {
        if self.previous.is_empty() {
            self.window_size
        } else {
            self.window_size / 2
        }
    }

    fn window_size(&mut self, window_size: WindowSize) {
        let window_size = window_size.inner();
        let half_window_size = window_size / 2;
        let window_size_f32 = window_size as f32;

        self.overlap_add.resize(window_size, 0.0);

        self.previous.clear();
        self.previous.reserve_exact(window_size);

        self.function = (0..window_size)
            .map(|i| {
                let i = i as f32;

                0.5 * (1.0 - f32::cos((2.0 * PI * i) / (window_size_f32 - 1.0)))
            })
            .collect::<Vec<f32>>();

        self.normalize = (0..half_window_size)
            .map(|i| {
                (self.function[i] * self.function[i])
                    + (self.function[i + half_window_size] * self.function[i + half_window_size])
            })
            .collect::<Vec<f32>>();
    }
}

/// 4-term Blackman-Harris window function
pub struct BlackmanHarrisWindow {
    window_size: usize,
    function: Vec<f32>,
    normalize: Vec<f32>,
    previous: Vec<f32>,
    overlap_add: Vec<f32>,
}

impl BlackmanHarrisWindow {
    const A_0: f32 = 0.35875;
    const A_1: f32 = 0.48829;
    const A_2: f32 = 0.14128;
    const A_3: f32 = 0.01168;

    pub fn new(window_size: &WindowSize) -> Self {
        let window_size = window_size.inner();
        let quarter_window_size = window_size / 4;
        let three_quarters_window_size = 3 * quarter_window_size;
        let window_size_f32 = window_size as f32;

        let overlap_add = vec![0.0; window_size];

        let previous = Vec::with_capacity(three_quarters_window_size);

        let function = (0..window_size)
            .map(|i| {
                let i = i as f32;

                let two = f32::cos((2.0 * PI * i) / (window_size_f32 - 1.0));
                let four = f32::cos((4.0 * PI * i) / (window_size_f32 - 1.0));
                let six = f32::cos((6.0 * PI * i) / (window_size_f32 - 1.0));

                Self::A_0 - (Self::A_1 * two) + (Self::A_2 * four) - (Self::A_3 * six)
            })
            .collect::<Vec<f32>>();

        #[rustfmt::skip]
        let normalize = (0..quarter_window_size)
            .map(|i| {
                (function[i] * function[i])
                    + (function[i + quarter_window_size] * function[i + quarter_window_size])
                    + (function[i + 2 * quarter_window_size] * function[i + 2 * quarter_window_size])
                    + (function[i + 3 * quarter_window_size] * function[i + 3 * quarter_window_size])
            })
            .collect::<Vec<f32>>();

        Self {
            window_size,
            function,
            normalize,
            previous,
            overlap_add,
        }
    }
}

impl WindowFunction for BlackmanHarrisWindow {
    fn apply(&mut self, data: &mut VecDeque<f32>) {
        let quarter_window_size = self.window_size / 4;

        if self.previous.is_empty() {
            for sample in data.iter().skip(quarter_window_size).copied() {
                self.previous.push(sample);
            }

            for (i, sample) in data.iter_mut().enumerate() {
                *sample *= self.function[i];
            }

            return;
        }

        for sample in self.previous.iter().rev().copied() {
            data.push_front(sample);
        }

        self.previous.clear();

        for sample in data.iter().skip(quarter_window_size).copied() {
            self.previous.push(sample);
        }

        for (i, sample) in data.iter_mut().enumerate() {
            *sample *= self.function[i];
        }
    }

    fn reverse(&mut self, data: &mut [f32]) {
        let quarter_window_size = self.window_size / 4;
        let three_quarters_window_size = 3 * quarter_window_size;

        for (i, sample) in data.iter().copied().enumerate() {
            self.overlap_add[i] += sample * self.function[i];
        }

        for (i, sample) in data.iter_mut().enumerate().take(quarter_window_size) {
            if self.normalize[i] > 1e-6 {
                *sample = self.overlap_add[i] / self.normalize[i];
            } else {
                *sample = 0.0;
            }
        }

        self.overlap_add.rotate_left(quarter_window_size);

        for sample in &mut self.overlap_add[three_quarters_window_size..] {
            *sample = 0.0;
        }
    }

    fn needed(&self) -> usize {
        if self.previous.is_empty() {
            self.window_size
        } else {
            self.window_size / 4
        }
    }

    fn window_size(&mut self, window_size: WindowSize) {
        let window_size = window_size.inner();
        let quarter_window_size = window_size / 4;
        let three_quarters_window_size = 3 * quarter_window_size;
        let window_size_f32 = window_size as f32;

        self.overlap_add.resize(window_size, 0.0);

        self.previous.clear();
        self.previous.reserve_exact(window_size);

        self.function = (0..window_size)
            .map(|i| {
                let i = i as f32;

                let two = f32::cos((2.0 * PI * i) / (window_size_f32 - 1.0));
                let four = f32::cos((4.0 * PI * i) / (window_size_f32 - 1.0));
                let six = f32::cos((6.0 * PI * i) / (window_size_f32 - 1.0));

                Self::A_0 - (Self::A_1 * two) + (Self::A_2 * four) - (Self::A_3 * six)
            })
            .collect::<Vec<f32>>();

        self.normalize = (0..quarter_window_size)
            .map(|i| {
                (self.function[i] * self.function[i])
                    + (self.function[i + quarter_window_size]
                        * self.function[i + quarter_window_size])
                    + (self.function[i + 2 * quarter_window_size]
                        * self.function[i + 2 * quarter_window_size])
                    + (self.function[i + 3 * quarter_window_size]
                        * self.function[i + 3 * quarter_window_size])
            })
            .collect::<Vec<f32>>();
    }
}

/// 4th order power of sine window function
pub struct Sine4Window {
    window_size: usize,
    function: Vec<f32>,
    normalize: Vec<f32>,
    previous: Vec<f32>,
    overlap_add: Vec<f32>,
}

impl Sine4Window {
    const A_0: f32 = 0.375;
    const A_1: f32 = 0.5;
    const A_2: f32 = 0.125;

    pub fn new(window_size: &WindowSize) -> Self {
        let window_size = window_size.inner();
        let quarter_window_size = window_size / 4;
        let three_quarters_window_size = 3 * quarter_window_size;
        let window_size_f32 = window_size as f32;

        let overlap_add = vec![0.0; window_size];

        let previous = Vec::with_capacity(three_quarters_window_size);

        let function = (0..window_size)
            .map(|i| {
                let i = i as f32;

                let two = f32::cos((2.0 * PI * i) / (window_size_f32 - 1.0));
                let four = f32::cos((4.0 * PI * i) / (window_size_f32 - 1.0));

                Self::A_0 - (Self::A_1 * two) + (Self::A_2 * four)
            })
            .collect::<Vec<f32>>();

        #[rustfmt::skip]
        let normalize = (0..quarter_window_size)
            .map(|i| {
                (function[i] * function[i])
                    + (function[i + quarter_window_size] * function[i + quarter_window_size])
                    + (function[i + 2 * quarter_window_size] * function[i + 2 * quarter_window_size])
                    + (function[i + 3 * quarter_window_size] * function[i + 3 * quarter_window_size])
            })
            .collect::<Vec<f32>>();

        Self {
            window_size,
            function,
            normalize,
            previous,
            overlap_add,
        }
    }
}

impl WindowFunction for Sine4Window {
    fn apply(&mut self, data: &mut VecDeque<f32>) {
        let quarter_window_size = self.window_size / 4;

        if self.previous.is_empty() {
            for sample in data.iter().skip(quarter_window_size).copied() {
                self.previous.push(sample);
            }

            for (i, sample) in data.iter_mut().enumerate() {
                *sample *= self.function[i];
            }

            return;
        }

        for sample in self.previous.iter().rev().copied() {
            data.push_front(sample);
        }

        self.previous.clear();

        for sample in data.iter().skip(quarter_window_size).copied() {
            self.previous.push(sample);
        }

        for (i, sample) in data.iter_mut().enumerate() {
            *sample *= self.function[i];
        }
    }

    fn reverse(&mut self, data: &mut [f32]) {
        let quarter_window_size = self.window_size / 4;
        let three_quarters_window_size = 3 * quarter_window_size;

        for (i, sample) in data.iter().copied().enumerate() {
            self.overlap_add[i] += sample * self.function[i];
        }

        for (i, sample) in data.iter_mut().enumerate().take(quarter_window_size) {
            if self.normalize[i] > 1e-6 {
                *sample = self.overlap_add[i] / self.normalize[i];
            } else {
                *sample = 0.0;
            }
        }

        self.overlap_add.rotate_left(quarter_window_size);

        for sample in &mut self.overlap_add[three_quarters_window_size..] {
            *sample = 0.0;
        }
    }

    fn needed(&self) -> usize {
        if self.previous.is_empty() {
            self.window_size
        } else {
            self.window_size / 4
        }
    }

    fn window_size(&mut self, window_size: WindowSize) {
        todo!()
    }
}

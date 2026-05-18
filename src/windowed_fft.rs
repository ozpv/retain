#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]
#![allow(unused)]
#![allow(dead_code)]

use crate::{
    window_function::{
        BlackmanHarrisWindow, HaemolacriaaWindow, HannWindow, RectangularWindow, Sine4Window,
        WindowFunction,
    },
    window_size::WindowSize,
};
use num_complex::Complex;
use realfft::{ComplexToReal, RealFftPlanner, RealToComplex};
use std::{collections::VecDeque, sync::Arc};

pub struct WindowedRealFft {
    window_size: usize,
    planner: RealFftPlanner<f32>,
    forward: Arc<dyn RealToComplex<f32>>,
    inverse: Arc<dyn ComplexToReal<f32>>,
    window_function: Box<dyn WindowFunction>,
    input: VecDeque<f32>,
    output: VecDeque<f32>,
    spectrum: Vec<Complex<f32>>,
    scratch: Vec<Complex<f32>>,
}

impl WindowedRealFft {
    pub fn new(window_size: WindowSize) -> Self {
        let window_function = Box::new(HaemolacriaaWindow::new(&window_size));
        let window_size = window_size.inner();

        let mut planner = RealFftPlanner::new();
        let forward = planner.plan_fft_forward(window_size);
        let inverse = planner.plan_fft_inverse(window_size);

        let input = VecDeque::with_capacity(window_size);
        let output = VecDeque::with_capacity(window_size);

        let spectrum = vec![Complex::ZERO; (window_size / 2) + 1];

        // scratches should be the same length for both left and right channels
        let scratch = forward.make_scratch_vec();

        Self {
            window_size,
            planner,
            forward,
            inverse,
            window_function,
            input,
            output,
            spectrum,
            scratch,
        }
    }

    pub fn window_function(&mut self, window_function: Box<dyn WindowFunction>) {}

    pub fn window_size(&mut self, window_size: WindowSize) {
        if window_size == self.window_size.into() {
            return;
        }

        self.window_function = Box::new(HaemolacriaaWindow::new(&window_size));
        self.window_size = window_size.inner();

        self.forward = self.planner.plan_fft_forward(self.window_size);
        self.inverse = self.planner.plan_fft_inverse(self.window_size);

        self.input.reserve_exact(self.window_size);
        self.output.reserve_exact(self.window_size);

        self.input.clear();
        self.output.clear();

        self.spectrum
            .resize((self.window_size / 2) + 1, Complex::ZERO);

        self.scratch
            .resize(self.forward.get_scratch_len(), Complex::ZERO);
    }

    pub fn clear_input(&mut self) {
        self.input.clear();
    }

    pub fn push_back_input(&mut self, value: f32) -> bool {
        self.input.push_back(value);

        self.input.len() >= self.window_function.needed()
    }

    pub fn pop_front_output(&mut self) -> f32 {
        self.output.pop_front().unwrap_or(0.0)
    }

    pub fn get_spectrum(&mut self) -> &mut [Complex<f32>] {
        &mut self.spectrum
    }

    pub fn forward(&mut self) {
        self.window_function.apply(&mut self.input);

        let _ = self.forward.process_with_scratch(
            self.input.make_contiguous(),
            &mut self.spectrum,
            &mut self.scratch,
        );
    }

    pub fn inverse(&mut self) {
        self.output.resize(self.window_size, 0.0);

        let _ = self.inverse.process_with_scratch(
            &mut self.spectrum,
            self.output.make_contiguous(),
            &mut self.scratch,
        );

        let window_size_f32 = self.window_size as f32;
        for sample in &mut self.output {
            *sample /= window_size_f32;
        }

        self.window_function.reverse(self.output.make_contiguous());
    }
}

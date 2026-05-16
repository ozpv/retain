#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]

/*use crate::window::{
    BlackmanHarrisWindow, HammingWindow, HannWindow, RectangularWindow, WindowFunction,
};*/
use crate::window_function::{RectangularWindow, WindowFunction};
use num_complex::Complex;
use realfft::{ComplexToReal, RealFftPlanner, RealToComplex};
use std::{collections::VecDeque, sync::Arc};

pub struct WindowedRealFft {
    fft_size: usize,
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
    pub fn new(mut fft_size: usize) -> Self {
		if fft_size == 0 {
			fft_size = 1;
		}
		
        let mut planner = RealFftPlanner::new();
        let forward = planner.plan_fft_forward(fft_size);
        let inverse = planner.plan_fft_inverse(fft_size);

        let window_function = Box::new(RectangularWindow::new(fft_size));

        let input = VecDeque::with_capacity(fft_size);
        let output = VecDeque::with_capacity(fft_size);

        let spectrum = vec![Complex::ZERO; (fft_size / 2) + 1];

        let max_scratch_len = forward.get_scratch_len().max(inverse.get_scratch_len());
        let scratch = vec![Complex::ZERO; max_scratch_len];

        Self {
            fft_size,
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

    pub fn fft_size(mut self, mut value: usize) -> Self {
		if value == 0 {
			value = 1;
		}
		
        self.fft_size = value;

        self.forward = self.planner.plan_fft_forward(self.fft_size);
        self.inverse = self.planner.plan_fft_inverse(self.fft_size);

        self.input.reserve_exact(self.fft_size);
        self.output.reserve_exact(self.fft_size);

        self.input.clear();
        self.output.clear();

        self.spectrum.resize((self.fft_size / 2) + 1, Complex::ZERO);

        let max_scratch_len = self
            .forward
            .get_scratch_len()
            .max(self.inverse.get_scratch_len());
        self.scratch.resize(max_scratch_len, Complex::ZERO);

        self
    }

    pub fn clear_input(&mut self) {
        self.input.clear();
    }

    pub fn push_front_input(&mut self, value: f32) -> bool {
        self.input.push_front(value);

        self.input.len() >= self.fft_size
    }

    pub fn pop_back_output(&mut self) -> f32 {
        self.output.pop_back().unwrap_or(0.0)
    }

    pub fn get_spectrum(&mut self) -> &mut [Complex<f32>] {
        &mut self.spectrum
    }

    pub fn forward(&mut self) {
        self.window_function.apply(self.input.make_contiguous());

        self.forward
            .process_with_scratch(
                self.input.make_contiguous(),
                &mut self.spectrum,
                &mut self.scratch,
            );
    }

    pub fn inverse(&mut self) {
		self.output.resize(self.fft_size, 0.0);
		
        self.inverse
            .process_with_scratch(
                &mut self.spectrum,
                self.output.make_contiguous(),
                &mut self.scratch,
            );

        let fft_size_f32 = self.fft_size as f32;
        for sample in &mut self.output {
            *sample = *sample / fft_size_f32;
        }

        self.window_function.reverse(self.output.make_contiguous());
    }
}

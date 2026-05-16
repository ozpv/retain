#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]

use crate::window::{
    BlackmanHarrisWindow, HammingWindow, HannWindow, RectangularWindow, WindowFunction,
};
use num_complex::Complex;
use realfft::{ComplexToReal, RealFftPlanner, RealToComplex};
use std::sync::Arc;

pub struct WindowedRealFft {
    fft_size: usize,
    planner: RealFftPlanner<f32>,
    forward: Arc<dyn RealToComplex<f32>>,
    inverse: Arc<dyn ComplexToReal<f32>>,
    window_function: Box<dyn WindowFunction>,
    original_length: usize,
}

impl WindowedRealFft {
    pub fn new(fft_size: usize) -> Self {
        let mut planner = RealFftPlanner::new();
        let forward = planner.plan_fft_forward(fft_size);
        let inverse = planner.plan_fft_inverse(fft_size);

        let window_function = Box::new(HannWindow::new(fft_size));

        Self {
            fft_size,
            planner,
            forward,
            inverse,
            window_function,
            original_length: 0,
        }
    }

    pub fn fft_size(mut self, value: usize) -> Self {
        self.fft_size = value;

        let forward = self.planner.plan_fft_forward(value);
        let inverse = self.planner.plan_fft_inverse(value);

        self.forward = forward;
        self.inverse = inverse;

        self
    }

    pub fn original_length(&mut self, value: usize) {
        self.original_length = value;
    }

    pub fn forward(&mut self, data: impl Into<Vec<f32>>) -> Vec<Vec<Complex<f32>>> {
        let mut data = data.into();

        let Some(new_length) = data.len().checked_next_multiple_of(self.fft_size) else {
            return vec![];
        };

        self.original_length = data.len();

        data.resize(new_length, 0.0);

        self.window_function
            .apply(data)
            .into_par_iter()
            .map(|mut chunk| {
                let mut output = self.forward.make_output_vec();

                self.forward.process(&mut chunk, &mut output).unwrap();

                output
            })
            .collect::<Vec<Vec<Complex<f32>>>>()
    }

    pub fn inverse(&mut self, data: Vec<Vec<Complex<f32>>>) -> Vec<f32> {
        let data = data
            .into_par_iter()
            .map(|mut chunk| {
                let mut real_output = self.inverse.make_output_vec();

                self.inverse.process(&mut chunk, &mut real_output).unwrap();

                let chunk_size_f32 = self.fft_size as f32;

                real_output
                    .into_iter()
                    .map(|sample| sample / chunk_size_f32)
                    .collect::<Vec<f32>>()
            })
            .collect::<Vec<Vec<f32>>>();

        let mut output = self.window_function.reverse(data);

        output.truncate(self.original_length);

        output
    }

    pub fn i32_to_f32(item: Vec<i32>) -> Vec<f32> {
        item.into_iter().map(f32::from).collect::<Vec<f32>>()
    }

    pub fn f32_to_i32(item: Vec<f32>) -> Vec<i32> {
        item.into_iter()
            .map(|value| value as i32)
            .collect::<Vec<i32>>()
    }
}

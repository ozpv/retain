use num_complex::Complex;
use rustc_hash::{FxBuildHasher, FxHashSet};

/// I thought about doing this to avoid allocating new memory in the audio thread
pub struct RetainAlgorithm {
    indexed_spectrum: Vec<(usize, Complex<f32>)>,
    top_indices: FxHashSet<usize>,
}

impl RetainAlgorithm {
    pub fn new() -> Self {
        let max_size = (32768 / 2) + 1;

        Self {
            indexed_spectrum: Vec::with_capacity(max_size),
            top_indices: FxHashSet::with_capacity_and_hasher(max_size, FxBuildHasher::default()),
        }
    }

    pub fn calculate(&mut self, spectrum: &mut [Complex<f32>], n: usize, complement: bool) {
        // you'd be keeping the entire signal
        if n >= spectrum.len() && !complement {
            return;
        }

        if n == 0 && complement {
            return;
        }

        // keeping nothing at all
        if n == 0 && !complement {
            for phasor in spectrum {
                *phasor = Complex::ZERO;
            }

            return;
        }

        if n >= spectrum.len() && complement {
            for phasor in spectrum {
                *phasor = Complex::ZERO;
            }

            return;
        }

        self.indexed_spectrum.clear();
        for (i, phasor) in spectrum.iter().copied().enumerate() {
            self.indexed_spectrum.push((i, phasor));
        }

        let target = spectrum.len() - n;
        self.indexed_spectrum
            .select_nth_unstable_by(target, |(_, c0), (_, c1)| c0.norm().total_cmp(&c1.norm()));

        self.top_indices.clear();
        for i in self.indexed_spectrum[target..].iter().map(|&(i, _)| i) {
            self.top_indices.insert(i);
        }

        for (i, phasor) in spectrum.iter_mut().enumerate() {
            if self.top_indices.contains(&i) && complement {
                *phasor = Complex::ZERO;
            }

            if !self.top_indices.contains(&i) && !complement {
                *phasor = Complex::ZERO;
            }
        }
    }
}

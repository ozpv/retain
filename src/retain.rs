use num_complex::Complex;
use rustc_hash::FxHashSet;

#[inline(always)]
pub fn retain_top_n_magnitudes(fft_real_signal: &mut [Complex<f32>], n: usize) {
    // you'd be keeping the entire signal
    // or keeping nothing at all
    if n >= fft_real_signal.len() {
        return;
    } else if n == 0 {
        for phasor in fft_real_signal {
            *phasor = Complex::ZERO;
        }

        return;
    }

    let mut indexed = fft_real_signal
        .iter()
        .copied()
        .enumerate()
        .collect::<Vec<(usize, Complex<f32>)>>();

    let target = fft_real_signal.len() - n;
    indexed.select_nth_unstable_by(target, |(_, c0), (_, c1)| c0.norm().total_cmp(&c1.norm()));

    let top_indices = indexed[target..]
        .iter()
        .map(|&(i, _)| i)
        .collect::<FxHashSet<usize>>();

    for (i, phasor) in fft_real_signal.iter_mut().enumerate() {
        if !top_indices.contains(&i) {
            *phasor = Complex::ZERO;
        }
    }
}

use hashbrown::HashSet;
use num_complex::Complex;

#[inline(always)]
pub fn retain_top_n_magnitudes(fft_real_signal: &mut [Complex<f32>], n: usize) {
    // you'd be keeping the entire signal
    if n >= fft_real_signal.len() {
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
        .collect::<HashSet<usize>>();

    for (i, val) in fft_real_signal.iter_mut().enumerate() {
        if !top_indices.contains(&i) {
            *val = Complex::ZERO;
        }
    }
}

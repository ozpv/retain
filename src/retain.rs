use num_complex::Complex;

/// Keeps the top `n` magnitudes in the spectrum.
///
/// This function is intentionally easy to follow: compute magnitudes,
/// sort them, and then keep or remove values based on the `complement` flag.
/// Future JS/WASM ports can reuse this same logic almost directly.
/// If you ever want to rewrite this in JavaScript, the core idea is already here.
#[inline(always)]
pub fn retain_top_n_magnitudes(spectrum: &mut [Complex<f32>], n: usize, complement: bool) {
    let len = spectrum.len();
    if len == 0 {
        return;
    }

    if n == 0 {
        if !complement {
            spectrum.fill(Complex::ZERO);
        }
        return;
    }

    if n >= len {
        if complement {
            spectrum.fill(Complex::ZERO);
        }
        return;
    }

    let mut magnitudes: Vec<(usize, f32)> = spectrum
        .iter()
        .enumerate()
        .map(|(index, value)| (index, value.norm()))
        .collect();

    magnitudes.sort_unstable_by(|a, b| a.1.total_cmp(&b.1));

    let mut keep = vec![false; len];
    for &(index, _) in &magnitudes[len - n..] {
        keep[index] = true;
    }

    for (index, value) in spectrum.iter_mut().enumerate() {
        let should_zero = if complement { keep[index] } else { !keep[index] };
        if should_zero {
            *value = Complex::ZERO;
        }
    }
}

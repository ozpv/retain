# Retain 🎛️

**Retain** is a clean and simple audio plugin built in Rust that keeps only the top magnitude frequencies in a signal.
It is made for audio experimentation, creative filtering, and learning how a frequency-retention effect behaves in the time domain.

---

## What it does ✨

- Keeps the **nth largest magnitude frequencies** in the audio spectrum
- Can optionally keep the **complement** instead, removing the top frequencies
- Supports **stereo processing** with separate left/right FFT paths
- Includes a **simple GUI** for parameter control and live auditioning

This project is written in Rust now, but the design is intentionally simple enough that it could be ported later to **JavaScript / WebAudio / WASM** if you want a browser-friendly version. 🌐

---

## Features 🚀

- `Order` control for how many spectral bins to keep
- `Window Size` selection for FFT resolution
- `Window Function` selection for different spectral shapes
- `Complement` toggle to switch between keep/top and remove/top mode
- Easy-to-read code with a clear path toward JS migration

---

## Getting Started 🛠️

### Prerequisites

- Rust toolchain installed (`rustup`, `cargo`)
- A CLAP-compatible host or plugin test environment

### Build

```bash
cargo build
```

### Check

```bash
cargo check
```

These commands will compile the plugin and verify the code without running it.

---

## Project Layout 📁

- `src/lib.rs` — plugin entry point and main CLAP setup
- `src/audio.rs` — audio processing and FFT pipeline
- `src/params.rs` — parameter state, saving/loading, and automation
- `src/gui.rs` — friendly UI code with egui/baseview
- `src/window_size.rs` — supported FFT window size options
- `src/window_type.rs` — window function selection helpers
- `src/windowed_fft.rs` — FFT buffer and transform helper
- `src/retain.rs` — the spectral filter logic

---

## Notes 💡

- The code is written to be **easy to understand** and easy to refactor.
- Many places in the code are already commented with a future JS migration in mind.
- If you want to build a browser version later, the core DSP path is already laid out clearly.

---

## Contributing 🤝

Contributions are welcome! If you want to add a feature, improve the UI, or help port this to JavaScript, please open a pull request.

If you contribute, keep it readable, clean, and emoji-friendly. 😊

---

## License 📜

This project is licensed under [GPLv3](https://github.com/ozpv/retain/blob/main/LICENSE).
That means any derivative work that distributes this code must also share source under the same license.

copyleft (ɔ) 2026 haemolacriaa. all wrongs reserved.

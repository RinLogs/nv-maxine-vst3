# NVIDIA Maxine Audio FX (VST3 Plugin)

[![Rust](https://img.shields.io/badge/Language-Rust-orange.svg)](https://www.rust-lang.org/)
[![Plugin Format](https://img.shields.io/badge/Format-VST3-blue.svg)](https://www.steinberg.net/vst3/)
[![Powered by](https://img.shields.io/badge/NVIDIA-Maxine%20SDK%201.6-76B900.svg)](https://www.nvidia.com/broadcast-sdk-resources)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

A lightweight, real-time AI audio enhancement VST3 plugin powered by **NVIDIA Maxine Audio Effects (AFX) SDK 1.6** and built from scratch in **Rust** using the [`nih-plug`](https://github.com/robbert-vdh/nih-plug) framework.

---

## ✨ Features

- **3 AI Processing Modes on TensorRT:**
  - **Noise Suppression (`denoiser`):** Removes fans, keyboard clicks, background babble, and environmental noise.
  - **Room Echo Removal (`dereverb`):** Eliminates room reverberation and hollow acoustic reflections.
  - **Denoise + De-Echo (`dereverb_denoiser`):** Single-pass combined model for simultaneous noise and echo suppression.
- **Hardware Acceleration:** Uses NVIDIA Tensor Cores & **CUDA Graphs** for ultra-low CPU overhead and minimal processing latency.
- **Real-Time Safe:** **Zero heap allocations** inside the audio thread for glitch-free streaming and recording.
- **Dual-Mono Optimization:** Automatically sums stereo laptop microphone arrays to prevent phase cancellation/doubling.
- **Hardware-Style Dark GUI (`egui`):** Includes a real-time **VAD (Voice Activity Detection)** indicator LED and an ergonomic non-linear suppression slider.
- **Crash-Proof Dynamic Loading (`libloading`):** Does not link statically against `.lib` files. If NVIDIA SDK is missing or GPU is unsupported, the plugin safely falls back to transparent **Bypass** mode without crashing your DAW.

---

## 💻 System Requirements

- **OS:** Windows 10 / 11 (64-bit)
- **GPU:** NVIDIA GeForce RTX series (RTX 20xx, 30xx, 40xx, 50xx) or RTX / Quadro Professional GPUs with Tensor Cores.
- **Driver / Dependencies:** [NVIDIA Broadcast Audio Effects SDK Redistributable](https://www.nvidia.com/broadcast-sdk-resources) installed.
- **Host DAW:** Any host with VST3 support (Elgato Wave Link, SteelSeries Sonar, OBS Studio, FL Studio, Reaper, Ableton Live, etc.)
- **Project Sample Rate:** Set to **48 000 Hz (48 kHz)** in your audio host.

---

## 🚀 Installation

1. Download the pre-built `nv_audio_fx_vst.vst3` bundle from [Releases](../../releases).
2. Copy the `nv_audio_fx_vst.vst3` folder to your system VST3 directory:
   ```text
   C:\Program Files\Common Files\VST3\
   ```
3. Open your DAW / audio mixer and rescan your VST3 plugins.
4. Add **NVIDIA Maxine Audio FX** to your microphone channel.

---

## 🛠️ Building from Source

### Prerequisites
- [Rust toolchain](https://rustup.rs/) (edition 2021)
- `cargo-nih-plug` bundler:
  ```powershell
  cargo install --git https://github.com/robbert-vdh/nih-plug.git cargo-nih-plug
  ```

### Build Command
```powershell
cargo nih-plug bundle nv_audio_fx_vst --release
```
The compiled VST3 bundle will be generated in `target/bundled/nv_audio_fx_vst.vst3`.

---

## 📜 License

This project is licensed under the [MIT License](LICENSE).

Developed by **RinLogs**.

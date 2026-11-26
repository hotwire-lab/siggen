# HotWire Signal Generator - Dual-Channel DDS, PWM, Sweep & Pattern Generator

[![Deploy to GitHub Pages](https://github.com/hotwire-lab/siggen/actions/workflows/deploy.yml/badge.svg)](https://github.com/hotwire-lab/siggen/actions/workflows/deploy.yml)
[![Live Web App](https://img.shields.io/badge/Live_Web_App-GitHub_Pages-0ea5e9?logo=googlechrome&logoColor=white)](https://hotwire-lab.github.io/siggen/)
[![Hardware](https://img.shields.io/badge/Hardware-XIAO_ESP32--S3_%2F_C6-10b981)](https://www.seeedstudio.com/XIAO-ESP32S3-p-5627.html)
[![Web Serial & WebUSB](https://img.shields.io/badge/USB-Web_Serial_%2F_WebUSB-8b5cf6)](https://developer.mozilla.org/en-US/docs/Web/API/Web_Serial_API)
[![Rust + WASM](https://img.shields.io/badge/Stack-Rust_%2B_Leptos_WASM-f97316?logo=rust&logoColor=white)](https://leptos.dev/)

Dual-channel DDS waveform, hardware LEDC Square/PWM (up to 20 MHz), frequency sweep, and 4-channel digital pattern generator for **Seeed Studio XIAO ESP32-S3** and **XIAO ESP32-C6**. Runs directly in Chrome or Edge on desktop and Android via Web Serial / WebUSB.

* **Live App:** https://hotwire-lab.github.io/siggen/
* **Repository:** https://github.com/hotwire-lab/siggen
* **Suite Home:** https://hotwire-lab.github.io/
* **Stack:** Rust + WebAssembly (Leptos 0.7 CSR + Trunk + `hotwire-core`)

> **Try it in 1 click (no hardware required):** Open **[https://hotwire-lab.github.io/siggen/](https://hotwire-lab.github.io/siggen/)** and click **Demo** to explore both waveform channels, sweep, and the pattern generator offline.

## Features

* **Unified HotWire Shell (`hotwire-core`):**
  * Top toolbar with instant **Logic / Scope / SDR / Serial / Gen** tool switcher, automatic `0x83` target software & version verification (`v1.1.1+`), swipeable mobile **BottomSheet** Web Flasher, and Web Serial (Desktop) / WebUSB (Android) auto-reconnect.
  * One-click `esptool-js` browser flashing for XIAO ESP32-S3 and ESP32-C6 plus an offline Demo simulator.
* **Dual-Channel Outputs (`D0` & `D1`):**
  * **Hardware LEDC Square & Variable-Duty PWM:** 1 Hz to 20 MHz square wave and 0.5%-99.5% variable-duty PWM with automatic timer resolution selection and zero CPU jitter. Duty resolution is resolution-limited at the top of the range (e.g. 1-bit/50% only at 20 MHz, 2-bit at 10 MHz), matching the ESP32 LEDC clock divider.
  * **PWM-DAC DDS Waveforms (`0.1 Hz`-`5 kHz`):** Sine, Triangle, Sawtooth, Reverse Sawtooth, DC Level, and White Noise modulated on a 312.5 kHz 8-bit PWM carrier. Add a simple external RC low-pass filter (e.g. `1 kΩ + 10 nF`, `fc ≈ 16 kHz`) for smooth analog voltage output (`0-3.3 V`). CH2 supports `0°-359°` phase offset relative to CH1.
* **CH1 Frequency Sweep:** Linear loop, Logarithmic loop, and One-Shot Linear sweep (`100 ms`-`30 s`) with live frequency telemetry.
* **4-Channel Digital Pattern Generator (`D2`-`D5`):** Programmable 16-step bit grid (`1 Hz`-`20 kHz`) with 4-bit Counter, Quadrature A/B + Index, Walking 1s, and SPI `0xA5` Burst presets.
* **Master Output Enable:** Single toolbar button to arm or mute both waveform channels and the pattern outputs.

## Pinout (3.3 V CMOS Max)

| Signal | XIAO Pin | ESP32-S3 GPIO | ESP32-C6 GPIO |
| :--- | :--- | :--- | :--- |
| **CH1 Output** | `D0` | `GPIO1` | `GPIO0` |
| **CH2 Output** | `D1` | `GPIO2` | `GPIO1` |
| **Pattern Bit 0** | `D2` | `GPIO3` | `GPIO2` |
| **Pattern Bit 1** | `D3` | `GPIO4` | `GPIO21` |
| **Pattern Bit 2** | `D4` | `GPIO5` | `GPIO22` |
| **Pattern Bit 3** | `D5` | `GPIO6` | `GPIO23` |

> ⚠ Outputs are 3.3 V CMOS only. Do not connect them directly to 5 V logic or negative voltages. Use the analog DDS modes only with an external RC low-pass filter.

## Firmware

* **XIAO ESP32-S3 Sketch:** [`firmware/HotWire_SigGen_XIAO_ESP32S3/HotWire_SigGen_XIAO_ESP32S3.ino`](firmware/HotWire_SigGen_XIAO_ESP32S3/HotWire_SigGen_XIAO_ESP32S3.ino)
  * FQBN: `esp32:esp32:XIAO_ESP32S3:USBMode=hwcdc,CDCOnBoot=default`
* **XIAO ESP32-C6 Sketch:** [`firmware/HotWire_SigGen_XIAO_ESP32C6/HotWire_SigGen_XIAO_ESP32C6.ino`](firmware/HotWire_SigGen_XIAO_ESP32C6/HotWire_SigGen_XIAO_ESP32C6.ino)
  * FQBN: `esp32:esp32:XIAO_ESP32C6:USBMode=hwcdc,CDCOnBoot=default`
* Firmware version: **`v1.1.1+`** (bundled binaries live in [`public/firmware/`](public/firmware/)).

## HotWire Ecosystem

* **[`hotwire-core`](https://github.com/hotwire-lab/hotwire-core)** - Shared Rust/WASM hardware transport, framing, firmware verification, Web Flasher & UI shell
* **[HotWire Logic Analyzer](https://github.com/hotwire-lab/logic)** ([Live](https://hotwire-lab.github.io/logic/)) - 8-Channel Logic Analyzer, Tri-State Force I/O & UART/SPI/I²C decoders
* **[HotWire Oscilloscope](https://github.com/hotwire-lab/scope)** ([Live](https://hotwire-lab.github.io/scope/)) - 4-Channel 12-bit GDMA ADC Oscilloscope & FFT Spectrum Analyzer
* **[HotWire SDR](https://github.com/hotwire-lab/sdr)** ([Live](https://hotwire-lab.github.io/sdr/)) - 1.84-2.79 GHz RF Spectrum Analyzer, Wideband Scanner & Waterfall
* **[HotWire Serial Terminal](https://github.com/hotwire-lab/serial)** ([Live](https://hotwire-lab.github.io/serial/)) - Cross-platform Web Serial & WebUSB terminal with XIAO UART Bridge

## Development

```bash
rustup target add wasm32-unknown-unknown
cargo test
cargo clippy --all-targets -- -D warnings
cargo clippy --target wasm32-unknown-unknown -- -D warnings
trunk serve          # local dev server (http://127.0.0.1:8084)
trunk build --release
```

> Web Serial and WebUSB require a secure context (`https://` or `http://localhost`).

## License

Copyright © 2026 Kari Vierimaa, Kempele - Licensed under the [Apache License 2.0](LICENSE).

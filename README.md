# HotWire Signal Generator

Dual-channel DDS waveform, hardware LEDC Square/PWM (up to 20 MHz), frequency sweep, and 4-channel digital pattern generator for **Seeed Studio XIAO ESP32-S3** and **XIAO ESP32-C6**. Runs directly in Chrome or Edge on desktop and Android via Web Serial / WebUSB.

* **Live App:** https://hotwire-lab.github.io/siggen/
* **Suite Home:** https://hotwire-lab.github.io/
* **Stack:** Rust + WebAssembly (Leptos 0.7 CSR + Trunk + `hotwire-core v0.5.0`)

## Features

* **Dual-Channel Outputs (`D0` & `D1`):**
  * **Hardware LEDC Square & Variable-Duty PWM:** 1 Hz to 20 MHz square wave and 0.5%–99.5% variable-duty PWM with automatic timer resolution selection and zero CPU jitter.
  * **PWM-DAC DDS Waveforms (`0.1 Hz`–`5 kHz`):** Sine, Triangle, Sawtooth, Reverse Sawtooth, DC Level, and White Noise modulated on a 312.5 kHz 8-bit PWM carrier. Add a simple external RC low-pass filter (e.g. `1 kΩ + 10 nF`, `fc ≈ 16 kHz`) for smooth analog voltage output (`0–3.3 V`). CH2 supports `0°–359°` phase offset relative to CH1.
* **CH1 Frequency Sweep:** Linear loop, Logarithmic loop, and One-Shot Linear sweep (`100 ms`–`30 s`) with live frequency telemetry.
* **4-Channel Digital Pattern Generator (`D2`–`D5`):** Programmable 16-step bit grid (`1 Hz`–`20 kHz`) with 4-bit Counter, Quadrature A/B + Index, Walking 1s, and SPI `0xA5` Burst presets.
* **Built-in Web Flasher & Demo Mode:** One-click `esptool-js` browser flashing for XIAO ESP32-S3 and ESP32-C6 plus an offline Demo simulator.

## Pinout (3.3 V CMOS Max)

| Signal | XIAO Pin | ESP32-S3 GPIO | ESP32-C6 GPIO |
| :--- | :--- | :--- | :--- |
| **CH1 Output** | `D0` | `GPIO1` | `GPIO0` |
| **CH2 Output** | `D1` | `GPIO2` | `GPIO1` |
| **Pattern Bit 0** | `D2` | `GPIO3` | `GPIO2` |
| **Pattern Bit 1** | `D3` | `GPIO4` | `GPIO21` |
| **Pattern Bit 2** | `D4` | `GPIO5` | `GPIO22` |
| **Pattern Bit 3** | `D5` | `GPIO6` | `GPIO23` |

## Development

```bash
rustup target add wasm32-unknown-unknown
cargo test
cargo clippy --all-targets -- -D warnings
cargo clippy --target wasm32-unknown-unknown -- -D warnings
trunk serve
trunk build --release
```

## License

Copyright © 2026 Kari Vierimaa, Kempele — Licensed under the [Apache License 2.0](LICENSE).

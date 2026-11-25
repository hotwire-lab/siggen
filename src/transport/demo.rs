use hotwire_core::protocol::{
    encode_frame, CMD_GET_CAPS, CMD_PING_STATUS, TYPE_BOARD_CAPABILITIES,
};
use hotwire_core::transport::DemoBackend;

use crate::protocol::types::{
    CMD_SIGGEN_CH_CFG, CMD_SIGGEN_MASTER, CMD_SIGGEN_PAT_CFG, CMD_SIGGEN_SWEEP_CFG,
    TYPE_SIGGEN_STATUS,
};

pub struct SigGenDemoBackend {
    master_enabled: bool,
    ch_enabled: [bool; 2],
    ch_freq_mhz: [u32; 2],
    sweep_enabled: bool,
    sweep_mode: u8,
    sweep_start_hz: u32,
    sweep_stop_hz: u32,
    sweep_duration_ms: u32,
    sweep_elapsed_ms: u32,
    sweep_finished: bool,
    pattern_enabled: bool,
}

impl Default for SigGenDemoBackend {
    fn default() -> Self {
        Self {
            master_enabled: true,
            ch_enabled: [true, false],
            ch_freq_mhz: [1_000_000, 1_000_000],
            sweep_enabled: false,
            sweep_mode: 0,
            sweep_start_hz: 100,
            sweep_stop_hz: 2000,
            sweep_duration_ms: 3000,
            sweep_elapsed_ms: 0,
            sweep_finished: false,
            pattern_enabled: false,
        }
    }
}

impl SigGenDemoBackend {
    fn build_caps(&self) -> Vec<u8> {
        let json = br#"{"board":"XIAO-ESP32-S3 (Demo)","firmware":"1.0.0-siggen","siggen":true,"max_channels":2,"pattern_channels":4}"#;
        encode_frame(TYPE_BOARD_CAPABILITIES, json)
    }

    fn current_sweep_mhz(&self) -> u32 {
        if !self.sweep_enabled || self.sweep_duration_ms == 0 {
            return self.ch_freq_mhz[0];
        }
        let t = (self.sweep_elapsed_ms as f64 / self.sweep_duration_ms as f64).clamp(0.0, 1.0);
        let f_hz = if self.sweep_mode == 1 {
            let f0 = (self.sweep_start_hz.max(1)) as f64;
            let f1 = (self.sweep_stop_hz.max(1)) as f64;
            f0 * (f1 / f0).powf(t)
        } else {
            self.sweep_start_hz as f64
                + (self.sweep_stop_hz as f64 - self.sweep_start_hz as f64) * t
        };
        (f_hz * 1000.0).round().max(100.0) as u32
    }

    fn build_status(&self) -> Vec<u8> {
        let mut payload = [0u8; 12];
        let mut mask = 0u8;
        if self.ch_enabled[0] {
            mask |= 1;
        }
        if self.ch_enabled[1] {
            mask |= 2;
        }
        let sw_active = self.master_enabled && self.ch_enabled[0] && self.sweep_enabled && !self.sweep_finished;
        let sw_mhz = if sw_active {
            self.current_sweep_mhz()
        } else {
            self.ch_freq_mhz[0]
        };
        payload[0] = u8::from(self.master_enabled);
        payload[1] = mask;
        payload[2] = u8::from(sw_active);
        payload[3] = u8::from(self.master_enabled && self.pattern_enabled);
        payload[4..8].copy_from_slice(&sw_mhz.to_le_bytes());
        payload[8..12].copy_from_slice(&self.ch_freq_mhz[0].to_le_bytes());
        encode_frame(TYPE_SIGGEN_STATUS, &payload)
    }
}

impl DemoBackend for SigGenDemoBackend {
    fn device_name(&self) -> &str {
        "XIAO-ESP32-S3 (Demo)"
    }

    fn vid_pid(&self) -> &str {
        "DEMO:5166"
    }

    fn tick_interval_ms(&self) -> Option<i32> {
        Some(100)
    }

    fn on_start(&mut self) -> Vec<u8> {
        let mut out = self.build_caps();
        out.extend_from_slice(&self.build_status());
        out
    }

    fn handle_command(&mut self, bytes: &[u8]) -> Vec<u8> {
        if bytes.len() < 8 || bytes[0] != 0xaa || bytes[1] != 0x55 {
            return Vec::new();
        }
        match bytes[2] {
            CMD_PING_STATUS | CMD_GET_CAPS => {
                let mut out = self.build_caps();
                out.extend_from_slice(&self.build_status());
                out
            }
            CMD_SIGGEN_MASTER => {
                self.master_enabled = bytes[3] != 0;
                if self.master_enabled && self.sweep_enabled {
                    self.sweep_elapsed_ms = 0;
                    self.sweep_finished = false;
                }
                self.build_status()
            }
            CMD_SIGGEN_CH_CFG if bytes.len() >= 5 + 14 => {
                let p = &bytes[5..];
                let ch = (p[0] & 1) as usize;
                self.ch_enabled[ch] = p[1] != 0;
                self.ch_freq_mhz[ch] = u32::from_le_bytes([p[3], p[4], p[5], p[6]]);
                self.build_status()
            }
            CMD_SIGGEN_SWEEP_CFG if bytes.len() >= 5 + 14 => {
                let p = &bytes[5..];
                self.sweep_enabled = p[0] != 0;
                self.sweep_mode = p[1];
                self.sweep_start_hz = u32::from_le_bytes([p[2], p[3], p[4], p[5]]).max(1);
                self.sweep_stop_hz = u32::from_le_bytes([p[6], p[7], p[8], p[9]]).max(1);
                self.sweep_duration_ms = u32::from_le_bytes([p[10], p[11], p[12], p[13]]).max(100);
                self.sweep_elapsed_ms = 0;
                self.sweep_finished = false;
                self.build_status()
            }
            CMD_SIGGEN_PAT_CFG if bytes.len() >= 5 + 22 => {
                let p = &bytes[5..];
                self.pattern_enabled = p[0] != 0;
                self.build_status()
            }
            _ => Vec::new(),
        }
    }

    fn tick(&mut self) -> Option<Vec<u8>> {
        if self.master_enabled && self.ch_enabled[0] && self.sweep_enabled && !self.sweep_finished {
            self.sweep_elapsed_ms = self.sweep_elapsed_ms.saturating_add(100);
            if self.sweep_elapsed_ms >= self.sweep_duration_ms {
                if self.sweep_mode == 2 {
                    self.sweep_elapsed_ms = self.sweep_duration_ms;
                    self.sweep_finished = true;
                } else {
                    self.sweep_elapsed_ms = 0;
                }
            }
            return Some(self.build_status());
        }
        None
    }
}

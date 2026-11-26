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
    ch_freq_dhz: [u32; 2],
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
            ch_freq_dhz: [10_000, 10_000],
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
        let json = br#"{"board":"XIAO-ESP32-S3 (Demo)","firmware":"1.1.0-siggen","siggen":true,"max_channels":2,"pattern_channels":4}"#;
        encode_frame(TYPE_BOARD_CAPABILITIES, json)
    }

    fn current_sweep_dhz(&self) -> u32 {
        if !self.sweep_enabled || self.sweep_duration_ms == 0 {
            return self.ch_freq_dhz[0];
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
        (f_hz * 10.0).round().clamp(1.0, u32::MAX as f64) as u32
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
        let sw_dhz = if sw_active {
            self.current_sweep_dhz()
        } else {
            self.ch_freq_dhz[0]
        };
        payload[0] = u8::from(self.master_enabled);
        payload[1] = mask;
        payload[2] = u8::from(sw_active);
        payload[3] = u8::from(self.master_enabled && self.pattern_enabled);
        payload[4..8].copy_from_slice(&sw_dhz.to_le_bytes());
        payload[8..12].copy_from_slice(&self.ch_freq_dhz[0].to_le_bytes());
        encode_frame(TYPE_SIGGEN_STATUS, &payload)
    }

    fn handle_one_frame(&mut self, cmd: u8, p: &[u8]) -> Vec<u8> {
        match cmd {
            CMD_PING_STATUS | CMD_GET_CAPS => {
                let mut out = self.build_caps();
                out.extend_from_slice(&self.build_status());
                out
            }
            CMD_SIGGEN_MASTER if !p.is_empty() => {
                self.master_enabled = p[0] != 0;
                if self.master_enabled && self.sweep_enabled {
                    self.sweep_elapsed_ms = 0;
                    self.sweep_finished = false;
                }
                self.build_status()
            }
            CMD_SIGGEN_CH_CFG if p.len() >= 14 => {
                let ch = (p[0] & 1) as usize;
                self.ch_enabled[ch] = p[1] != 0;
                self.ch_freq_dhz[ch] = u32::from_le_bytes([p[3], p[4], p[5], p[6]]);
                self.build_status()
            }
            CMD_SIGGEN_SWEEP_CFG if p.len() >= 14 => {
                self.sweep_enabled = p[0] != 0;
                self.sweep_mode = p[1];
                self.sweep_start_hz = u32::from_le_bytes([p[2], p[3], p[4], p[5]]).max(1);
                self.sweep_stop_hz = u32::from_le_bytes([p[6], p[7], p[8], p[9]]).max(1);
                self.sweep_duration_ms = u32::from_le_bytes([p[10], p[11], p[12], p[13]]).max(100);
                self.sweep_elapsed_ms = 0;
                self.sweep_finished = false;
                self.build_status()
            }
            CMD_SIGGEN_PAT_CFG if p.len() >= 22 => {
                self.pattern_enabled = p[0] != 0;
                self.build_status()
            }
            _ => Vec::new(),
        }
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
        let mut out = Vec::new();
        let mut pos = 0usize;
        while pos + 6 <= bytes.len() && bytes[pos] == 0xaa && bytes[pos + 1] == 0x55 {
            let cmd = bytes[pos + 2];
            if matches!(cmd, CMD_PING_STATUS | CMD_GET_CAPS | CMD_SIGGEN_MASTER) {
                if pos + 8 > bytes.len() {
                    break;
                }
                out.extend_from_slice(&self.handle_one_frame(cmd, &bytes[pos + 3..pos + 7]));
                pos += 8;
            } else {
                let payload_len =
                    u16::from_le_bytes([bytes[pos + 3], bytes[pos + 4]]) as usize;
                let end = pos + 6 + payload_len;
                if end > bytes.len() {
                    break;
                }
                out.extend_from_slice(
                    &self.handle_one_frame(cmd, &bytes[pos + 5..pos + 5 + payload_len]),
                );
                pos = end;
            }
        }
        out
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::codec::{
        build_ch_cfg_frame, build_master_cmd, build_pattern_cfg_frame, build_sweep_cfg_frame,
    };
    use crate::protocol::types::{ChannelConfig, PatternConfig, SweepConfig};

    #[test]
    fn handles_all_frames_in_a_batched_write() {
        let mut sim = SigGenDemoBackend::default();
        let mut ch2 = ChannelConfig::default_ch2();
        ch2.enabled = true;
        ch2.freq_hz = 2_000.0;
        let mut pattern = PatternConfig::preset_quadrature();
        pattern.enabled = true;

        let mut batch = Vec::new();
        batch.extend_from_slice(&build_ch_cfg_frame(0, &ChannelConfig::default_ch1()));
        batch.extend_from_slice(&build_ch_cfg_frame(1, &ch2));
        batch.extend_from_slice(&build_sweep_cfg_frame(&SweepConfig::default()));
        batch.extend_from_slice(&build_pattern_cfg_frame(&pattern));
        batch.extend_from_slice(&build_master_cmd(true));

        let out = sim.handle_command(&batch);
        assert!(!out.is_empty());
        assert!(sim.master_enabled);
        assert!(sim.ch_enabled[1]);
        assert_eq!(sim.ch_freq_dhz[1], 20_000);
        assert!(sim.pattern_enabled);
    }
}

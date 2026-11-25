use hotwire_core::protocol::{encode_command_8, encode_frame};

use crate::protocol::types::{
    ChannelConfig, PatternConfig, SweepConfig, CMD_SIGGEN_CH_CFG, CMD_SIGGEN_MASTER,
    CMD_SIGGEN_PAT_CFG, CMD_SIGGEN_SWEEP_CFG,
};

pub fn build_ch_cfg_frame(ch: u8, cfg: &ChannelConfig) -> Vec<u8> {
    let freq_mhz = (cfg.clamped_freq_hz() * 1000.0).round().clamp(100.0, u32::MAX as f64) as u32;
    let duty = cfg.duty_permille.clamp(5, 995);
    let amp = cfg.amp_mv.min(3300);
    let offset = cfg.offset_mv.min(3300);
    let phase = cfg.phase_deg % 360;

    let mut payload = [0u8; 15];
    payload[0] = ch & 0x01;
    payload[1] = u8::from(cfg.enabled);
    payload[2] = cfg.waveform as u8;
    payload[3..7].copy_from_slice(&freq_mhz.to_le_bytes());
    payload[7..9].copy_from_slice(&duty.to_le_bytes());
    payload[9..11].copy_from_slice(&amp.to_le_bytes());
    payload[11..13].copy_from_slice(&offset.to_le_bytes());
    payload[13..15].copy_from_slice(&phase.to_le_bytes());
    encode_frame(CMD_SIGGEN_CH_CFG, &payload)
}

pub fn build_sweep_cfg_frame(cfg: &SweepConfig) -> Vec<u8> {
    let mut payload = [0u8; 14];
    payload[0] = u8::from(cfg.enabled);
    payload[1] = cfg.mode as u8;
    payload[2..6].copy_from_slice(&cfg.start_hz.max(1).to_le_bytes());
    payload[6..10].copy_from_slice(&cfg.stop_hz.max(1).to_le_bytes());
    payload[10..14].copy_from_slice(&cfg.duration_ms.clamp(100, 30_000).to_le_bytes());
    encode_frame(CMD_SIGGEN_SWEEP_CFG, &payload)
}

pub fn build_pattern_cfg_frame(cfg: &PatternConfig) -> Vec<u8> {
    let mut payload = [0u8; 22];
    payload[0] = u8::from(cfg.enabled);
    payload[1] = cfg.step_count.clamp(1, 16);
    payload[2..6].copy_from_slice(&cfg.rate_hz.clamp(1, 20_000).to_le_bytes());
    payload[6..22].copy_from_slice(&cfg.steps);
    encode_frame(CMD_SIGGEN_PAT_CFG, &payload)
}

pub fn build_master_cmd(enabled: bool) -> [u8; 8] {
    encode_command_8(CMD_SIGGEN_MASTER, u8::from(enabled), 0, 0, 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::types::Waveform;

    #[test]
    fn encodes_channel_config_frame_with_valid_length_and_checksum() {
        let mut cfg = ChannelConfig::default_ch1();
        cfg.waveform = Waveform::Pwm;
        cfg.freq_hz = 25_000.0;
        cfg.duty_permille = 350;
        let frame = build_ch_cfg_frame(0, &cfg);
        assert_eq!(frame[0], 0xaa);
        assert_eq!(frame[1], 0x55);
        assert_eq!(frame[2], CMD_SIGGEN_CH_CFG);
        assert_eq!(u16::from_le_bytes([frame[3], frame[4]]), 15);
        assert_eq!(frame.len(), 6 + 15);
    }

    #[test]
    fn encodes_sweep_and_pattern_frames() {
        let sw = build_sweep_cfg_frame(&SweepConfig::default());
        assert_eq!(sw[2], CMD_SIGGEN_SWEEP_CFG);
        assert_eq!(sw.len(), 6 + 14);

        let pat = build_pattern_cfg_frame(&PatternConfig::preset_quadrature());
        assert_eq!(pat[2], CMD_SIGGEN_PAT_CFG);
        assert_eq!(pat.len(), 6 + 22);

        let m = build_master_cmd(true);
        assert_eq!(m[2], CMD_SIGGEN_MASTER);
        assert_eq!(m[3], 1);
    }
}

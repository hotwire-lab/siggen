use crate::protocol::types::SigGenStatus;

/// Decodes the fixed 12-byte `TYPE_SIGGEN_STATUS` payload.
pub fn decode_status(payload: &[u8]) -> Option<SigGenStatus> {
    if payload.len() != 12 {
        return None;
    }
    Some(SigGenStatus {
        master_enabled: payload[0] != 0,
        ch_mask: payload[1],
        sweep_active: payload[2] != 0,
        pattern_active: payload[3] != 0,
        sweep_freq_dhz: u32::from_le_bytes([payload[4], payload[5], payload[6], payload[7]]),
        actual_ch1_dhz: u32::from_le_bytes([payload[8], payload[9], payload[10], payload[11]]),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use hotwire_core::protocol::{encode_frame, TYPE_BOARD_CAPABILITIES};

    #[test]
    fn decodes_status_payload() {
        let mut payload = [0u8; 12];
        payload[0] = 1;
        payload[1] = 0x03;
        payload[2] = 1;
        payload[3] = 0;
        payload[4..8].copy_from_slice(&1_500_000u32.to_le_bytes());
        payload[8..12].copy_from_slice(&1_000_000u32.to_le_bytes());
        let st = decode_status(&payload).expect("status");
        assert!(st.master_enabled);
        assert_eq!(st.ch_mask, 0x03);
        assert_eq!(st.sweep_freq_dhz, 1_500_000);
        assert_eq!(st.actual_ch1_dhz, 1_000_000);

        let caps = encode_frame(
            TYPE_BOARD_CAPABILITIES,
            br#"{"schema":2,"tool":"siggen","board":"XIAO-ESP32-S3","firmware":"1.1.0-siggen","siggen":true}"#,
        );
        assert_eq!(caps[2], TYPE_BOARD_CAPABILITIES);
        assert_eq!(decode_status(&caps), None);
    }
}

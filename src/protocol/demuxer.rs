use hotwire_core::protocol::{BoardIdentity, FrameDemuxer, FrameRule, TYPE_BOARD_CAPABILITIES};

use crate::protocol::types::{DecodedFrame, SigGenStatus, TYPE_SIGGEN_STATUS};

pub struct ProtocolDemuxer {
    inner: FrameDemuxer,
}

impl Default for ProtocolDemuxer {
    fn default() -> Self {
        Self::new()
    }
}

impl ProtocolDemuxer {
    pub fn new() -> Self {
        Self {
            inner: FrameDemuxer::new(vec![
                FrameRule::board_capabilities(),
                FrameRule::new(TYPE_SIGGEN_STATUS, 12, 12),
            ]),
        }
    }

    pub fn reset(&mut self) {
        self.inner.reset();
    }

    pub fn feed<F>(&mut self, chunk: &[u8], mut on_frame: F)
    where
        F: FnMut(DecodedFrame),
    {
        self.inner.feed(chunk, |raw| match raw.type_id {
            TYPE_BOARD_CAPABILITIES => {
                if let Some(id) = BoardIdentity::decode_capabilities(raw.payload) {
                    on_frame(DecodedFrame::Capabilities(id));
                }
            }
            TYPE_SIGGEN_STATUS if raw.payload.len() == 12 => {
                let status = SigGenStatus {
                    master_enabled: raw.payload[0] != 0,
                    ch_mask: raw.payload[1],
                    sweep_active: raw.payload[2] != 0,
                    pattern_active: raw.payload[3] != 0,
                    sweep_freq_mhz: u32::from_le_bytes([
                        raw.payload[4],
                        raw.payload[5],
                        raw.payload[6],
                        raw.payload[7],
                    ]),
                    actual_ch1_mhz: u32::from_le_bytes([
                        raw.payload[8],
                        raw.payload[9],
                        raw.payload[10],
                        raw.payload[11],
                    ]),
                };
                on_frame(DecodedFrame::Status(status));
            }
            _ => {}
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hotwire_core::protocol::encode_frame;

    #[test]
    fn decodes_status_and_capabilities_frames() {
        let mut demux = ProtocolDemuxer::new();
        let caps = encode_frame(
            TYPE_BOARD_CAPABILITIES,
            br#"{"board":"XIAO-ESP32-S3","firmware":"1.0.0-siggen","siggen":true}"#,
        );
        let mut payload = [0u8; 12];
        payload[0] = 1;
        payload[1] = 0x03;
        payload[2] = 1;
        payload[3] = 0;
        payload[4..8].copy_from_slice(&1_500_000u32.to_le_bytes());
        payload[8..12].copy_from_slice(&1_000_000u32.to_le_bytes());
        let st = encode_frame(TYPE_SIGGEN_STATUS, &payload);

        let mut stream = Vec::new();
        stream.extend_from_slice(&caps);
        stream.extend_from_slice(&st);

        let mut got_caps = false;
        let mut got_status = None;
        demux.feed(&stream, |f| match f {
            DecodedFrame::Capabilities(id) => {
                assert_eq!(id.board, "XIAO-ESP32-S3");
                got_caps = true;
            }
            DecodedFrame::Status(s) => got_status = Some(s),
        });
        assert!(got_caps);
        let s = got_status.expect("status");
        assert!(s.master_enabled);
        assert_eq!(s.ch_mask, 0x03);
        assert_eq!(s.sweep_freq_mhz, 1_500_000);
    }
}

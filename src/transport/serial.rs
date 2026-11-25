pub use hotwire_core::transport::{
    HotWireTransport as SerialTransport, SerialConfig, TransportSnapshot,
};

use crate::transport::demo::SigGenDemoBackend;

pub fn create_siggen_transport() -> SerialTransport {
    SerialTransport::with_demo(
        SerialConfig::hotwire_cdc("hotwire_siggen_last_device"),
        SigGenDemoBackend::default(),
    )
}

use hotwire_core::flasher::{FirmwareProfile, FlasherPanel};
use hotwire_core::protocol::{BoardIdentity, FirmwareMismatch, ToolId};
use hotwire_core::ui::BottomSheet;
use leptos::prelude::*;

use crate::transport::serial::SerialTransport;

fn siggen_firmware_profiles() -> Vec<FirmwareProfile> {
    FirmwareProfile::standard_profiles(ToolId::SignalGenerator)
}

#[component]
pub fn BottomDrawer(
    open: RwSignal<bool>,
    on_close: Callback<()>,
    identity: RwSignal<Option<BoardIdentity>>,
    mismatch: RwSignal<Option<FirmwareMismatch>>,
    serial_transport: StoredValue<SerialTransport, LocalStorage>,
) -> impl IntoView {
    view! {
        <BottomSheet
            open=open
            on_close=on_close
            title="⚡ Signal Generator Web Flasher & Pinout"
        >
            <div class="drawer-col">
                <h4>"Web Flasher"</h4>
                <FlasherPanel
                    profiles=siggen_firmware_profiles()
                    transport=serial_transport
                    expected_tool=ToolId::SignalGenerator
                    identity=identity
                    mismatch=mismatch
                />
            </div>

            <div class="drawer-col">
                <h4>"Output Pin Mapping (3.3 V CMOS)"</h4>
                <p class="hw-flash-hint">
                    "CH1: D0 (S3 GPIO1 / C6 GPIO0) · CH2: D1 (S3 GPIO2 / C6 GPIO1) · 4-Ch Pattern: D2–D5 (S3 GPIO3–6 / C6 GPIO2, 21–23)."
                </p>
                <p class="hw-flash-hint">
                    "Square & PWM modes use native hardware LEDC timers (1 Hz – 20 MHz). Analog DDS modes (Sine, Triangle, Sawtooth, DC, Noise) modulate a 312.5 kHz PWM carrier — add a simple 1 kΩ + 10 nF RC low-pass filter on D0/D1 for smooth analog voltage output."
                </p>
            </div>
        </BottomSheet>
    }
}

use hotwire_core::protocol::{BoardIdentity, FirmwareMismatch, ToolId};
use hotwire_core::transport::HandshakeGuard;
use hotwire_core::ui::{FirmwareMismatchModal, HotWireTheme};
use leptos::prelude::*;

use crate::components::bottom_drawer::BottomDrawer;
use crate::components::channel_card::ChannelCard;
use crate::components::pattern_panel::PatternPanel;
use crate::components::sweep_panel::SweepPanel;
use crate::components::top_toolbar::TopToolbar;
use crate::components::waveform_preview::WaveformPreview;
use crate::protocol::codec::{
    build_ch_cfg_frame, build_master_cmd, build_pattern_cfg_frame, build_sweep_cfg_frame,
};
use crate::protocol::demuxer::ProtocolDemuxer;
use crate::protocol::types::{
    ChannelConfig, DecodedFrame, PatternConfig, SigGenStatus, SweepConfig,
};
use crate::transport::serial::{create_siggen_transport, TransportSnapshot};

#[component]
pub fn App() -> impl IntoView {
    let serial_transport = StoredValue::new_local(create_siggen_transport());

    let transport = RwSignal::new(serial_transport.with_value(|t| t.snapshot()));
    let identity = RwSignal::new(None::<BoardIdentity>);
    let mismatch = RwSignal::new(None::<FirmwareMismatch>);
    let flasher_mismatch = RwSignal::new(None::<FirmwareMismatch>);
    let handshake_guard = StoredValue::new_local(HandshakeGuard::new(ToolId::SignalGenerator));

    let master_enabled = RwSignal::new(false);
    let ch1 = RwSignal::new(ChannelConfig::default_ch1());
    let ch2 = RwSignal::new(ChannelConfig::default_ch2());
    let sweep = RwSignal::new(SweepConfig::default());
    let pattern = RwSignal::new(PatternConfig::default());
    let status = RwSignal::new(SigGenStatus::default());
    let active_tab = RwSignal::new("waveforms".to_string());
    let drawer_open = RwSignal::new(false);

    let demuxer = StoredValue::new_local(ProtocolDemuxer::new());

    let sync_all_to_device = move || {
        let mut batch = Vec::with_capacity(96);
        batch.extend_from_slice(&build_ch_cfg_frame(0, &ch1.get_untracked()));
        batch.extend_from_slice(&build_ch_cfg_frame(1, &ch2.get_untracked()));
        batch.extend_from_slice(&build_sweep_cfg_frame(&sweep.get_untracked()));
        batch.extend_from_slice(&build_pattern_cfg_frame(&pattern.get_untracked()));
        batch.extend_from_slice(&build_master_cmd(master_enabled.get_untracked()));
        serial_transport.with_value(|t| t.send_bytes(&batch));
    };

    serial_transport.with_value(|st| {
        st.init(
            move |snap: TransportSnapshot| {
                let connected = snap.status.is_active();
                let is_usb = snap.status.is_usb_connected();
                let dev_name = snap.device_name.clone();
                transport.set(snap);
                if !connected {
                    handshake_guard.update_value(|g| g.cancel());
                    master_enabled.set(false);
                    identity.set(None);
                    status.set(SigGenStatus::default());
                    demuxer.update_value(|d| d.reset());
                } else if is_usb {
                    mismatch.set(None);
                    flasher_mismatch.set(None);
                    handshake_guard.update_value(|g| {
                        g.arm_timeout(dev_name, 5000, move |mm| {
                            mismatch.set(Some(mm.clone()));
                            flasher_mismatch.set(Some(mm));
                            serial_transport.with_value(|t| t.disconnect());
                        });
                    });
                } else {
                    handshake_guard.update_value(|g| g.cancel());
                    mismatch.set(None);
                    flasher_mismatch.set(None);
                }
            },
            move |bytes: &[u8]| {
                demuxer.update_value(|d| {
                    d.feed(bytes, |frame| match frame {
                        DecodedFrame::Capabilities(id) => {
                            serial_transport.with_value(|t| t.notify_handshake_ok());
                            let is_usb = transport.get_untracked().status.is_usb_connected();
                            if is_usb {
                                let mut verdict = Ok(());
                                handshake_guard.update_value(|g| {
                                    verdict = g.verify(&id);
                                });
                                if let Err(mm) = verdict {
                                    mismatch.set(Some(mm.clone()));
                                    flasher_mismatch.set(Some(mm));
                                    serial_transport.with_value(|t| t.disconnect());
                                    return;
                                }
                            } else {
                                handshake_guard.update_value(|g| g.cancel());
                                master_enabled.set(true);
                            }
                            mismatch.set(None);
                            flasher_mismatch.set(None);
                            drawer_open.set(false);
                            identity.set(Some(id));
                            sync_all_to_device();
                        }
                        DecodedFrame::Status(st) => {
                            status.set(st);
                            master_enabled.set(st.master_enabled);
                        }
                    });
                });
            },
        );
    });

    let on_connect = Callback::new(move |_| serial_transport.with_value(|t| t.connect_usb()));
    let on_disconnect = Callback::new(move |_| {
        master_enabled.set(false);
        let cmd = build_master_cmd(false);
        serial_transport.with_value(|t| {
            t.send_bytes(&cmd);
            t.disconnect();
        });
    });
    let on_start_demo = Callback::new(move |_| serial_transport.with_value(|t| t.start_demo()));
    let on_toggle_master = Callback::new(move |_| {
        let next = !master_enabled.get_untracked();
        master_enabled.set(next);
        let cmd = build_master_cmd(next);
        serial_transport.with_value(|t| t.send_bytes(&cmd));
    });
    let on_ch1_change = Callback::new(move |cfg: ChannelConfig| {
        let cmd = build_ch_cfg_frame(0, &cfg);
        serial_transport.with_value(|t| t.send_bytes(&cmd));
    });
    let on_ch2_change = Callback::new(move |cfg: ChannelConfig| {
        let cmd = build_ch_cfg_frame(1, &cfg);
        serial_transport.with_value(|t| t.send_bytes(&cmd));
    });
    let on_sweep_change = Callback::new(move |cfg: SweepConfig| {
        let cmd = build_sweep_cfg_frame(&cfg);
        serial_transport.with_value(|t| t.send_bytes(&cmd));
    });
    let on_pattern_change = Callback::new(move |cfg: PatternConfig| {
        let cmd = build_pattern_cfg_frame(&cfg);
        serial_transport.with_value(|t| t.send_bytes(&cmd));
    });
    let on_toggle_drawer = Callback::new(move |_| drawer_open.update(|v| *v = !*v));
    let on_close_drawer = Callback::new(move |_| drawer_open.set(false));
    let on_open_flasher_from_mismatch = Callback::new(move |_| {
        mismatch.set(None);
        drawer_open.set(true);
    });

    view! {
        <HotWireTheme />
        <div class="siggen-app">
            <TopToolbar
                transport=transport
                identity=identity
                master_enabled=master_enabled
                on_toggle_master=on_toggle_master
                on_connect=on_connect
                on_disconnect=on_disconnect
                on_start_demo=on_start_demo
                drawer_open=drawer_open
                on_toggle_drawer=on_toggle_drawer
            />

            <main class="siggen-main">
                <WaveformPreview
                    master_enabled=master_enabled
                    ch1=ch1
                    ch2=ch2
                    sweep=sweep
                    pattern=pattern
                    status=status
                />

                <div class="siggen-tabs" role="tablist">
                    <button
                        type="button"
                        role="tab"
                        class=move || if active_tab.get() == "waveforms" { "siggen-tab active" } else { "siggen-tab" }
                        on:click=move |_| active_tab.set("waveforms".to_string())
                    >
                        "CH1 & CH2 Waveforms"
                    </button>
                    <button
                        type="button"
                        role="tab"
                        class=move || if active_tab.get() == "sweep" { "siggen-tab active" } else { "siggen-tab" }
                        on:click=move |_| active_tab.set("sweep".to_string())
                    >
                        "Frequency Sweep"
                    </button>
                    <button
                        type="button"
                        role="tab"
                        class=move || if active_tab.get() == "pattern" { "siggen-tab active" } else { "siggen-tab" }
                        on:click=move |_| active_tab.set("pattern".to_string())
                    >
                        "4-Ch Digital Pattern"
                    </button>
                </div>

                <div class="siggen-controls-area">
                    {move || match active_tab.get().as_str() {
                        "sweep" => view! {
                            <SweepPanel sweep=sweep status=status on_change=on_sweep_change />
                        }.into_any(),
                        "pattern" => view! {
                            <PatternPanel pattern=pattern on_change=on_pattern_change />
                        }.into_any(),
                        _ => view! {
                            <div class="siggen-dual-grid">
                                <ChannelCard
                                    ch_index=0
                                    pin_label="D0"
                                    accent="#fb7185"
                                    config=ch1
                                    on_change=on_ch1_change
                                />
                                <ChannelCard
                                    ch_index=1
                                    pin_label="D1"
                                    accent="#38bdf8"
                                    config=ch2
                                    on_change=on_ch2_change
                                />
                            </div>
                        }.into_any(),
                    }}
                </div>
            </main>

            <BottomDrawer
                open=drawer_open
                on_close=on_close_drawer
                identity=identity
                mismatch=flasher_mismatch
                serial_transport=serial_transport
            />

            <FirmwareMismatchModal
                mismatch=mismatch
                on_open_flasher=on_open_flasher_from_mismatch
            />
        </div>
    }
}

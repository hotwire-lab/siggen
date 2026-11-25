use hotwire_core::protocol::{BoardIdentity, ToolId};
use hotwire_core::transport::TransportSnapshot;
use hotwire_core::ui::HotWireToolbar;
use leptos::prelude::*;

#[component]
pub fn TopToolbar(
    transport: RwSignal<TransportSnapshot>,
    identity: RwSignal<Option<BoardIdentity>>,
    master_enabled: RwSignal<bool>,
    on_toggle_master: Callback<()>,
    on_connect: Callback<()>,
    on_disconnect: Callback<()>,
    on_start_demo: Callback<()>,
    drawer_open: RwSignal<bool>,
    on_toggle_drawer: Callback<()>,
) -> impl IntoView {
    let connected = move || transport.get().status.is_active();
    let subtitle = Signal::derive(|| "Dual-Ch DDS / PWM / Pattern".to_string());

    view! {
        <HotWireToolbar
            active_tool=ToolId::SignalGenerator
            subtitle=subtitle
            transport=transport
            identity=identity
            on_connect=on_connect
            on_disconnect=on_disconnect
            on_start_demo=on_start_demo
            flasher_open=Signal::derive(move || drawer_open.get())
            on_toggle_flasher=on_toggle_drawer
        >
            <button
                type="button"
                class=move || {
                    if master_enabled.get() {
                        "hw-btn siggen-master-btn active"
                    } else {
                        "hw-btn siggen-master-btn"
                    }
                }
                disabled=move || !connected()
                on:click=move |_| on_toggle_master.run(())
                title="Master Output Enable / All-Off (D0–D5)"
            >
                <span class="siggen-led" />
                {move || if master_enabled.get() { "OUT ON" } else { "OUT OFF" }}
            </button>
        </HotWireToolbar>
    }
}

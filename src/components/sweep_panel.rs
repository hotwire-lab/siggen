use leptos::prelude::*;

use crate::protocol::types::{format_freq_hz, SigGenStatus, SweepConfig, SweepMode};

fn apply_sweep_update(
    sweep: RwSignal<SweepConfig>,
    on_change: Callback<SweepConfig>,
    f: impl FnOnce(&mut SweepConfig),
) {
    let mut next = sweep.get_untracked();
    f(&mut next);
    sweep.set(next);
    on_change.run(next);
}

#[component]
pub fn SweepPanel(
    sweep: RwSignal<SweepConfig>,
    status: RwSignal<SigGenStatus>,
    on_change: Callback<SweepConfig>,
) -> impl IntoView {
    view! {
        <section class="siggen-card siggen-wide-card" style="--ch-accent: #f0b429;">
            <div class="siggen-card-head">
                <div class="siggen-card-title">
                    <span class="siggen-ch-bar" />
                    <span>"CH1 Frequency Sweep"</span>
                    <span class="siggen-mode-pill">
                        {move || {
                            let st = status.get();
                            if st.sweep_active {
                                format!("Live: {}", format_freq_hz(st.sweep_freq_dhz as f64 / 10.0))
                            } else {
                                "Idle".to_string()
                            }
                        }}
                    </span>
                </div>
                <button
                    type="button"
                    class=move || if sweep.get().enabled { "hw-btn hw-btn-sm siggen-en-btn active" } else { "hw-btn hw-btn-sm siggen-en-btn" }
                    on:click=move |_| apply_sweep_update(sweep, on_change, |s| s.enabled = !s.enabled)
                >
                    {move || if sweep.get().enabled { "Sweep Active" } else { "Enable Sweep" }}
                </button>
            </div>

            <div class="siggen-sweep-grid">
                <label class="siggen-field">
                    <span class="siggen-label">"Mode"</span>
                    <select
                        class="hw-select"
                        prop:value=move || (sweep.get().mode as u8).to_string()
                        on:change=move |ev| {
                            if let Ok(v) = event_target_value(&ev).parse::<u8>() {
                                apply_sweep_update(sweep, on_change, |s| s.mode = SweepMode::from_u8(v));
                            }
                        }
                    >
                        <option value="0">"Linear (Loop)"</option>
                        <option value="1">"Logarithmic (Loop)"</option>
                        <option value="2">"One-Shot Linear"</option>
                    </select>
                </label>

                <label class="siggen-field">
                    <span class="siggen-label">"Start Freq (Hz)"</span>
                    <input
                        type="number"
                        class="siggen-num-input"
                        min="1"
                        max="20000000"
                        prop:value=move || sweep.get().start_hz.to_string()
                        on:change=move |ev| {
                            if let Ok(v) = event_target_value(&ev).parse::<u32>() {
                                apply_sweep_update(sweep, on_change, |s| s.start_hz = v.max(1));
                            }
                        }
                    />
                </label>

                <label class="siggen-field">
                    <span class="siggen-label">"Stop Freq (Hz)"</span>
                    <input
                        type="number"
                        class="siggen-num-input"
                        min="1"
                        max="20000000"
                        prop:value=move || sweep.get().stop_hz.to_string()
                        on:change=move |ev| {
                            if let Ok(v) = event_target_value(&ev).parse::<u32>() {
                                apply_sweep_update(sweep, on_change, |s| s.stop_hz = v.max(1));
                            }
                        }
                    />
                </label>

                <label class="siggen-field">
                    <span class="siggen-label">"Duration (ms)"</span>
                    <input
                        type="number"
                        class="siggen-num-input"
                        min="100"
                        max="30000"
                        step="100"
                        prop:value=move || sweep.get().duration_ms.to_string()
                        on:change=move |ev| {
                            if let Ok(v) = event_target_value(&ev).parse::<u32>() {
                                apply_sweep_update(sweep, on_change, |s| s.duration_ms = v.clamp(100, 30_000));
                            }
                        }
                    />
                </label>
            </div>
        </section>
    }
}

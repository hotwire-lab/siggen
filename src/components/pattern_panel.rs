use leptos::prelude::*;

use crate::protocol::types::PatternConfig;

fn apply_pat_update(
    pattern: RwSignal<PatternConfig>,
    on_change: Callback<PatternConfig>,
    f: impl FnOnce(&mut PatternConfig),
) {
    let mut next = pattern.get_untracked();
    f(&mut next);
    pattern.set(next);
    on_change.run(next);
}

#[component]
pub fn PatternPanel(
    pattern: RwSignal<PatternConfig>,
    on_change: Callback<PatternConfig>,
) -> impl IntoView {

    let apply_preset = move |preset: PatternConfig| {
        pattern.set(preset);
        on_change.run(preset);
    };

    view! {
        <section class="siggen-card siggen-wide-card" style="--ch-accent: #4ade80;">
            <div class="siggen-card-head">
                <div class="siggen-card-title">
                    <span class="siggen-ch-bar" />
                    <span>"4-Channel Digital Pattern Generator (D2–D5)"</span>
                </div>
                <button
                    type="button"
                    class=move || if pattern.get().enabled { "hw-btn hw-btn-sm siggen-en-btn active" } else { "hw-btn hw-btn-sm siggen-en-btn" }
                    on:click=move |_| apply_pat_update(pattern, on_change, |p| p.enabled = !p.enabled)
                >
                    {move || if pattern.get().enabled { "Pattern Active" } else { "Enable Pattern" }}
                </button>
            </div>

            <div class="siggen-pat-controls">
                <label class="siggen-field">
                    <span class="siggen-label">"Step Rate (Hz)"</span>
                    <input
                        type="number"
                        class="siggen-num-input"
                        min="1"
                        max="20000"
                        prop:value=move || pattern.get().rate_hz.to_string()
                        on:change=move |ev| {
                            if let Ok(v) = event_target_value(&ev).parse::<u32>() {
                                apply_pat_update(pattern, on_change, |p| p.rate_hz = v.clamp(1, 20_000));
                            }
                        }
                    />
                </label>

                <label class="siggen-field">
                    <span class="siggen-label">"Steps (1–16)"</span>
                    <input
                        type="number"
                        class="siggen-num-input"
                        min="1"
                        max="16"
                        prop:value=move || pattern.get().step_count.to_string()
                        on:change=move |ev| {
                            if let Ok(v) = event_target_value(&ev).parse::<u8>() {
                                apply_pat_update(pattern, on_change, |p| p.step_count = v.clamp(1, 16));
                            }
                        }
                    />
                </label>

                <div class="siggen-presets">
                    <button type="button" class="siggen-preset-chip" on:click=move |_| apply_preset(PatternConfig::preset_binary_counter())>
                        "4-bit Counter"
                    </button>
                    <button type="button" class="siggen-preset-chip" on:click=move |_| apply_preset(PatternConfig::preset_quadrature())>
                        "Quadrature A/B"
                    </button>
                    <button type="button" class="siggen-preset-chip" on:click=move |_| apply_preset(PatternConfig::preset_walking_one())>
                        "Walking 1s"
                    </button>
                    <button type="button" class="siggen-preset-chip" on:click=move |_| apply_preset(PatternConfig::preset_spi_burst())>
                        "SPI 0xA5 Burst"
                    </button>
                </div>
            </div>

            <div class="siggen-pat-grid-wrap">
                <div class="siggen-pat-grid">
                    {(0..4usize).map(|bit| {
                        let pin_name = ["D2", "D3", "D4", "D5"][bit];
                        view! {
                            <div class="siggen-pat-row">
                                <span class="siggen-pat-pin">{pin_name}</span>
                                {(0..16usize).map(|step| {
                                    view! {
                                        <button
                                            type="button"
                                            class=move || {
                                                let p = pattern.get();
                                                let active_step = step < p.step_count as usize;
                                                let high = (p.steps[step] & (1 << bit)) != 0;
                                                if !active_step {
                                                    "siggen-pat-cell dim"
                                                } else if high {
                                                    "siggen-pat-cell high"
                                                } else {
                                                    "siggen-pat-cell"
                                                }
                                            }
                                            on:click=move |_| {
                                                apply_pat_update(pattern, on_change, |p| {
                                                    p.steps[step] ^= 1 << bit;
                                                });
                                            }
                                            title=format!("{pin_name} Step {step}")
                                        >
                                            {move || if (pattern.get().steps[step] & (1 << bit)) != 0 { "1" } else { "0" }}
                                        </button>
                                    }
                                }).collect_view()}
                            </div>
                        }
                    }).collect_view()}
                </div>
            </div>
        </section>
    }
}

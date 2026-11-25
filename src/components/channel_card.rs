use leptos::prelude::*;

use crate::protocol::types::{ChannelConfig, Waveform, MAX_DDS_HZ, MAX_PWM_HZ};

fn apply_ch_update(
    config: RwSignal<ChannelConfig>,
    on_change: Callback<ChannelConfig>,
    f: impl FnOnce(&mut ChannelConfig),
) {
    let mut next = config.get_untracked();
    f(&mut next);
    next.freq_hz = next.clamped_freq_hz();
    config.set(next);
    on_change.run(next);
}

#[component]
pub fn ChannelCard(
    ch_index: u8,
    pin_label: &'static str,
    accent: &'static str,
    config: RwSignal<ChannelConfig>,
    on_change: Callback<ChannelConfig>,
) -> impl IntoView {
    let update_and_send = move |f: fn(&mut ChannelConfig)| apply_ch_update(config, on_change, f);

    let freq_presets = move || {
        if config.get().waveform.uses_hw_ledc() {
            vec![
                ("100 Hz", 100.0),
                ("1 kHz", 1_000.0),
                ("10 kHz", 10_000.0),
                ("100 kHz", 100_000.0),
                ("1 MHz", 1_000_000.0),
                ("10 MHz", 10_000_000.0),
            ]
        } else {
            vec![
                ("10 Hz", 10.0),
                ("100 Hz", 100.0),
                ("440 Hz", 440.0),
                ("1 kHz", 1_000.0),
                ("2.5 kHz", 2_500.0),
                ("5 kHz", 5_000.0),
            ]
        }
    };

    view! {
        <section class="siggen-card" style=format!("--ch-accent: {accent};")>
            <div class="siggen-card-head">
                <div class="siggen-card-title">
                    <span class="siggen-ch-bar" />
                    <span>{format!("CH{} · {}", ch_index + 1, pin_label)}</span>
                    <span class="siggen-mode-pill">
                        {move || if config.get().waveform.uses_hw_ledc() { "HW LEDC · up to 20 MHz" } else { "PWM-DAC DDS · 0.1 Hz–5 kHz" }}
                    </span>
                </div>
                <button
                    type="button"
                    class=move || if config.get().enabled { "hw-btn hw-btn-sm siggen-en-btn active" } else { "hw-btn hw-btn-sm siggen-en-btn" }
                    on:click=move |_| update_and_send(|c| c.enabled = !c.enabled)
                >
                    {move || if config.get().enabled { "Enabled" } else { "Disabled" }}
                </button>
            </div>

            <div class="siggen-wave-grid">
                {Waveform::ALL.into_iter().map(|w| {
                    view! {
                        <button
                            type="button"
                            class=move || if config.get().waveform == w { "siggen-wave-btn active" } else { "siggen-wave-btn" }
                            on:click=move |_| apply_ch_update(config, on_change, |c| c.waveform = w)
                        >
                            {w.label()}
                        </button>
                    }
                }).collect_view()}
            </div>

            <div class="siggen-param-row">
                <label class="siggen-label">"Frequency (Hz)"</label>
                <input
                    type="number"
                    class="siggen-num-input"
                    min=move || if config.get().waveform.uses_hw_ledc() { "1" } else { "0.1" }
                    max=move || if config.get().waveform.uses_hw_ledc() { MAX_PWM_HZ.to_string() } else { MAX_DDS_HZ.to_string() }
                    step=move || if config.get().waveform.uses_hw_ledc() { "1" } else { "0.1" }
                    prop:value=move || {
                        let f = config.get().freq_hz;
                        if f >= 100.0 { format!("{:.0}", f) } else { format!("{:.1}", f) }
                    }
                    on:change=move |ev| {
                        if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                            if v.is_finite() && v > 0.0 {
                                apply_ch_update(config, on_change, |c| c.freq_hz = v);
                            }
                        }
                    }
                />
            </div>

            <div class="siggen-presets">
                {move || freq_presets().into_iter().map(|(label, hz)| {
                    view! {
                        <button
                            type="button"
                            class="siggen-preset-chip"
                            on:click=move |_| apply_ch_update(config, on_change, |c| c.freq_hz = hz)
                        >
                            {label}
                        </button>
                    }
                }).collect_view()}
            </div>

            {move || {
                let c = config.get();
                if c.waveform == Waveform::Pwm {
                    view! {
                        <div class="siggen-slider-row">
                            <span class="siggen-label">"Duty Cycle"</span>
                            <input
                                type="range"
                                min="5"
                                max="995"
                                step="5"
                                prop:value=move || config.get().duty_permille.to_string()
                                on:input=move |ev| {
                                    if let Ok(v) = event_target_value(&ev).parse::<u16>() {
                                        apply_ch_update(config, on_change, |cfg| cfg.duty_permille = v);
                                    }
                                }
                            />
                            <span class="siggen-val">{move || format!("{:.1}%", config.get().duty_permille as f64 / 10.0)}</span>
                        </div>
                    }.into_any()
                } else if !c.waveform.uses_hw_ledc() {
                    view! {
                        <div class="siggen-slider-row">
                            <span class="siggen-label">"Amplitude"</span>
                            <input
                                type="range"
                                min="0"
                                max="3300"
                                step="50"
                                prop:value=move || config.get().amp_mv.to_string()
                                on:input=move |ev| {
                                    if let Ok(v) = event_target_value(&ev).parse::<u16>() {
                                        apply_ch_update(config, on_change, |cfg| cfg.amp_mv = v);
                                    }
                                }
                            />
                            <span class="siggen-val">{move || format!("{:.2} Vpp", config.get().amp_mv as f64 / 1000.0)}</span>
                        </div>
                        <div class="siggen-slider-row">
                            <span class="siggen-label">"DC Offset"</span>
                            <input
                                type="range"
                                min="0"
                                max="3300"
                                step="50"
                                prop:value=move || config.get().offset_mv.to_string()
                                on:input=move |ev| {
                                    if let Ok(v) = event_target_value(&ev).parse::<u16>() {
                                        apply_ch_update(config, on_change, |cfg| cfg.offset_mv = v);
                                    }
                                }
                            />
                            <span class="siggen-val">{move || format!("{:.2} V", config.get().offset_mv as f64 / 1000.0)}</span>
                        </div>
                    }.into_any()
                } else {
                    view! { <div class="siggen-hint">"50% duty square wave · direct hardware LEDC clock divider"</div> }.into_any()
                }
            }}

            {move || {
                if ch_index == 1 && !config.get().waveform.uses_hw_ledc() {
                    Some(view! {
                        <div class="siggen-slider-row">
                            <span class="siggen-label">"Phase vs CH1"</span>
                            <input
                                type="range"
                                min="0"
                                max="355"
                                step="5"
                                prop:value=move || config.get().phase_deg.to_string()
                                on:input=move |ev| {
                                    if let Ok(v) = event_target_value(&ev).parse::<u16>() {
                                        apply_ch_update(config, on_change, |cfg| cfg.phase_deg = v);
                                    }
                                }
                            />
                            <span class="siggen-val">{move || format!("{}°", config.get().phase_deg)}</span>
                        </div>
                    })
                } else {
                    None
                }
            }}
        </section>
    }
}

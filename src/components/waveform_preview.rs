use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::protocol::types::{
    format_freq_hz, ChannelConfig, PatternConfig, SigGenStatus, SweepConfig, Waveform,
};

fn eval_wave_volts(cfg: &ChannelConfig, phase_cycles: f64, seed: u32) -> f64 {
    let frac = phase_cycles.rem_euclid(1.0);
    if cfg.waveform.uses_hw_ledc() {
        let duty = if cfg.waveform == Waveform::Square {
            0.5
        } else {
            (cfg.duty_permille as f64 / 1000.0).clamp(0.005, 0.995)
        };
        return if frac < duty { 3.3 } else { 0.0 };
    }

    let norm = match cfg.waveform {
        Waveform::Sine => (2.0 * std::f64::consts::PI * frac).sin(),
        Waveform::Triangle => {
            if frac < 0.25 {
                frac * 4.0
            } else if frac < 0.75 {
                2.0 - frac * 4.0
            } else {
                frac * 4.0 - 4.0
            }
        }
        Waveform::Sawtooth => 2.0 * frac - 1.0,
        Waveform::RevSaw => 1.0 - 2.0 * frac,
        Waveform::Dc => 0.0,
        Waveform::Noise => {
            let mut x = seed ^ 0x9e37_79b9;
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            ((x & 0xffff) as f64 / 32767.5) - 1.0
        }
        Waveform::Square | Waveform::Pwm => 0.0,
    };

    let center_v = cfg.offset_mv as f64 / 1000.0;
    let half_amp_v = (cfg.amp_mv as f64 / 1000.0) * 0.5;
    (center_v + norm * half_amp_v).clamp(0.0, 3.3)
}

#[component]
pub fn WaveformPreview(
    master_enabled: RwSignal<bool>,
    ch1: RwSignal<ChannelConfig>,
    ch2: RwSignal<ChannelConfig>,
    sweep: RwSignal<SweepConfig>,
    pattern: RwSignal<PatternConfig>,
    status: RwSignal<SigGenStatus>,
) -> impl IntoView {
    let canvas_ref = NodeRef::<html::Canvas>::new();
    let resize_rev = RwSignal::new(0u64);

    Effect::new(move |_| {
        let cb = wasm_bindgen::closure::Closure::<dyn FnMut()>::new(move || {
            resize_rev.update(|v| *v = v.wrapping_add(1));
        });
        if let Some(w) = web_sys::window() {
            let _ = w.add_event_listener_with_callback("resize", cb.as_ref().unchecked_ref());
        }
        cb.forget();
    });

    Effect::new(move |_| {
        let _rev = resize_rev.get();
        let master = master_enabled.get();
        let c1 = ch1.get();
        let c2 = ch2.get();
        let sw = sweep.get();
        let pat = pattern.get();
        let st = status.get();

        let Some(canvas) = canvas_ref.get() else {
            return;
        };
        let rect = canvas.get_bounding_client_rect();
        let css_w = rect.width().max(260.0);
        let css_h = rect.height().max(150.0);
        let dpr = web_sys::window()
            .map(|w| w.device_pixel_ratio().clamp(1.0, 2.5))
            .unwrap_or(1.0);
        let tw = (css_w * dpr).round() as u32;
        let th = (css_h * dpr).round() as u32;
        if canvas.width() != tw || canvas.height() != th {
            canvas.set_width(tw);
            canvas.set_height(th);
        }

        let Ok(Some(ctx_obj)) = canvas.get_context("2d") else {
            return;
        };
        let Ok(ctx) = ctx_obj.dyn_into::<web_sys::CanvasRenderingContext2d>() else {
            return;
        };
        let _ = ctx.set_transform(dpr, 0.0, 0.0, dpr, 0.0, 0.0);

        // Background
        ctx.set_fill_style_str("#060a10");
        ctx.fill_rect(0.0, 0.0, css_w, css_h);

        let pad_l = 44.0;
        let pad_r = 12.0;
        let pad_t = 24.0;
        let pad_b = 20.0;
        let plot_w = (css_w - pad_l - pad_r).max(120.0);
        let plot_h = (css_h - pad_t - pad_b).max(80.0);

        // 10x4 voltage graticule (0V .. 3.3V)
        ctx.set_stroke_style_str("rgba(148, 163, 184, 0.12)");
        ctx.set_line_width(1.0);
        ctx.begin_path();
        for i in 0..=10 {
            let x = pad_l + (i as f64 / 10.0) * plot_w;
            ctx.move_to(x, pad_t);
            ctx.line_to(x, pad_t + plot_h);
        }
        for j in 0..=4 {
            let y = pad_t + (j as f64 / 4.0) * plot_h;
            ctx.move_to(pad_l, y);
            ctx.line_to(pad_l + plot_w, y);
        }
        ctx.stroke();

        // Y-axis voltage labels
        ctx.set_font("10px 'IBM Plex Mono', monospace");
        ctx.set_fill_style_str("#7d8796");
        for (j, label) in ["3.3V", "2.5V", "1.65V", "0.8V", "0.0V"].iter().enumerate() {
            let y = pad_t + (j as f64 / 4.0) * plot_h + 3.5;
            let _ = ctx.fill_text(label, 4.0, y);
        }

        // Choose reference time window = 3 cycles of CH1 (or CH2 if CH1 disabled)
        let eff_c1_hz = if st.sweep_active && st.sweep_freq_dhz > 0 {
            st.sweep_freq_dhz as f64 / 10.0
        } else {
            c1.clamped_freq_hz()
        };
        let ref_hz = if c1.enabled {
            eff_c1_hz
        } else if c2.enabled {
            c2.clamped_freq_hz()
        } else {
            1000.0
        };
        let window_sec = 3.0 / ref_hz.max(0.1);

        let v_to_y = |v: f64| pad_t + (1.0 - (v / 3.3).clamp(0.0, 1.0)) * plot_h;

        let draw_channel = |cfg: &ChannelConfig, f_hz: f64, phase_deg: f64, color: &str| {
            if !cfg.enabled {
                return;
            }
            let alpha_stroke = if master { color } else { "rgba(148, 163, 184, 0.32)" };
            ctx.save();
            ctx.set_stroke_style_str(alpha_stroke);
            ctx.set_line_width(if master { 2.0 } else { 1.2 });
            ctx.begin_path();
            let steps = (plot_w as usize).clamp(160, 640);
            let phase_off = phase_deg / 360.0;
            for i in 0..=steps {
                let frac_x = i as f64 / steps as f64;
                let t = frac_x * window_sec;
                let v = eval_wave_volts(cfg, t * f_hz + phase_off, i as u32 * 2654435761);
                let x = pad_l + frac_x * plot_w;
                let y = v_to_y(v);
                if i == 0 {
                    ctx.move_to(x, y);
                } else {
                    ctx.line_to(x, y);
                }
            }
            ctx.stroke();
            ctx.restore();
        };

        draw_channel(&c2, c2.clamped_freq_hz(), c2.phase_deg as f64, "#38bdf8");
        draw_channel(&c1, eff_c1_hz, 0.0, "#fb7185");

        // Border
        ctx.set_stroke_style_str("#232a35");
        ctx.set_line_width(1.0);
        ctx.stroke_rect(pad_l, pad_t, plot_w, plot_h);

        // Top HUD line
        ctx.set_font("600 10.5px 'IBM Plex Mono', monospace");
        ctx.set_fill_style_str(if master { "#fb7185" } else { "#7d8796" });
        let ch1_hud = if c1.enabled {
            let mode_tag = if c1.waveform.uses_hw_ledc() { "LEDC" } else { "DDS" };
            let sw_tag = if sw.enabled { " SWEEP" } else { "" };
            format!("CH1(D0): {} {} [{}{}]", c1.waveform.label(), format_freq_hz(eff_c1_hz), mode_tag, sw_tag)
        } else {
            "CH1(D0): OFF".to_string()
        };
        let _ = ctx.fill_text(&ch1_hud, pad_l + 2.0, pad_t - 7.0);

        let ch2_hud = if c2.enabled {
            format!("CH2(D1): {} {}", c2.waveform.label(), format_freq_hz(c2.clamped_freq_hz()))
        } else if pat.enabled {
            format!("PAT(D2-D5): {} steps @ {}", pat.step_count, format_freq_hz(pat.rate_hz as f64))
        } else {
            "CH2(D1): OFF".to_string()
        };
        ctx.set_fill_style_str(if master && (c2.enabled || pat.enabled) { "#38bdf8" } else { "#7d8796" });
        let tw = ctx.measure_text(&ch2_hud).map(|m| m.width()).unwrap_or(140.0);
        let _ = ctx.fill_text(&ch2_hud, pad_l + plot_w - tw, pad_t - 7.0);

        // Bottom timebase + RC filter hint
        ctx.set_font("10px 'IBM Plex Mono', monospace");
        ctx.set_fill_style_str("#7d8796");
        let any_dds = (c1.enabled && !c1.waveform.uses_hw_ledc()) || (c2.enabled && !c2.waveform.uses_hw_ledc());
        let footer = if any_dds {
            format!("Window: {:.2} ms · DDS uses 312.5 kHz PWM carrier (add 1kΩ + 10nF RC LPF for smooth analog)", window_sec * 1000.0)
        } else {
            format!("Window: {:.3} ms · Native hardware LEDC 3.3 V CMOS output", window_sec * 1000.0)
        };
        let _ = ctx.fill_text(&footer, pad_l + 2.0, pad_t + plot_h + 14.0);
    });

    view! {
        <div class="siggen-preview-wrap">
            <canvas node_ref=canvas_ref class="siggen-preview-canvas" />
        </div>
    }
}

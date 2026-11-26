use hotwire_core::protocol::BoardIdentity;

pub const CMD_SIGGEN_CH_CFG: u8 = 0x50;
pub const CMD_SIGGEN_SWEEP_CFG: u8 = 0x51;
pub const CMD_SIGGEN_PAT_CFG: u8 = 0x52;
pub const CMD_SIGGEN_MASTER: u8 = 0x53;

pub const TYPE_SIGGEN_STATUS: u8 = 0x87;

pub const MAX_DDS_HZ: f64 = 5_000.0;
pub const MAX_PWM_HZ: f64 = 20_000_000.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Waveform {
    Sine = 0,
    Square = 1,
    Triangle = 2,
    Sawtooth = 3,
    RevSaw = 4,
    Pwm = 5,
    Dc = 6,
    Noise = 7,
}

impl Waveform {
    pub const ALL: [Waveform; 8] = [
        Waveform::Sine,
        Waveform::Square,
        Waveform::Triangle,
        Waveform::Sawtooth,
        Waveform::RevSaw,
        Waveform::Pwm,
        Waveform::Dc,
        Waveform::Noise,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Waveform::Sine => "Sine",
            Waveform::Square => "Square",
            Waveform::Triangle => "Triangle",
            Waveform::Sawtooth => "Sawtooth",
            Waveform::RevSaw => "Rev Saw",
            Waveform::Pwm => "PWM",
            Waveform::Dc => "DC Level",
            Waveform::Noise => "White Noise",
        }
    }

    /// Returns `true` when the waveform uses native hardware LEDC (1 Hz – 20 MHz, 3.3 V CMOS).
    pub const fn uses_hw_ledc(self) -> bool {
        matches!(self, Waveform::Square | Waveform::Pwm)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChannelConfig {
    pub enabled: bool,
    pub waveform: Waveform,
    pub freq_hz: f64,
    /// Duty cycle in permille (`5..=995` -> `0.5%..=99.5%`)
    pub duty_permille: u16,
    /// Peak-to-peak amplitude in mV (`0..=3300`) for DDS modes
    pub amp_mv: u16,
    /// DC offset center in mV (`0..=3300`) for DDS modes
    pub offset_mv: u16,
    /// Phase offset in degrees (`0..360`) for CH2 relative to CH1
    pub phase_deg: u16,
}

impl ChannelConfig {
    pub fn default_ch1() -> Self {
        Self {
            enabled: true,
            waveform: Waveform::Sine,
            freq_hz: 1000.0,
            duty_permille: 500,
            amp_mv: 3000,
            offset_mv: 1650,
            phase_deg: 0,
        }
    }

    pub fn default_ch2() -> Self {
        Self {
            enabled: false,
            waveform: Waveform::Square,
            freq_hz: 1000.0,
            duty_permille: 500,
            amp_mv: 3000,
            offset_mv: 1650,
            phase_deg: 90,
        }
    }

    pub fn clamped_freq_hz(&self) -> f64 {
        if self.waveform.uses_hw_ledc() {
            self.freq_hz.clamp(1.0, MAX_PWM_HZ)
        } else {
            self.freq_hz.clamp(0.1, MAX_DDS_HZ)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SweepMode {
    LinearLoop = 0,
    LogLoop = 1,
    OneShotLinear = 2,
}

impl SweepMode {
    pub const fn from_u8(v: u8) -> Self {
        match v {
            1 => SweepMode::LogLoop,
            2 => SweepMode::OneShotLinear,
            _ => SweepMode::LinearLoop,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SweepConfig {
    pub enabled: bool,
    pub mode: SweepMode,
    pub start_hz: u32,
    pub stop_hz: u32,
    pub duration_ms: u32,
}

impl Default for SweepConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: SweepMode::LinearLoop,
            start_hz: 100,
            stop_hz: 2000,
            duration_ms: 3000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PatternConfig {
    pub enabled: bool,
    pub step_count: u8,
    pub rate_hz: u32,
    pub steps: [u8; 16],
}

impl Default for PatternConfig {
    fn default() -> Self {
        Self::preset_binary_counter()
    }
}

impl PatternConfig {
    pub fn preset_binary_counter() -> Self {
        let mut steps = [0u8; 16];
        for (i, s) in steps.iter_mut().enumerate() {
            *s = (i as u8) & 0x0f;
        }
        Self {
            enabled: false,
            step_count: 16,
            rate_hz: 1000,
            steps,
        }
    }

    pub fn preset_quadrature() -> Self {
        // CH0=A, CH1=B (90 deg shifted), CH2=Index pulse on step 0, CH3=Clock
        let mut steps = [0u8; 16];
        let quad = [0b00, 0b01, 0b11, 0b10];
        for (i, s) in steps.iter_mut().enumerate() {
            let ab = quad[i % 4];
            let idx = if i == 0 { 0b0100 } else { 0 };
            let clk = if i % 2 == 0 { 0b1000 } else { 0 };
            *s = ab | idx | clk;
        }
        Self {
            enabled: true,
            step_count: 8,
            rate_hz: 1000,
            steps,
        }
    }

    pub fn preset_walking_one() -> Self {
        let mut steps = [0u8; 16];
        for (i, s) in steps.iter_mut().enumerate() {
            *s = 1u8 << (i % 4);
        }
        Self {
            enabled: true,
            step_count: 4,
            rate_hz: 500,
            steps,
        }
    }

    pub fn preset_spi_burst() -> Self {
        // D2=CS (active low), D3=SCK, D4=MOSI (0xA5), D5=Sync
        let bits = [1u8, 0, 1, 0, 0, 1, 0, 1];
        let mut steps = [0u8; 16];
        for i in 0..8 {
            let mosi = bits[i] << 2;
            steps[i * 2] = mosi; // CS=0, SCK=0
            steps[i * 2 + 1] = 0b0010 | mosi; // CS=0, SCK=1
        }
        steps[0] |= 0b1000; // Sync pulse on first edge
        steps[15] = 0b0001; // CS high idle
        Self {
            enabled: true,
            step_count: 16,
            rate_hz: 2000,
            steps,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SigGenStatus {
    pub master_enabled: bool,
    pub ch_mask: u8,
    pub sweep_active: bool,
    pub pattern_active: bool,
    pub sweep_freq_dhz: u32,
    pub actual_ch1_dhz: u32,
}

#[derive(Debug, Clone)]
pub enum DecodedFrame {
    Capabilities(BoardIdentity),
    Status(SigGenStatus),
}

pub fn format_freq_hz(hz: f64) -> String {
    if hz >= 1_000_000.0 {
        format!("{:.3} MHz", hz / 1_000_000.0)
    } else if hz >= 1_000.0 {
        format!("{:.2} kHz", hz / 1_000.0)
    } else if hz >= 10.0 {
        format!("{:.1} Hz", hz)
    } else {
        format!("{:.2} Hz", hz)
    }
}

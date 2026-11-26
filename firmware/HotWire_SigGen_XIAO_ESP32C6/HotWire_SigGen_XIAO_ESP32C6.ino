/*
 * SPDX-License-Identifier: Apache-2.0
 * Copyright (c) 2026 Kari Vierimaa, Kempele
 *
 * HotWire Signal Generator — Dual-Channel DDS Waveform, Hardware PWM/Square,
 * Frequency Sweep & 4-Channel Digital Pattern Generator
 * Target: Seeed Studio XIAO ESP32-C6 (RISC-V)
 *
 * Pinout (3.3 V max):
 *   - CH1 Output:      D0 (GPIO0)  — Hardware LEDC Square/PWM (1 Hz–20 MHz) or 312.5 kHz PWM-DAC DDS (0.1 Hz–5 kHz)
 *   - CH2 Output:      D1 (GPIO1)  — Independent Square/PWM or phase-locked DDS (0°–359° relative to CH1)
 *   - Pattern Out 0-3: D2..D5 (GPIO2, GPIO21, GPIO22, GPIO23) — 4-ch programmable 16-step digital pattern (1 Hz–20 kHz)
 *
 * Arduino IDE Settings (arduino-esp32 3.x / ESP-IDF 5.x):
 *   - Board: "XIAO_ESP32C6"
 *   - USB CDC On Boot: "Enabled"
 */

#include <Arduino.h>
#include <math.h>
#include <string.h>
#include "driver/gpio.h"
#include "driver/ledc.h"
#include "esp_system.h"
#include "esp_timer.h"
#include "soc/lp_aon_reg.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "freertos/semphr.h"

#ifndef CONFIG_IDF_TARGET_ESP32C6
  #error "This sketch targets Seeed Studio XIAO ESP32-C6."
#endif

#define HOTWIRE_FW_VERSION "1.1.0-siggen-c6"
#define HOTWIRE_BOARD_NAME "XIAO-ESP32-C6"

static const int CH_PINS[2]  = { 0, 1 };            // D0, D1
static const int PAT_PINS[4] = { 2, 21, 22, 23 };   // D2, D3, D4, D5

static const uint8_t SYNC_0 = 0xAA;
static const uint8_t SYNC_1 = 0x55;

static const uint8_t CMD_PING_STATUS      = 0x01;
static const uint8_t CMD_GET_CAPS         = 0x02;
static const uint8_t CMD_SIGGEN_CH_CFG    = 0x50;
static const uint8_t CMD_SIGGEN_SWEEP_CFG = 0x51;
static const uint8_t CMD_SIGGEN_PAT_CFG   = 0x52;
static const uint8_t CMD_SIGGEN_MASTER    = 0x53;
static const uint8_t CMD_BOOTLOADER       = 0xF0;

static const uint8_t TYPE_BOARD_CAPS      = 0x83;
static const uint8_t TYPE_SIGGEN_STATUS   = 0x87;

static const uint8_t WAVE_SINE     = 0;
static const uint8_t WAVE_SQUARE   = 1;
static const uint8_t WAVE_TRIANGLE = 2;
static const uint8_t WAVE_SAW      = 3;
static const uint8_t WAVE_REV_SAW  = 4;
static const uint8_t WAVE_PWM      = 5;
static const uint8_t WAVE_DC       = 6;
static const uint8_t WAVE_NOISE    = 7;

static const uint32_t DDS_SAMPLE_RATE_HZ = 20000;
static const uint32_t DDS_CARRIER_HZ     = 312500;

struct ChannelCfg {
  bool     enabled;
  uint8_t  waveform;
  uint32_t freq_dhz;      // decihertz (10 = 1.0 Hz, 200000000 = 20 MHz)
  uint16_t duty_permille;
  uint16_t amp_mv;
  uint16_t offset_mv;
  uint16_t phase_deg;
};

struct SweepCfg {
  bool     enabled;
  uint8_t  mode;
  uint32_t start_hz;
  uint32_t stop_hz;
  uint32_t duration_ms;
  uint32_t start_ms;
  uint32_t current_dhz;
  bool     finished;
};

struct PatternCfg {
  bool     enabled;
  uint8_t  step_count;
  uint32_t rate_hz;
  uint8_t  steps[16];
  uint8_t  cur_step;
};

static bool       g_master_en = false;
static ChannelCfg g_ch[2] = {
  { true,  WAVE_SINE,   10000, 500, 3000, 1650, 0 },
  { false, WAVE_SQUARE, 10000, 500, 3000, 1650, 90 }
};
static SweepCfg   g_sweep = { false, 0, 100, 2000, 3000, 0, 1000, false };
static PatternCfg g_pat   = { false, 8, 1000, {0,1,2,3,4,5,6,7,0,0,0,0,0,0,0,0}, 0 };

static bool     g_ch_is_dds[2] = { false, false };
static bool     g_ch_hw_active[2] = { false, false };
static uint32_t g_ch_actual_dhz[2] = { 10000, 10000 };
static uint32_t g_dds_phase_acc[2] = { 0, 0 };
static uint32_t g_dds_phase_inc[2] = { 0, 0 };
static uint32_t g_prng_state = 0x12345678u;

static uint8_t g_sine_lut[256];
static SemaphoreHandle_t g_serial_mutex = nullptr;
static SemaphoreHandle_t g_state_mutex  = nullptr;
static esp_timer_handle_t g_dds_timer   = nullptr;
static esp_timer_handle_t g_pat_timer   = nullptr;
static bool g_dds_timer_running = false;
static bool g_pat_timer_running = false;

static uint8_t compute_xor(const uint8_t* data, size_t len) {
  uint8_t x = 0;
  for (size_t i = 0; i < len; ++i) x ^= data[i];
  return x;
}

static bool serial_write_all(const uint8_t* data, size_t len) {
  size_t sent = 0;
  uint32_t start_ms = millis();
  while (sent < len) {
    size_t chunk = min<size_t>(512, len - sent);
    size_t written = Serial.write(data + sent, chunk);
    if (written > 0) {
      sent += written;
      start_ms = millis();
    } else {
      if ((millis() - start_ms) > 150) return false;
      vTaskDelay(pdMS_TO_TICKS(1));
    }
  }
  return true;
}

static void send_frame(uint8_t type_id, const uint8_t* payload, uint16_t len) {
  uint8_t hdr[5] = {
    SYNC_0, SYNC_1, type_id,
    (uint8_t)(len & 0xFF), (uint8_t)((len >> 8) & 0xFF)
  };
  uint8_t chk = compute_xor(hdr, 5) ^ compute_xor(payload, len);
  if (g_serial_mutex) xSemaphoreTake(g_serial_mutex, portMAX_DELAY);
  if (serial_write_all(hdr, 5)) {
    if (len == 0 || serial_write_all(payload, len)) {
      serial_write_all(&chk, 1);
    }
  }
  if (g_serial_mutex) xSemaphoreGive(g_serial_mutex);
}

static void send_capabilities() {
  const char* json =
    "{\"board\":\"" HOTWIRE_BOARD_NAME "\","
    "\"firmware\":\"" HOTWIRE_FW_VERSION "\","
    "\"siggen\":true,\"max_channels\":2,\"pattern_channels\":4,"
    "\"max_pwm_hz\":20000000,\"dds_rate_hz\":20000}";
  send_frame(TYPE_BOARD_CAPS, (const uint8_t*)json, (uint16_t)strlen(json));
}

static void send_status() {
  uint8_t payload[12];
  uint8_t ch_mask = 0;
  uint32_t sweep_dhz = 0;
  uint32_t ch1_dhz = 0;
  bool master = false;
  bool sw_active = false;
  bool pat_active = false;

  if (g_state_mutex) xSemaphoreTake(g_state_mutex, portMAX_DELAY);
  master = g_master_en;
  if (g_ch[0].enabled) ch_mask |= 0x01;
  if (g_ch[1].enabled) ch_mask |= 0x02;
  sw_active = g_master_en && g_ch[0].enabled && g_sweep.enabled && !g_sweep.finished;
  pat_active = g_master_en && g_pat.enabled;
  sweep_dhz = sw_active ? g_sweep.current_dhz : g_ch_actual_dhz[0];
  ch1_dhz = g_ch_actual_dhz[0];
  if (g_state_mutex) xSemaphoreGive(g_state_mutex);

  payload[0] = master ? 1 : 0;
  payload[1] = ch_mask;
  payload[2] = sw_active ? 1 : 0;
  payload[3] = pat_active ? 1 : 0;
  memcpy(&payload[4], &sweep_dhz, 4);
  memcpy(&payload[8], &ch1_dhz, 4);
  send_frame(TYPE_SIGGEN_STATUS, payload, sizeof(payload));
}

static bool waveform_uses_hw_ledc(uint8_t wave) {
  return wave == WAVE_SQUARE || wave == WAVE_PWM;
}

static ledc_timer_bit_t choose_ledc_bits(uint32_t freq_hz) {
  if (freq_hz <= 4000)    return LEDC_TIMER_13_BIT;
  if (freq_hz <= 15000)   return LEDC_TIMER_12_BIT;
  if (freq_hz <= 60000)   return LEDC_TIMER_10_BIT;
  if (freq_hz <= 250000)  return LEDC_TIMER_8_BIT;
  if (freq_hz <= 1000000) return LEDC_TIMER_6_BIT;
  if (freq_hz <= 4000000) return LEDC_TIMER_4_BIT;
  if (freq_hz <= 10000000) return LEDC_TIMER_2_BIT;
  return LEDC_TIMER_1_BIT;
}

static uint32_t calc_dds_phase_inc(uint32_t freq_dhz) {
  uint64_t num = ((uint64_t)freq_dhz << 32) / 10ULL;
  return (uint32_t)(num / DDS_SAMPLE_RATE_HZ);
}

static void stop_channel_hw(int ch) {
  ledc_channel_t lch = (ch == 0) ? LEDC_CHANNEL_0 : LEDC_CHANNEL_1;
  if (g_ch_hw_active[ch]) {
    ledc_stop(LEDC_LOW_SPEED_MODE, lch, 0);
    g_ch_hw_active[ch] = false;
  }
  g_ch_is_dds[ch] = false;
  pinMode(CH_PINS[ch], OUTPUT);
  digitalWrite(CH_PINS[ch], LOW);
}

static void apply_channel_hw_locked(int ch, uint32_t override_freq_dhz) {
  ChannelCfg& c = g_ch[ch];
  ledc_timer_t ltimer = (ch == 0) ? LEDC_TIMER_0 : LEDC_TIMER_1;
  ledc_channel_t lch  = (ch == 0) ? LEDC_CHANNEL_0 : LEDC_CHANNEL_1;

  if (!g_master_en || !c.enabled) {
    stop_channel_hw(ch);
    return;
  }

  uint32_t eff_dhz = (override_freq_dhz > 0) ? override_freq_dhz : c.freq_dhz;

  if (waveform_uses_hw_ledc(c.waveform)) {
    uint32_t freq_hz = max<uint32_t>(1, min<uint32_t>(20000000, (eff_dhz + 5u) / 10u));
    ledc_timer_bit_t bits = choose_ledc_bits(freq_hz);
    uint32_t max_duty = (1u << bits);
    uint16_t duty_pm = (c.waveform == WAVE_SQUARE) ? 500 : constrain(c.duty_permille, 5, 995);
    uint32_t duty = (uint32_t)(((uint64_t)max_duty * duty_pm) / 1000ULL);
    if (duty == 0) duty = 1;
    if (duty >= max_duty) duty = max_duty - 1;

    ledc_timer_config_t tcfg = {};
    tcfg.speed_mode = LEDC_LOW_SPEED_MODE;
    tcfg.duty_resolution = bits;
    tcfg.timer_num = ltimer;
    tcfg.freq_hz = freq_hz;
    tcfg.clk_cfg = LEDC_AUTO_CLK;
    if (ledc_timer_config(&tcfg) != ESP_OK) {
      tcfg.duty_resolution = LEDC_TIMER_1_BIT;
      duty = 1;
      ledc_timer_config(&tcfg);
    }

    ledc_channel_config_t ccfg = {};
    ccfg.gpio_num = CH_PINS[ch];
    ccfg.speed_mode = LEDC_LOW_SPEED_MODE;
    ccfg.channel = lch;
    ccfg.timer_sel = ltimer;
    ccfg.duty = duty;
    ccfg.hpoint = 0;
    ledc_channel_config(&ccfg);

    g_ch_is_dds[ch] = false;
    g_ch_hw_active[ch] = true;
    g_ch_actual_dhz[ch] = freq_hz * 10u;
  } else {
    uint32_t clamped_dhz = constrain(eff_dhz, 1u, 50000u);
    if (!g_ch_is_dds[ch] || !g_ch_hw_active[ch]) {
      ledc_timer_config_t tcfg = {};
      tcfg.speed_mode = LEDC_LOW_SPEED_MODE;
      tcfg.duty_resolution = LEDC_TIMER_8_BIT;
      tcfg.timer_num = ltimer;
      tcfg.freq_hz = DDS_CARRIER_HZ;
      tcfg.clk_cfg = LEDC_AUTO_CLK;
      ledc_timer_config(&tcfg);

      ledc_channel_config_t ccfg = {};
      ccfg.gpio_num = CH_PINS[ch];
      ccfg.speed_mode = LEDC_LOW_SPEED_MODE;
      ccfg.channel = lch;
      ccfg.timer_sel = ltimer;
      ccfg.duty = 128;
      ccfg.hpoint = 0;
      ledc_channel_config(&ccfg);
    }
    g_dds_phase_inc[ch] = calc_dds_phase_inc(clamped_dhz);
    if (ch == 1) {
      uint32_t offset_acc = (uint32_t)(((uint64_t)(c.phase_deg % 360) << 32) / 360ULL);
      g_dds_phase_acc[1] = g_dds_phase_acc[0] + offset_acc;
    }
    g_ch_is_dds[ch] = true;
    g_ch_hw_active[ch] = true;
    g_ch_actual_dhz[ch] = clamped_dhz;
  }
}

static void sync_dds_timer_locked() {
  bool need = g_master_en && (g_ch_is_dds[0] || g_ch_is_dds[1]);
  if (need && !g_dds_timer_running && g_dds_timer) {
    esp_timer_start_periodic(g_dds_timer, 1000000ULL / DDS_SAMPLE_RATE_HZ);
    g_dds_timer_running = true;
  } else if (!need && g_dds_timer_running && g_dds_timer) {
    esp_timer_stop(g_dds_timer);
    g_dds_timer_running = false;
  }
}

static void sync_pat_timer_locked() {
  if (g_pat_timer_running && g_pat_timer) {
    esp_timer_stop(g_pat_timer);
    g_pat_timer_running = false;
  }
  if (g_master_en && g_pat.enabled && g_pat.step_count > 0) {
    uint32_t rate = constrain(g_pat.rate_hz, 1u, 20000u);
    uint64_t period_us = max<uint64_t>(50ULL, 1000000ULL / rate);
    g_pat.cur_step = 0;
    if (g_pat_timer) {
      esp_timer_start_periodic(g_pat_timer, period_us);
      g_pat_timer_running = true;
    }
  } else {
    for (int i = 0; i < 4; ++i) {
      gpio_set_level((gpio_num_t)PAT_PINS[i], 0);
    }
  }
}

static inline uint8_t eval_dds_sample(int ch) {
  const ChannelCfg& c = g_ch[ch];
  g_dds_phase_acc[ch] += g_dds_phase_inc[ch];
  uint8_t idx = (uint8_t)(g_dds_phase_acc[ch] >> 24);
  int32_t raw = 128;

  switch (c.waveform) {
    case WAVE_SINE:     raw = g_sine_lut[idx]; break;
    case WAVE_TRIANGLE: raw = (idx < 128) ? (idx * 2) : ((255 - idx) * 2); break;
    case WAVE_SAW:      raw = idx; break;
    case WAVE_REV_SAW:  raw = 255 - idx; break;
    case WAVE_DC:       raw = 128; break;
    case WAVE_NOISE: {
      uint32_t x = g_prng_state;
      x ^= x << 13;
      x ^= x >> 17;
      x ^= x << 5;
      g_prng_state = x;
      raw = (int32_t)(x & 0xFF);
      break;
    }
    default:            raw = 128; break;
  }

  int32_t centered = raw - 128;
  int32_t mv = (int32_t)c.offset_mv + ((centered * (int32_t)c.amp_mv) >> 8);
  if (mv < 0) mv = 0;
  if (mv > 3300) mv = 3300;
  return (uint8_t)((mv * 255) / 3300);
}

static void dds_timer_cb(void* arg) {
  (void)arg;
  if (!g_master_en) return;
  if (g_ch_is_dds[0]) {
    uint8_t d = eval_dds_sample(0);
    ledc_set_duty(LEDC_LOW_SPEED_MODE, LEDC_CHANNEL_0, d);
    ledc_update_duty(LEDC_LOW_SPEED_MODE, LEDC_CHANNEL_0);
  }
  if (g_ch_is_dds[1]) {
    uint8_t d = eval_dds_sample(1);
    ledc_set_duty(LEDC_LOW_SPEED_MODE, LEDC_CHANNEL_1, d);
    ledc_update_duty(LEDC_LOW_SPEED_MODE, LEDC_CHANNEL_1);
  }
}

static void pat_timer_cb(void* arg) {
  (void)arg;
  if (!g_master_en || !g_pat.enabled || g_pat.step_count == 0) return;
  uint8_t mask = g_pat.steps[g_pat.cur_step];
  for (int i = 0; i < 4; ++i) {
    gpio_set_level((gpio_num_t)PAT_PINS[i], (mask >> i) & 1);
  }
  g_pat.cur_step++;
  if (g_pat.cur_step >= g_pat.step_count) g_pat.cur_step = 0;
}

static void handle_frame(uint8_t cmd_id, const uint8_t* payload, uint16_t len) {
  switch (cmd_id) {
    case CMD_PING_STATUS:
    case CMD_GET_CAPS:
      send_capabilities();
      send_status();
      break;

    case CMD_SIGGEN_CH_CFG:
      if (len >= 14) {
        uint8_t ch = payload[0] & 0x01;
        if (g_state_mutex) xSemaphoreTake(g_state_mutex, portMAX_DELAY);
        g_ch[ch].enabled       = payload[1] != 0;
        g_ch[ch].waveform      = min<uint8_t>(payload[2], WAVE_NOISE);
        memcpy(&g_ch[ch].freq_dhz, &payload[3], 4);
        memcpy(&g_ch[ch].duty_permille, &payload[7], 2);
        memcpy(&g_ch[ch].amp_mv, &payload[9], 2);
        memcpy(&g_ch[ch].offset_mv, &payload[11], 2);
        g_ch[ch].phase_deg     = (uint16_t)payload[13] * 2;
        if (len >= 15) {
          memcpy(&g_ch[ch].phase_deg, &payload[13], 2);
        }
        g_ch[ch].phase_deg %= 360;
        apply_channel_hw_locked(ch, 0);
        sync_dds_timer_locked();
        if (g_state_mutex) xSemaphoreGive(g_state_mutex);
        send_status();
      }
      break;

    case CMD_SIGGEN_SWEEP_CFG:
      if (len >= 14) {
        if (g_state_mutex) xSemaphoreTake(g_state_mutex, portMAX_DELAY);
        g_sweep.enabled = payload[0] != 0;
        g_sweep.mode    = min<uint8_t>(payload[1], 2);
        memcpy(&g_sweep.start_hz, &payload[2], 4);
        memcpy(&g_sweep.stop_hz, &payload[6], 4);
        memcpy(&g_sweep.duration_ms, &payload[10], 4);
        g_sweep.start_hz    = max<uint32_t>(1, g_sweep.start_hz);
        g_sweep.stop_hz     = max<uint32_t>(1, g_sweep.stop_hz);
        g_sweep.duration_ms = constrain(g_sweep.duration_ms, 100u, 30000u);
        g_sweep.start_ms    = millis();
        g_sweep.finished    = false;
        g_sweep.current_dhz = g_sweep.start_hz * 10u;
        if (!g_sweep.enabled) {
          apply_channel_hw_locked(0, 0);
          sync_dds_timer_locked();
        }
        if (g_state_mutex) xSemaphoreGive(g_state_mutex);
        send_status();
      }
      break;

    case CMD_SIGGEN_PAT_CFG:
      if (len >= 22) {
        if (g_state_mutex) xSemaphoreTake(g_state_mutex, portMAX_DELAY);
        g_pat.enabled    = payload[0] != 0;
        g_pat.step_count = constrain(payload[1], 1, 16);
        memcpy(&g_pat.rate_hz, &payload[2], 4);
        memcpy(g_pat.steps, &payload[6], 16);
        sync_pat_timer_locked();
        if (g_state_mutex) xSemaphoreGive(g_state_mutex);
        send_status();
      }
      break;

    case CMD_SIGGEN_MASTER:
      if (len >= 1) {
        if (g_state_mutex) xSemaphoreTake(g_state_mutex, portMAX_DELAY);
        g_master_en = payload[0] != 0;
        if (g_master_en && g_sweep.enabled) {
          g_sweep.start_ms = millis();
          g_sweep.finished = false;
        }
        apply_channel_hw_locked(0, 0);
        apply_channel_hw_locked(1, 0);
        sync_dds_timer_locked();
        sync_pat_timer_locked();
        if (g_state_mutex) xSemaphoreGive(g_state_mutex);
        send_status();
      }
      break;

    case CMD_BOOTLOADER:
      if (len >= 4 && payload[0] == 0xB0 && payload[1] == 0x07 && payload[2] == 0x10 && payload[3] == 0xAD) {
        g_master_en = false;
        stop_channel_hw(0);
        stop_channel_hw(1);
        sync_dds_timer_locked();
        sync_pat_timer_locked();
        vTaskDelay(pdMS_TO_TICKS(20));
        REG_SET_BIT(LP_AON_SYS_CFG_REG, LP_AON_FORCE_DOWNLOAD_BOOT);
        vTaskDelay(pdMS_TO_TICKS(10));
        esp_restart();
      }
      break;

    default:
      break;
  }
}

static void protocol_rx_task(void* arg) {
  (void)arg;
  uint8_t buf[64];
  size_t idx = 0;
  uint16_t expected_len = 0;
  bool is_fixed_8 = false;

  while (true) {
    while (Serial.available() > 0) {
      uint8_t b = (uint8_t)Serial.read();
      if (idx == 0) {
        if (b == SYNC_0) buf[idx++] = b;
      } else if (idx == 1) {
        if (b == SYNC_1) buf[idx++] = b;
        else idx = (b == SYNC_0) ? 1 : 0;
      } else if (idx == 2) {
        buf[idx++] = b;
        is_fixed_8 = (b == CMD_PING_STATUS || b == CMD_GET_CAPS || b == CMD_BOOTLOADER || b == CMD_SIGGEN_MASTER);
      } else {
        buf[idx++] = b;
        if (is_fixed_8) {
          if (idx == 8) {
            if (compute_xor(buf, 7) == buf[7]) {
              handle_frame(buf[2], &buf[3], 4);
            }
            idx = 0;
          }
        } else {
          if (idx == 5) {
            expected_len = (uint16_t)buf[3] | ((uint16_t)buf[4] << 8);
            if (expected_len > sizeof(buf) - 6) {
              idx = 0;
            }
          } else if (idx >= 6 && idx == (size_t)(6 + expected_len)) {
            if (compute_xor(buf, 5 + expected_len) == buf[5 + expected_len]) {
              handle_frame(buf[2], &buf[5], expected_len);
            }
            idx = 0;
          }
        }
      }
    }
    vTaskDelay(pdMS_TO_TICKS(1));
  }
}

void setup() {
  Serial.begin(921600);
  Serial.setRxBufferSize(2048);

  for (int i = 0; i < 256; ++i) {
    float s = sinf((2.0f * (float)M_PI * (float)i) / 256.0f);
    g_sine_lut[i] = (uint8_t)constrain((int)lroundf((s + 1.0f) * 127.5f), 0, 255);
  }

  for (int i = 0; i < 2; ++i) {
    pinMode(CH_PINS[i], OUTPUT);
    digitalWrite(CH_PINS[i], LOW);
  }
  for (int i = 0; i < 4; ++i) {
    pinMode(PAT_PINS[i], OUTPUT);
    digitalWrite(PAT_PINS[i], LOW);
  }

  g_serial_mutex = xSemaphoreCreateMutex();
  g_state_mutex  = xSemaphoreCreateMutex();

  esp_timer_create_args_t dds_args = {};
  dds_args.callback = dds_timer_cb;
  dds_args.dispatch_method = ESP_TIMER_TASK;
  dds_args.name = "hw_dds";
  esp_timer_create(&dds_args, &g_dds_timer);

  esp_timer_create_args_t pat_args = {};
  pat_args.callback = pat_timer_cb;
  pat_args.dispatch_method = ESP_TIMER_TASK;
  pat_args.name = "hw_pat";
  esp_timer_create(&pat_args, &g_pat_timer);

  xTaskCreate(protocol_rx_task, "siggen_rx", 4096, nullptr, 2, nullptr);
}

void loop() {
  static uint32_t last_status_ms = 0;
  uint32_t now = millis();

  if (g_state_mutex) xSemaphoreTake(g_state_mutex, portMAX_DELAY);
  if (g_master_en && g_ch[0].enabled && g_sweep.enabled && !g_sweep.finished) {
    uint32_t elapsed = now - g_sweep.start_ms;
    if (elapsed >= g_sweep.duration_ms) {
      if (g_sweep.mode == 2) {
        elapsed = g_sweep.duration_ms;
        g_sweep.finished = true;
      } else {
        g_sweep.start_ms = now;
        elapsed = 0;
      }
    }
    float t = (float)elapsed / (float)g_sweep.duration_ms;
    float f_hz;
    if (g_sweep.mode == 1) {
      float f0 = max(1.0f, (float)g_sweep.start_hz);
      float f1 = max(1.0f, (float)g_sweep.stop_hz);
      f_hz = f0 * powf(f1 / f0, t);
    } else {
      f_hz = (float)g_sweep.start_hz + ((float)g_sweep.stop_hz - (float)g_sweep.start_hz) * t;
    }
    uint32_t target_dhz = (uint32_t)max(1.0f, f_hz * 10.0f);
    if (!waveform_uses_hw_ledc(g_ch[0].waveform)) {
      target_dhz = min<uint32_t>(target_dhz, 50000u);
    }
    if (target_dhz != g_sweep.current_dhz) {
      g_sweep.current_dhz = target_dhz;
      apply_channel_hw_locked(0, target_dhz);
      sync_dds_timer_locked();
    }
  }
  if (g_state_mutex) xSemaphoreGive(g_state_mutex);

  if (now - last_status_ms >= 100) {
    last_status_ms = now;
    send_status();
  }
  vTaskDelay(pdMS_TO_TICKS(20));
}

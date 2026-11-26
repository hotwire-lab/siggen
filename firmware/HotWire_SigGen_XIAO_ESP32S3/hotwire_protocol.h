// Shared HotWire ESP32 firmware protocol constants and helpers.
// SPDX-License-Identifier: Apache-2.0
#pragma once

#include <Arduino.h>

#define HOTWIRE_CAPS_SCHEMA 2
#define HOTWIRE_SYNC_0 0xAA
#define HOTWIRE_SYNC_1 0x55
#define HOTWIRE_BOOTLOADER_MAGIC_0 0xB0
#define HOTWIRE_BOOTLOADER_MAGIC_1 0x07
#define HOTWIRE_BOOTLOADER_MAGIC_2 0x10
#define HOTWIRE_BOOTLOADER_MAGIC_3 0xAD

static inline uint8_t hotwire_xor(const uint8_t* data, size_t len) {
  uint8_t value = 0;
  for (size_t i = 0; i < len; ++i) {
    value ^= data[i];
  }
  return value;
}

static inline bool hotwire_is_bootloader_magic(uint8_t a, uint8_t b, uint8_t c, uint8_t d) {
  return a == HOTWIRE_BOOTLOADER_MAGIC_0 && b == HOTWIRE_BOOTLOADER_MAGIC_1 &&
         c == HOTWIRE_BOOTLOADER_MAGIC_2 && d == HOTWIRE_BOOTLOADER_MAGIC_3;
}

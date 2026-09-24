#pragma once
#include <cstdint>

namespace ldm {
// Low 16 bits retain the original BIOS character/scan pair. Native-only tags:
// bit 16 marks an auto-repeat, bits 17-23 hold its alternative movement scan,
// bits 24-30 identify the physical key for release even if Num Lock changes.
constexpr uint32_t KeyRepeat=1u<<16;
constexpr unsigned KeyMovementShift=17,KeySourceShift=24;
inline bool direction_scan(uint16_t scan) {
    switch(scan) {
    case 0x47:case 0x48:case 0x49:case 0x4b:
    case 0x4d:case 0x4f:case 0x50:case 0x51:return true;
    default:return false;
    }
}
inline uint16_t bios_key(uint32_t key,bool movement) {
    auto scan=(key>>KeyMovementShift)&0x7f;
    return movement && scan?uint16_t(scan<<8):uint16_t(key);
}
inline bool repeat_from(uint32_t key,unsigned source) {
    return (key&KeyRepeat) && (key>>KeySourceShift)==source;
}
}

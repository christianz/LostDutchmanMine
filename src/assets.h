#pragma once
#include <cstdint>
#include <vector>
namespace ldm {
// LDM's 0x9d01 stream: MSB-first codes, 0x100 grows code width;
// each consecutive pair of decoded tokens defines a dictionary entry.
std::vector<uint8_t> decode_asset(const std::vector<uint8_t>& packed);
}

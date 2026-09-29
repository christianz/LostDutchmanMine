// A C interface to one ymfm YM3812. Timers expire against a clock advanced by
// the caller, exactly as the original native build's Audio::Impl did.
#include "ymfm_opl.h"
#include <cstring>
#include <vector>

namespace {
struct Chip : ymfm::ymfm_interface {
    uint64_t clock = 0;
    int64_t timers[2] = {-1, -1};
    ymfm::ym3812 chip{*this};

    Chip() { chip.reset(); }
    void ymfm_set_timer(uint32_t number, int32_t clocks) override {
        timers[number] = clocks < 0 ? -1 : int64_t(clock) + clocks;
    }
    void advance(uint32_t clocks) {
        clock += clocks;
        for (unsigned i = 0; i < 2; i++)
            if (timers[i] >= 0 && uint64_t(timers[i]) <= clock) {
                timers[i] = -1;
                m_engine->engine_timer_expired(i);
            }
    }
    // ymfm's saved state has no 64-bit fields: the clock and timers lead as
    // three little-endian 64-bit values, followed by the chip's own state.
    void save(std::vector<uint8_t>& buffer) {
        buffer.clear();
        for (uint64_t value : {clock, uint64_t(timers[0]), uint64_t(timers[1])})
            for (int shift = 0; shift < 64; shift += 8) buffer.push_back(uint8_t(value >> shift));
        std::vector<uint8_t> state_bytes;
        ymfm::ymfm_saved_state state(state_bytes, true);
        chip.save_restore(state);
        buffer.insert(buffer.end(), state_bytes.begin(), state_bytes.end());
    }
    void restore(const uint8_t* data, size_t size) {
        if (size < 24) return;
        auto word = [&](size_t at) {
            uint64_t value = 0;
            for (int i = 0; i < 8; i++) value |= uint64_t(data[at + i]) << (8 * i);
            return value;
        };
        clock = word(0);
        timers[0] = int64_t(word(8));
        timers[1] = int64_t(word(16));
        std::vector<uint8_t> state_bytes(data + 24, data + size);
        ymfm::ymfm_saved_state state(state_bytes, false);
        chip.save_restore(state);
    }
};
}

extern "C" {
void* ldm_opl_new() { return new Chip; }
void ldm_opl_free(void* chip) { delete static_cast<Chip*>(chip); }
void ldm_opl_write(void* chip, uint32_t offset, uint8_t value) { static_cast<Chip*>(chip)->chip.write(offset, value); }
uint8_t ldm_opl_read(void* chip, uint32_t offset) { return static_cast<Chip*>(chip)->chip.read(offset); }
void ldm_opl_advance(void* chip, uint32_t clocks) { static_cast<Chip*>(chip)->advance(clocks); }
int32_t ldm_opl_generate(void* chip) {
    ymfm::ym3812::output_data sample;
    static_cast<Chip*>(chip)->chip.generate(&sample);
    return sample.data[0];
}
// Writes the state into `out` (capacity `size`); returns the bytes needed.
size_t ldm_opl_save(void* chip, uint8_t* out, size_t size) {
    std::vector<uint8_t> buffer;
    static_cast<Chip*>(chip)->save(buffer);
    if (buffer.size() <= size) std::memcpy(out, buffer.data(), buffer.size());
    return buffer.size();
}
void ldm_opl_restore(void* chip, const uint8_t* data, size_t size) {
    static_cast<Chip*>(chip)->restore(data, size);
}
}

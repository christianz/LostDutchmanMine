#pragma once
#include <cstddef>
#include <cstdint>
#include <memory>

namespace ldm {
// Native FM synthesis and PC speaker output. No CPU or DOS execution here.
class Audio {
public:
    Audio();
    ~Audio();
    void write(unsigned offset, uint8_t value);
    uint8_t read(unsigned offset);
    void speaker(unsigned hz);
    // Advance the chip's timers by emulated time when no audio device renders.
    void advance_clock(unsigned clocks);
    void render(float* output, size_t count);
    uint64_t writes() const;
    uint64_t audible_samples() const;
private:
    struct Impl;
    std::unique_ptr<Impl> impl;
};
}

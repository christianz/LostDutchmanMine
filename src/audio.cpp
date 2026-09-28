#include "audio.h"
#include "ymfm_opl.h"
#include <algorithm>
#include <array>
#include <atomic>
#include <cmath>
#include <mutex>

namespace ldm {
struct Audio::Impl : ymfm::ymfm_interface {
    std::mutex mutex;
    uint64_t clock=0;
    std::array<int64_t,2> timers{{-1,-1}};
    ymfm::ym3812 chip{*this};
    double fraction=0,phase=0;
    float previous=0,current=0;
    unsigned speaker_hz=0;
    std::atomic<uint64_t> register_writes{0},nonzero_samples{0};

    Impl() { chip.reset(); }
    void ymfm_set_timer(uint32_t number,int32_t clocks) override {
        timers.at(number)=clocks<0?-1:int64_t(clock)+clocks;
    }
    void advance(unsigned clocks) {
        clock+=clocks;
        for(unsigned i=0;i<2;i++)if(timers[i]>=0 && uint64_t(timers[i])<=clock) {
            timers[i]=-1;
            m_engine->engine_timer_expired(i);
        }
    }
};
Audio::Audio():impl(std::make_unique<Impl>()) {}
Audio::~Audio()=default;
void Audio::write(unsigned offset,uint8_t value) {
    std::lock_guard<std::mutex> lock(impl->mutex);
    impl->advance(12); // OPL port accesses retain the original ISA delay loops.
    impl->chip.write(offset,value);
    if(offset&1)++impl->register_writes;
}
uint8_t Audio::read(unsigned offset) {
    std::lock_guard<std::mutex> lock(impl->mutex);
    impl->advance(12);
    return impl->chip.read(offset);
}
void Audio::speaker(unsigned hz) {
    std::lock_guard<std::mutex> lock(impl->mutex);
    impl->speaker_hz=hz;
}
void Audio::advance_clock(unsigned clocks) {
    std::lock_guard<std::mutex> lock(impl->mutex);
    impl->advance(clocks);
}
void Audio::render(float* output,size_t count) {
    std::lock_guard<std::mutex> lock(impl->mutex);
    const double ratio=3579545.0/72.0/48000.0;
    uint64_t audible=0;
    for(size_t i=0;i<count;i++) {
        impl->fraction+=ratio;
        while(impl->fraction>=1) {
            impl->fraction-=1;
            ymfm::ym3812::output_data sample;
            impl->chip.generate(&sample);
            impl->advance(72);
            impl->previous=impl->current;
            impl->current=sample.data[0]/32768.0f;
        }
        float fm=impl->previous+(impl->current-impl->previous)*float(impl->fraction);
        impl->phase+=impl->speaker_hz/48000.0;
        impl->phase-=std::floor(impl->phase);
        float pc=impl->speaker_hz?(impl->phase<0.5?0.08f:-0.08f):0.0f;
        output[i]=std::clamp(fm*0.7f+pc,-1.0f,1.0f);
        if(std::abs(output[i])>0.0001f)++audible;
    }
    impl->nonzero_samples+=audible;
}
uint64_t Audio::writes() const {return impl->register_writes.load();}
uint64_t Audio::audible_samples() const {return impl->nonzero_samples.load();}
}

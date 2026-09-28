#pragma once
#include "display.h"
#include "legacy.h"
#include <atomic>
#include <exception>
#include <mutex>
#include <ostream>
#include <thread>

namespace ldm {
struct Snapshot {
    Pixels pixels{};
    uint64_t sequence=0,boundaries=0;
    int video_mode=3,x=0,y=0,town_page=0,building=0;
    int mouse_visibility=-1,mouse_mode=1,mouse_x=160,mouse_y=100;
    bool pointer_visible=false,desert_view=false,map_view=false;
    bool cave_view=false;
    int map_scroll_x=0,map_scroll_y=0,return_x=0,return_y=0;
    int survival_ticks=0;
    uint8_t directions=0;
    bool custom_cursor=false;
    bool panning_active=false;
    int gold_bags=0;
    uint32_t cash=0;
    int assay_pounds=0,assay_grade=0;
    bool qol_improvements=true;
    bool combat_active=false;
    int bullets=0;
    int sight_x=0,sight_y=0;
    bool mining_space_held=false;
    int mining_strokes=0;
};
void read_frame(const State& state,Pixels& pixels);
// FNV-1a 64 over the registers, 1 MiB memory and palette: the trace identity.
uint64_t state_hash(const State& state);
// Threaded sessions follow the wall clock. Deterministic sessions run exactly
// one emulated millisecond per step(), so identical inputs reproduce exactly.
enum class SessionMode { Threaded, Deterministic };
// The game has a dedicated clock. A blocking GPU present, slow monitor or open
// settings window must never change the cadence of the original timer handler.
class Session {
public:
    explicit Session(State& state,SessionMode mode=SessionMode::Threaded);
    ~Session();
    Session(const Session&)=delete;
    void step();
    // Append a trace line every 100 emulated milliseconds, and at finish_trace().
    void set_trace(std::ostream* out);
    void finish_trace();
    void key(uint32_t code);
    void release_repeat(uint16_t physical_key);
    void directions(uint8_t mask);
    void space(bool held);
    void mouse(int x,int y);
    void buttons(int mask);
    void clear_input();
    void qol(bool enabled);
    void pause(bool paused);
    bool paused() const {return paused_;}
    bool finished();
    void snapshot(Snapshot& output);
    void stop();
private:
    enum class Kind { Key,Release,Directions,Space,Mouse,Buttons,Clear,Qol };
    struct Command {Kind kind;int a=0,b=0;};
    State& state_;
    SessionMode mode_;
    std::mutex mutex_,frame_mutex_;
    std::vector<Command> commands_;
    Snapshot frame_;
    std::exception_ptr error_;
    std::atomic<bool> stopping_{false},done_{false},paused_{false};
    uint8_t directions_=0;
    uint64_t pit_clocks_=0,quanta_=0,traced_=0;
    std::ostream* trace_=nullptr;
    std::thread thread_;
    void send(Command command);
    void run();
    void quantum(uint64_t pit_clocks,uint64_t opl_clocks);
    Snapshot capture() const;
    void write_trace();
};
}

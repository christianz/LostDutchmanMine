#pragma once
#include "display.h"
#include "game_ui.h"
#include <array>
#include <filesystem>

namespace ldm {
// An optional native activity. Original inventory and rewards remain in the
// recovered game; this returns only whether the player kept any gold.
class Panning {
public:
    enum class Phase { Idle, Rock, Rinse, Result, Done };
    void art_path(const std::filesystem::path& path){art_path_=path;}
    void begin();
    void update(double seconds,uint8_t directions);
    void key(uint16_t bios,bool repeat=false);
    void pointer(int x,int y);
    void buttons(int mask);
    void clear_input();
    void draw(Pixels& pixels,const GameUI* ui=nullptr,const std::array<uint32_t,256>* palette=nullptr) const;
    bool engaged() const{return phase_!=Phase::Idle;}
    bool active() const{return engaged() && phase_!=Phase::Done;}
    bool done() const{return phase_==Phase::Done;}
    bool take_result();
    Phase phase() const{return phase_;}
    int round() const{return round_;}
    int loosened() const{return loosened_;}
    int gold() const{return gold_;}
    double tilt() const{return tilt_;}
private:
    Phase phase_=Phase::Idle;
    int round_=0,loosened_=0,gold_=5,lost_last_=0,last_side_=0,mouse_x_=160,mouse_y_=100,buttons_=0;
    bool dragging_=false,success_=false;
    double tilt_=0,age_=0,rinse_time_=0,stroke_time_=0,drag_origin_=0,drag_tilt_=0;
    std::filesystem::path art_path_;
    std::array<uint32_t,320*112> creek_{};
    bool art_loaded_=false;
    void wash();
    void finish(bool keep);
    void load_art();
};
}

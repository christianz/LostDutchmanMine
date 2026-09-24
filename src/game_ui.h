#pragma once
#include "display.h"
#include <array>
#include <string>

namespace ldm {
struct State;
struct GameButton {
    int x,y,w,h;
    bool contains(int px,int py) const{return px>=x && px<x+w && py>=y && py<y+h;}
};

// Reuses the player's original panel atlas and eight-pixel lettering. These
// presentation changes never write into the original framebuffer or save data.
class GameUI {
public:
    std::array<uint8_t,2048> font{};
    bool choosing=false;
    bool mule_shop=false,mule_shop_visible=false;
    unsigned context_buttons=0;
    void prepare(const State& state);
    void context(const State& state);
    void map_click(State& state) const;
    void draw(Pixels& pixels,const State& state) const;
    void button(Pixels& pixels,const std::array<uint32_t,256>& palette,GameButton bounds,
                const std::string& label,bool hover=false,bool pressed=false,bool enabled=true) const;
    void text(Pixels& pixels,int x,int y,const std::string& value,uint32_t colour,bool centre=false) const;
    bool ready() const{return ready_;}
    bool pointer_visible(const State& state) const;
    static GameButton toolbar(int i){return {52+i*42,167,40,32};}
    static GameButton action(int i){return {i<2?72:152,i%2?138:118,75,19};}
private:
    bool ready_=false;
    std::array<uint8_t,64000> atlas_{};
    bool visible(const State& state) const;
};
}

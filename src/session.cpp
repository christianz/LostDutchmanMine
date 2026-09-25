#include "session.h"
#include <algorithm>
#include <chrono>

namespace ldm {
void read_frame(const State& s,Pixels& pixels) {
    if(s.video_mode==0x13) {
        for(size_t i=0;i<pixels.size();i++)pixels[i]=s.palette[s.memory[0xa0000+i]];
    }else if(s.video_mode==4 || s.video_mode==5) {
        const uint32_t cga[]={0xff000000,0xff55ffff,0xffff55ff,0xffffffff};
        for(unsigned y=0;y<200;y++)for(unsigned x=0;x<320;x++) {
            auto v=s.memory[0xb8000+(y%2)*8192+(y/2)*80+x/4];
            pixels[y*320+x]=cga[(v>>(6-2*(x%4)))&3];
        }
    }else pixels.fill(0xff000000);
    // The encounter's six-tick loop still owns enemies, shots and hit tests.
    // Its sight is composited from the resident original sprite at display
    // cadence, so mouse motion need not wait for the next simulation step.
    if(s.video_mode==0x13 && s.combat_active && s.combat_sight_visible &&
       s.u16(0x82bd,0x5e0c) && !s.game_ui.menu_open() &&
       !s.u16(0x82bd,0x5302) && s.u16(0x82bd,0x53e0)) {
        auto aim=s.combat_aim();
        auto sprite=s.u16(LoadSegment+0x1b14,0xfb37); // Original sprite page 1.
        for(int y=0;y<16;y++)for(int x=0;x<16;x++) {
            int px=aim.x+x,py=aim.y+y;
            if(px<0 || px>=320 || py<11 || py>=111)continue;
            auto c=s.u8(sprite,uint16_t((160+y)*320+120+x));
            // Match the original transparent VGA blitter at 1265:0427.
            if(c<128 || c==255)pixels[py*320+px]=s.palette[c==255?0:c&15];
        }
    }
    s.game_ui.draw(pixels,s);
    if(s.custom_cursor && s.game_ui.pointer_visible(s)) {
        // A small outlined arrow with its tip at the hit-test coordinate.
        static constexpr const char* arrow[]={
            "X           ","XX          ","X.X         ","X..X        ",
            "X...X       ","X....X      ","X.....X     ","X......X    ",
            "X.......X   ","X....XXXXX  ","X..X..X     ","X.X X..X    ",
            "XX  X..X    ","X    X..X   ","     X..X   ","      XX    "};
        bool arrow_cursor=s.qol_improvements;
        for(int y=0;y<16;y++)for(int x=0;x<16;x++) {
            int px=s.mouse.current().x-(arrow_cursor?0:s.mouse_hot_x)+x;
            int py=s.mouse.current().y-(arrow_cursor?0:s.mouse_hot_y)+y;
            if(px<0 || px>=320 || py<0 || py>=200)continue;
            auto& c=pixels[py*320+px];
            if(arrow_cursor) {
                if(x<12 && arrow[y][x]!=' ')c=arrow[y][x]=='X'?0xff000000:0xffffffff;
            }else {
                if(!(s.mouse_mask[y]&(0x8000>>x)))c=0xff000000;
                if(s.mouse_mask[y+16]&(0x8000>>x))c^=0xffffff;
            }
        }
    }
}
Session::Session(State& s):state_(s),thread_([this]{run();}) {}
Session::~Session(){stop();}
void Session::stop(){stopping_=true;if(thread_.joinable())thread_.join();}
void Session::send(Command command){std::lock_guard<std::mutex> lock(mutex_);commands_.push_back(command);}
void Session::key(uint32_t code){send({Kind::Key,int(code)});}
void Session::release_repeat(uint16_t physical_key){send({Kind::Release,physical_key});}
void Session::directions(uint8_t mask){send({Kind::Directions,mask});}
void Session::space(bool held){send({Kind::Space,held});}
void Session::mouse(int x,int y){send({Kind::Mouse,x,y});}
void Session::buttons(int mask){send({Kind::Buttons,mask});}
void Session::clear_input(){send({Kind::Clear});}
void Session::qol(bool enabled){send({Kind::Qol,enabled});}
void Session::pause(bool paused){paused_=paused;}
bool Session::finished(){return done_;}
void Session::snapshot(Snapshot& output) {
    std::lock_guard<std::mutex> lock(frame_mutex_);
    if(error_)std::rethrow_exception(error_);
    output=frame_;
}
void Session::run() {
    using Clock=std::chrono::steady_clock;
    auto last=Clock::now(),published=last;
    double timer_elapsed=0;uint8_t directions=0;
    try {
        while(!stopping_ && state_.running) {
            auto now=Clock::now();
            if(paused_) {
                last=now;
                std::this_thread::sleep_for(std::chrono::milliseconds(1));continue;
            }
            double elapsed=std::chrono::duration<double>(now-last).count();
            timer_elapsed+=elapsed;last=now;
            bool was_panning=state_.panning_active;
            std::vector<Command> commands;
            {std::lock_guard<std::mutex> lock(mutex_);commands.swap(commands_);}
            for(auto command:commands)switch(command.kind) {
                case Kind::Key:
                    if((state_.qol_improvements || state_.u16(0x82bd,0x5e0a)) && (uint32_t(command.a)&KeyRepeat) &&
                       (bios_key(uint32_t(command.a),false)>>8)==0x39)break;
                    if(!was_panning)state_.keys.push_back(uint32_t(command.a));
                    break;
                case Kind::Release:
                    state_.keys.erase(std::remove_if(state_.keys.begin(),state_.keys.end(),[&](uint32_t key){return repeat_from(key,unsigned(command.a));}),state_.keys.end());break;
                case Kind::Directions:directions=uint8_t(command.a);break;
                case Kind::Space:state_.mining_space_held=!was_panning && command.a!=0;break;
                case Kind::Mouse:state_.mouse.move(command.a,command.b);break;
                case Kind::Buttons:if(!was_panning)state_.mouse.buttons(command.a);break;
                case Kind::Clear:directions=0;state_.mining_space_held=false;state_.keys.clear();state_.mouse.clear();state_.reset_combat_pointer();state_.reset_world_pointer();break;
                case Kind::Qol:state_.qol_improvements=command.a!=0;state_.reset_combat_pointer();state_.reset_world_pointer();break;
            }
            while(timer_elapsed>=((state_.pit_divisor?state_.pit_divisor:65536)/1193182.0)) {
                timer_elapsed-=(state_.pit_divisor?state_.pit_divisor:65536)/1193182.0;
                state_.timer_interrupt();
            }
            state_.set_movement(was_panning?0:directions);
            for(int i=0;i<4096 && state_.running;i++) {
                native_step(state_);if(state_.waiting)break;
            }
            if(was_panning!=state_.panning_active) {
                directions=0;state_.mining_space_held=false;state_.set_movement(0);state_.keys.clear();state_.mouse.clear();
            }
            state_.audio.speaker((state_.ports[0x61]&3)==3?1193182u/(state_.speaker_divisor?state_.speaker_divisor:65536):0);
            if(now-published>=std::chrono::milliseconds(4)) {
                Snapshot next;
                read_frame(state_,next.pixels);next.video_mode=state_.video_mode;
                next.x=state_.u16(0x82bd,0x5b4a);next.y=state_.u16(0x82bd,0x5b4c);
                next.town_page=state_.u16(0x82bd,0x5b5a);next.building=state_.u16(0x82bd,0x5b5e);
                next.directions=directions;next.custom_cursor=state_.custom_cursor;next.boundaries=state_.boundaries;
                next.mouse_visibility=state_.mouse_visibility;next.mouse_mode=state_.u16(0x82bd,0x5d62);
                next.mouse_x=state_.mouse.current().x;next.mouse_y=state_.mouse.current().y;
                next.pointer_visible=state_.game_ui.pointer_visible(state_);
                next.desert_view=state_.desert_view_active;next.map_view=state_.u16(0x82bd,0x5e06)!=0;
                next.cave_view=state_.u16(0x82bd,0x5e0a)!=0;
                next.map_scroll_x=state_.u16(0x82bd,0x5b56);next.map_scroll_y=state_.u16(0x82bd,0x5b58);
                next.return_x=state_.u16(0x82bd,0x5b60);next.return_y=state_.u16(0x82bd,0x5b62);
                next.survival_ticks=state_.u16(0x82bd,0x5406);
                next.panning_active=state_.panning_active;
                next.gold_bags=state_.u16(0x82bd,0x53ea);next.qol_improvements=state_.qol_improvements;
                next.cash=state_.u16(0x82bd,0x53f0)|(uint32_t(state_.u16(0x82bd,0x53f2))<<16);
                next.assay_pounds=state_.u16(0x82bd,0x5b82);next.assay_grade=state_.u16(0x82bd,0x59d0);
                next.combat_active=state_.combat_active;next.bullets=state_.u16(0x82bd,0x53e2);
                auto aim=state_.combat_aim();next.sight_x=aim.x;next.sight_y=aim.y;
                next.mining_space_held=state_.mining_space_held;next.mining_strokes=state_.u16(0x82bd,0x93a);
                {std::lock_guard<std::mutex> lock(frame_mutex_);next.sequence=frame_.sequence+1;frame_=std::move(next);}
                published=now;
            }
            std::this_thread::sleep_for(std::chrono::milliseconds(1));
        }
    }catch(...) {
        std::lock_guard<std::mutex> lock(frame_mutex_);error_=std::current_exception();
    }
    done_=true;
}
}

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
    if(s.custom_cursor && s.mouse_visibility>=0) {
        for(int y=0;y<16;y++)for(int x=0;x<16;x++) {
            int px=s.mouse_x-s.mouse_hot_x+x,py=s.mouse_y-s.mouse_hot_y+y;
            if(px<0 || px>=320 || py<0 || py>=200)continue;
            auto& c=pixels[py*320+px];
            if(!(s.mouse_mask[y]&(0x8000>>x)))c=0xff000000;
            if(s.mouse_mask[y+16]&(0x8000>>x))c^=0xffffff;
        }
    }
}
Session::Session(State& s):state_(s),thread_([this]{run();}) {}
Session::~Session(){stop();}
void Session::stop(){stopping_=true;if(thread_.joinable())thread_.join();}
void Session::send(Command command){std::lock_guard<std::mutex> lock(mutex_);commands_.push_back(command);}
void Session::key(uint32_t code){send({Kind::Key,int(code)});}
void Session::release_repeat(uint16_t code){send({Kind::Release,code});}
void Session::directions(uint8_t mask){send({Kind::Directions,mask});}
void Session::mouse(int x,int y){send({Kind::Mouse,x,y});}
void Session::buttons(int mask){send({Kind::Buttons,mask});}
void Session::clear_input(){send({Kind::Clear});}
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
            timer_elapsed+=std::chrono::duration<double>(now-last).count();last=now;
            std::vector<Command> commands;
            {std::lock_guard<std::mutex> lock(mutex_);commands.swap(commands_);}
            for(auto command:commands)switch(command.kind) {
                case Kind::Key:state_.keys.push_back(uint32_t(command.a));break;
                case Kind::Release:
                    state_.keys.erase(std::remove_if(state_.keys.begin(),state_.keys.end(),[&](uint32_t key){return (key&0x10000) && uint16_t(key)==command.a;}),state_.keys.end());break;
                case Kind::Directions:directions=uint8_t(command.a);break;
                case Kind::Mouse:state_.mouse_x=command.a;state_.mouse_y=command.b;break;
                case Kind::Buttons:state_.mouse_buttons=command.a;break;
                case Kind::Clear:directions=0;state_.keys.clear();state_.mouse_buttons=0;break;
            }
            while(timer_elapsed>=((state_.pit_divisor?state_.pit_divisor:65536)/1193182.0)) {
                timer_elapsed-=(state_.pit_divisor?state_.pit_divisor:65536)/1193182.0;
                state_.timer_interrupt();
            }
            state_.set_movement(directions);
            for(int i=0;i<4096 && state_.running;i++) {
                native_step(state_);if(state_.waiting)break;
            }
            state_.audio.speaker((state_.ports[0x61]&3)==3?1193182u/(state_.speaker_divisor?state_.speaker_divisor:65536):0);
            if(now-published>=std::chrono::milliseconds(4)) {
                Snapshot next;
                read_frame(state_,next.pixels);next.video_mode=state_.video_mode;
                next.x=state_.u16(0x82bd,0x5b4a);next.y=state_.u16(0x82bd,0x5b4c);
                next.town_page=state_.u16(0x82bd,0x5b5a);next.building=state_.u16(0x82bd,0x5b5e);
                next.directions=directions;next.custom_cursor=state_.custom_cursor;next.boundaries=state_.boundaries;
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

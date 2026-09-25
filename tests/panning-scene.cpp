#include "legacy.h"
#include "image_info.h"
#include <array>
#include <fstream>
#include <iostream>
#include <memory>
#include <stdexcept>

namespace {
void require(bool ok,const char* message){if(!ok)throw std::runtime_error(message);}
void capture(const ldm::State& s,int frame) {
    std::filesystem::create_directories("captures/panning-original");
    std::ofstream f("captures/panning-original/frame-"+std::to_string(frame)+".ppm",std::ios::binary);
    f<<"P6\n320 200\n255\n";
    for(int i=0;i<64000;i++) {
        auto c=s.palette[s.memory[0xa0000+i]];
        char rgb[]={char(c>>16),char(c>>8),char(c)};f.write(rgb,3);
    }
}
}
int main(int argc,char** argv) {
    if(argc!=2){std::cerr<<"Usage: test-panning-scene <original game directory>\n";return 2;}
    try {
        auto s=std::make_unique<ldm::State>();
        s->data_dir=std::filesystem::absolute(argv[1]);
        s->save_dir=std::filesystem::absolute(".local/panning-scene-saves");
        s->load("recovered/load-image.bin",image_relocations,entry_cs,entry_ip,stack_ss,stack_sp);
        uint64_t next_timer=1000,ticks=0;
        auto step=[&](){ldm::native_step(*s);if(s->boundaries>=next_timer){next_timer=s->boundaries+1000;s->timer_interrupt();++ticks;}};
        while(!(s->cs==ldm::LoadSegment+0xfa7 && s->ip==6 && s->u16(0x82bd,0x5e04)==1)) {
            require(s->boundaries<120000000,"Original startup did not reach town");step();
        }
        // Use the original initialized sprites, page buffers and timer handler.
        // The desktop scenario separately exercises the river's real Pan button.
        for(bool qol:{false,true})for(int quality:{0,1,2}) {
            s->qol_improvements=qol;s->w16(s->ds,0x53dc,1);s->w16(s->ds,0x53ea,0);
            for(int slot=1;slot<11;slot++)s->w16(s->ds,uint16_t(0x500e + slot*8),0x2b);
            s->w16(s->ds,0x505e,0xf); // Actual pan, leaving the first reward slot free.
            s->keys.clear();s->mouse.clear();s->set_movement(0);
            s->sp=0x8000;s->push(quality);s->push(0xffff);s->push(0xfffe);
            s->cs=ldm::LoadSegment+0x033f;s->ip=0x02cc;
            auto start=s->boundaries,start_ticks=ticks;
            std::array<int,3> frames{};int shown=0;bool switched=false;
            while(s->cs!=0xffff) {
                require(s->boundaries-start<4000000,"Restored animation did not finish");
                // After each original presentation call, its frame index remains
                // in the local variable. The fourth pose returns to the middle.
                if(s->cs==ldm::LoadSegment+0x033f && (s->ip==0x0395 || s->ip==0x040d)) {
                    int frame=s->ip==0x040d?1:(s->u16(s->ss,uint16_t(s->bp-4))-240)/24;
                    require(frame>=0 && frame<3,"Animation used a sprite outside the original three poses");
                    if(quality==0 && !frames[frame])capture(*s,frame);
                    ++frames[frame];++shown;
                    require(s->panning_active && s->u16(s->ds,0x53ea)==0,"Reward must wait for the entire original animation");
                    if(!switched) {
                        // Changing presentation preferences cannot interrupt it.
                        s->qol_improvements=!qol;switched=true;
                        s->keys.push_back(0x3920);s->mouse.buttons(1);s->set_movement(8);
                    }
                }
                step();
            }
            require(frames[0]>=5 && frames[0]<=9 && frames[2]==frames[0] && frames[1]==2*frames[0],"Original three-frame loop and middle return did not complete 5-9 cycles");
            require(ticks-start_ticks>=uint64_t(shown*12),"Original animation delays were bypassed");
            require(s->ip==0xfffe && s->sp==0x7ffe && !s->panning_active,"Animation did not restore the original stack and return");
            require(s->u16(s->ds,0x53ea)==1 && s->u16(s->ds,0x5016)==0x20+quality && s->u16(s->ds,0x501e)==0x2b,"Animation must award exactly one original graded bag");
            require(s->u16(s->ds,0x5b4c)==55 && s->keys.empty() && s->mouse.current().buttons==0,"Animation left a displaced player or stale input");
            std::cout<<"QoL "<<qol<<", grade "<<quality<<": "<<shown<<" original poses, "<<(ticks-start_ticks)<<" timer ticks, one bag\n";
        }
        std::cout<<"PASS: original sprite/presentation/timer routines, finite animation, deferred graded reward and input cleanup with QoL on/off (synthetic clock)\n";
    }catch(const std::exception& e){std::cerr<<e.what()<<"\n";return 1;}
}

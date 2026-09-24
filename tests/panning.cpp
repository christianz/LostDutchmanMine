#include "legacy.h"
#include "image_info.h"
#include <iostream>
#include <memory>
#include <stdexcept>

namespace {
void require(bool ok,const char* message){if(!ok)throw std::runtime_error(message);}
std::unique_ptr<ldm::State> state(bool enabled,int pans=1,bool full=false,int quality=0) {
    auto s=std::make_unique<ldm::State>();
    s->load("recovered/load-image.bin",image_relocations,entry_cs,entry_ip,stack_ss,stack_sp);
    s->ds=s->ss=0x82bd;s->sp=0x8000;s->video_mode=0x13;s->qol_improvements=enabled;
    s->w16(s->ds,0x53dc,pans);s->w16(s->ds,0x53ea,0);
    for(int slot=1;slot<11;slot++)s->w16(s->ds,uint16_t(0x500e + slot*8),full?0x10:0x2b);
    s->push(quality);s->push(0xffff);s->push(0xfffe);
    s->cs=ldm::LoadSegment+0x033f;s->ip=0x02cc;
    return s;
}
void run_to_boundary(ldm::State& s) {
    auto start=s.boundaries;
    do {
        ldm::native_step(s);
        require(s.boundaries-start<10000,"Original panning path did not reach its boundary");
    }while(!(s.cs==ldm::LoadSegment+0x0fc5 && s.ip==0x0d12) &&
           !(s.cs==ldm::LoadSegment+0x0652 && s.ip==0x01c2) && s.cs!=0xffff);
}
}
int main() {
    try {
        for(int quality:{0,1,2}) {
            auto s=state(false,1,false,quality);run_to_boundary(*s);
            require(s->cs==0xffff && s->ip==0xfffe && s->sp==0x7ffe,"Instant Pan must preserve the original return and caller argument");
            require(!s->panning_active && s->u16(s->ds,0x53ea)==1,"QoL off must award one bag immediately");
            require(s->u16(s->ds,0x5016)==0x20+quality && s->u16(s->ds,0x501e)==0x2b,"Instant Pan must retain the original river grade and single reward");
        }
        auto animated=state(true);run_to_boundary(*animated);
        require(animated->panning_active && animated->u16(animated->ds,0x53ea)==0 && animated->cs==ldm::LoadSegment+0x0fc5,"Pan must reach the original background blitter before awarding gold");
        auto missing=state(true,0);run_to_boundary(*missing);
        require(missing->cs==0xffff && !missing->panning_active && missing->u16(missing->ds,0x53ea)==0,"No pan must mean no animation or gold");
        require(missing->sp==0x7ffe,"No-pan return must preserve the caller argument");
        auto full=state(true,1,true);run_to_boundary(*full);
        require(!full->panning_active && full->u16(full->ds,0x53ea)==0 && full->cs==ldm::LoadSegment+0x0652 && full->ip==0x01c2,"Full pack must reach the original message without animating or awarding gold");
        std::cout<<"PASS: restored original drawing path, immediate QoL-off rewards in all three grades, no pan/full pack and original stack cleanup\n";
    }catch(const std::exception& e){std::cerr<<e.what()<<"\n";return 1;}
}

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
    for(int row=0;row<4;row++) {
        if(row)s->w16(s->ds,uint16_t(0x5d58+row*2),0);
        for(int slot=1;slot<11;slot++)s->w16(s->ds,uint16_t(0x500e + slot*8+row*2),full?0x10:0x2b);
    }
    if(pans)s->w16(s->ds,0x505e,0xf); // A pan in the last player slot.
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
        for(bool enabled:{false,true}) {
            auto missing=state(enabled);missing->w16(missing->ds,0x505e,0x2b);
            run_to_boundary(*missing);
            require(missing->cs==0xffff && !missing->panning_active && missing->u16(missing->ds,0x53ea)==0,
                    "Stale pan counter permits panning without an inventory pan");
            require(missing->sp==0x7ffe,"Rejected Pan did not preserve the caller argument");
            auto unrelated=state(enabled,0);
            unrelated->w16(unrelated->ds,0x500e,0xf);unrelated->w16(unrelated->ds,0x5bdc,0xf);
            run_to_boundary(*unrelated);
            require(unrelated->cs==0xffff && !unrelated->panning_active && unrelated->u16(unrelated->ds,0x53ea)==0,
                    "Carrier icons or the food table must not count as an inventory pan");
            for(int row=0;row<4;row++)for(int slot:{1,10})for(bool owned:{false,true}) {
                auto s=state(enabled,0);
                s->w16(s->ds,uint16_t(0x500e + slot*8+row*2),0xf);
                if(row)s->w16(s->ds,uint16_t(0x5d58+row*2),owned);
                run_to_boundary(*s);
                bool allowed=row==0 || owned;
                require(s->panning_active==allowed && s->u16(s->ds,0x53ea)==0,
                        "Pan eligibility does not match an actual player/owned-mule inventory slot");
            }
        }
        for(int quality:{0,1,2})for(bool enabled:{false,true}) {
            auto s=state(enabled,1,false,quality);run_to_boundary(*s);
            require(s->panning_active && s->u16(s->ds,0x53ea)==0 && s->cs==ldm::LoadSegment+0x0fc5,
                    "Every grade must animate before awarding gold with QoL on or off");
        }
        auto animated=state(true);run_to_boundary(*animated);
        require(animated->panning_active && animated->u16(animated->ds,0x53ea)==0 && animated->cs==ldm::LoadSegment+0x0fc5,"Pan must reach the original background blitter before awarding gold");
        auto missing=state(true,0);run_to_boundary(*missing);
        require(missing->cs==0xffff && !missing->panning_active && missing->u16(missing->ds,0x53ea)==0,"No pan must mean no animation or gold");
        require(missing->sp==0x7ffe,"No-pan return must preserve the caller argument");
        auto full=state(true,1,true);run_to_boundary(*full);
        require(!full->panning_active && full->u16(full->ds,0x53ea)==0 && full->cs==ldm::LoadSegment+0x0652 && full->ip==0x01c2,"Full pack must reach the original message without animating or awarding gold");
        std::cout<<"PASS: actual player/owned-mule pans, stale counters, missing pans, original animation, all three grades, full pack and stack cleanup with QoL on/off\n";
    }catch(const std::exception& e){std::cerr<<e.what()<<"\n";return 1;}
}

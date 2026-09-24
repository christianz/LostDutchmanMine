#include "legacy.h"
#include "image_info.h"
#include <iostream>
#include <memory>
#include <stdexcept>

namespace {
void require(bool ok,const char* message){if(!ok)throw std::runtime_error(message);}
// Run the original world selector, stopping at a real status-menu entry or
// its far return. This catches changes to the legacy ABI and action routing.
int select(int x,int y,bool qol,unsigned context=15,bool combat=false) {
    auto s=std::make_unique<ldm::State>();
    s->load("recovered/load-image.bin",image_relocations,entry_cs,entry_ip,stack_ss,stack_sp);
    s->ds=s->ss=0x82bd;s->sp=0x8000;s->qol_improvements=qol;s->video_mode=0x13;
    s->combat_active=combat;
    s->w8(s->ds,0xaa6,1);s->w16(s->ds,0xa64,2);s->game_ui.context_buttons=context;
    s->push(y);s->push(x);s->push(0xffff);s->push(0xfffe);
    s->cs=ldm::LoadSegment;s->ip=0x093e;
    const int targets[]={0x00aa,0x21c2,0x0d60,0x036a,0x00c2,0x1a52};
    for(int steps=0;steps<2000;steps++) {
        ldm::native_step(*s);
        if(s->cs==ldm::LoadSegment+0x0652)
            for(int i=0;i<6;i++)if(s->ip==targets[i])return 10+i;
        if(s->cs==0xffff) {
            require(s->sp==0x7ffc && s->ip==0xfffe,"World selector damaged its far return or caller stack");
            if(!qol)require(s->u16(s->ss,s->sp)==x && s->u16(s->ss,s->sp+2)==y,"QoL off rewrote original input");
            return s->ax;
        }
    }
    throw std::runtime_error("World selector failed to dispatch or return");
}
}
int main() {
    try {
        for(int i=0;i<6;i++) {
            for(int x:{52+i*42,91+i*42})for(int y:{167,198})
                require(select(x,y,true)==10+i,"Toolbar corner did not reach its original menu");
            require(select(69+i*42,181,false)==10+i,"Original icon interior no longer dispatches");
            require(select(52+i*42,198,false)==0,"QoL off retained an enlarged target");
            require(select(92+i*42,180,true)==0,"Gap between toolbar buttons must not select anything");
            require(select(69+i*42,165,true)==0,"Logo below the title became a toolbar target");
        }
        for(int i=0;i<4;i++) {
            int x=i<2?72:152,y=i%2?138:118;
            for(int px:{x,x+74})for(int py:{y,y+18})
                require(select(px,py,true)==i+1,"Context-button bevel did not reach its original action");
            require(select(x,y,false)==0,"QoL off changed a contextual edge");
            require(select(x,y,true,0)==0,"An absent contextual button gained a new target");
        }
        require(select(100,100,true)==0,"Persistent-pointer scenery click restarts the scene");
        require(select(100,100,false)==9 && select(100,100,true,15,true)==9,
                "Classic or combat scene click routing changed");
        require(select(150,150,true)==0,"Space between context columns became clickable");
        std::cout<<"PASS: enlarged toolbar/context corners dispatch through original menus; gaps, absent choices, scene clicks, far returns and QoL-off input preserved\n";
    }catch(const std::exception& e){std::cerr<<e.what()<<"\n";return 1;}
}

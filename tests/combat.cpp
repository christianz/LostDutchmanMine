#include "legacy.h"
#include "image_info.h"
#include <iostream>
#include <memory>
#include <stdexcept>

void require(bool ok,const char* message){if(!ok)throw std::runtime_error(message);}
int main() {
    try {
        auto s=std::make_unique<ldm::State>();
        s->load("recovered/load-image.bin",image_relocations,entry_cs,entry_ip,stack_ss,stack_sp);
        s->ds=s->ss=0x82bd;s->w16(s->ds,0xa64,2);s->w8(s->ds,0xaa6,1);
        s->w16(s->ds,0xa6c,0);s->w16(s->ds,0x3118,ldm::LoadSegment+0x72ba);
        s->w16(s->ds,0x5d62,1);s->w16(s->ds,0x53e0,1);
        s->w16(s->ds,0x5b4a,160);s->w16(s->ds,0x5b4c,64);s->begin_combat();
        auto point=[&](int x,int y){return s->u16(s->ds,0x5b4a)==x && s->u16(s->ds,0x5b4c)==y;};
        auto poll=[&]() {
            s->cs=ldm::LoadSegment+0x040a;s->ip=0x0288;s->bp=0x8000;s->sp=0x7fc0;
            const auto start=s->boundaries;
            for(;;) {
                ldm::native_step(*s);
                require(s->boundaries-start<2000,"Combat input did not finish");
                bool shot=s->cs==ldm::LoadSegment+0x040a && s->ip==0x09e2;
                bool wait=s->cs==ldm::LoadSegment+0x0505 && s->ip==0x008c;
                if(shot || wait) {
                    require(!s->combat_input_read,"Combat input context leaked beyond its reader");
                    require(s->sp==(shot?0x7fb6:0x7fba),"Combat input corrupted the original call stack");
                    return shot;
                }
            }
        };
        auto clear=[&](){s->mouse.clear();s->reset_combat_pointer();s->set_movement(0);s->keys.clear();s->w16(s->ds,0x5d62,1);};
        for(bool held:{false,true}) {
            clear();s->w16(s->ds,0x5d62,0);s->mouse.move(100,147);s->mouse.buttons(1);
            if(!held)s->mouse.buttons(0);
            s->begin_combat();
            require(!poll() && s->u16(s->ds,0x5d62)==1,
                    "Entry click held over the old panel reopened the hand cursor");
            s->mouse.move(70,45);
            require(!poll() && point(62,37) && s->u16(s->ds,0x5d62)==1,
                    "Hand-mode encounter entry did not enable immediate aim or replayed its entry click");
            require(!poll(),"Holding the entry click fired a shot");
            s->mouse.buttons(0);require(!poll(),"Releasing the entry click fired a shot");
            s->mouse.buttons(1);require(poll(),"Fresh press after encounter entry did not fire");
        }
        clear();s->mouse.move(160,100);s->reset_combat_pointer();
        s->w16(s->ds,0x5b4a,160);s->w16(s->ds,0x5b4c,64);
        require(!poll() && point(160,64),"Entering combat must not jump to a stationary mouse");
        s->mouse.move(70,45);require(!poll() && point(62,37),"Mouse motion must position the original sight's centre");
        s->set_movement(8);require(!poll() && point(72,37),"Stationary mouse must not undo keyboard aiming");
        require(!poll() && point(82,37),"Held aiming must keep working without mouse motion");
        s->mouse.move(240,80);require(!poll() && point(232,72),"Fresh mouse aim must win over a simultaneous direction");
        s->set_movement(0);s->mouse.move(80,60);s->mouse.buttons(1);s->mouse.buttons(0);s->mouse.move(245,80);
        require(poll() && point(72,52),"Quick click must fire at the latched click position");
        require(!poll(),"Release must not fire a second shot");
        require(!poll() && point(237,72),"Latest mouse motion must resume after queued click edges");
        require(s->u16(s->ds,0x5d62)==1,"Firing must not enter hand mode");
        s->mouse.buttons(1);require(poll(),"Left press must fire");
        require(!poll() && !poll(),"Holding the mouse must not empty the ammunition");
        clear();s->mouse.move(100,70);s->mouse.buttons(1);clear();
        require(!poll() && point(237,72),"Focus loss must clear queued shots and stale aim");
        s->mouse.move(0,0);require(!poll() && point(20,20),"Aim must stop at the upper-left bounds");
        s->mouse.move(319,111);require(!poll() && point(290,84),"Aim must stop at the lower-right bounds");
        s->mouse.move(90,140);require(!poll() && point(290,84),"Status panel motion must not move the sight");
        s->mouse.buttons(1);require(!poll() && s->u16(s->ds,0x5d62)==0,"Status clicks must retain the hand cursor");
        clear();s->mouse.move(90,60);s->mouse.buttons(2);
        require(!poll() && point(290,84) && s->u16(s->ds,0x5d62)==0,"Right click must open hand mode without aiming or firing");
        clear();s->w16(s->ds,0x5d62,0);s->mouse.move(50,30);s->mouse.buttons(1);
        require(!poll() && point(290,84),"Hand menus must retain their original selection input");
        clear();s->w16(s->ds,0x5302,1);s->mouse.move(80,40);s->mouse.buttons(1);
        require(!poll() && point(290,84),"Victory choices must not fire or aim");
        clear();s->w16(s->ds,0x5302,0);s->w16(s->ds,0x53e0,0);s->mouse.move(110,60);s->mouse.buttons(1);
        require(!poll() && point(290,84),"No gun must preserve the original input path");
        clear();s->w16(s->ds,0x53e0,1);s->qol_improvements=false;s->mouse.move(70,45);
        s->w16(s->ds,0x5d62,0);s->begin_combat();
        require(s->u16(s->ds,0x5d62)==0,"QoL off changed the encounter entry mode");
        s->w16(s->ds,0x5d62,1);
        require(!poll() && point(290,84),"QoL off must retain original aiming");
        s->mouse.buttons(1);require(!poll() && s->u16(s->ds,0x5d62)==0,"QoL off must retain mouse selection");
        clear();s->keys={0x3920};require(poll(),"Space must still call the original shooting routine");
        std::cout<<"PASS: original combat loop mouse aim, click-position firing, one shot per press, keyboard coexistence, bounds, focus reset, hand/status/victory/no-gun paths and QoL off\n";
    }catch(const std::exception& e){std::cerr<<e.what()<<"\n";return 1;}
}

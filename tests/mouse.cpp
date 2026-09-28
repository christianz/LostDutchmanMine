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
        s->ds=s->ss=0x82bd;s->sp=0x8000;
        s->w16(s->ds,0xa64,2); // Original mouse-driver present flag.
        s->w8(s->ds,0xaa6,1); // Original wrappers use far returns.
        auto read=[&]() {
            s->push(0x6004);s->push(0x6002);s->push(0x6000);
            s->push(0xffff);s->push(0xfffe);
            s->cs=ldm::LoadSegment+0x0fc5;s->ip=0x0038;
            auto start=s->boundaries;
            while(!(s->cs==0xffff && s->ip==0xfffe)) {
                require(s->boundaries-start<1000,"Original mouse helper failed to return");
                ldm::native_step(*s);
                // A new desktop movement can arrive between the three BIOS
                // calls; it must not change the coordinates of a queued click.
                s->mouse.move(300,190);
            }
            s->sp+=6;require(s->sp==0x8000,"Mouse polling corrupted stack");
            return ldm::MouseSample{int(s->u16(s->ds,0x6000)),int(s->u16(s->ds,0x6002)),int(s->u16(s->ds,0x6004))};
        };
        s->mouse.move(123,87);s->mouse.buttons(1);s->mouse.buttons(0);
        auto down=read(),up=read(),moved=read();
        require(down.buttons==1 && down.x==123 && down.y==87,"Quick press or click position was lost");
        require(up.buttons==0 && up.x==123 && up.y==87,"Quick release was lost");
        require(moved.buttons==0 && moved.x==300 && moved.y==190,"Current movement did not resume");
        s->mouse.buttons(2);s->mouse.buttons(2);
        require(read().buttons==2 && read().buttons==2,"Held right button was not preserved");
        s->mouse.buttons(0);s->mouse.buttons(1);s->mouse.buttons(0);s->mouse.clear();
        require(read().buttons==0 && read().buttons==0,"Focus loss retained a queued click");
        s->mouse.warp(25,40);s->mouse.buttons(1);auto warped=read();
        require(warped.x==25 && warped.y==40 && warped.buttons==1,"Original pointer positioning failed");
        // Clicks expire after one second of emulated time, never host time.
        s->mouse.clear();s->mouse.set_time(1000);
        s->mouse.buttons(1);s->mouse.buttons(0);s->mouse.set_time(3000);
        require(read().buttons==0,"A click made during loading was replayed later");
        s->mouse.set_time(1000);s->mouse.buttons(1);s->mouse.set_time(3000);
        require(read().buttons==1,"A physically held button expired");
        s->mouse.clear();s->mouse.set_time(1000);
        s->mouse.buttons(1);s->mouse.buttons(0);s->mouse.set_time(1999);
        require(read().buttons==1,"A click younger than one second was dropped");
        s->mouse.clear();s->mouse.buttons(1);s->mouse.buttons(0);s->mouse.buttons(2);s->mouse.buttons(0);
        for(int mask:{1,0,2,0})require(read().buttons==mask,"Consecutive clicks were reordered");
        for(bool qol:{false,true}) {
            s->qol_improvements=qol;s->mouse.move(33,44);s->ax=4;s->cx=480;s->dx=140;s->interrupt(0x33);
            require(s->mouse.current().x==(qol?33:240) && s->mouse.current().y==(qol?44:140),
                    "Legacy cursor parking must be ignored only with QoL enabled");
        }
        std::cout<<"PASS: original mouse helper preserves quick clicks, coherent coordinates, holds and order; clears focus loss and stale clicks\n";
    }catch(const std::exception& e){std::cerr<<e.what()<<"\n";return 1;}
}

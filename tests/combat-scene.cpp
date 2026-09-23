#include "legacy.h"
#include "image_info.h"
#include <fstream>
#include <iostream>
#include <memory>
#include <stdexcept>

namespace {
void require(bool ok,const char* message){if(!ok)throw std::runtime_error(message);}
void capture(const ldm::State& s,const char* name) {
    std::filesystem::create_directories("captures/combat");
    std::ofstream f(std::string("captures/combat/")+name+".ppm",std::ios::binary);
    f<<"P6\n320 200\n255\n";
    for(int i=0;i<64000;i++) {
        auto c=s.palette[s.memory[0xa0000+i]];
        char rgb[]={char(c>>16),char(c>>8),char(c)};f.write(rgb,3);
    }
}
}
int main(int argc,char** argv) {
    if(argc!=2){std::cerr<<"Usage: test-combat-scene <original game directory>\n";return 2;}
    try {
        auto s=std::make_unique<ldm::State>();
        s->data_dir=std::filesystem::absolute(argv[1]);s->save_dir=std::filesystem::absolute(".local/combat-scene-saves");
        s->load("recovered/load-image.bin",image_relocations,entry_cs,entry_ip,stack_ss,stack_sp);
        uint64_t next_timer=1000;
        auto step=[&](){ldm::native_step(*s);if(s->boundaries>=next_timer){next_timer=s->boundaries+1000;s->timer_interrupt();}};
        // Boot original initialization/assets with a synthetic clock, stopping
        // at the town input call before entering a controlled encounter fixture.
        while(!(s->cs==ldm::LoadSegment+0xfa7 && s->ip==6 && s->u16(0x82bd,0x5e04)==1)) {
            if(s->boundaries>=120000000)s->fail("Original startup did not reach town");
            step();
        }
        s->keys.clear();s->mouse.clear();s->w16(s->ds,0x533c,1); // Original Native American encounter type.
        s->w16(s->ds,0x4eec,0);s->w16(s->ds,0x5302,0);s->w16(s->ds,0x5b64,0);
        s->w16(s->ds,0x5e0c,1);s->w16(s->ds,0x53e0,1);s->w16(s->ds,0x53e2,20);
        s->sp=0x8000;s->push(0xffff);s->push(0xfffe);s->cs=ldm::LoadSegment+0x40a;s->ip=2;
        auto at_input=[&](){return s->cs==ldm::LoadSegment+0xfa7 && s->ip==6;};
        auto frame=[&](){auto start=s->boundaries;do{step();require(s->boundaries-start<2000000,"Encounter did not reach its next input");}while(!at_input());};
        frame();frame();require(s->combat_active,"Original encounter hook did not activate");
        capture(*s,"native-encounter-start");
        s->mouse.move(80,45);frame();
        require(s->u16(s->ds,0x5b4a)==72 && s->u16(s->ds,0x5b4c)==37,"Native American encounter did not accept mouse aim");
        capture(*s,"native-encounter-left");
        s->mouse.move(245,75);frame();
        require(s->u16(s->ds,0x5b4a)==237 && s->u16(s->ds,0x5b4c)==67,"Original crosshair did not follow rightward aim");
        capture(*s,"native-encounter-right");
        // Original hit test compares sight x-8 with the 24-pixel target's x,
        // and sight y with target y. Aim at that target, then call its real shot.
        int target_x=s->u16(s->ss,uint16_t(s->bp-0x16));
        int target_y=s->u16(s->ss,uint16_t(s->bp-0x18));
        require(s->u16(s->ds,0x5312)==0,"Encounter fixture already registered a hit");
        s->mouse.move(target_x+16,target_y+8);s->mouse.buttons(1);s->mouse.buttons(0);
        auto start=s->boundaries;
        do{step();require(s->boundaries-start<2000000,"Original shot did not return");}
        while(!(s->cs==ldm::LoadSegment+0x40a && s->ip==0x0300));
        require(s->u16(s->ds,0x53e2)==19,"Mouse firing did not spend one original bullet");
        require(s->u16(s->ds,0x5312)==1,"Mouse aim did not hit through original hit testing");
        require(s->u16(s->ds,0x5d62)==1,"Mouse shot unexpectedly entered hand mode");
        std::cout<<"PASS: original Native American encounter renders, mouse moves its original crosshair, and a click hits via original ammunition/hit logic (synthetic clock)\n";
    }catch(const std::exception& e){std::cerr<<e.what()<<"\n";return 1;}
}

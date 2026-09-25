#include "session.h"
#include "image_info.h"
#include <fstream>
#include <iostream>
#include <memory>
#include <stdexcept>

namespace {
void require(bool ok,const char* message){if(!ok)throw std::runtime_error(message);}
void capture(const ldm::State& s,const std::string& name) {
    std::filesystem::create_directories("captures/combat");
    std::ofstream f(std::string("captures/combat/")+name+".ppm",std::ios::binary);
    f<<"P6\n320 200\n255\n";
    ldm::Pixels pixels;ldm::read_frame(s,pixels);
    for(auto c:pixels) {
        char rgb[]={char(c>>16),char(c>>8),char(c)};f.write(rgb,3);
    }
}
void compare_original_sight(ldm::State& s) {
    // Repaint just the original sight/presentation on the same background.
    // Use a separate stack, then restore the suspended encounter registers.
    auto registers=std::array<uint16_t,14>{s.ax,s.bx,s.cx,s.dx,s.si,s.di,s.bp,s.sp,s.cs,s.ip,s.ds,s.ss,s.es,s.flags};
    auto call=[&](int cs,int ip,std::initializer_list<uint16_t> args) {
        s.sp=0x7000;for(auto p=args.end();p!=args.begin();)s.push(*--p);
        s.push(0xffff);s.push(0xfffe);s.cs=ldm::LoadSegment+cs;s.ip=ip;
        auto start=s.boundaries;
        while(s.cs!=0xffff){ldm::native_step(s);require(s.boundaries-start<100000,"Original sight comparison did not return");}
    };
    ldm::Pixels displayed;ldm::read_frame(s,displayed);
    auto aim=s.combat_aim();
    call(0xfc5,0xe8a,{1,0,12,0,120,160,uint16_t(aim.x),uint16_t(aim.y),16,16});
    call(0x505,0x254,{});
    for(int y=0;y<16;y++)for(int x=0;x<16;x++) {
        int at=(aim.y+y)*320+aim.x+x;
        require(displayed[at]==s.palette[s.memory[0xa0000+at]],"Display-rate crosshair differs from original sprite/blitter");
    }
    s.ax=registers[0];s.bx=registers[1];s.cx=registers[2];s.dx=registers[3];s.si=registers[4];s.di=registers[5];
    s.bp=registers[6];s.sp=registers[7];s.cs=registers[8];s.ip=registers[9];s.ds=registers[10];s.ss=registers[11];s.es=registers[12];s.flags=registers[13];
}
}
int main(int argc,char** argv) {
    if(argc!=2){std::cerr<<"Usage: test-combat-scene <original game directory>\n";return 2;}
    try {
      for(bool wanted:{false,true})for(bool hand:{true,false}) {
        const auto name=std::string(wanted?"wanted":"native")+(hand?"-from-hand":"-from-keys");
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
        s->keys.clear();s->mouse.clear();s->w16(s->ds,0x533c,wanted?0:1);
        s->w16(s->ds,0x4eec,0);s->w16(s->ds,0x5302,0);s->w16(s->ds,0x5b64,wanted?10:0);
        s->w16(s->ds,0x5d62,hand?0:1);
        if(hand){s->mouse.move(100,147);s->mouse.buttons(1);}
        s->w16(s->ds,0x5e0c,1);s->w16(s->ds,0x53e0,1);s->w16(s->ds,0x53e2,20);
        s->sp=0x8000;s->push(0xffff);s->push(0xfffe);s->cs=ldm::LoadSegment+0x40a;s->ip=2;
        auto at_input=[&](){return s->cs==ldm::LoadSegment+0xfa7 && s->ip==6;};
        auto frame=[&](){auto start=s->boundaries;do{step();require(s->boundaries-start<2000000,"Encounter did not reach its next input");}while(!at_input());};
        frame();require(s->combat_active,"Original encounter hook did not activate");
        capture(*s,name+"-start");
        if(hand)frame(); // Still holding the previous screen's panel click.
        s->mouse.move(80,45);frame();
        require(s->u16(s->ds,0x5b4a)==72 && s->u16(s->ds,0x5b4c)==37,"Encounter did not accept mouse aim on its first input");
        require(s->u16(s->ds,0x53e2)==20,"Entering the encounter fired without a new click");
        capture(*s,name+"-left");
        s->mouse.move(245,75);frame();
        require(s->u16(s->ds,0x5b4a)==237 && s->u16(s->ds,0x5b4c)==67,"Original crosshair did not follow rightward aim");
        capture(*s,name+"-right");
        auto boundaries=s->boundaries;
        auto bullets=s->u16(s->ds,0x53e2),clock=s->u16(s->ds,0x5406);
        ldm::Pixels before,moved;ldm::read_frame(*s,before);
        s->mouse.move(100,40);ldm::read_frame(*s,moved);
        require(s->combat_aim().x==92 && s->combat_aim().y==32 && before!=moved,
                "Crosshair still waits for an encounter tick after mouse motion");
        require(s->boundaries==boundaries && s->u16(s->ds,0x53e2)==bullets && s->u16(s->ds,0x5406)==clock &&
                s->u16(s->ds,0x5b4a)==237 && s->u16(s->ds,0x5b4c)==67,
                "Display-rate motion advanced shots, aim simulation or the world clock");
        s->game_ui.begin_menu();ldm::read_frame(*s,before);
        s->mouse.move(190,30);ldm::read_frame(*s,moved);
        require(before==moved,"Crosshair drew over an open status menu");s->game_ui.end_menu();
        s->mouse.move(245,75);
        compare_original_sight(*s);
        s->mouse.buttons(0);frame();
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
        std::cout<<"PASS: "<<name<<" displays immediate aim without advancing the encounter, matches the original sight pixels, hides it under menus, and hits through original ammunition/hit logic\n";
      }
    }catch(const std::exception& e){std::cerr<<e.what()<<"\n";return 1;}
}

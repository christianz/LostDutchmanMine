#include "legacy.h"
#include "image_info.h"
#include <algorithm>
#include <fstream>
#include <iostream>
#include <memory>
#include <stdexcept>

namespace {
void require(bool ok,const char* message){if(!ok)throw std::runtime_error(message);}
void advance(ldm::Panning& p,int direction,double seconds=.8) {
    for(int i=0;i<int(seconds*120+.5);i++)p.update(1.0/120,uint8_t(direction));
}
void settle(ldm::Panning& p) {
    for(int i=0;i<5;i++)advance(p,i%2?8:4);
    require(p.loosened()==100,"Full pan swings must loosen the gravel");
}
void win(ldm::Panning& p) {
    for(int i=0;i<3;i++){settle(p);p.key(0x3920);advance(p,0,1.8);}
    require(p.phase()==ldm::Panning::Phase::Result && p.gold()==5,"Careful washing must retain all gold");
    p.key(0x1c0d);require(p.done(),"Enter must collect the result");
}
std::unique_ptr<ldm::State> state(bool enabled,int pans=1,bool full=false,int quality=0) {
    auto s=std::make_unique<ldm::State>();
    s->load("recovered/load-image.bin",image_relocations,entry_cs,entry_ip,stack_ss,stack_sp);
    s->ds=s->ss=0x82bd;s->sp=0x8000;s->video_mode=0x13;s->qol_improvements=enabled;
    s->panning.art_path("resources/panning/creek.ppm");
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
    }while(!s.panning.active() && !(s.cs==ldm::LoadSegment+0x0652 && s.ip==0x01c2) && s.cs!=0xffff);
}
void capture(ldm::Panning& p,const char* name) {
    ldm::Pixels pixels{};p.draw(pixels);
    std::filesystem::create_directories("captures/panning");
    std::ofstream out(std::string("captures/panning/")+name+".ppm",std::ios::binary);
    out<<"P6\n320 200\n255\n";
    for(auto pixel:pixels){char rgb[]={char(pixel>>16),char(pixel>>8),char(pixel)};out.write(rgb,3);}
}
}
int main() {
    try {
        ldm::Panning p;p.art_path("resources/panning/creek.ppm");p.begin();
        advance(p,0,10);require(p.loosened()==0 && p.gold()==5,"Waiting must not win or lose gold");
        advance(p,4,10);require(p.loosened()==20,"Holding one direction must not farm swings");
        p.key(0x3920,true);require(p.phase()==ldm::Panning::Phase::Rock,"OS repeats must not wash");
        p.key(0x011b);require(!p.take_result(),"Leaving mid-pan must not award gold");
        p.begin();capture(p,"start");settle(p);capture(p,"loosened");p.key(0x3920);
        advance(p,0,.8);capture(p,"rinse");advance(p,0,1);
        for(int i=0;i<2;i++){settle(p);p.key(0x3920);advance(p,0,1.8);}
        capture(p,"result");p.key(0x1c0d);require(p.take_result(),"Careful pan must succeed");
        p.begin();for(int i=0;i<3;i++){p.key(0x3920);advance(p,0,1.8);}
        require(p.gold()==0 && p.phase()==ldm::Panning::Phase::Result,"Unsettled washes must lose gold");
        capture(p,"empty");p.key(0x1c0d);require(!p.take_result(),"Empty pan must not award a bag");
        p.begin();p.pointer(160,65);p.buttons(1);
        for(int i=0;i<5;i++){p.pointer(i%2?200:120,65);advance(p,0);}
        require(p.loosened()==100,"Mouse dragging must settle gravel");
        p.clear_input();advance(p,0);require(p.tilt()==0,"Focus loss must release drag");
        p.pointer(180,184);p.buttons(1);p.buttons(0);
        require(p.phase()==ldm::Panning::Phase::Rinse,"A quick mouse click must start washing");
        p.key(0x011b);require(!p.take_result(),"Escape during rinsing must abandon the pan");
        for(int quality:{0,2}) {
            auto s=state(true,1,false,quality);run_to_boundary(*s);
            require(s->panning.active() && s->u16(s->ds,0x53ea)==0,"Original Pan must wait for the minigame before awarding gold");
            win(s->panning);run_to_boundary(*s);
            require(s->cs==0xffff && s->ip==0xfffe,"Successful minigame must return through original cleanup");
            require(s->sp==0x7ffe,"Panning reward must preserve the caller's stack and argument");
            require(s->u16(s->ds,0x5016)==0x20+quality && s->u16(s->ds,0x53ea)==1,"Original inventory must receive exactly one bag with the river's original grade");
            require(s->u16(s->ds,0x501e)==0x2b && !s->panning.engaged(),"Panning must not duplicate the award");
        }
        auto cancel=state(true);run_to_boundary(*cancel);cancel->panning.key(0x011b);run_to_boundary(*cancel);
        require(cancel->u16(cancel->ds,0x53ea)==0 && cancel->u16(cancel->ds,0x5016)==0x2b,"Cancelled Pan must leave inventory unchanged");
        auto original=state(false);run_to_boundary(*original);
        require(!original->panning.engaged() && original->u16(original->ds,0x53ea)==1,"QoL off must keep the original instant reward");
        auto missing=state(true,0);run_to_boundary(*missing);
        require(missing->cs==0xffff && !missing->panning.engaged() && missing->u16(missing->ds,0x53ea)==0,"No pan must mean no minigame or gold");
        require(missing->sp==0x7ffe,"Original no-pan return must preserve the caller's argument");
        auto full=state(true,1,true);run_to_boundary(*full);
        require(!full->panning.engaged() && full->u16(full->ds,0x53ea)==0 && full->cs==ldm::LoadSegment+0x0652 && full->ip==0x01c2,"Full pack must use the original message before starting a minigame");
        std::cout<<"PASS: keyboard/mouse rock-and-rinse, no idle/repeat farming, failure, cancellation, focus release, original graded reward exactly once, no pan/full pack and QoL-off behavior\n";
    }catch(const std::exception& e){std::cerr<<e.what()<<"\n";return 1;}
}

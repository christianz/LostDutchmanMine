#include "session.h"
#include "image_info.h"
#include <fstream>
#include <iostream>
#include <memory>
#include <stdexcept>

namespace {
void require(bool ok,const char* message){if(!ok)throw std::runtime_error(message);}
struct Game {
    std::unique_ptr<ldm::State> state=std::make_unique<ldm::State>();
    ldm::State& s=*state;
    uint64_t next_timer=1000;
    Game(const char* data,bool qol) {
        s.data_dir=std::filesystem::absolute(data);
        s.save_dir=std::filesystem::absolute(".local/pan-inventory-saves");
        s.load("recovered/load-image.bin",image_relocations,entry_cs,entry_ip,stack_ss,stack_sp);
        until([&]{return at(0xfa7,6) && word(0x5e04);});s.qol_improvements=qol;
        for(int row=0;row<4;row++) {
            if(row)set(0x5d58+row*2,0);
            for(int slot=1;slot<11;slot++)set(0x500e + slot*8+row*2,0x2b);
        }
        set(0x53dc,0);set(0x53ea,0);set(0x53f0,1000);set(0x53f2,0);set(0x5b72,3000);
    }
    int word(int a)const{return s.u16(s.ds,uint16_t(a));}
    void set(int a,int value){s.w16(s.ds,uint16_t(a),uint16_t(value));}
    bool at(int cs,int ip)const{return s.cs==ldm::LoadSegment+cs && s.ip==ip;}
    void step(){ldm::native_step(s);if(s.boundaries>=next_timer){next_timer=s.boundaries+1000;s.timer_interrupt();}}
    template<class F> void until(F done) {
        auto start=s.boundaries;
        while(!done()){if(s.boundaries-start>=120000000)s.fail("Pan inventory scenario timed out");step();}
    }
    void call(int cs,int ip,std::initializer_list<uint16_t> args={}) {
        s.keys.clear();s.mouse.clear();s.reset_world_pointer();s.set_movement(0);set(0x5a1a,0);
        s.sp=0x8000;for(auto arg:args)s.push(arg);
        s.push(0xffff);s.push(0xfffe);s.cs=ldm::LoadSegment+cs;s.ip=ip;
    }
    void returned(){until([&]{return s.cs==0xffff;});require(s.ip==0xfffe,"Corrupted far return");}
    void river_state() {
        set(0x5e04,0);set(0x5e06,0);set(0x5e08,1);set(0x5e0a,0);set(0x5e0c,0);
        set(0x5b5e,0);set(0x5b86,1);set(0x5b4a,40);set(0x5b4c,55);set(0x532c,0);
    }
    void river(){river_state();call(0x33f,0xe);until([&]{return at(0xfa7,6);});}
    void blocked_pan() {
        require(at(0xfa7,6),"Pan must start from the river input poll");
        int bags=word(0x53ea);
        // Keyboard P and a click at the Pan location must both remain harmless.
        for(bool click:{false,true}) {
            if(click && !s.qol_improvements) {
                // Classic input consumes the first click to open the hand.
                // Wait past its release drain before the selection click.
                set(0x5d62,0);step();
                until([&]{return at(0xfc5,0x38) && s.u16(s.ss,s.sp)==0x0876;});
            }
            if(click){s.mouse.move(100,147);s.mouse.buttons(1);s.mouse.buttons(0);}
            else s.keys.push_back(0x1970);
            step();until([&]{return at(0xfa7,6) || s.panning_active;});
            require(!s.panning_active && word(0x53ea)==bags,"Missing pan still animates or awards gold");
            s.mouse.clear();s.reset_world_pointer();set(0x5d62,1);
        }
    }
    void capture(const std::string& name) {
        std::filesystem::create_directories("captures/pan-inventory");
        ldm::Pixels pixels;ldm::read_frame(s,pixels);
        std::ofstream f("captures/pan-inventory/"+name+(s.qol_improvements?"-qol":"-classic")+".ppm",std::ios::binary);f<<"P6\n320 200\n255\n";
        for(auto c:pixels){char rgb[]={char(c>>16),char(c>>8),char(c)};f.write(rgb,3);}
    }
    void fixture(const std::filesystem::path& dir)const {
        require(!std::filesystem::exists(dir),"Fixture directory must be fresh");
        std::filesystem::create_directories(dir);
        std::ofstream f(dir/"LDMSAVE1.SAV",std::ios::binary);
        for(auto block:std::array<std::pair<int,int>,8>{{{0x5b4a,0x36},{0x5e04,0x20},
            {0x5314,0x24},{0x53d4,0x32},{0x5d5a,6},{0x500e,0x58},{0x5bd4,0x58},{0x389c,0x1650}}})
            for(int i=0;i<block.second;i++)f.put(char(s.u8(s.ds,uint16_t(block.first+i))));
        std::filesystem::copy_file(s.file_path("LDMSAVE.LDM",false),dir/"LDMSAVE.LDM");
    }
};
void scenario(const char* data,bool qol) {
    Game g(data,qol);auto& s=g.s;
    for(int slot=1;slot<11;slot++)g.set(0x500e + slot*8,0x12);
    g.call(0x8c0,0x2718,{0xf});g.returned(); // Buy with a full pack.
    require(g.word(0x53dc)==1 && g.word(0x53f0)==1000 && !s.has_pan(),
            "Full-pack purchase did not reproduce the stale pan counter");
    g.call(0x652,0x17e0,{0,1});g.returned(); // Discard an ordinary item to make room.
    g.river();require(!(s.game_ui.context_buttons&2),"A phantom pan enables the river's Pan button");
    g.blocked_pan();g.capture("rejected-purchase");
    g.call(0x8c0,0x2718,{0xf});g.returned(); // Purchase a real pan in the free slot.
    require(s.has_pan() && g.word(0x5016)==0xf && g.word(0x53f0)<1000,
            "Successful purchase did not put a paid pan in the pack");
    g.river();require(s.game_ui.context_buttons&2,"A purchased pan did not enable Pan");
    g.capture("owned-pan");
    // Discard the last pan through the original routine while preserving the
    // active river stack, so its already-drawn Pan button is also exercised.
    auto cs=s.cs,ip=s.ip,sp=s.sp;
    s.push(0);s.push(1);s.push(0xffff);s.push(0xfffe);
    s.cs=ldm::LoadSegment+0x652;s.ip=0x17e0;g.returned();
    s.cs=cs;s.ip=ip;s.sp=sp;
    require(!s.has_pan() && g.word(0x53dc)==1,"Last-pan discard did not retain the phantom count fixture");
    g.blocked_pan();
    g.river();require(!(s.game_ui.context_buttons&2),"Discarding the last pan left Pan available on entry");
    g.blocked_pan();g.capture("last-pan-discarded");
    std::cout<<"PASS: full-pack purchase, real purchase and last-pan discard use actual inventory; river mouse/P commands reject phantom pans; QoL "<<(qol?"on":"off")<<'\n';
}
}
int main(int argc,char** argv) {
    try {
        require(argc==2 || argc==3,"Usage: test-pan-inventory GAME [FRESH-FIXTURE-DIRECTORY]");
        if(argc==3) {
            Game g(argv[1],true);g.river_state();g.set(0x53dc,1);
            g.fixture(std::filesystem::path(argv[2])/"missing");
            g.set(0x5016,0xf);g.fixture(std::filesystem::path(argv[2])/"owned");return 0;
        }
        scenario(argv[1],true);scenario(argv[1],false);
    }catch(const std::exception& e){std::cerr<<e.what()<<'\n';return 1;}
}

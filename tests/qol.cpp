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
    uint64_t next_timer=1000,timers=0;
    explicit Game(const char* data) {
        s.data_dir=std::filesystem::absolute(data);
        s.save_dir=std::filesystem::absolute(".local/qol-test-saves");
        s.load("recovered/load-image.bin",image_relocations,entry_cs,entry_ip,stack_ss,stack_sp);
        until([&]{return at(0xfa7,6) && s.u16(0x82bd,0x5e04)==1;});
    }
    bool at(int cs,int ip)const{return s.cs==ldm::LoadSegment+cs && s.ip==ip;}
    void step() {
        ldm::native_step(s);
        if(s.boundaries>=next_timer){next_timer=s.boundaries+1000;s.timer_interrupt();++timers;}
    }
    template<class F> void until(F done) {
        auto start=s.boundaries;
        while(!done()){require(s.boundaries-start<120000000,"Scenario did not reach its original boundary");step();}
    }
    void call(int cs,int ip,std::initializer_list<uint16_t> args={}) {
        s.keys.clear();s.mouse.clear();s.reset_world_pointer();s.set_movement(0);
        s.w16(s.ds,0x5a1a,0);s.sp=0x8000;
        for(auto arg:args)s.push(arg);
        s.push(0xffff);s.push(0xfffe);s.cs=ldm::LoadSegment+cs;s.ip=ip;
    }
    void returned(){until([&]{return s.cs==0xffff;});require(s.ip==0xfffe,"Corrupted original far return");}
    void capture(const std::string& name) {
        std::filesystem::create_directories("captures/qol");
        ldm::Pixels pixels;ldm::read_frame(s,pixels);
        std::ofstream f("captures/qol/"+name+".ppm",std::ios::binary);f<<"P6\n320 200\n255\n";
        for(auto c:pixels){char rgb[]={char(c>>16),char(c>>8),char(c)};f.write(rgb,3);}
    }
};
void pointer(const char* data) {
    Game g(data);auto& s=g.s;
    require(s.mouse_visibility<0 && s.game_ui.pointer_visible(s),"Walking hides the QoL pointer");
    g.capture("town-pointer");
    s.qol_improvements=false;
    require(!s.game_ui.pointer_visible(s),"QoL off shows a walking pointer");
    s.qol_improvements=true;
    g.call(0xfa7,6);s.set_movement(8);
    s.mouse.move(275,180);s.mouse.buttons(1);s.mouse.buttons(0);g.returned();
    require(s.ax==8 && s.world_click_pending && s.u16(s.ds,0x5d62)==1,
            "A quick click interrupted held movement or was lost");
    // Preserve the pending click when invoking the real world command poll.
    s.sp=0x8000;s.push(0);s.push(0xffff);s.push(0xfffe);s.cs=ldm::LoadSegment;s.ip=0x7f2;
    g.until([&]{return g.at(0x652,0x1a52);});
    require(!s.world_click_pending,"Single toolbar click was not consumed");
    s.mouse.clear();s.reset_world_pointer();
    require(!s.world_click_pending,"Focus reset retained a click");
    std::cout<<"PASS: visible walking pointer, held direction plus one quick F6 click, QoL-off pointer and cleared input\n";
}
void building(const char* data) {
    for(bool qol:{false,true}) {
        Game g(data);auto& s=g.s;s.qol_improvements=qol;
        s.w16(s.ds,0x5b5e,2);s.w16(s.ds,0x5e04,0);s.w16(s.ds,0x5b86,1);
        s.w16(s.ds,0x5b4a,150);s.w16(s.ds,0x5b4c,63);
        s.w16(s.ds,0x5b60,80);s.w16(s.ds,0x5b5a,40);
        g.call(0x8c0,0x74);g.until([&]{return g.at(0xfa7,6);});
        require(s.u16(s.ds,0x5b60)==80,"Loading saloon overwrote the saved street position");
        g.capture(qol?"saloon-loaded":"saloon-loaded-classic");
        s.keys.push_back(0x1265);g.returned();
        require(s.u16(s.ds,0x5b4a)==80 && s.u16(s.ds,0x5b4c)==59 &&
                s.u16(s.ds,0x5e04)==1 && s.u16(s.ds,0x5b5e)==0,
                "Saloon exit did not restore the saved doorway");
        g.call(0,0x238);g.until([&]{return g.at(0xfa7,6);});
        g.capture(qol?"saloon-exit":"saloon-exit-classic");
        s.w16(s.ds,0x5b86,0);s.w16(s.ds,0x5b4a,240);s.w16(s.ds,0x5b5e,2);
        g.call(0x8c0,0x74);g.until([&]{return g.at(0x8c0,0x9d4);});
        require(s.u16(s.ds,0x5b60)==240,"Normal building entry did not remember the street position");
    }
    std::cout<<"PASS: loaded saloon exits to its saved doorway with QoL on/off; normal entry still records the street\n";
}
void mules(const char* data) {
    Game g(data);auto& s=g.s;
    const int prices[]={800,1200,2000};
    for(bool qol:{false,true})for(int owned=0;owned<8;owned++)for(int pick=0;pick<3;pick++) {
        s.qol_improvements=qol;
        for(int i=0;i<3;i++)s.w16(s.ds,uint16_t(0x5d5a+i*2),(owned>>i)&1);
        s.w16(s.ds,0x53f0,10000);s.w16(s.ds,0x53f2,0);
        bool allowed=!(owned&(1<<pick)) && (!qol || (owned&((1<<pick)-1))==((1<<pick)-1));
        g.call(0x8c0,0x220c,{uint16_t(pick+1)});g.returned();
        require(s.u16(s.ds,0x53f0)==10000-(allowed?prices[pick]:0),"Mule selection charged the wrong price or sold a blocked mule");
        for(int i=0;i<3;i++)require(s.u16(s.ds,uint16_t(0x5d5a+i*2))==
                unsigned(((owned>>i)&1) || (allowed && i==pick)),"Mule purchase changed the wrong inventory row");
    }
    s.qol_improvements=true;
    for(int owned:{0,1,3,7}) {
        for(int i=0;i<3;i++)s.w16(s.ds,uint16_t(0x5d5a+i*2),(owned>>i)&1);
        g.call(0x8c0,0x1de2);g.until([&]{return g.at(0x8c0,0x1c4);});
        require(s.game_ui.mule_shop_visible,"Mule availability labels missing at shop input");
        g.capture("mules-"+std::to_string(owned));
    }
    s.keys.push_back(0x3b00);g.until([&]{return g.at(0x652,0xaa);});
    require(!s.game_ui.mule_shop_visible,"Mule labels cover a keyboard-opened status dialog");
    std::cout<<"PASS: 48 actual mule purchases/rejections cover all ownership patterns, cheapest-first, prices, row ownership and QoL off\n";
}
void desert(const char* data) {
    Game g(data);auto& s=g.s;
    s.w16(s.ds,0x5e04,0);s.w16(s.ds,0x5e06,1);
    s.w16(s.ds,0x5b4a,40);s.w16(s.ds,0x5b4c,55);
    g.call(0x5d6,0x5de,{1});g.until([&]{return s.desert_view_active;});
    g.capture("desert-open");
    auto start=g.timers;
    s.keys.push_back(ldm::KeyRepeat|0x3920);
    while(g.timers-start<200)g.step();
    require(s.desert_view_active,"Desert closed on a held Space repeat or its old timeout");
    s.keys.push_back(0x3920);g.returned();g.capture("desert-closed");
    require(!s.desert_view_active && s.keys.empty() && s.u16(s.ds,0x5a1a)==0,
            "Desert dismissal leaked into the next map input");
    g.call(0xfa7,6);g.returned();require(s.ax==0,"Second Space reopened the desert");
    s.qol_improvements=false;g.call(0x5d6,0x5de,{1});g.returned();
    require(!s.desert_view_active,"QoL off replaced the original timed preview");
    std::cout<<"PASS: Space close-up waits, ignores repeats, consumes dismissal, returns to map; classic timed preview preserved\n";
}
void walking(const char* data,bool mine) {
    uint64_t elapsed[2]{};int clock[2]{},distance[2]{};
    for(int qol=0;qol<2;qol++) {
        Game g(data);auto& s=g.s;s.qol_improvements=qol;
        if(mine) {
            s.w16(s.ds,0x5e04,0);s.w16(s.ds,0x5e0a,1);
            s.w16(s.ds,0x5b6a,0);s.w16(s.ds,0x5b68,0);
            s.w16(s.ds,0x5016,0x12);s.w16(s.ds,0x53e4,10);
            g.call(0xbb4,0x358);g.until([&]{return g.at(0xfa7,6);});
        }
        s.set_movement(8);s.w16(s.ds,0x5406,0);
        // One warm-up loop establishes the held-input cadence.
        g.step();g.until([&]{return g.at(0xfa7,6);});
        auto start=g.timers;int clock_start=s.u16(s.ds,0x5406);
        int x=s.u16(s.ds,0x5b4a)+4*s.u16(s.ds,0x5b5a);
        for(int i=0;i<8;i++){g.step();g.until([&]{return g.at(0xfa7,6);});}
        elapsed[qol]=g.timers-start;clock[qol]=s.u16(s.ds,0x5406)-clock_start;
        distance[qol]=s.u16(s.ds,0x5b4a)+4*s.u16(s.ds,0x5b5a)-x;
        g.capture(std::string(mine?"mine-walk":"town-walk")+(qol?"-qol":"-classic"));
        s.set_movement(0);g.step();g.until([&]{return g.at(0xfa7,6);});
        require(!s.walk_fast && !s.walk_extra_tick,"Key release did not restore ordinary timing");
    }
    require(distance[0]>0 && distance[0]==distance[1],"Faster walking changed original collision steps");
    require(elapsed[1]*4<elapsed[0]*3,"Held walking did not reduce frame delay");
    require(clock[0]==8 && clock[1]==4,"Faster walking accelerated the survival clock");
    std::cout<<"PASS: "<<(mine?"mine":"town")<<" walking: "<<distance[1]<<" pixels in "<<elapsed[0]<<" -> "<<elapsed[1]
             <<" synthetic PIT ticks; clock "<<clock[0]<<" -> "<<clock[1]<<"; original collision steps and key-release cadence\n";
}
}
int main(int argc,char** argv) {
    if(argc!=2){std::cerr<<"Usage: test-qol <original game directory>\n";return 2;}
    try {pointer(argv[1]);building(argv[1]);mules(argv[1]);desert(argv[1]);walking(argv[1],false);walking(argv[1],true);}
    catch(const std::exception& e){std::cerr<<e.what()<<"\n";return 1;}
}

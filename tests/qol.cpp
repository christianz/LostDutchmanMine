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
void pointer_parking(const char* data) {
    Game g(data);auto& s=g.s;
    for(bool qol:{false,true}) {
        s.qol_improvements=qol;s.mouse.move(33,44);
        g.call(0x505,0x30e);g.returned(); // The real selector's cursor setup.
        require(s.mouse.current().x==(qol?33:240) && s.mouse.current().y==(qol?44:140),
                "Selector setup must park only the classic hand cursor");
    }
    std::cout<<"PASS: original selector parks the classic hand at the clock; QoL keeps the physical pointer position\n";
}
void menu_hover(const char* data) {
    Game g(data);auto& s=g.s;s.custom_cursor=false;
    s.w16(s.ds,0x5e04,0);s.w16(s.ds,0x5e08,1);
    s.w16(s.ds,0x53dc,1);s.w16(s.ds,0x53e8,1);
    s.w16(s.ds,0x505e,0xf);
    g.call(0x33f,0xe);g.until([&]{return g.at(0xfa7,6);});
    auto hover=[&](int x,int y) {
        ldm::Pixels normal,pointed;s.mouse.move(0,0);ldm::read_frame(s,normal);
        s.mouse.move(x,y);ldm::read_frame(s,pointed);return normal!=pointed;
    };
    require(hover(100,125),"River button does not highlight before opening a menu");
    for(int menu:{1,3})for(bool mouse:{false,true}) {
        auto name=std::string(menu==1?"health":"inventory")+(mouse?"-mouse":"-key");
        if(mouse)g.call(0,0x93e,{181,uint16_t(69+menu*42)});
        else g.call(0,0xae4,{0,0,uint16_t(0x3b+menu)});
        g.until([&]{return g.at(0xfc5,0x38);});
        s.mouse.move(100,125);g.capture("modal-"+name);
        for(int i=0;i<4;i++) {
            auto r=ldm::GameUI::action(i);
            require(!hover(r.x+5,r.y+5),"A menu highlights a hidden river button");
        }
        require(!hover(69,181),"An inactive toolbar button highlights behind a menu");
        s.keys.push_back(0x1c0d);g.returned();
        require(hover(100,125),"Closing a menu did not restore river button hover");
        g.capture("restored-"+name);
    }
    std::cout<<"PASS: health/inventory opened by mouse and keyboard hide underlying hover targets and restore river hover on close\n";
}
void map_diagonals(const char* data) {
    Game g(data);auto& s=g.s;
    s.w16(s.ds,0x5e04,0);s.w16(s.ds,0x5e06,1);
    for(int mask:{5,9,6,10})for(int scan:{mask&1?0x48:0x50,mask&4?0x4b:0x4d})for(bool repeat:{false,true}) {
        s.w16(s.ds,0x5b4a,160);s.w16(s.ds,0x5b4c,55);
        s.w16(s.ds,0x5b56,0);s.w16(s.ds,0x5b58,0);
        g.call(0,0x4da);g.until([&]{return g.at(0xfa7,6);});
        int step=s.u16(s.ds,0x19a);
        s.mouse.move(33,44);
        s.set_movement(mask);s.keys.push_back(uint32_t(scan<<8)|(repeat?ldm::KeyRepeat:0));
        g.until([&]{return g.at(0,0x67e);}); // Before the next location/event check.
        require(s.u16(s.ds,0x5b4a)==160+(mask&4?-2:2)*step &&
                s.u16(s.ds,0x5b4c)==55+(mask&1?-1:1)*step,
                "World-map diagonal lost an axis when a single key event arrived");
        require(s.mouse.current().x==33 && s.mouse.current().y==44,"Map movement relocated the pointer");
    }
    std::cout<<"PASS: all four world-map diagonals retain both axes with either last key and with auto-repeat, using original terrain steps\n";
}
void sleep_hover(const char* data) {
    for(bool mouse:{false,true}) {
        Game g(data);auto& s=g.s;s.custom_cursor=false;
        s.w16(s.ds,0x5b5e,2);s.w16(s.ds,0x5e04,0);s.w16(s.ds,0x5b86,1);
        s.w16(s.ds,0x5b4a,150);s.w16(s.ds,0x5b4c,63);s.w16(s.ds,0x5b60,80);
        s.w16(s.ds,0x5328,22);
        g.call(0x8c0,0x74);g.until([&]{return g.at(0xfa7,6);});
        auto hover=[&](int x,int y) {
            s.custom_cursor=false; // Building entry installs its original cursor.
            ldm::Pixels normal,pointed;s.mouse.move(0,0);ldm::read_frame(s,normal);
            s.mouse.move(x,y);ldm::read_frame(s,pointed);return normal!=pointed;
        };
        require(hover(100,147),"Saloon Sleep button does not highlight before sleeping");
        if(mouse){s.mouse.move(100,147);s.mouse.buttons(1);s.mouse.buttons(0);}
        else s.keys.push_back(0x1f73);
        g.until([&]{return g.at(0x8c0,0x29b6);});g.step();
        g.until([&]{return g.at(0x505,0xc);});
        s.mouse.move(100,147);g.capture(mouse?"sleep-mouse":"sleep-key");
        for(int i=0;i<4;i++) {
            auto r=ldm::GameUI::action(i);
            require(!hover(r.x+5,r.y+5),"Sleep highlights an inactive saloon button");
        }
        for(int i=0;i<6;i++) {
            auto r=ldm::GameUI::toolbar(i);
            require(!hover(r.x+5,r.y+5),"Sleep highlights an inactive toolbar button");
        }
        g.returned();
        require(s.u16(s.ds,0x5328)==9 && s.u16(s.ds,0x5b5e)==0 &&
                s.u16(s.ds,0x5b4a)==80,"Sleep did not wake at the original town doorway");
        g.call(0,0x238);g.until([&]{return g.at(0xfa7,6);});
        require(hover(69,181),"Waking up did not restore toolbar hover");
        require(!hover(100,147),"Waking up restored a stale saloon Sleep target");
        g.capture(mouse?"awake-mouse":"awake-key");
    }
    std::cout<<"PASS: saloon sleep by mouse/key suspends all hidden hover targets and restores the town toolbar on waking\n";
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
        ldm::Pixels labels;s.custom_cursor=false;ldm::read_frame(s,labels);
        for(int i=0;i<3;i++)if(!s.mule_available(i))
            for(int y=85;y<96;y++)for(int x=26+i*100;x<94+i*100;x++)
                require(labels[y*320+x]==s.palette[0],"Old mule lettering remains above SOLD OUT");
        for(int x=0;x<320;x++)
            require(labels[110*320+x]==s.palette[s.memory[0xa0000+110*320+x]],"SOLD OUT erases the shop's lower border");
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
void walking(const char* data,int scene) {
    bool mine=scene==1,saloon=scene==2;
    const char* name=mine?"mine":saloon?"saloon":"town";
    uint64_t elapsed[2]{};int clock[2]{},distance[2]{};
    for(int qol=0;qol<2;qol++) {
        Game g(data);auto& s=g.s;s.qol_improvements=qol;
        if(mine) {
            s.w16(s.ds,0x5e04,0);s.w16(s.ds,0x5e0a,1);
            s.w16(s.ds,0x5b6a,0);s.w16(s.ds,0x5b68,0);
            s.w16(s.ds,0x5016,0x12);s.w16(s.ds,0x53e4,10);
            g.call(0xbb4,0x358);g.until([&]{return g.at(0xfa7,6);});
        }else if(saloon) {
            s.w16(s.ds,0x5e04,0);s.w16(s.ds,0x5b5e,2);
            g.call(0x8c0,0x74);g.until([&]{return g.at(0xfa7,6);});
        }
        s.set_movement(8);s.w16(s.ds,0x5406,0);
        // One warm-up loop establishes the held-input cadence.
        g.step();g.until([&]{return g.at(0xfa7,6);});
        auto start=g.timers;int clock_start=s.u16(s.ds,0x5406);
        int x=s.u16(s.ds,0x5b4a)+4*s.u16(s.ds,0x5b5a);
        for(int i=0;i<8;i++){g.step();g.until([&]{return g.at(0xfa7,6);});}
        elapsed[qol]=g.timers-start;clock[qol]=s.u16(s.ds,0x5406)-clock_start;
        distance[qol]=s.u16(s.ds,0x5b4a)+4*s.u16(s.ds,0x5b5a)-x;
        g.capture(std::string(name)+"-walk"+(qol?"-qol":"-classic"));
        s.set_movement(0);g.step();g.until([&]{return g.at(0xfa7,6);});
        require(!s.walk_fast && !s.walk_extra_tick,"Key release did not restore ordinary timing");
    }
    require(distance[0]>0 && distance[0]==distance[1],"Faster walking changed original collision steps");
    require(elapsed[1]*4<elapsed[0]*3,"Held walking did not reduce frame delay");
    require(clock[0]==8 && clock[1]==4,"Faster walking accelerated the survival clock");
    std::cout<<"PASS: "<<name<<" walking: "<<distance[1]<<" pixels in "<<elapsed[0]<<" -> "<<elapsed[1]
             <<" synthetic PIT ticks; clock "<<clock[0]<<" -> "<<clock[1]<<"; original collision steps and key-release cadence\n";
}
}
int main(int argc,char** argv) {
    if(argc!=2){std::cerr<<"Usage: test-qol <original game directory>\n";return 2;}
    try {sleep_hover(argv[1]);pointer(argv[1]);pointer_parking(argv[1]);menu_hover(argv[1]);map_diagonals(argv[1]);building(argv[1]);mules(argv[1]);desert(argv[1]);walking(argv[1],0);walking(argv[1],1);walking(argv[1],2);}
    catch(const std::exception& e){std::cerr<<e.what()<<"\n";return 1;}
}

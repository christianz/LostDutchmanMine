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
    int pick_frames=0;
    int x=135,y=61,scroll_x=0,scroll_y=0;
    explicit Game(const char* data) {
        s.data_dir=std::filesystem::absolute(data);
        s.save_dir=std::filesystem::absolute(".local/cave-test-saves");
        s.load("recovered/load-image.bin",image_relocations,entry_cs,entry_ip,stack_ss,stack_sp);
        until([&]{return input() && word(0x5e04);});
        // Pick an actual generated cave on a scrolled part of the world map.
        bool found=false;
        for(int tx=20;tx<40 && !found;tx++)for(int ty=4;ty<12 && !found;ty++)
            if(s.u8(s.ds,uint16_t(0x0f60+tx*18+ty))) {
                scroll_x=tx-x/16;scroll_y=ty-(y-8)/16;found=true;
            }
        require(found,"No generated cave for the round-trip fixture");
        for(int a:{0x5e04,0x5e08,0x5e0a,0x5e0c,0x5b5e,0x5b86})set(a,0);
        set(0x5e06,1);set(0x5b4a,x);set(0x5b4c,y);
        set(0x5b56,scroll_x);set(0x5b58,scroll_y);
        set(0x5016,0x12);set(0x53e4,10); // A lamp and oil.
        s.keys.clear();s.mouse.clear();s.reset_world_pointer();s.set_movement(0);
        // Resume the outer game dispatcher after startup with a fresh frame.
        // This retains its real handling of scene return codes, including 9.
        s.sp=0x8000;s.bp=0x8004;s.cs=ldm::LoadSegment;s.ip=0x95;
        until([&]{return input();});
    }
    int word(int a)const{return s.u16(s.ds,uint16_t(a));}
    void set(int a,int value){s.w16(s.ds,uint16_t(a),uint16_t(value));}
    bool at(int cs,int ip)const{return s.cs==ldm::LoadSegment+cs && s.ip==ip;}
    bool input()const{return at(0xfa7,6);}
    bool entrance()const {
        return at(0x505,0xc) && s.u16(s.ss,s.sp)==0x3a1 &&
               s.u16(s.ss,uint16_t(s.sp+2))==ldm::LoadSegment+0xbb4;
    }
    void step() {
        if(at(0xbb4,0x1047))++pick_frames;
        ldm::native_step(s);
        if(s.boundaries>=next_timer){next_timer=s.boundaries+1000;s.timer_interrupt();}
    }
    template<class F> void until(F done) {
        auto start=s.boundaries;
        while(!done()){require(s.boundaries-start<120000000,"Cave scenario timed out");step();}
    }
    void next_input() {step();until([&]{return input() || entrance();});}
    void click(bool held) {s.mouse.move(180,50);s.mouse.buttons(1);if(!held)s.mouse.buttons(0);}
    void dispatch() {s.sp=0x8000;s.bp=0x8004;s.cs=ldm::LoadSegment;s.ip=0x95;}
    void leave() {
        s.mouse.buttons(0);s.set_movement(4);
        until([&]{return input() && word(0x5e06);});s.set_movement(0);
        position_restored();
    }
    void fixture(const std::filesystem::path& dir)const {
        require(!std::filesystem::exists(dir),"Fixture output must be a fresh directory");
        std::filesystem::create_directories(dir);
        std::ofstream f(dir/"LDMSAVE1.SAV",std::ios::binary);
        // The eight blocks used by the original save/load routines. This only
        // exports isolated QA state; it does not modify any supplied save.
        for(auto block:std::array<std::pair<int,int>,8>{{{0x5b4a,0x36},{0x5e04,0x20},
            {0x5314,0x24},{0x53d4,0x32},{0x5d5a,6},{0x500e,0x58},{0x5bd4,0x58},{0x389c,0x1650}}})
            for(int i=0;i<block.second;i++)f.put(char(s.u8(s.ds,uint16_t(block.first+i))));
        std::filesystem::copy_file(s.file_path("LDMSAVE.LDM",false),dir/"LDMSAVE.LDM");
    }
    void position_restored()const {
        require(word(0x5b4a)==x && word(0x5b4c)==y &&
                word(0x5b56)==scroll_x && word(0x5b58)==scroll_y,
                "Cave exit changed the world-map position or scroll offsets");
    }
    void capture(const std::string& name) {
        std::filesystem::create_directories("captures/cave");
        ldm::Pixels pixels;ldm::read_frame(s,pixels);
        std::ofstream f("captures/cave/"+name+".ppm",std::ios::binary);f<<"P6\n320 200\n255\n";
        for(auto c:pixels){char rgb[]={char(c>>16),char(c>>8),char(c)};f.write(rgb,3);}
    }
};
void round_trip(const char* data,bool during_preview,bool held) {
    Game g(data);auto& s=g.s;
    g.capture("map-before");s.keys.push_back(0x3920);
    g.until([&]{return g.entrance();});g.capture("entrance");
    if(during_preview)g.click(held);
    g.step();g.until([&]{return g.input();});g.capture("inside");
    require(g.word(0x5e0a) && !g.word(0x5e06),"Space did not enter the generated cave");
    require(g.word(0x5b60)==g.x && g.word(0x5b62)==g.y,"Cave entry lost its map return position");
    if(!during_preview)g.click(held);
    // The whole original scene dispatcher stays live: a leaked command 9
    // would restart the entrance and later save indoor coordinates as map X/Y.
    for(int loop=0;loop<8;loop++) {
        g.next_input();
        require(!g.entrance(),"Scenery click restarted the cave entrance");
        require(g.word(0x5b60)==g.x && g.word(0x5b62)==g.y,"Scenery click overwrote the map return position");
    }
    g.leave();g.capture("map-after");
    std::cout<<"PASS: "<<(held?"held":"quick")<<" click "<<(during_preview?"during entrance":"inside cave")
             <<" stays inside; walking out restores exact map position and scroll\n";
}
void resume(const char* data,bool qol) {
    Game g(data);auto& s=g.s;s.qol_improvements=qol;
    s.keys.push_back(0x3920);g.step();g.until([&]{return g.input() && g.word(0x5e0a);});
    // Match the original loaded-scene flag while retaining saved exterior X/Y.
    g.set(0x5b86,1);g.set(0x5b4a,90);g.dispatch();
    g.until([&]{return g.input() || g.entrance();});
    require(!g.entrance() && g.word(0x5b4a)==90,"Resuming cave replayed its entrance or reset indoor position");
    if(qol){g.click(false);g.next_input();require(!g.entrance(),"Click restarted a resumed cave");}
    g.leave();
    std::cout<<"PASS: resumed cave preserves indoor position and exterior return with QoL "<<(qol?"on":"off")<<'\n';
}
void no_light(const char* data,bool lamp) {
    Game g(data);auto& s=g.s;
    g.set(lamp?0x53e4:0x5016,0);
    s.keys.push_back(0x3920);g.step();g.until([&]{return g.entrance();});g.click(false);
    g.step();g.until([&]{return g.input() && g.word(0x5e06);});
    g.position_restored();
    std::cout<<"PASS: rejected cave entry without "<<(lamp?"oil":"lamp")<<" restores exact map position\n";
}
void held_pick(const char* data,bool qol) {
    Game g(data);auto& s=g.s;
    s.keys.push_back(0x3920);g.step();g.until([&]{return g.input() && g.word(0x5e0a);});
    s.qol_improvements=qol;
    g.set(0x501e,0x15); // A pick, leaving the lamp in its existing slot.
    g.set(0x5b4a,160);g.set(0x5b4c,50);g.set(0x5d62,1);g.set(0x93a,0);
    s.mining_space_held=true;s.keys.push_back(0x3920);
    g.step();g.until([&]{return g.pick_frames>=5;});
    g.capture(qol?"held-pick-qol":"held-pick-classic");
    s.mining_space_held=false;
    g.until([&]{return g.input() && s.u16(s.ss,s.sp)==0x06bd;});
    int strokes=g.pick_frames;
    require(strokes==5,"Releasing Space allowed an extra pick stroke");
    for(int i=0;i<3;i++)g.next_input();
    require(g.pick_frames==strokes,"Mining continued after release");
    s.keys.push_back(0x3920);g.step();g.until([&]{return g.input() && s.u16(s.ss,s.sp)==0x06bd;});
    require(g.pick_frames==strokes+1,"A Space tap must still make one stroke");
    std::cout<<"PASS: held Space makes five original pick strokes; release stops and a tap makes one, with QoL "<<qol<<'\n';
}
}
int main(int argc,char** argv) {
    try {
        require(argc>=2 && argc<=4,"Usage: test-cave GAME-DIRECTORY [FRESH-MAP-FIXTURE [FRESH-MINING-FIXTURE]]");
        if(argc>=3) {
            Game g(argv[1]);g.fixture(argv[2]);
            if(argc==4) {
                g.s.keys.push_back(0x3920);g.step();g.until([&]{return g.input() && g.word(0x5e0a);});
                g.set(0x501e,0x15);g.set(0x5b4a,160);g.set(0x5b4c,50);
                g.fixture(argv[3]);
            }
            return 0;
        }
        for(bool preview:{false,true})for(bool held:{false,true})round_trip(argv[1],preview,held);
        for(bool qol:{false,true})resume(argv[1],qol);
        for(bool lamp:{false,true})no_light(argv[1],lamp);
        for(bool qol:{false,true})held_pick(argv[1],qol);
        return 0;
    }catch(const std::exception& e){std::cerr<<"FAIL: "<<e.what()<<'\n';return 1;}
}

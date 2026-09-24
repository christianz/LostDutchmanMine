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
    explicit Game(const char* data,bool qol) {
        s.data_dir=std::filesystem::absolute(data);
        s.save_dir=std::filesystem::absolute(".local/assay-test-saves");
        s.load("recovered/load-image.bin",image_relocations,entry_cs,entry_ip,stack_ss,stack_sp);
        until([&]{return at(0xfa7,6) && word(0x5e04);});
        s.qol_improvements=qol;
        set(0x5e04,0);set(0x5b5e,4);set(0x5b86,1);set(0x5b60,80);
        set(0x5b5a,40);set(0x53ea,8);set(0x53f0,1000);set(0x53f2,0);
        // Original loading restores cash from DS:5b72 / 3, not the cash block.
        set(0x5b72,3000);
        for(int row=0;row<4;row++) {
            if(row)set(0x5d58+row*2,1);
            for(int slot=1;slot<11;slot++)set(item(slot,row),0x2b);
            set(item(1,row),0x12); // Non-ore equipment must be ignored.
            set(item(6,row),0x20+row%3);set(item(7,row),0x23+row%3);
        }
    }
    static int item(int slot,int row){return 0x500e + slot*8+row*2;}
    int word(int a)const{return s.u16(s.ds,uint16_t(a));}
    void set(int a,int value){s.w16(s.ds,uint16_t(a),uint16_t(value));}
    uint32_t cash()const{return word(0x53f0)|(uint32_t(word(0x53f2))<<16);}
    bool at(int cs,int ip)const{return s.cs==ldm::LoadSegment+cs && s.ip==ip;}
    bool input()const{return at(0x8c0,0x1c4);}
    void step() {
        ldm::native_step(s);
        if(s.boundaries>=next_timer){next_timer=s.boundaries+1000;s.timer_interrupt();}
    }
    template<class F> void until(F done) {
        auto start=s.boundaries;
        while(!done()){require(s.boundaries-start<120000000,"Assay scenario timed out");step();}
    }
    void enter() {
        s.keys.clear();s.mouse.clear();s.reset_world_pointer();s.set_movement(0);
        s.sp=0x8000;s.push(0xffff);s.push(0xfffe);s.cs=ldm::LoadSegment+0x8c0;s.ip=0x74;
        until([&]{return input();});
    }
    void click(int x,int y) {
        require(input(),"Click must start at the building's input poll");
        s.mouse.move(x,y);s.mouse.buttons(1);s.mouse.buttons(0);
        // Finish both mouse edges and wait for the building to poll again.
        for(int i=0;i<3;i++){step();until([&]{return input() || s.cs==0xffff;});if(s.cs==0xffff)break;}
    }
    void capture(const std::string& name) {
        std::filesystem::create_directories("captures/assay");
        ldm::Pixels pixels;ldm::read_frame(s,pixels);
        std::ofstream f("captures/assay/"+name+".ppm",std::ios::binary);f<<"P6\n320 200\n255\n";
        for(auto c:pixels){char rgb[]={char(c>>16),char(c>>8),char(c)};f.write(rgb,3);}
    }
    void fixture(const std::filesystem::path& dir)const {
        require(!std::filesystem::exists(dir),"Fixture output must be fresh");
        std::filesystem::create_directories(dir);
        std::ofstream f(dir/"LDMSAVE1.SAV",std::ios::binary);
        for(auto block:std::array<std::pair<int,int>,8>{{{0x5b4a,0x36},{0x5e04,0x20},
            {0x5314,0x24},{0x53d4,0x32},{0x5d5a,6},{0x500e,0x58},{0x5bd4,0x58},{0x389c,0x1650}}})
            for(int i=0;i<block.second;i++)f.put(char(s.u8(s.ds,uint16_t(block.first+i))));
        std::filesystem::copy_file(s.file_path("LDMSAVE.LDM",false),dir/"LDMSAVE.LDM");
    }
};
void assay(const char* data,bool qol) {
    Game g(data,qol);auto& s=g.s;g.enter();g.click(108,147);g.capture("select-bag");
    require(s.game_ui.context_buttons==11,"Assay did not open Next/Done/Exit");
    int remaining=8;
    for(int row=0;row<4;row++) {
        auto before=g.cash();g.click(42,98);
        require(g.word(0x53ea)==remaining && g.cash()==before && g.word(Game::item(1,row))==0x12,
                "Assay sold non-ore equipment");
        for(int slot:{6,7}) {
            before=g.cash();int bag=g.word(Game::item(slot,row));
            g.click(slot*29+14,98);
            require(g.word(0x53ea)==--remaining && g.word(Game::item(slot,row))==0x2b,
                    "Clicking a gold bag did not assay exactly that bag");
            int pounds=bag-(slot==6?31:30),grade=g.word(0x59d0);
            require(g.word(0x5b82)==pounds && grade>=(slot==6?0:5) && grade<=(slot==6?4:10),
                    "Assay changed the original weight or grade rules");
            require(g.cash()==before+unsigned(pounds*grade*10),"Assay payout was not credited exactly once");
            g.capture("row-"+std::to_string(row)+"-slot-"+std::to_string(slot));
            before=g.cash();g.click(slot*29+14,98);
            require(g.cash()==before && g.word(0x53ea)==remaining,"Empty slot paid for a bag again");
        }
        if(row<3)g.click(108,127); // Next mule's inventory row.
    }
    g.click(108,147);g.click(188,147);
    require(s.cs==0xffff && s.ip==0xfffe && g.word(0x5e04) && g.word(0x5b5e)==0,
            "Done/Exit did not return from the assay office");
    require(g.word(0x5b4a)==80 && g.word(0x5b4c)==59,"Assay exit lost the saved doorway");
    std::cout<<"PASS: mouse assay sells eight bags across all four inventory rows, credits each payout once, ignores equipment/empty slots, and Done/Exit returns to town; QoL "<<(qol?"on":"off")<<'\n';
}
}
int main(int argc,char** argv) {
    try {
        require(argc==2 || argc==3,"Usage: test-assay GAME-DIRECTORY [FRESH-FIXTURE-DIRECTORY]");
        if(argc==3){Game g(argv[1],true);g.fixture(argv[2]);return 0;}
        assay(argv[1],true);assay(argv[1],false);return 0;
    }catch(const std::exception& e){std::cerr<<"FAIL: "<<e.what()<<'\n';return 1;}
}

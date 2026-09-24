#include "session.h"
#include "image_info.h"
#include <algorithm>
#include <fstream>
#include <iostream>
#include <memory>
#include <stdexcept>
#include <vector>

namespace {
void require(bool ok,const char* message){if(!ok)throw std::runtime_error(message);}
void capture(const ldm::Pixels& pixels,const char* name) {
    std::filesystem::create_directories("captures/menu");
    std::ofstream f(std::string("captures/menu/")+name+".ppm",std::ios::binary);
    f<<"P6\n320 200\n255\n";
    for(auto c:pixels){char rgb[]={char(c>>16),char(c>>8),char(c)};f.write(rgb,3);}
}
}
int main(int argc,char** argv) {
    if(argc!=2){std::cerr<<"Usage: test-menu-scene <original game directory>\n";return 2;}
    try {
        auto s=std::make_unique<ldm::State>();
        s->data_dir=std::filesystem::absolute(argv[1]);s->save_dir=std::filesystem::absolute(".local/menu-scene-saves");
        s->load("recovered/load-image.bin",image_relocations,entry_cs,entry_ip,stack_ss,stack_sp);
        uint64_t next_timer=1000;
        while(!(s->cs==ldm::LoadSegment+0xfa7 && s->ip==6 && s->u16(0x82bd,0x5e04)==1)) {
            require(s->boundaries<120000000,"Original startup did not reach town");
            ldm::native_step(*s);
            if(s->boundaries>=next_timer){next_timer=s->boundaries+1000;s->timer_interrupt();}
        }
        require(s->game_ui.ready(),"Original initialization did not prepare the panel skin");
        s->custom_cursor=false;s->qol_improvements=false;
        ldm::Pixels original,enhanced;ldm::read_frame(*s,original);capture(original,"original");
        for(size_t i=0;i<original.size();i++)require(original[i]==s->palette[s->memory[0xa0000+i]],"QoL off changed the original VGA frame");
        auto memory=s->memory;s->qol_improvements=true;ldm::read_frame(*s,enhanced);capture(enhanced,"labels");
        int changed=0;
        for(int y=0;y<200;y++)for(int x=0;x<320;x++)if(enhanced[y*320+x]!=original[y*320+x]) {
            bool inside=false;for(int i=0;i<6;i++)inside|=ldm::GameUI::toolbar(i).contains(x,y);
            require(inside,"Toolbar repaint covered original scenery or context actions");
            require(std::find(s->palette.begin(),s->palette.begin()+16,enhanced[y*320+x])!=s->palette.begin()+16,"Panel atlas escaped the original 16-colour blitter palette");
            ++changed;
        }
        require(changed>500,"Toolbar enhancement did not render");
        s->game_ui.choosing=true;s->mouse_visibility=0;s->mouse.move(281,196);
        ldm::read_frame(*s,enhanced);capture(enhanced,"hover");
        require(s->memory==memory,"Native menu rendering wrote into original game memory");
        s->game_ui.choosing=false;
        // Run the original health/food update, including its thresholds and
        // critical-health flash. This catches an overlay hiding live warnings
        // behind the healthy artwork stored in PANL_VGA.
        auto status=[&](int supplies,const char* name) {
            for(int address:{0x5314,0x5316,0x5318})s->w16(s->ds,address,0);
            s->w16(s->ds,0x531a,supplies);s->w16(s->ds,0x531c,supplies);
            s->sp=0x8000;s->push(0xffff);s->push(0xfffe);
            s->cs=ldm::LoadSegment+0x0652;s->ip=0x1d5e;
            auto start=s->boundaries;
            while(s->cs!=0xffff) {
                require(s->boundaries-start<100000,"Original health update did not return");ldm::native_step(*s);
            }
            require(s->ip==0xfffe && s->sp==0x8000,"Original health update corrupted its return");
            auto health=s->u16(s->ds,0x531e);
            require(health==std::min(supplies+1,96),"Fixture did not produce the intended original health value");
            s->qol_improvements=false;ldm::read_frame(*s,original);
            capture(original,(std::string("health-")+name+"-original").c_str());
            auto unchanged=s->memory;
            s->qol_improvements=true;ldm::read_frame(*s,enhanced);
            capture(enhanced,(std::string("health-")+name+"-labels").c_str());
            require(s->memory==unchanged,"Toolbar rendering changed original health or framebuffer data");
            // A clear background pixel in the original portrait must remain
            // its warning colour after the icon is fitted above the label.
            require(enhanced[170*320+103]==original[171*320+100],"Labelled Life icon hides the original health warning colour");
            std::vector<uint32_t> food;
            for(int y=165;y<187;y++)for(int x=143;x<169;x++)food.push_back(enhanced[y*320+x]);
            return std::pair<uint32_t,std::vector<uint32_t>>{enhanced[170*320+103],food};
        };
        auto healthy=status(96,"healthy");
        auto green_edge=status(63,"64");
        auto amber=status(62,"63");
        auto amber_edge=status(31,"32");
        auto red=status(30,"31");
        auto flashing_a=status(3,"critical-a");
        auto flashing_b=status(3,"critical-b");
        auto recovered=status(96,"recovered");
        auto colour=[](const auto& frame){return frame.first;};
        require(colour(healthy)==colour(green_edge) && colour(healthy)==colour(recovered),"Healthy colour did not return after recovery");
        require(colour(healthy)!=colour(amber) && colour(amber)==colour(amber_edge) && colour(amber)!=colour(red),"Original green/warning/danger thresholds are hidden");
        require(colour(flashing_a)!=colour(flashing_b),"Critical health no longer flashes");
        require(healthy.second!=amber.second && amber.second!=red.second,"Labelled Food icon hides the original food warnings");
        require(healthy.second==recovered.second,"Food icon did not recover with original supplies");
        std::cout<<"PASS: original panel/font, confined labels, pixel-exact QoL off, live health/food warnings, 64/32 thresholds, critical flashing and recovery without changing game memory\n";
    }catch(const std::exception& e){std::cerr<<e.what()<<"\n";return 1;}
}

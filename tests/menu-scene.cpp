#include "session.h"
#include "image_info.h"
#include <algorithm>
#include <fstream>
#include <iostream>
#include <memory>
#include <stdexcept>

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
        std::cout<<"PASS: original startup loads the 16-colour panel/font, labelled toolbar stays in bounds, and QoL off is pixel exact without writing game memory\n";
    }catch(const std::exception& e){std::cerr<<e.what()<<"\n";return 1;}
}

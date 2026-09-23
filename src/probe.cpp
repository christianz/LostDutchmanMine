#include "legacy.h"
#include "image_info.h"
#include <fstream>
#include <iostream>
#include <memory>
#include <string>

int main(int argc,char**argv) {
    if(argc<2){std::cerr<<"usage: ldm-probe <original-data-dir> [boundary-limit] [keys] [synthetic-timer-period]\n";return 2;}
    auto s=std::make_unique<ldm::State>();s->data_dir=std::filesystem::absolute(argv[1]);s->save_dir=std::filesystem::absolute(".local/saves");
    auto limit=argc>2?std::stoull(argv[2]):1000000ULL;
    auto timer_period=argc>4?std::stoull(argv[4]):0ULL;
    auto next_timer=timer_period;
    if(argc>3)for(char c:std::string(argv[3]))s->keys.push_back(uint8_t(c));
    try {
        s->load("recovered/load-image.bin",image_relocations,entry_cs,entry_ip,stack_ss,stack_sp);
        while(s->running && s->boundaries<limit) {
            ldm::native_step(*s);
            if(timer_period && s->boundaries>=next_timer){next_timer=s->boundaries+timer_period;s->timer_interrupt();}
            if(s->waiting){std::cerr<<"Waiting for input\n";break;}
        }
        std::cerr<<"Stopped: boundaries="<<s->boundaries<<" running="<<s->running<<" mode="<<s->video_mode<<" cs:ip="<<std::hex<<s->cs-ldm::LoadSegment<<":"<<s->ip<<std::dec<<"\n";
    } catch(const std::exception&e) {
        std::cerr<<e.what()<<"\n";
        std::ofstream(".local/memory.bin",std::ios::binary).write(reinterpret_cast<const char*>(s->memory.data()),s->memory.size());
        return 1;
    }
    std::ofstream(".local/memory.bin",std::ios::binary).write(reinterpret_cast<const char*>(s->memory.data()),s->memory.size());
}

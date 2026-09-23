#include "assets.h"
#include "legacy.h"
#include <algorithm>
#include <fstream>
#include <iostream>
#include <memory>

int main(int argc,char**argv) {
    if(argc!=2){std::cerr<<"usage: test-assets <LDMG-directory>\n";return 2;}
    unsigned count=0;
    try {
        for(auto& e:std::filesystem::directory_iterator(argv[1])) {
            auto ext=e.path().extension().string();if(ext!=".BIN" && ext!=".ZZZ")continue;
            std::cout<<"Checking "<<e.path().filename().string()<<std::endl;
            std::ifstream f(e.path(),std::ios::binary);
            std::vector<uint8_t> packed{std::istreambuf_iterator<char>(f),{}};
            auto native=ldm::decode_asset(packed);
            auto s=std::make_unique<ldm::State>();
            s->cs=ldm::LoadSegment+0x1265;s->ip=0x1250;s->ds=0x3000;s->si=0x100;s->es=0x5000;s->di=0;s->ss=0x9000;s->sp=0xfffe;
            s->push(0xffff);
            std::copy(packed.begin(),packed.end(),s->memory.begin()+0x30100);
            while(s->ip!=0xffff && s->boundaries<2000000)ldm::native_step(*s);
            if(s->ip!=0xffff || s->flag(ldm::CF) || s->ax!=native.size() ||
                !std::equal(native.begin(),native.end(),s->memory.begin()+0x50000))
                throw std::runtime_error("Original decoder differs: "+e.path().filename().string());
            std::cout<<e.path().filename().string()<<": "<<native.size()<<" identical bytes\n";
            ++count;
            std::filesystem::create_directories("recovered/assets");
            std::ofstream out(std::filesystem::path("recovered/assets")/(e.path().filename().string()+".raw"),std::ios::binary);
            out.write(reinterpret_cast<const char*>(native.data()),native.size());
            packed.pop_back();bool rejected=false;
            try{ldm::decode_asset(packed);}catch(const std::runtime_error&){rejected=true;}
            if(!rejected)throw std::runtime_error("Truncated stream accepted");
        }
        if(count!=21)throw std::runtime_error("Expected all 21 supplied packed assets");
        std::cout<<"PASS: "<<count<<" original/native decoder comparisons and truncation checks\n";
    } catch(const std::exception&e){std::cerr<<e.what()<<"\n";return 1;}
}

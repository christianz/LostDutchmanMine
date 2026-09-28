#include "session.h"
#include "image_info.h"
#include <iostream>
#include <memory>
#include <sstream>
#include <stdexcept>

namespace {
void require(bool ok,const char* message){if(!ok)throw std::runtime_error(message);}
std::unique_ptr<ldm::State> boot(const char* data,const char* saves) {
    auto s=std::make_unique<ldm::State>();
    s->data_dir=std::filesystem::absolute(data);
    s->save_dir=std::filesystem::absolute(saves);
    std::filesystem::remove_all(s->save_dir);
    s->load("recovered/load-image.bin",image_relocations,entry_cs,entry_ip,stack_ss,stack_sp);
    return s;
}
std::string trace(const char* data,const char* saves) {
    auto s=boot(data,saves);
    std::ostringstream out;
    {
        ldm::Session session(*s,ldm::SessionMode::Deterministic);
        session.set_trace(&out);
        for(int q=0;q<3000;q++)session.step();
        session.finish_trace();
    }
    return out.str();
}
}
int main(int argc,char** argv) {
    try {
        require(argc==2,"Usage: test-trace GAME-DIRECTORY");
        auto a=trace(argv[1],".local/trace-test-a"),b=trace(argv[1],".local/trace-test-b");
        require(a.rfind("ms=100 blocks=",0)==0,"The first trace line must describe the first 100 ms");
        require(std::count(a.begin(),a.end(),'\n')==30,"3,000 ms must produce one line per 100 ms");
        require(a==b,"Two deterministic runs must produce identical traces");
        auto s=boot(argv[1],".local/trace-test-a");
        auto original=ldm::state_hash(*s);
        s->memory[0x12345]^=1;
        require(ldm::state_hash(*s)!=original,"A changed memory byte must change the hash");
        s->memory[0x12345]^=1;s->ip^=1;
        require(ldm::state_hash(*s)!=original,"A changed register must change the hash");
        s->ip^=1;s->palette[7]^=1;
        require(ldm::state_hash(*s)!=original,"A changed palette entry must change the hash");
        std::cout<<"PASS: deterministic sessions reproduce identical traces; the hash covers registers, memory and palette\n";
    }catch(const std::exception& e){std::cerr<<"FAIL: "<<e.what()<<'\n';return 1;}
}

#include "legacy.h"
#include <iostream>
#include <memory>
#include <stdexcept>

namespace {
void require(bool ok,const char* message){if(!ok)throw std::runtime_error(message);}
}
int main() {
    try {
        auto s=std::make_unique<ldm::State>();
        auto call=[&](uint16_t ax){s->ax=ax;s->interrupt(0x21);};
        call(0x2a00);
        require(s->cx==2026 && s->dx==0x0101 && uint8_t(s->ax)==4,"The default date must be Thursday 1 January 2026");
        call(0x2c00);
        require(s->cx==0x0c00 && s->dx==0,"The default time must be 12:00:00.00");
        s->emulated_ms=3723450;
        call(0x2c00);
        require(s->cx==0x0d02 && s->dx==0x032d,"Emulated time must advance the clock to 13:02:03.45");
        s->emulated_ms=0;s->clock_base=1803859200;
        call(0x2a00);
        require(s->cx==2027 && s->dx==0x0301 && uint8_t(s->ax)==1,"1 March 2027 must be a Monday");
        s->clock_base=1835395200;
        call(0x2a00);
        require(s->cx==2028 && s->dx==0x021d && uint8_t(s->ax)==2,"29 February 2028 must be a Tuesday");
        std::cout<<"PASS: DOS date and time derive from the clock base and emulated time, including leap days\n";
    }catch(const std::exception& e){std::cerr<<"FAIL: "<<e.what()<<'\n';return 1;}
}

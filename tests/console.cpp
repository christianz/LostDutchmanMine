#include "legacy.h"
#include "image_info.h"
#include <iostream>
#include <memory>
#include <stdexcept>

void require(bool ok,const char* message){if(!ok)throw std::runtime_error(message);}
int main() {
    try {
        auto s=std::make_unique<ldm::State>();
        s->load("recovered/load-image.bin",image_relocations,entry_cs,entry_ip,stack_ss,stack_sp);
        s->ds=s->ss=0x82bd;s->sp=0x8000;
        s->w16(s->ds,0x1f4c,0xffff); // Original C getch buffer is empty.
        s->w16(s->ds,0x20a6,0); // No optional C-runtime input hook.
        auto call=[&](uint16_t ip) {
            s->push(0xffff);s->push(0xfffe);
            s->cs=ldm::LoadSegment+0x13b4;s->ip=ip;
            auto start=s->boundaries;
            while(!(s->cs==0xffff && s->ip==0xfffe)) {
                require(s->boundaries-start<1000,"Original console helper did not return");
                ldm::native_step(*s);
                require(!s->waiting,"Console status must never wait for a key");
            }
            require(s->sp==0x8000,"Console helper corrupted stack");
            return s->ax;
        };
        require(call(0x1872)==0,"Empty keyboard must report no input");
        s->keys={0x1e61,0x011b};auto pending=s->keys;
        require(call(0x1872)==0xff && call(0x1872)==0xff,"Pending input must stay available across repeated status checks");
        require(s->keys==pending,"Status check consumed a key");
        require(call(0x1898)=='a' && s->keys.size()==1,"Original getch lost the queued character");
        require(call(0x1872)==0xff && call(0x1898)==0x1b,"Escape must remain readable after the status check");
        require(call(0x1872)==0 && s->keys.empty(),"Status did not return to empty after reading input");
        s->keys={0x4800};pending=s->keys;
        require(call(0x1872)==0xff && s->keys==pending,"A non-character key must still report pending input");
        s->keys.clear();s->w16(s->ds,0x1f4c,'z');
        require(call(0x1872)==0xff && call(0x1898)=='z',"Original C-runtime buffered input changed");
        require(call(0x1872)==0,"C-runtime buffered character did not clear");
        std::cout<<"PASS: original kbhit/getch poll without waiting or consuming input, preserve Escape and buffered characters, and recognize non-character keys\n";
    }catch(const std::exception& e){std::cerr<<e.what()<<"\n";return 1;}
}

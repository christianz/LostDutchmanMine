#include "legacy.h"
#include "image_info.h"
#include <iostream>
#include <memory>
#include <stdexcept>

// The original graphics shutdown callback is invoked indirectly on Quit Game.
// Exercise its original near-return path and register/stack preservation.
int main() {
    try {
      for(auto flags:{0,8}) {
        auto s=std::make_unique<ldm::State>();
        s->load("recovered/load-image.bin",image_relocations,entry_cs,entry_ip,stack_ss,stack_sp);
        s->ds=s->ss=0x82bd;s->sp=0x8000;
        s->bp=0x1234;s->si=0x5678;s->di=0x9abc;
        s->video_mode=0x13;
        s->w8(s->ds,0x2041,3); // Original desktop text mode.
        s->w8(s->ds,0x2044,0); // No extended text font to restore.
        s->w8(s->ds,0x380a,flags); // VGA restores text scan lines before mode 3.
        s->w16(s->ds,0x36fc,0x1903);
        s->push(0xfffe); // Near return sentinel.
        s->cs=ldm::LoadSegment+0x1613;s->ip=0x1c7d;
        while(s->ip!=0xfffe) {
            if(s->boundaries>10000)throw std::runtime_error("Graphics shutdown did not return");
            ldm::native_step(*s);
        }
        if(s->cs!=ldm::LoadSegment+0x1613 || s->sp!=0x8000 ||
           s->bp!=0x1234 || s->si!=0x5678 || s->di!=0x9abc)
            throw std::runtime_error("Graphics shutdown corrupted registers or stack");
        if(s->video_mode!=3 || s->u8(0x40,0x49)!=3 || s->text_scan_lines!=400 || s->u16(0x40,0x85)!=16)
            throw std::runtime_error("Graphics shutdown did not restore the original video mode");
      }
        std::cout<<"PASS: original Quit Game graphics cleanup restores video mode and returns safely\n";
    }catch(const std::exception& e){std::cerr<<e.what()<<"\n";return 1;}
}

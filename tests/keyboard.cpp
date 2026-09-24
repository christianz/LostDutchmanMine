#define SDL_MAIN_HANDLED
#include "keyboard.h"
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
        s->w16(s->ds,0xa6c,0); // The original helper reads our held-direction byte.
        s->w16(s->ds,0x3118,ldm::LoadSegment+0x72ba);
        auto read=[&](uint32_t event,bool movement,uint8_t held=0) {
            s->keys={event};s->set_movement(held);s->w16(s->ds,0x5a1a,0);
            if(movement)s->push(1);
            s->push(0xffff);s->push(0xfffe);
            s->cs=ldm::LoadSegment+0x0fa7;s->ip=movement?0x0066:0x003a;
            auto start=s->boundaries;
            while(!(s->cs==0xffff && s->ip==0xfffe)) {
                require(s->boundaries-start<2000,"Original input helper failed to return");
                ldm::native_step(*s);
            }
            if(movement)s->sp+=2;
            require(s->sp==0x8000,"Keyboard read corrupted stack");
            require(!s->movement_key_read,"Movement aliases leaked into text input");
            return s->ax;
        };
        SDL_KeyboardEvent key{};
        const SDL_Scancode wasd[]={SDL_SCANCODE_W,SDL_SCANCODE_A,SDL_SCANCODE_S,SDL_SCANCODE_D};
        const char letters[]="wasd";const int directions[]={1,4,2,8};
        for(int i=0;i<4;i++)for(int mod:{0,int(KMOD_LSHIFT),int(KMOD_CAPS),int(KMOD_LSHIFT|KMOD_CAPS)}) {
            key.keysym.scancode=wasd[i];key.keysym.sym=letters[i];key.keysym.mod=mod;
            auto event=ldm::input::key_event(key);
            require(read(event,true)==directions[i],"WASD tap did not reach the original movement handler");
            auto text=read(event,false);
            char expected=bool(mod&KMOD_SHIFT)!=bool(mod&KMOD_CAPS)?letters[i]-32:letters[i];
            require(uint8_t(text)==expected,"WASD letters or capitalization lost in text input");
        }
        const int keypad_directions[]={6,2,10,4,0,8,5,1,9,0x80};
        for(int i=0;i<10;i++)for(int mod:{0,int(KMOD_NUM)}) {
            key.keysym.scancode=SDL_Scancode(SDL_SCANCODE_KP_1+i);
            key.keysym.sym=SDLK_UNKNOWN;key.keysym.mod=mod;
            auto event=ldm::input::key_event(key);
            require(read(event,true)==keypad_directions[i],"Numpad movement changed with Num Lock");
            if(mod) {
                int digit=(i+1)%10;
                require(read(event,false)==uint16_t(((digit?digit+1:0xb)<<8)|('0'+digit)),"Numeric keypad must select save slots and enter digits");
            }
        }
        key.keysym.scancode=SDL_SCANCODE_KP_ENTER;
        require(read(ldm::input::key_event(key),false)==0x1c0d,"Numpad Enter did not confirm text input");
        std::array<bool,SDL_NUM_SCANCODES> held{};
        held[SDL_SCANCODE_W]=held[SDL_SCANCODE_D]=true;
        require(ldm::input::movement(held)==9,"W+D must produce a diagonal");
        held[SDL_SCANCODE_S]=true;
        require(ldm::input::movement(held)==8,"Opposing vertical keys must cancel independently");
        held[SDL_SCANCODE_KP_6]=true;held[SDL_SCANCODE_D]=false;
        require(ldm::input::movement(held)==8,"Releasing one alias must preserve another held key");
        // The original reader starts with the held mask, then used to replace
        // it with the last single key's scan, including OS auto-repeat.
        for(int vertical:{0,2})for(int horizontal:{1,3})for(int last:{vertical,horizontal})for(int repeat:{0,1}) {
            held.fill(false);held[wasd[vertical]]=held[wasd[horizontal]]=true;
            key.keysym.scancode=wasd[last];key.keysym.sym=letters[last];key.keysym.mod=0;key.repeat=repeat;
            auto event=ldm::input::key_event(key);auto mask=ldm::input::movement(held);
            require(read(event,true,mask)==mask,"An individual key event overrides a held diagonal");
            require(uint8_t(read(event,false,mask))==letters[last],"Held combinations leaked into text entry");
        }
        key.keysym.scancode=SDL_SCANCODE_SPACE;key.keysym.sym=SDLK_SPACE;key.repeat=0;
        require(read(ldm::input::key_event(key),true,5)==0x80,"Held movement swallowed Space");
        key.keysym.scancode=SDL_SCANCODE_F2;key.keysym.sym=SDLK_F2;
        require(read(ldm::input::key_event(key),true,5)==0 && s->u16(s->ds,0xa6a)==0x3c,"Held movement swallowed a status-menu key");
        std::cout<<"PASS: original movement/text helpers preserve WASD taps and typing, Num Lock on/off, eight keypad directions, numeric slot input and Enter; diagonals/opposing aliases pass\n";
    }catch(const std::exception& e){std::cerr<<e.what()<<"\n";return 1;}
}

#pragma once
#include "keyboard_event.h"
#include <SDL.h>
#include <array>

namespace ldm::input {
inline uint8_t direction(SDL_Scancode key) {
    switch(key) {
    case SDL_SCANCODE_UP:case SDL_SCANCODE_KP_8:case SDL_SCANCODE_W:return 1;
    case SDL_SCANCODE_DOWN:case SDL_SCANCODE_KP_2:case SDL_SCANCODE_S:return 2;
    case SDL_SCANCODE_LEFT:case SDL_SCANCODE_KP_4:case SDL_SCANCODE_A:return 4;
    case SDL_SCANCODE_RIGHT:case SDL_SCANCODE_KP_6:case SDL_SCANCODE_D:return 8;
    case SDL_SCANCODE_HOME:case SDL_SCANCODE_KP_7:return 5;
    case SDL_SCANCODE_PAGEUP:case SDL_SCANCODE_KP_9:return 9;
    case SDL_SCANCODE_END:case SDL_SCANCODE_KP_1:return 6;
    case SDL_SCANCODE_PAGEDOWN:case SDL_SCANCODE_KP_3:return 10;
    default:return 0;
    }
}
inline uint8_t movement(const std::array<bool,SDL_NUM_SCANCODES>& held) {
    uint8_t mask=0;
    for(unsigned i=0;i<held.size();i++)if(held[i])mask|=direction(SDL_Scancode(i));
    if((mask&3)==3)mask&=~3; // Opposite directions cancel, independently per axis.
    if((mask&12)==12)mask&=~12;
    return mask;
}
inline uint16_t keycode(const SDL_KeyboardEvent& e) {
    // Physical keypad identities are stable across Num Lock and host keymaps.
    // Numeric input uses the top-row BIOS scan too: the original save selector
    // examines scan codes rather than ASCII when selecting a numbered slot.
    auto physical=e.keysym.scancode;
    if(physical>=SDL_SCANCODE_KP_1 && physical<=SDL_SCANCODE_KP_0) {
        int digit=physical==SDL_SCANCODE_KP_0?0:physical-SDL_SCANCODE_KP_1+1;
        if(bool(e.keysym.mod&KMOD_NUM)!=bool(e.keysym.mod&KMOD_SHIFT))
            return uint16_t(((digit?digit+1:0x0b)<<8)|('0'+digit));
        const uint16_t scans[]={0x52,0x4f,0x50,0x51,0x4b,0x4c,0x4d,0x47,0x48,0x49};
        return scans[digit]<<8;
    }
    if(physical==SDL_SCANCODE_KP_ENTER)return 0x1c0d;
    uint16_t scan=0;
    switch(e.keysym.sym) {
    case SDLK_ESCAPE: return 0x011b;
    case SDLK_RETURN:case SDLK_KP_ENTER:return 0x1c0d;
    case SDLK_BACKSPACE:return 0x0e08;
    case SDLK_TAB:return 0x0f09;
    case SDLK_SPACE:return 0x3920;
    case SDLK_UP:scan=0x48;break;
    case SDLK_DOWN:scan=0x50;break;
    case SDLK_LEFT:scan=0x4b;break;
    case SDLK_RIGHT:scan=0x4d;break;
    case SDLK_HOME:case SDLK_KP_7:scan=0x47;break;
    case SDLK_PAGEUP:case SDLK_KP_9:scan=0x49;break;
    case SDLK_END:case SDLK_KP_1:scan=0x4f;break;
    case SDLK_PAGEDOWN:case SDLK_KP_3:scan=0x51;break;
    case SDLK_INSERT:case SDLK_KP_0:scan=0x52;break;
    case SDLK_KP_8:scan=0x48;break;
    case SDLK_KP_2:scan=0x50;break;
    case SDLK_KP_4:scan=0x4b;break;
    case SDLK_KP_6:scan=0x4d;break;
    case SDLK_F1:scan=0x3b;break;
    case SDLK_F2:scan=0x3c;break;
    case SDLK_F3:scan=0x3d;break;
    case SDLK_F4:scan=0x3e;break;
    case SDLK_F5:scan=0x3f;break;
    case SDLK_F6:scan=0x40;break;
    case SDLK_F7:scan=0x41;break;
    case SDLK_F8:scan=0x42;break;
    case SDLK_F9:scan=0x43;break;
    case SDLK_F10:scan=0x44;break;
    default:
        if(e.keysym.sym>=32 && e.keysym.sym<=126) {
            int c=e.keysym.sym;
            const unsigned letter_scan[]={0x1e,0x30,0x2e,0x20,0x12,0x21,0x22,0x23,0x17,0x24,0x25,0x26,0x32,0x31,0x18,0x19,0x10,0x13,0x1f,0x14,0x16,0x2f,0x11,0x2d,0x15,0x2c};
            if(c>='a'&&c<='z') {
                scan=letter_scan[c-'a'];
                if(bool(e.keysym.mod&KMOD_SHIFT)!=bool(e.keysym.mod&KMOD_CAPS))c-=32;
            } else if(c>='1'&&c<='9') {
                scan=c-'1'+2;if(e.keysym.mod&KMOD_SHIFT)c="!@#$%^&*("[c-'1'];
            } else if(c=='0'){scan=0x0b;if(e.keysym.mod&KMOD_SHIFT)c=')';}
            else {
                switch(c) {
                case '-':scan=0x0c;break;case '=':scan=0x0d;break;case '[':scan=0x1a;break;case ']':scan=0x1b;break;
                case ';':scan=0x27;break;case '\'':scan=0x28;break;case '`':scan=0x29;break;case '\\':scan=0x2b;break;
                case ',':scan=0x33;break;case '.':scan=0x34;break;case '/':scan=0x35;break;
                }
            }
            return (scan<<8)|uint8_t(c);
        }
    }
    return scan<<8;
}
inline uint32_t key_event(const SDL_KeyboardEvent& key) {
    auto scan=key.keysym.scancode;uint32_t move=0;
    switch(direction(scan)) {
    case 1:move=0x48;break;case 2:move=0x50;break;
    case 4:move=0x4b;break;case 8:move=0x4d;break;
    case 5:move=0x47;break;case 9:move=0x49;break;
    case 6:move=0x4f;break;case 10:move=0x51;break;
    }
    if(scan==SDL_SCANCODE_KP_0)move=0x52; // Original Insert/action key.
    return keycode(key)|(move<<KeyMovementShift)|
        (move?(uint32_t(scan)<<KeySourceShift):0)|
        (key.repeat && direction(scan)?KeyRepeat:0);
}
inline SDL_Keycode menu_key(const SDL_KeyboardEvent& key) {
    switch(direction(key.keysym.scancode)) {
    case 1:return SDLK_UP;case 2:return SDLK_DOWN;
    case 4:return SDLK_LEFT;case 8:return SDLK_RIGHT;
    default:return key.keysym.sym;
    }
}
}

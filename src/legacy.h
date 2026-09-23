#pragma once
#include "audio.h"
#include "keyboard_event.h"
#include "mouse.h"
#include "panning.h"
#include <array>
#include <cstdint>
#include <cstdio>
#include <filesystem>
#include <map>
#include <stdexcept>
#include <string>
#include <vector>

namespace ldm {
constexpr uint16_t LoadSegment = 0x1000;
constexpr uint16_t CF=1, PF=4, AF=16, ZF=64, SF=128, TF=256, IF=512, DF=1024, OF=2048;
enum class Op { Add, Adc, Sub, Sbb, And, Or, Xor, Inc, Dec, Neg };
enum class Shift { Shl, Shr, Sar, Rol, Ror, Rcl, Rcr };

// Original 16-bit data representation, retained while the game is lifted to C++.
// There is no instruction decoder: all executable control flow is generated C++.
struct State {
    uint16_t ax=0,bx=0,cx=0,dx=0,si=0,di=0,bp=0,sp=0,cs=0,ds=0,es=0,ss=0,ip=0;
    uint16_t flags=IF|2;
    std::array<uint8_t, 1<<20> memory{};
    bool running=true,waiting=false;
    uint64_t boundaries=0;
    std::filesystem::path data_dir,save_dir;
    std::map<int,FILE*> files;
    uint16_t alloc_segment=0x9000;
    std::array<uint8_t,65536> ports{};
    Audio audio;
    std::array<uint32_t,256> palette{};
    std::array<uint8_t,16> attributes{};
    // Native event tags are interpreted only at the original movement reader;
    // text fields receive their ordinary BIOS character and scan pair.
    std::vector<uint32_t> keys;
    bool movement_key_read=false;
    bool qol_improvements=true;
    Panning panning;
    void pan_action();
    int video_mode=3,text_scan_lines=400;
    MouseInput mouse;
    int mouse_visibility=-1,mouse_hot_x=0,mouse_hot_y=0;
    bool custom_cursor=false;
    std::array<uint16_t,32> mouse_mask{};
    uint32_t ticks=0;
    uint16_t pit_divisor=0,pit_partial=0;
    unsigned pit_write_phase=0;
    uint16_t speaker_divisor=0,speaker_partial=0;
    unsigned speaker_write_phase=0;
    bool timer_active=false;
    void timer_interrupt();
    // Original joystick state, read once per game movement tick. Native held
    // directions use that existing path instead of desktop keyboard auto-repeat.
    void set_movement(uint8_t directions) { w8(LoadSegment+0x72ba,1,directions); }
    uint8_t u8(uint16_t seg, uint16_t off) const { return memory[((uint32_t(seg)<<4)+off)&0xfffff]; }
    uint16_t u16(uint16_t seg, uint16_t off) const { return u8(seg,off)|(uint16_t(u8(seg,uint16_t(off+1)))<<8); }
    void w8(uint16_t seg,uint16_t off,uint8_t v) { memory[((uint32_t(seg)<<4)+off)&0xfffff]=v; }
    void w16(uint16_t seg,uint16_t off,uint16_t v) { w8(seg,off,v); w8(seg,uint16_t(off+1),v>>8); }
    void push(uint16_t v) { sp-=2; w16(ss,sp,v); }
    uint16_t pop() { auto v=u16(ss,sp); sp+=2; return v; }
    bool flag(uint16_t f) const { return (flags&f)!=0; }
    void set_flag(uint16_t f,bool v) { flags=(flags&~f)|(v?f:0); }
    void common_flags(uint32_t v,int bits) {
        uint32_t mask=(1u<<bits)-1; v&=mask;
        set_flag(ZF,v==0); set_flag(SF,(v>>(bits-1))&1);
        uint8_t p=uint8_t(v); p^=p>>4; p&=15;
        set_flag(PF,((0x6996>>p)&1)==0);
    }
    uint16_t alu(Op op,uint16_t av,uint16_t bv,int bits) {
        uint32_t mask=(1u<<bits)-1,sign=1u<<(bits-1);
        uint32_t a=av&mask,b=bv&mask,r=0,c=flag(CF); bool carry=false;
        switch(op) {
        case Op::Add: case Op::Adc: case Op::Inc:
            r=a+b+(op==Op::Adc?c:0); carry=r>mask;
            set_flag(OF,(~(a^b)&(a^r)&sign)!=0); set_flag(AF,((a^b^r)&16)!=0); break;
        case Op::Sub: case Op::Sbb: case Op::Dec: case Op::Neg:
            r=a-b-(op==Op::Sbb?c:0); carry=a<b+(op==Op::Sbb?c:0);
            set_flag(OF,((a^b)&(a^r)&sign)!=0); set_flag(AF,((a^b^r)&16)!=0); break;
        case Op::And: r=a&b; set_flag(OF,false); break;
        case Op::Or: r=a|b; set_flag(OF,false); break;
        case Op::Xor: r=a^b; set_flag(OF,false); break;
        }
        if(op!=Op::Inc && op!=Op::Dec) set_flag(CF,carry);
        common_flags(r,bits); return r&mask;
    }
    uint16_t shift(Shift op,uint16_t a,unsigned n,int bits) {
        uint32_t mask=(1u<<bits)-1, sign=1u<<(bits-1),r=a&mask;
        if(!n) return r;
        bool old_top=(r&sign)!=0;
        for(unsigned i=0;i<n;i++) {
            bool c=flag(CF),next=false;
            switch(op) {
            case Shift::Shl: next=r&sign; r=(r<<1)&mask; break;
            case Shift::Shr: next=r&1; r>>=1; break;
            case Shift::Sar: next=r&1; r=(r>>1)|(r&sign); break;
            case Shift::Rol: next=r&sign; r=((r<<1)|next)&mask; break;
            case Shift::Ror: next=r&1; r=(r>>1)|(next?sign:0); break;
            case Shift::Rcl: next=r&sign; r=((r<<1)|c)&mask; break;
            case Shift::Rcr: next=r&1; r=(r>>1)|(c?sign:0); break;
            }
            set_flag(CF,next);
        }
        if(op==Shift::Shl || op==Shift::Shr || op==Shift::Sar) common_flags(r,bits);
        if(n==1) {
            if(op==Shift::Shr) set_flag(OF,old_top);
            else if(op==Shift::Sar) set_flag(OF,false);
            else if(op==Shift::Ror || op==Shift::Rcr) set_flag(OF,((r>>(bits-1))^(r>>(bits-2)))&1);
            else set_flag(OF,bool(r&sign)!=flag(CF));
        }
        return r;
    }
    void multiply(uint16_t value,int bits,bool signed_op);
    void divide(uint16_t value,int bits,bool signed_op);
    void interrupt(uint8_t number);
    void out(uint16_t port,uint16_t value,int bits);
    uint16_t in(uint16_t port,int bits);
    void string_op(const char* op,int bytes,int rep,uint16_t source_segment);
    void fail(const std::string& message) const;
    void load(const std::filesystem::path& image,const std::vector<uint32_t>& reloc,uint16_t initial_cs,uint16_t initial_ip,uint16_t initial_ss,uint16_t initial_sp);
    std::string string_at(uint16_t seg,uint16_t off,char end=0) const;
    std::filesystem::path file_path(const std::string& name,bool write) const;
};
void native_step(State& s);
}

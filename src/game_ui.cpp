#include "game_ui.h"
#include "assets.h"
#include "legacy.h"
#include "pixel_art.h"
#include <algorithm>
#include <fstream>

namespace ldm {
namespace {
constexpr uint32_t ink=0xff000000,cream=0xffeee2bb,gold=0xffffd34e;
// Preserve the corner bevels when resizing an original metal button.
int slice(int at,int size,int source,int edge) {
    if(at<edge)return at;
    if(at>=size-edge)return source-(size-at);
    return edge+(at-edge)*(source-2*edge)/(size-2*edge);
}
void outline(Pixels& p,GameButton r,uint32_t c) {
    pixel::line(p,r.x+2,r.y,r.x+r.w-3,r.y,c);
    pixel::line(p,r.x,r.y+2,r.x,r.y+r.h-3,c);
    pixel::line(p,r.x+2,r.y+r.h-1,r.x+r.w-3,r.y+r.h-1,c);
    pixel::line(p,r.x+r.w-1,r.y+2,r.x+r.w-1,r.y+r.h-3,c);
}
}
void GameUI::prepare(const State& s) {
    if(ready_ || s.video_mode!=0x13)return;
    std::ifstream in(s.file_path("LDMG/PANL_VGA.ZZZ",false),std::ios::binary);
    if(!in)throw std::runtime_error("Cannot read original VGA panel artwork");
    auto data=decode_asset(std::vector<uint8_t>{std::istreambuf_iterator<char>(in),{}});
    if(data.size()<atlas_.size())throw std::runtime_error("Original VGA panel artwork is incomplete");
    // This VGA atlas still uses the game's 16-colour drawing path. The original
    // blitter masks each source byte to its low nibble (upper bits are not
    // palette indices). Treating these as 256-colour pixels corrupts the skin.
    std::transform(data.begin(),data.begin()+atlas_.size(),atlas_.begin(),[](uint8_t c){return c&15;});ready_=true;
}
void GameUI::context(const State& s) {
    context_buttons=0;
    for(int i=0;i<4;i++)if(s.u8(s.ds,s.u16(s.ss,uint16_t(s.sp+4+i*2))))context_buttons|=1u<<i;
}
void GameUI::map_click(State& s) const {
    if(!s.qol_improvements || s.video_mode!=0x13)return;
    int x=s.u16(s.ss,uint16_t(s.sp+4)),y=s.u16(s.ss,uint16_t(s.sp+6));
    // Only the world's own selector calls this hook, after its release/debounce
    // check. Inventory, poker, save names and other dialogs keep their readers.
    for(int i=0;i<6;i++)if(toolbar(i).contains(x,y)) {
        s.w16(s.ss,uint16_t(s.sp+4),69+i*42);s.w16(s.ss,uint16_t(s.sp+6),181);return;
    }
    for(int i=0;i<4;i++)if((context_buttons&(1u<<i)) && action(i).contains(x,y)) {
        s.w16(s.ss,uint16_t(s.sp+4),i<2?108:188);
        s.w16(s.ss,uint16_t(s.sp+6),i%2?147:127);return;
    }
}
bool GameUI::visible(const State& s) const {
    if(!ready_ || s.video_mode!=0x13)return false;
    // The original modal screens can replace all or part of the panel. Match
    // its six button borders before painting, including during screen changes.
    for(int i=0;i<6;i++)for(int y:{167,180,191})for(int x:{54,84}) {
        int at=y*320+x+i*42;
        if(s.memory[0xa0000+at]!=atlas_[at])return false;
    }
    return true;
}
bool GameUI::pointer_visible(const State& s) const {
    return s.mouse_visibility>=0 || (s.qol_improvements && !s.combat_active &&
        !s.panning_active && !s.desert_view_active && visible(s));
}
void GameUI::text(Pixels& p,int x,int y,const std::string& value,uint32_t c,bool centre) const {
    if(!font['A'*8+1]){pixel::text(p,x,y,value,c,centre);return;}
    auto span=[&](unsigned char ch) {
        uint8_t bits=0;for(int row=0;row<8;row++)bits|=font[ch*8+row];
        int first=0,last=7;
        while(first<8 && !(bits&(128>>first)))++first;
        while(last>=first && !(bits&(128>>last)))--last;
        return std::pair<int,int>{first,last};
    };
    int width=0;
    for(unsigned char ch:value){auto [first,last]=span(ch);width+=first==8?4:last-first+2;}
    if(centre)x-=(width-1)/2;
    for(unsigned char ch:value) {
        auto [first,last]=span(ch);
        for(int row=0;row<8;row++)for(int col=first;col<=last;col++)
            if(font[ch*8+row]&(128>>col))pixel::dot(p,x+col-first,y+row,c);
        x+=first==8?4:last-first+2;
    }
}
void GameUI::button(Pixels& p,const std::array<uint32_t,256>& pal,GameButton r,
                    const std::string& label,bool hover,bool pressed,bool enabled) const {
    for(int y=0;y<r.h;y++)for(int x=0;x<r.w;x++) {
        // PANL_VGA's original blank 75x19 action button, used by Fish/Water/Pan.
        int sx=slice(x,r.w,75,6),sy=slice(y,r.h,19,4);
        uint32_t c=ready_?pal[atlas_[(64+sy)*320+181+sx]]:0xffaaaaaa;
        if(!enabled)c=0xff000000|((c&0xfefefe)>>1);
        pixel::dot(p,r.x+x,r.y+y,c);
    }
    if(hover && enabled)outline(p,r,pressed?cream:gold);
    if(pressed && enabled) {
        pixel::line(p,r.x+3,r.y+1,r.x+r.w-4,r.y+1,0xff555555);
        pixel::line(p,r.x+1,r.y+3,r.x+1,r.y+r.h-4,0xff555555);
    }
    text(p,r.x+r.w/2+(pressed?1:0),r.y+(r.h-8)/2+(pressed?1:0),label,ink,true);
}
void GameUI::draw(Pixels& p,const State& s) const {
    if(!s.qol_improvements || !visible(s))return;
    const char* names[]={"Cash","Life","Food","Tools","Ammo","Game"};
    const char* hints[]={"Cash - F1","Health - F2","Food / Drink - F3","Tools / Pack - F4","Ammunition - F5","Save / Load / Quit - F6"};
    auto mouse=s.mouse.current();bool interactive=pointer_visible(s);
    for(int i=0;i<6;i++) {
        auto r=toolbar(i);bool hover=interactive && r.contains(mouse.x,mouse.y),pressed=hover && (mouse.buttons&1);
        button(p,s.palette,r,"",hover,pressed);
        // Read the live original framebuffer: the health and food icons change
        // with the player's condition, and critical health flashes. The atlas
        // only contains their healthy defaults. Reduce enough to fit a label.
        for(int y=0;y<18;y++)for(int x=0;x<24;x++)
            pixel::dot(p,r.x+8+x+(pressed?1:0),r.y+3+y+(pressed?1:0),
                s.palette[s.memory[0xa0000+(166+y*26/18)*320+56+i*42+x*28/24]]);
        text(p,r.x+20,r.y+22+(pressed?1:0),names[i],ink,true);
        if(hover) {
            // The top strip belongs to original status messages. A hint can use
            // it only while it is entirely blank; it never covers game text.
            bool blank=std::all_of(p.begin(),p.begin()+320*10,[](uint32_t c){return (c&0xffffff)==0;});
            if(blank)text(p,160,1,hints[i],cream,true);
        }
    }
    for(int i=0;i<4;i++)if(interactive && (context_buttons&(1u<<i)) && action(i).contains(mouse.x,mouse.y))
        outline(p,action(i),(mouse.buttons&1)?cream:gold);
    if(mule_shop_visible)for(int i=0;i<3;i++)if(!s.mule_available(i)) {
        pixel::rect(p,26+i*100,89,68,22,s.palette[0]);
        text(p,60+i*100,96,"SOLD OUT",cream,true);
    }
}
}

#include "panning.h"
#include "pixel_art.h"
#include <algorithm>
#include <cmath>
#include <fstream>
#include <stdexcept>

namespace ldm {
namespace {
constexpr uint32_t black=0xff080808,cream=0xffeee2bb,ochre=0xffab7c2c,brown=0xff563909;
constexpr uint32_t yellow=0xffffd34e,light=0xffc4c4b8,steel=0xff747c7b,dark=0xff343c40;
int button_at(int x,int y) {
    if(y<174 || y>=194)return -1;
    if(x>=10 && x<68)return 0;
    if(x>=74 && x<132)return 1;
    if(x>=140 && x<230)return 2;
    if(x>=238 && x<310)return 3;
    return -1;
}
}
void Panning::load_art() {
    if(art_loaded_)return;
    std::ifstream in(art_path_,std::ios::binary);
    std::string magic;int width=0,height=0,max=0;char separator=0;
    in>>magic>>width>>height>>max;in.get(separator);
    if(!in || magic!="P6" || width!=320 || height!=112 || max!=255 || separator!='\n')
        throw std::runtime_error("Cannot load panning artwork: keep panning-creek.ppm beside the executable.");
    std::array<unsigned char,320*112*3> rgb{};
    if(!in.read(reinterpret_cast<char*>(rgb.data()),rgb.size()) || in.peek()!=std::char_traits<char>::eof())
        throw std::runtime_error("Invalid panning artwork");
    for(size_t i=0;i<creek_.size();i++)creek_[i]=0xff000000|(uint32_t(rgb[i*3])<<16)|(uint32_t(rgb[i*3+1])<<8)|rgb[i*3+2];
    art_loaded_=true;
}
void Panning::begin() {
    if(engaged())return;
    load_art();phase_=Phase::Rock;round_=0;loosened_=0;gold_=5;lost_last_=0;last_side_=0;
    tilt_=age_=rinse_time_=stroke_time_=0;success_=false;clear_input();
}
void Panning::clear_input(){dragging_=false;buttons_=0;}
void Panning::finish(bool keep){success_=keep && gold_>0;phase_=Phase::Done;clear_input();}
bool Panning::take_result() {
    if(!done())throw std::logic_error("Panning result is not ready");
    bool success=success_;phase_=Phase::Idle;clear_input();return success;
}
void Panning::wash() {
    if(phase_==Phase::Result){finish(true);return;}
    if(phase_!=Phase::Rock)return;
    // Washing unseparated gravel carries heavy flakes over the lip too.
    lost_last_=std::min(gold_,loosened_<40?2:loosened_<80?1:0);
    gold_-=lost_last_;rinse_time_=0;phase_=Phase::Rinse;dragging_=false;
}
void Panning::key(uint16_t bios,bool repeat) {
    if(!active() || repeat)return;
    auto ascii=uint8_t(bios),scan=uint8_t(bios>>8);
    if(ascii==27 || scan==1)finish(phase_==Phase::Result);
    else if(ascii==' ' || ascii==13)wash();
}
void Panning::pointer(int x,int y){mouse_x_=std::clamp(x,0,319);mouse_y_=std::clamp(y,0,199);}
void Panning::buttons(int mask) {
    bool press=(mask&1) && !(buttons_&1);buttons_=mask;
    if(!(mask&1))dragging_=false;
    if(!press || !active())return;
    if(phase_==Phase::Result) {
        if(mouse_x_>=100 && mouse_x_<220 && mouse_y_>=174 && mouse_y_<194)finish(true);
        return;
    }
    int button=button_at(mouse_x_,mouse_y_);
    if(button==3)finish(phase_==Phase::Result);
    else if(button==2)wash();
    else if(phase_==Phase::Rock && mouse_x_>=65 && mouse_x_<=255 && mouse_y_>=25 && mouse_y_<=112) {
        dragging_=true;drag_origin_=mouse_x_;drag_tilt_=tilt_;
    }
}
void Panning::update(double seconds,uint8_t directions) {
    if(!active())return;
    double dt=std::clamp(seconds,0.0,.05);age_+=dt;stroke_time_+=dt;
    double target=0;
    if(phase_==Phase::Rock) {
        bool left=directions&4,right=directions&8;
        int button=button_at(mouse_x_,mouse_y_);
        if(buttons_&1){left|=button==0;right|=button==1;}
        if(left!=right)target=left?-1:1;
        else if(dragging_)target=std::clamp(drag_tilt_+(mouse_x_-drag_origin_)/40.0,-1.0,1.0);
    }
    tilt_+=std::clamp(target-tilt_,-dt*2.8,dt*2.8);
    if(phase_==Phase::Rock) {
        int side=tilt_<-.6?-1:tilt_>.6?1:0;
        // Count actual full swings, never raw key repeats or mouse-event count.
        if(side && side!=last_side_ && stroke_time_>=.3) {
            loosened_=std::min(100,loosened_+20);last_side_=side;stroke_time_=0;
        }
    }else if(phase_==Phase::Rinse) {
        rinse_time_+=dt;
        if(rinse_time_>=1.6) {
            ++round_;loosened_=0;last_side_=0;stroke_time_=0;
            phase_=round_==3?Phase::Result:Phase::Rock;
        }
    }
}
void Panning::draw(Pixels& p) const {
    using namespace pixel;
    if(!active())return;
    std::copy(creek_.begin(),creek_.end(),p.begin());
    rect(p,0,0,320,14,black);text(p,160,4,"PAN FOR GOLD",cream,true);
    // Only the pan moves. The riverbank and camera remain still.
    int cx=160+int(tilt_*7),cy=65;double lean=tilt_;
    ellipse(p,cx,cy+12,88,31,black,lean);
    ellipse(p,cx,cy+10,86,31,dark,lean);
    ellipse(p,cx,cy+7,86,31,steel,lean);
    ellipse(p,cx,cy+2,87,31,black,lean);
    ellipse(p,cx,cy,86,30,light,lean);
    ellipse(p,cx,cy,82,27,steel,lean);
    ellipse(p,cx,cy,77,24,dark,lean);
    ellipse(p,cx,cy+1,73,22,0xff8c8870,lean);
    ellipse(p,cx,cy+3,68,19,0xff777458,lean);
    // Riveted rim, wear and riffles: whole source pixels with flat palette shades.
    for(int x=-60;x<=60;x+=20)dot(p,cx+x,cy-21+std::abs(x)/10+int(x*lean*.09),cream);
    for(int y=0;y<3;y++) {
        line(p,cx-66,cy-8+y*6-int(lean*6),cx-50,cy-11+y*6-int(lean*4),light);
        line(p,cx+50,cy-11+y*6+int(lean*4),cx+66,cy-8+y*6+int(lean*6),light);
    }
    double washed=round_+(phase_==Phase::Rinse?std::min(1.0,rinse_time_/1.6):0);
    int grains=int(94*(3-washed)/3);
    uint32_t rng=0x1989u;
    for(int i=0;i<94;i++) {
        rng=rng*1664525u+1013904223u;int x=int((rng>>8)%119)-59;
        rng=rng*1664525u+1013904223u;int y=int((rng>>8)%33)-16;
        if(i>=grains || x*x*256+y*y*3481>3481*256)continue;
        int dx=cx+x+int(tilt_*(2+i%4)),dy=cy+y+int(x*lean*.09);
        uint32_t shades[]={0xff403526,0xffb3a179,0xffd2bc8c,0xff786349};
        rect(p,dx,dy,2+(i%4==0),1+(i%3==0),shades[i%4]);
        if(i%5==0)dot(p,dx,dy-1,cream);
    }
    for(int i=0;i<gold_;i++)if(washed>.5 || i<2 || loosened_>=80) {
        int x=cx-22+i*11+int(tilt_*2),y=cy+5+(i%2)*5+int((i*11-22)*lean*.09);
        rect(p,x-1,y,4,3,0xff996315);rect(p,x,y,3,2,yellow);dot(p,x,y,0xffffed9c);
    }
    if(phase_==Phase::Rinse) {
        int wave=int(rinse_time_*27)%22;
        for(int y=-16;y<17;y+=6)for(int x=-55;x<56;x++)
            if(x*x*256+y*y*3481<3481*256 && (x+wave)%19<12)dot(p,cx+x,cy+y+int(x*lean*.09),0xff6cbbcf);
        for(int i=0;i<7;i++) {
            int y=cy+25+(int(rinse_time_*32)+i*5)%16,x=cx+35+i*4;
            line(p,x,y,x+1,y+2,0xff8cd4df);
        }
    }
    // The prospector's red cuffs and weathered hands hold either edge.
    for(int side:{-1,1}) {
        int x=cx+side*84-(side<0?12:0),y=cy+13+int(side*lean*7);
        rect(p,x-1,y-2,15,17,black);rect(p,x,y+7,13,10,0xff941608);
        rect(p,x,y,13,10,0xffaa7545);rect(p,x+2,y-1,9,8,0xffd6ae79);
        for(int j=0;j<3;j++)line(p,x+3+j*3,y+1,x+3+j*3,y+5,0xffaa7545);
    }
    rect(p,0,112,320,88,black);rect(p,2,114,316,84,ochre);rect(p,5,117,310,78,brown);
    line(p,5,117,314,117,cream);line(p,5,117,5,194,cream);line(p,6,194,314,194,0xff382400);
    if(phase_==Phase::Result) {
        text(p,160,123,gold_?"PAY DIRT!":"NO GOLD THIS TIME.",gold_?yellow:cream,true);
        text(p,160,136,gold_?"A BAG OF GOLD FOR YOUR PACK.":"THE GOLD WASHED OVER THE RIM.",cream,true);
        text(p,160,150,gold_==5?"A STEADY HAND, PROSPECTOR.":gold_?"NEXT TIME, ROCK BEFORE WASHING.":"ROCK UNTIL THE BAR TURNS GOLD.",cream,true);
        text(p,160,162,"ENTER TO RETURN TO THE RIVER",light,true);
    }else {
        text(p,12,122,"WASH "+std::to_string(round_+1)+"/3",cream);
        text(p,237,122,"GOLD "+std::to_string(gold_)+"/5",yellow);
        text(p,12,135,"LOOSEN",cream);rect(p,56,133,251,10,black);
        rect(p,58,135,247*loosened_/100,6,loosened_>=80?yellow:ochre);
        std::string hint=phase_==Phase::Rinse?(lost_last_?"TOO SOON! SOME GOLD WASHED OUT.":"WASHING AWAY THE SAND..."):loosened_>=80?"READY TO WASH. KEEP THE GOLD!":"ROCK SIDE TO SIDE TO LOOSEN SAND.";
        text(p,160,148,hint,cream,true);
        text(p,160,160,"A/D OR DRAG PAN. SPACE TO WASH.",light,true);
    }
    const int xs[]={10,74,140,238},widths[]={58,58,90,72};
    const char* labels[]={"< LEFT","RIGHT >",phase_==Phase::Result?(gold_?"TAKE GOLD":"CONTINUE"):"WASH",phase_==Phase::Result?"RETURN":"LEAVE"};
    for(int i=0;i<4;i++) {
        if(phase_==Phase::Result) {
            if(i!=2)continue;
            rect(p,100,174,120,20,black);rect(p,101,175,118,18,ochre);
            line(p,101,175,218,175,cream);text(p,160,181,labels[i],cream,true);continue;
        }
        bool selected=button_at(mouse_x_,mouse_y_)==i,pressed=selected && (buttons_&1);
        rect(p,xs[i],174,widths[i],20,black);rect(p,xs[i]+1,175,widths[i]-2,18,pressed?brown:ochre);
        line(p,xs[i]+1,175,xs[i]+widths[i]-2,175,selected?yellow:cream);
        text(p,xs[i]+widths[i]/2,181+(pressed?1:0),labels[i],cream,true);
    }
    // A small outlined crosshair stays legible on the pan and the buttons.
    line(p,mouse_x_-3,mouse_y_,mouse_x_+3,mouse_y_,black);
    line(p,mouse_x_,mouse_y_-3,mouse_x_,mouse_y_+3,black);
    dot(p,mouse_x_-2,mouse_y_,cream);dot(p,mouse_x_+2,mouse_y_,cream);
    dot(p,mouse_x_,mouse_y_-2,cream);dot(p,mouse_x_,mouse_y_+2,cream);
}
}

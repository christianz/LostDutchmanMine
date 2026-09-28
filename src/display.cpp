#include "display.h"
#include <algorithm>
#include <cmath>
#include <fstream>
#include <unordered_map>
#include <sstream>
#include <stdexcept>
#ifdef _WIN32
#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>
#endif

namespace ldm {
namespace {
bool integer(const std::string& value,int& number) {
    std::istringstream in(value);if(!(in>>number))return false;in>>std::ws;
    return in.eof();
}
int cycle(int value,int count,int direction){return (value+count+direction)%count;}
const int fullscreen_sizes[]={100,85,70};
// The Display row offers the three windows, then fullscreen at each picture size.
int display_choice(const DisplaySettings& s) {
    if(s.window<3)return s.window;
    return s.size==85?4:s.size==70?5:3;
}
}
DisplaySettings load_settings(const std::filesystem::path& file) {
    DisplaySettings s;
    std::ifstream in(file);std::string line;
    while(std::getline(in,line)) {
        auto eq=line.find('=');if(eq==std::string::npos)continue;
        auto key=line.substr(0,eq),value=line.substr(eq+1);int n=0;
        if(!integer(value,n))continue;
        if(key=="window" && n>=0 && n<=3)s.window=n;
        else if(key=="size" && (n==70 || n==85 || n==100))s.size=n;
        else if(key=="scaling" && n>=0 && n<=2)s.scaling=Scaling(n);
        else if(key=="colour" && n>=0 && n<=3)s.colour=Colour(n);
        else if(key=="crt" && n>=0 && n<=2)s.crt=Crt(n);
        else if(key=="brightness" && n>=80 && n<=120 && n%10==0)s.brightness=n;
        else if(key=="startup" && (n==0 || n==1))s.startup=n;
        else if(key=="qol" && (n==0 || n==1))s.qol=n;
    }
    return s;
}
void save_settings(const std::filesystem::path& file,const DisplaySettings& s) {
    if(!file.parent_path().empty())std::filesystem::create_directories(file.parent_path());
    auto temp=file;temp+=".tmp";
    {
        std::ofstream out(temp,std::ios::trunc);
        out<<"# Lost Dutchman Mine display settings. F11 opens the settings window.\n"
           <<"window="<<s.window<<"\nsize="<<s.size<<"\nscaling="<<int(s.scaling)
           <<"\ncolour="<<int(s.colour)<<"\ncrt="<<int(s.crt)<<"\nbrightness="<<s.brightness
           <<"\nstartup="<<s.startup<<"\nqol="<<s.qol<<"\n";
        out.close();if(!out)throw std::runtime_error("Cannot save display settings to "+file.string());
    }
#ifdef _WIN32
    if(!MoveFileExW(temp.c_str(),file.c_str(),MOVEFILE_REPLACE_EXISTING|MOVEFILE_WRITE_THROUGH))
        throw std::runtime_error("Cannot replace display settings: "+file.string());
#else
    std::filesystem::rename(temp,file);
#endif
}
const char* menu_label(int item,bool startup) {
    const char* labels[]={"Display","Scaling","CRT monitor","Colour","Brightness","Show at startup","QoL improvements","Comfort","Original look"};
    if(item==Cancel)return startup?"Quit":"Cancel";
    if(item==Apply)return startup?"Play":"Apply & resume";
    return item>=0 && item<Cancel?labels[item]:"";
}
std::string menu_value(const DisplaySettings& s,int row) {
    const char* windows[]={"960 x 720 window","1280 x 960 window","1600 x 1200 window","Fullscreen","Fullscreen 85%","Fullscreen 70%"};
    const char* filters[]={"Crisp pixels","Soft pixels","Pixel art"};
    const char* crt[]={"Off","Subtle","Strong"};
    const char* colours[]={"Original","Warm","Vivid","Gentle"};
    switch(row) {
    case Display:return windows[display_choice(s)];
    case ScalingFilter:return filters[int(s.scaling)];
    case CrtMonitor:return crt[int(s.crt)];
    case ColourProfile:return colours[int(s.colour)];
    case Brightness:return std::to_string(s.brightness)+"%";
    case Startup:return s.startup?"Yes":"No";
    case QualityOfLife:return s.qol?"On":"Off";
    }
    return "";
}
std::array<const char*,2> menu_help(int item,bool startup) {
    switch(item) {
    case Display:return {"Fullscreen keeps the 4:3 shape at any resolution.","85% or 70% leaves a border on large monitors."};
    case ScalingFilter:return {"Soft blends edges. Pixel art rounds diagonals.","Crisp keeps the original hard pixel edges."};
    case CrtMonitor:return {"Scanlines, phosphor texture and a gentle glow.","Subtle is light. Strong gives a bolder effect."};
    case ColourProfile:return {"Adjust the colour of the original artwork.","Original keeps the game's palette unchanged."};
    case Brightness:return {"Adjust picture brightness to suit your room.","100% keeps the original brightness."};
    case Startup:return {"Choose whether this menu opens at launch.","You can always open it again with F11."};
    case QualityOfLife:return {"Toolbar labels, arrow pointer, mouse aiming,","faster walking; mules unlock one at a time."};
    case Comfort:return {"Fullscreen 85%, soft pixels, gentle colours,","CRT off and 100% brightness. Adjust to taste."};
    case Original:return {"Crisp pixels, original colours, CRT off,","in a 960 x 720 window with the 4:3 shape."};
    case Cancel:
        if(startup)return {"Quit closes the game without saving settings.","Your saved games are not affected."};
        return {"Cancel discards these changes and resumes.","F11 opens this menu again during play."};
    case Apply:
        if(startup)return {"Play saves these settings and starts the game.","F11 opens this menu again during play."};
        return {"Apply saves these settings and resumes play.","F11 opens this menu again during play."};
    }
    return {"",""};
}
const char* menu_keys(bool startup) {
    return startup?"Arrows adjust   Enter confirms   Esc quits":"Arrows adjust   Enter confirms   Esc cancels";
}
void change_setting(DisplaySettings& s,int row,int dir) {
    switch(row) {
    case Display:{int i=cycle(display_choice(s),6,dir);s.window=std::min(i,3);if(i>=3)s.size=fullscreen_sizes[i-3];break;}
    case ScalingFilter:s.scaling=Scaling(cycle(int(s.scaling),3,dir));break;
    case CrtMonitor:s.crt=Crt(cycle(int(s.crt),3,dir));break;
    case ColourProfile:s.colour=Colour(cycle(int(s.colour),4,dir));break;
    case Brightness:s.brightness=80+10*cycle((s.brightness-80)/10,5,dir);break;
    case Startup:s.startup=!s.startup;break;
    case QualityOfLife:s.qol=!s.qol;break;
    }
}
DisplaySettings comfort_settings(bool startup){DisplaySettings s;s.window=3;s.size=85;s.colour=Colour::Gentle;s.startup=startup;return s;}
DisplaySettings original_settings(bool startup){DisplaySettings s;s.window=0;s.scaling=Scaling::Crisp;s.startup=startup;return s;}
int picture_percent(const DisplaySettings& s){return s.window==3?s.size:100;}
Rect picture_rect(int width,int height,int percent) {
    if(width<=0 || height<=0)return {};
    // VGA's rectangular pixels are presented at their original 4:3 aspect.
    const double scale=std::min(width/4.0,height/3.0)*std::clamp(percent,1,100)/100.0;
    int w=std::max(1,int(std::floor(scale*4))),h=std::max(1,int(std::floor(scale*3)));
    return {(width-w)/2,(height-h)/2,w,h};
}
bool picture_point(const Rect& r,int px,int py,int& x,int& y) {
    if(r.w<=0 || r.h<=0)return false;
    x=std::clamp(int((int64_t(px)-r.x)*320/r.w),0,319);
    y=std::clamp(int((int64_t(py)-r.y)*200/r.h),0,199);
    return px>=r.x && py>=r.y && px<r.x+r.w && py<r.y+r.h;
}
uint32_t colour_pixel(uint32_t p,const DisplaySettings& s) {
    if(s.colour==Colour::Original && s.brightness==100)return p;
    double r=(p>>16)&255,g=(p>>8)&255,b=p&255;
    double grey=.2126*r+.7152*g+.0722*b;
    double saturation=1,contrast=1;
    switch(s.colour) {
    case Colour::Original:break;
    case Colour::Warm:r*=1.035;g*=.985;b*=.90;break;
    case Colour::Vivid:saturation=1.14;contrast=1.04;break;
    case Colour::Gentle:saturation=.82;contrast=.90;break;
    }
    auto channel=[&](double v) {
        v=grey+(v-grey)*saturation;v=(v-127.5)*contrast+127.5;
        return uint32_t(std::clamp(std::lround(v*s.brightness/100.0),0l,255l));
    };
    return (p&0xff000000)|(channel(r)<<16)|(channel(g)<<8)|channel(b);
}
void display_pixels(const Pixels& input,const DisplaySettings& s,std::vector<uint32_t>& out,int& w,int& h) {
    // Soft uses a 2x point enlargement followed by linear sampling in SDL. It
    // softens pixel edges without blurring each original pixel across its width.
    int factor=s.scaling==Scaling::Crisp?1:2;w=320*factor;h=200*factor;out.resize(w*h);
    auto index=[](int x,int y){return std::clamp(y,0,199)*320+std::clamp(x,0,319);};
    // Grade each source pixel once, not each enlarged subpixel. Palette lookup
    // avoids repeating floating-point colour math for the original 256 colours.
    std::vector<uint32_t> graded;
    const uint32_t* rgb=input.data();
    if(s.colour!=Colour::Original || s.brightness!=100) {
        std::unordered_map<uint32_t,uint32_t> colours;colours.reserve(512);
        graded.resize(input.size());
        for(size_t i=0;i<input.size();i++) {
            auto [it,inserted]=colours.emplace(input[i],0);
            if(inserted)it->second=colour_pixel(input[i],s);
            graded[i]=it->second;
        }
        rgb=graded.data();
    }
    for(int y=0;y<200;y++)for(int x=0;x<320;x++) {
        auto e=rgb[y*320+x];
        if(factor==1){out[y*w+x]=e;continue;}
        uint32_t a=e,b=e,c=e,d=e;
        if(s.scaling==Scaling::PixelArt) {
            // Scale2x neighbourhood rule, independently implemented from the
            // published algorithm: https://www.scale2x.it/algorithm
            int u=index(x,y-1),v=index(x,y+1),l=index(x-1,y),r=index(x+1,y);
            auto up=input[u],down=input[v],left=input[l],right=input[r];
            if(up!=down && left!=right) {
                if(left==up)a=rgb[l];
                if(up==right)b=rgb[r];
                if(left==down)c=rgb[l];
                if(down==right)d=rgb[r];
            }
        }
        int at=y*2*w+x*2;out[at]=a;out[at+1]=b;
        out[at+w]=c;out[at+w+1]=d;
    }
}
void crt_mask(Crt effect,int width,int height,std::vector<uint32_t>& output) {
    if(width<=0 || height<=0){output.clear();return;}
    output.resize(size_t(width)*height);
    if(effect==Crt::Off){std::fill(output.begin(),output.end(),0xffffffff);return;}
    constexpr double pi=3.14159265358979323846;
    bool strong=effect==Crt::Strong;
    double beam=strong?.28:.15,phosphor=strong?.88:.96,edge=strong?.14:.06;
    int stripe=std::max(1,int(std::lround(height/1080.0)));
    std::vector<std::array<double,3>> columns(width);
    for(int x=0;x<width;x++) {
        double nx=(x+.5)*2/width-1,vignette=1-edge*.5*nx*nx*nx*nx;
        for(int c=0;c<3;c++)columns[x][c]=255*vignette*((x/stripe)%3==c?1:phosphor);
    }
    // Average each output pixel's slice of the 200-line beam pattern. Small
    // previews fade towards the average instead of aliasing into dark bands.
    double span=pi*200/height,average=std::sin(span)/span;
    for(int y=0;y<height;y++) {
        double ny=(y+.5)*2/height-1,vignette=1-edge*.5*ny*ny*ny*ny;
        double light=(1-beam*(.5+.5*std::cos(2*pi*(y+.5)*200/height)*average))*vignette;
        for(int x=0;x<width;x++) {
            auto c=columns[x];auto channel=[&](int n){return uint32_t(std::lround(c[n]*light));};
            output[size_t(y)*width+x]=0xff000000|(channel(0)<<16)|(channel(1)<<8)|channel(2);
        }
    }
}
void crt_glow(const std::vector<uint32_t>& picture,int width,int height,Pixels& output) {
    if((width!=320 && width!=640) || height!=width*200/320 || picture.size()!=size_t(width)*height)
        throw std::runtime_error("Invalid CRT source image");
    int factor=width/320;
    // Extract a small highlight image before blurring. Blacks stay black and
    // bloom follows the artwork rather than brightening the whole rectangle.
    std::vector<std::array<unsigned,3>> highlights(64000);
    for(int y=0;y<200;y++)for(int x=0;x<320;x++) {
        auto& h=highlights[y*320+x];
        for(int dy=0;dy<factor;dy++)for(int dx=0;dx<factor;dx++) {
            auto p=picture[(y*factor+dy)*width+x*factor+dx];
            for(int c=0;c<3;c++)h[c]+=unsigned(std::max(0,int((p>>(16-c*8))&255)-64));
        }
        for(auto& c:h)c/=factor*factor;
    }
    for(int y=0;y<200;y++)for(int x=0;x<320;x++) {
        unsigned rgb[3]{};
        for(int dy=-1;dy<=1;dy++)for(int dx=-1;dx<=1;dx++) {
            auto h=highlights[std::clamp(y+dy,0,199)*320+std::clamp(x+dx,0,319)];
            unsigned weight=(dx==0?2:1)*(dy==0?2:1);
            for(int c=0;c<3;c++)rgb[c]+=h[c]*weight;
        }
        output[y*320+x]=0xff000000|((rgb[0]/16)<<16)|((rgb[1]/16)<<8)|(rgb[2]/16);
    }
}
}

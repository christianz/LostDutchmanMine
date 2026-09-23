#include "display.h"
#include <algorithm>
#include <chrono>
#include <fstream>
#include <iostream>
#include <memory>
#include <stdexcept>

namespace {
void require(bool condition,const char* message){if(!condition)throw std::runtime_error(message);}
}
int main() {
    auto temp=std::filesystem::temp_directory_path()/("ldm-display-"+std::to_string(std::chrono::steady_clock::now().time_since_epoch().count()));
    try {
        std::filesystem::create_directory(temp);auto ini=temp/"display.ini";
        auto s=ldm::load_settings(ini);require(s.startup && s.vsync,"First launch should show settings with VSync");
        s.window=3;s.size=85;s.scaling=ldm::Scaling::PixelArt;s.colour=ldm::Colour::Warm;s.brightness=110;s.startup=false;
        ldm::save_settings(ini,s);auto loaded=ldm::load_settings(ini);
        require(loaded.window==3 && loaded.size==85 && loaded.scaling==s.scaling && loaded.colour==s.colour && loaded.brightness==110 && !loaded.startup,"Settings must round-trip");
        s.window=0;s.colour=ldm::Colour::Original;ldm::save_settings(ini,s);loaded=ldm::load_settings(ini);
        require(loaded.window==0 && loaded.colour==ldm::Colour::Original,"Existing settings must be replaceable");
        std::ofstream(ini)<<"window=999\nsize=-100\nscaling=4294967296\ncolour=garbage\nbrightness=+\nvsync=2\nstartup=0junk\n";
        auto invalid=ldm::load_settings(ini),defaults=ldm::DisplaySettings{};
        require(invalid.window==defaults.window && invalid.size==defaults.size && invalid.scaling==defaults.scaling && invalid.colour==defaults.colour && invalid.brightness==100 && invalid.vsync && invalid.startup,"Malformed settings must fall back safely");
        auto r=ldm::picture_rect(3840,2160,100);
        require(r.x==480 && r.y==0 && r.w==2880 && r.h==2160,"4K viewport must preserve 4:3");
        auto small=ldm::picture_rect(3840,2160,85);
        require(small.x==696 && small.y==162 && small.w==2448 && small.h==1836,"Comfort picture must be centred");
        int x,y;
        require(ldm::picture_point(r,1920,1080,x,y) && x==160 && y==100,"Mouse centre must map to original coordinates");
        require(ldm::picture_point(r,r.x+r.w-1,r.y+r.h-1,x,y) && x==319 && y==199,"Bottom-right mouse mapping");
        require(!ldm::picture_point(r,10,100,x,y) && x==0,"Letterbox clicks must be rejected");
        require(!ldm::picture_point({},0,0,x,y),"Empty viewport must be safe");
        auto pixels=std::make_unique<ldm::Pixels>();
        for(size_t i=0;i<pixels->size();i++)(*pixels)[i]=0xff000000|uint32_t(i*997);
        auto before=*pixels;std::vector<uint32_t> output;int w,h;
        s=ldm::DisplaySettings{};s.scaling=ldm::Scaling::Crisp;
        ldm::display_pixels(*pixels,s,output,w,h);
        require(w==320 && h==200 && std::equal(output.begin(),output.end(),pixels->begin()),"Original mode must preserve every pixel");
        s.scaling=ldm::Scaling::Soft;ldm::display_pixels(*pixels,s,output,w,h);
        require(w==640 && h==400 && output[0]==(*pixels)[0] && output[1]==output[0] && output[640]==output[0],"Soft pre-scaling must preserve source texels");
        require(*pixels==before,"Presentation must never modify source artwork");
        pixels->fill(0xff777777);(*pixels)[99*320+100]=(*pixels)[100*320+99]=0xffffffff;
        (*pixels)[101*320+100]=(*pixels)[100*320+101]=0xff000000;
        s.scaling=ldm::Scaling::PixelArt;ldm::display_pixels(*pixels,s,output,w,h);
        int at=200*640+200;
        require(output[at]==0xffffffff && output[at+1]==0xff777777 && output[at+640]==0xff777777 && output[at+641]==0xff000000,"Pixel-art diagonal reconstruction");
        for(auto colour:{ldm::Colour::Original,ldm::Colour::Warm,ldm::Colour::Vivid,ldm::Colour::Gentle}) {
            s.colour=colour;s.brightness=120;
            require((ldm::colour_pixel(0xfffefefe,s)>>24)==255,"Colour grading must preserve alpha");
        }
        s.brightness=100;s.colour=ldm::Colour::Original;
        require(ldm::colour_pixel(0xff3479be,s)==0xff3479be,"Original palette must be unchanged");
        s.colour=ldm::Colour::Warm;auto warm=ldm::colour_pixel(0xff808080,s);
        require(((warm>>16)&255)>(warm&255),"Warm profile should reduce blue relative to red");
        std::filesystem::remove_all(temp);
        std::cout<<"PASS: settings persistence/invalid input, 4K aspect and mouse mapping, source preservation, pixel-art diagonals and colour grading\n";
    }catch(const std::exception& e){std::filesystem::remove_all(temp);std::cerr<<e.what()<<"\n";return 1;}
}

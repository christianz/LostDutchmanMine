#include "display.h"
#include "ui-font.h"
#include <algorithm>
#include <chrono>
#include <fstream>
#include <iostream>
#include <memory>
#include <sstream>
#include <stdexcept>

namespace {
void require(bool condition,const char* message){if(!condition)throw std::runtime_error(message);}
bool contains(const std::string& text,const char* part){return text.find(part)!=std::string::npos;}
// Width in the settings menu's 1040x740 layout units, as Presentation::text draws it.
float text_width(const std::string& value,float size) {
    float width=0;
    for(unsigned char c:value)if(c>=32 && c<=126)width+=font_glyphs[c-32].advance*size/88.0f;
    return width;
}
void check_settings_menu(const std::filesystem::path& ini) {
    require(ldm::Comfort==7,"The settings menu has seven rows");
    const char* rows[]={"Display","Scaling","CRT monitor","Colour","Brightness","Show at startup","QoL improvements"};
    for(int row=0;row<ldm::Comfort;row++)require(std::string(ldm::menu_label(row,true))==rows[row],"Settings menu row labels");

    ldm::DisplaySettings s;std::vector<std::string> seen;
    for(int i=0;i<6;i++){seen.push_back(ldm::menu_value(s,ldm::Display));ldm::change_setting(s,ldm::Display,1);}
    require(seen==std::vector<std::string>{"1280 x 960 window","1600 x 1200 window","Fullscreen","Fullscreen 85%","Fullscreen 70%","960 x 720 window"},
        "Display cycles through the windows, then the fullscreen picture sizes");
    require(ldm::menu_value(s,ldm::Display)=="1280 x 960 window","Display wraps around");
    s.window=0;ldm::change_setting(s,ldm::Display,-1);
    require(s.window==3 && s.size==70 && ldm::picture_percent(s)==70,"Left from the smallest window selects Fullscreen 70%");
    s.window=1;s.size=85;require(ldm::picture_percent(s)==100,"Windows always use the whole window");
    s.window=3;require(ldm::picture_percent(s)==85,"Fullscreen keeps its picture size");

    std::ofstream(ini)<<"window=1\nsize=85\nvsync=0\n";
    auto legacy=ldm::load_settings(ini);
    require(legacy.window==1 && ldm::picture_percent(legacy)==100,"Old windowed picture sizes load and are ignored");
    ldm::save_settings(ini,legacy);std::stringstream saved;saved<<std::ifstream(ini).rdbuf();
    require(!contains(saved.str(),"vsync"),"VSync is no longer a setting");

    s=ldm::DisplaySettings{};std::vector<std::string> crt;
    for(int i=0;i<3;i++){crt.push_back(ldm::menu_value(s,ldm::CrtMonitor));ldm::change_setting(s,ldm::CrtMonitor,1);}
    require(crt==std::vector<std::string>{"Off","Subtle","Strong"},"CRT strengths are not named like Soft pixels");
    require(ldm::menu_value(s,ldm::QualityOfLife)=="On","QoL is an On/Off value");
    ldm::change_setting(s,ldm::QualityOfLife,1);require(ldm::menu_value(s,ldm::QualityOfLife)=="Off","QoL toggles");

    require(std::string(ldm::menu_label(ldm::Comfort,true))=="Comfort","The comfort preset is not specific to 4K");
    require(std::string(ldm::menu_label(ldm::Cancel,true))=="Quit" && std::string(ldm::menu_label(ldm::Cancel,false))=="Cancel","Quit at startup, Cancel from F11");
    require(std::string(ldm::menu_label(ldm::Apply,true))=="Play" && std::string(ldm::menu_label(ldm::Apply,false))=="Apply & resume","Play at startup, Apply from F11");
    auto help=[](int item,bool startup){auto lines=ldm::menu_help(item,startup);return std::string(lines[0])+" "+lines[1];};
    require(contains(help(ldm::QualityOfLife,false),"mule") && contains(help(ldm::QualityOfLife,false),"walking"),"QoL help names its gameplay changes");
    require(!contains(help(ldm::CrtMonitor,false),"Soft") && !contains(help(ldm::CrtMonitor,false),"Classic"),"CRT help uses the new strength names");
    require(contains(help(ldm::Cancel,true),"Quit") && contains(help(ldm::Cancel,false),"Cancel"),"Quit and Cancel explain themselves");
    require(contains(help(ldm::Apply,true),"Play") && contains(help(ldm::Apply,false),"Apply"),"Play and Apply explain themselves");
    require(contains(ldm::menu_keys(true),"Esc quits") && contains(ldm::menu_keys(false),"Esc cancels"),"The key hint names what Esc does");
    require(!contains(ldm::menu_keys(true),"F11") && !contains(ldm::menu_keys(false),"F11"),"F11 does nothing inside the menu");

    auto comfort=ldm::comfort_settings(true);
    require(comfort.window==3 && comfort.size==85 && comfort.scaling==ldm::Scaling::Soft && comfort.colour==ldm::Colour::Gentle
        && comfort.crt==ldm::Crt::Off && comfort.brightness==100 && comfort.startup,"Comfort preset");
    require(ldm::menu_value(comfort,ldm::Display)=="Fullscreen 85%","Comfort shows as Fullscreen 85%");
    auto original=ldm::original_settings(false);
    require(original.window==0 && ldm::picture_percent(original)==100 && original.scaling==ldm::Scaling::Crisp
        && original.colour==ldm::Colour::Original && original.crt==ldm::Crt::Off && !original.startup,"Original look preset");

    // Boxes from Presentation::menu: labels end before the "<", values sit between
    // the arrows, help fits under the preview and the key hint ends before Apply.
    for(bool startup:{true,false}) {
        for(int item=0;item<ldm::MenuItemCount;item++) {
            auto lines=ldm::menu_help(item,startup);
            require(text_width(lines[0],17)<=432 && text_width(lines[1],17)<=432,"Help text fits under the preview");
        }
        for(int row=0;row<ldm::Comfort;row++)require(text_width(ldm::menu_label(row,startup),18)<=205,"Row labels fit");
        const float buttons[]={240,240,132,200};
        for(int i=0;i<4;i++)require(text_width(ldm::menu_label(ldm::Comfort+i,startup),18)<=buttons[i]-16,"Button labels fit");
        require(text_width(ldm::menu_keys(startup),15)<=580,"Key hint fits");
    }
    s=ldm::DisplaySettings{};
    for(int row=0;row<ldm::Comfort;row++)for(int i=0;i<6;i++){
        require(text_width(ldm::menu_value(s,row),17)<=190,"Values fit between the arrows");ldm::change_setting(s,row,1);
    }
}
}
int main() {
    auto temp=std::filesystem::temp_directory_path()/("ldm-display-"+std::to_string(std::chrono::steady_clock::now().time_since_epoch().count()));
    try {
        std::filesystem::create_directory(temp);auto ini=temp/"display.ini";
        auto s=ldm::load_settings(ini);require(s.startup,"First launch should show settings");
        require(s.crt==ldm::Crt::Off,"Existing configurations must keep CRT disabled");
        require(s.qol,"New and existing display configurations enable optional improvements by default");
        s.window=3;s.size=85;s.scaling=ldm::Scaling::PixelArt;s.colour=ldm::Colour::Warm;s.crt=ldm::Crt::Strong;s.brightness=110;s.startup=false;s.qol=false;
        ldm::save_settings(ini,s);auto loaded=ldm::load_settings(ini);
        require(loaded.window==3 && loaded.size==85 && loaded.scaling==s.scaling && loaded.colour==s.colour && loaded.crt==s.crt && loaded.brightness==110 && !loaded.startup && !loaded.qol,"Settings and disabled QoL must round-trip");
        s.window=0;s.colour=ldm::Colour::Original;ldm::save_settings(ini,s);loaded=ldm::load_settings(ini);
        require(loaded.window==0 && loaded.colour==ldm::Colour::Original,"Existing settings must be replaceable");
        std::ofstream(ini)<<"window=999\nsize=-100\nscaling=4294967296\ncolour=garbage\ncrt=-1\nbrightness=+\nvsync=2\nstartup=0junk\nqol=2\n";
        auto invalid=ldm::load_settings(ini),defaults=ldm::DisplaySettings{};
        require(invalid.window==defaults.window && invalid.size==defaults.size && invalid.scaling==defaults.scaling && invalid.colour==defaults.colour && invalid.crt==ldm::Crt::Off && invalid.brightness==100 && invalid.startup && invalid.qol,"Malformed settings must fall back safely");
        check_settings_menu(ini);
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
        std::vector<uint32_t> subtle,strong,off;
        ldm::crt_mask(ldm::Crt::Off,2880,2160,off);
        require(off.size()==2880*2160 && std::all_of(off.begin(),off.end(),[](auto c){return c==0xffffffff;}),"CRT Off must have an identity mask");
        ldm::crt_mask(ldm::Crt::Subtle,2880,2160,subtle);
        ldm::crt_mask(ldm::Crt::Strong,2880,2160,strong);
        auto light=[](uint32_t c){return int((c>>16)&255)+int((c>>8)&255)+int(c&255);};
        require(light(subtle[1080*2880+1440])>light(strong[1080*2880+1440]),"Strong must be stronger than Subtle");
        require(light(strong[1080*2880+1440])>light(strong[1080*2880]),"CRT corners/edges must darken gently");
        require(((strong[1080*2880+1440]>>16)&255)!=(strong[1080*2880+1440]&255),"CRT must include a coloured phosphor mask");
        require(light(strong[1080*2880+1440])!=light(strong[1085*2880+1440]),"CRT must include scanlines");
        ldm::crt_mask(ldm::Crt::Strong,0,2160,strong);require(strong.empty(),"Empty CRT viewport must be safe");
        auto glow=std::make_unique<ldm::Pixels>();output.assign(64000,0xff000000);
        ldm::crt_glow(output,320,200,*glow);
        require(std::all_of(glow->begin(),glow->end(),[](auto c){return c==0xff000000;}),"CRT must not glow in empty black areas");
        output[100*320+160]=0xffffffff;auto source=output;
        ldm::crt_glow(output,320,200,*glow);
        require(light((*glow)[100*320+160])>light((*glow)[100*320+161]) && light((*glow)[100*320+161])>0,"Highlight glow must spread softly");
        require((*glow)[100*320+164]==0xff000000 && source==output,"Glow must stay local and preserve the source image");
        std::filesystem::remove_all(temp);
        std::cout<<"PASS: settings persistence/invalid input, settings menu rows/values/help/presets/text fit, 4K aspect/mouse mapping, source preservation, filters, CRT scanlines/phosphors/glow\n";
    }catch(const std::exception& e){std::filesystem::remove_all(temp);std::cerr<<e.what()<<"\n";return 1;}
}

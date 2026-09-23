#include "presentation.h"
#include "ui-font.h"
#include <algorithm>
#include <cmath>
#include <stdexcept>

namespace ldm {
namespace {
constexpr uint32_t ink=0xfff0e9dd,muted=0xffb9ad9a,gold=0xffe0b877,panel=0xff292319,line=0xff4b4030;
void draw_colour(SDL_Renderer* renderer,uint32_t c){SDL_SetRenderDrawColor(renderer,c>>16,c>>8,c,c>>24);}
SDL_Texture* load_texture(SDL_Renderer* renderer,const std::filesystem::path& path,bool required) {
    auto surface=SDL_LoadBMP(path.string().c_str());
    if(!surface) {
        if(required)throw std::runtime_error("Cannot load settings font: keep ui-font.bmp beside the executable.");
        return nullptr;
    }
    auto texture=SDL_CreateTextureFromSurface(renderer,surface);SDL_FreeSurface(surface);
    if(!texture)throw std::runtime_error(SDL_GetError());
    SDL_SetTextureBlendMode(texture,SDL_BLENDMODE_BLEND);SDL_SetTextureScaleMode(texture,SDL_ScaleModeLinear);
    return texture;
}
int cycle(int value,int count,int direction){return (value+count+direction)%count;}
}
Presentation::Presentation(SDL_Window* window,SDL_Renderer* renderer,const std::filesystem::path& app):window_(window),renderer_(renderer) {
    // Text is a pre-baked trusted bitmap; no font parser or system fonts at runtime.
    font_=load_texture(renderer_,app/"ui-font.bmp",true);
    icon_=load_texture(renderer_,app/"LostDutchmanMine.bmp",false);
}
Presentation::~Presentation(){
    for(auto texture:{texture_,font_,icon_,glow_,mask_})if(texture)SDL_DestroyTexture(texture);
}
int Presentation::refresh_rate() const {
    SDL_DisplayMode mode{};
    if(SDL_GetCurrentDisplayMode(std::max(0,SDL_GetWindowDisplayIndex(window_)),&mode)==0 && mode.refresh_rate>0)return mode.refresh_rate;
    return 60;
}
void Presentation::apply(const DisplaySettings& s,bool resize) {
    if(resize) {
        if(s.window==3) {
            if(SDL_SetWindowFullscreen(window_,SDL_WINDOW_FULLSCREEN_DESKTOP)<0)throw std::runtime_error(SDL_GetError());
        }else {
            if(SDL_SetWindowFullscreen(window_,0)<0)throw std::runtime_error(SDL_GetError());
            const int widths[]={960,1280,1600},heights[]={720,960,1200};
            SDL_Rect available{0,0,1920,1080};SDL_GetDisplayUsableBounds(std::max(0,SDL_GetWindowDisplayIndex(window_)),&available);
            double fit=std::min({1.0,double(std::max(320,available.w-32))/widths[s.window],double(std::max(240,available.h-64))/heights[s.window]});
            SDL_SetWindowSize(window_,int(widths[s.window]*fit),int(heights[s.window]*fit));
        }
        SDL_RenderSetViewport(renderer_,nullptr);
    }
    vsync_available_=SDL_RenderSetVSync(renderer_,s.vsync?1:0)==0;
}
void Presentation::upload(const Pixels& pixels,const DisplaySettings& s) {
    int style=int(s.scaling)|(int(s.colour)<<8)|(s.brightness<<16)|(int(s.crt)<<24);
    if(style==previous_style_ && pixels==*previous_)return;
    int w,h;display_pixels(pixels,s,processed_,w,h);
    if(w!=texture_w_ || h!=texture_h_) {
        if(texture_)SDL_DestroyTexture(texture_);
        texture_=SDL_CreateTexture(renderer_,SDL_PIXELFORMAT_ARGB8888,SDL_TEXTUREACCESS_STREAMING,w,h);
        if(!texture_)throw std::runtime_error(SDL_GetError());
        texture_w_=w;texture_h_=h;
    }
    SDL_SetTextureScaleMode(texture_,s.scaling==Scaling::Crisp?SDL_ScaleModeNearest:SDL_ScaleModeLinear);
    if(SDL_UpdateTexture(texture_,nullptr,processed_.data(),w*4)<0)throw std::runtime_error(SDL_GetError());
    if(s.crt!=Crt::Off) {
        if(!glow_) {
            glow_=SDL_CreateTexture(renderer_,SDL_PIXELFORMAT_ARGB8888,SDL_TEXTUREACCESS_STREAMING,320,200);
            if(!glow_)throw std::runtime_error(SDL_GetError());
            SDL_SetTextureScaleMode(glow_,SDL_ScaleModeLinear);SDL_SetTextureBlendMode(glow_,SDL_BLENDMODE_ADD);
        }
        crt_glow(processed_,w,h,*glow_pixels_);
        if(SDL_UpdateTexture(glow_,nullptr,glow_pixels_->data(),320*4)<0)throw std::runtime_error(SDL_GetError());
    }
    *previous_=pixels;previous_style_=style;
}
void Presentation::picture(const SDL_Rect& dest,const DisplaySettings& s) {
    if(dest.w<=0 || dest.h<=0)return;
    SDL_RenderCopy(renderer_,texture_,nullptr,&dest);
    if(s.crt==Crt::Off)return;
    if(dest.w!=mask_w_ || dest.h!=mask_h_ || s.crt!=mask_style_) {
        std::vector<uint32_t> pixels;crt_mask(s.crt,dest.w,dest.h,pixels);
        if(mask_)SDL_DestroyTexture(mask_);
        mask_=SDL_CreateTexture(renderer_,SDL_PIXELFORMAT_ARGB8888,SDL_TEXTUREACCESS_STATIC,dest.w,dest.h);
        if(!mask_)throw std::runtime_error(SDL_GetError());
        SDL_SetTextureBlendMode(mask_,SDL_BLENDMODE_MOD);SDL_SetTextureScaleMode(mask_,SDL_ScaleModeNearest);
        if(SDL_UpdateTexture(mask_,nullptr,pixels.data(),dest.w*4)<0)throw std::runtime_error(SDL_GetError());
        mask_w_=dest.w;mask_h_=dest.h;mask_style_=s.crt;
    }
    SDL_RenderCopy(renderer_,mask_,nullptr,&dest);
    SDL_SetTextureAlphaMod(glow_,s.crt==Crt::Classic?30:18);
    SDL_RenderCopy(renderer_,glow_,nullptr,&dest);
}
void Presentation::game(const Pixels& pixels,const DisplaySettings& s) {
    upload(pixels,s);
    int w,h;SDL_GetRendererOutputSize(renderer_,&w,&h);auto bounds=picture_rect(w,h,s.size);
    SDL_Rect dest{bounds.x,bounds.y,bounds.w,bounds.h};
    draw_colour(renderer_,0xff000000);SDL_RenderClear(renderer_);
    picture(dest,s);
}
void Presentation::physical_point(int wx,int wy,int& px,int& py) {
    int ww,wh,rw,rh;SDL_GetWindowSize(window_,&ww,&wh);SDL_GetRendererOutputSize(renderer_,&rw,&rh);
    px=int(int64_t(wx)*rw/std::max(1,ww));py=int(int64_t(wy)*rh/std::max(1,wh));
}
bool Presentation::game_point(int wx,int wy,const DisplaySettings& s,int& x,int& y) {
    int px,py,w,h;physical_point(wx,wy,px,py);SDL_GetRendererOutputSize(renderer_,&w,&h);
    return picture_point(picture_rect(w,h,s.size),px,py,x,y);
}
void Presentation::ui_layout() {
    int w,h;SDL_GetRendererOutputSize(renderer_,&w,&h);
    ui_scale_=std::min(w/1040.0f,h/740.0f);ui_x_=(w-1040*ui_scale_)/2;ui_y_=(h-740*ui_scale_)/2;
}
SDL_FRect Presentation::ui_rect(float x,float y,float w,float h) const {return {ui_x_+x*ui_scale_,ui_y_+y*ui_scale_,w*ui_scale_,h*ui_scale_};}
void Presentation::box(float x,float y,float w,float h,uint32_t colour,bool outline) {
    auto r=ui_rect(x,y,w,h);draw_colour(renderer_,colour);
    if(outline)SDL_RenderDrawRectF(renderer_,&r);else SDL_RenderFillRectF(renderer_,&r);
}
void Presentation::text(float x,float y,float size,const std::string& value,uint32_t colour,bool centre) {
    float scale=size/88.0f,width=0;
    for(unsigned char c:value)if(c>=32 && c<=126)width+=font_glyphs[c-32].advance*scale;
    if(centre)x-=width/2;
    SDL_SetTextureColorMod(font_,colour>>16,colour>>8,colour);SDL_SetTextureAlphaMod(font_,colour>>24);
    for(unsigned char c:value) {
        if(c<32 || c>126)continue;
        auto g=font_glyphs[c-32];SDL_Rect src{g.x,g.y,g.w,g.h};
        auto dst=ui_rect(x+g.left*scale,y+g.top*scale,g.w*scale,g.h*scale);
        SDL_RenderCopyF(renderer_,font_,&src,&dst);x+=g.advance*scale;
    }
}
void Presentation::menu(const Pixels& pixels,const DisplaySettings& s,bool startup,int selected,const std::string& message) {
    ui_layout();draw_colour(renderer_,0xff17140f);SDL_RenderClear(renderer_);
    if(icon_){auto r=ui_rect(38,32,64,64);SDL_RenderCopyF(renderer_,icon_,nullptr,&r);}
    text(118,64,32,"Lost Dutchman Mine",ink);text(119,91,15,"DISPLAY & GAMEPLAY",gold);
    box(40,122,960,1,line);
    const char* labels[]={"Display","Picture size","Scaling","CRT monitor","Colour","Brightness","VSync","Show at startup","QoL improvements"};
    const char* windows[]={"960 x 720 window","1280 x 960 window","1600 x 1200 window","Fullscreen"};
    const char* filters[]={"Crisp pixels","Soft pixels","Pixel art"};
    const char* colours[]={"Original","Warm","Vivid","Gentle"};
    const char* crt[]={"Off","Soft","Classic"};
    std::string values[]={windows[s.window],std::to_string(s.size)+"%",filters[int(s.scaling)],crt[int(s.crt)],colours[int(s.colour)],std::to_string(s.brightness)+"%",s.vsync?"On":"Off",s.startup?"Yes":"No",s.qol?"On":"Off"};
    for(int i=0;i<Comfort;i++) {
        float y=154+i*44;box(40,y,496,38,panel);box(40,y,496,38,i==selected?gold:line,true);
        text(55,y+26,18,labels[i],i==selected?ink:muted);
        if(i==QualityOfLife) {
            box(348,y+9,20,20,ink,true);
            if(s.qol){text(351,y+25,18,"x",gold);}
            text(402,y+26,17,values[i],ink,true);
        }else {
            text(270,y+27,22,"<",gold);text(502,y+27,22,">",gold);
            text(388,y+26,17,values[i],ink,true);
        }
    }
    text(568,153,15,"LIVE PREVIEW",gold);
    box(568,174,432,324,0xff000000);
    upload(pixels,s);auto p=picture_rect(432,324,s.size);
    auto dst=ui_rect(568+p.x,174+p.y,p.w,p.h);
    picture({int(std::lround(dst.x)),int(std::lround(dst.y)),int(std::lround(dst.w)),int(std::lround(dst.h))},s);
    box(568,174,432,324,line,true);
    const char* help[][2]={
        {"Fullscreen uses your monitor's resolution.","The picture keeps its original 4:3 shape."},
        {"Use a smaller picture on a large monitor.","Black borders keep the image centred."},
        {"Soft blends edges. Pixel art rounds diagonals.","Crisp keeps the original hard pixel edges."},
        {"Scanlines, phosphor texture and a soft glow.","Soft is subtle. Classic gives a stronger effect."},
        {"Adjust the colour of the original artwork.","Original keeps the game's palette unchanged."},
        {"Adjust picture brightness to suit your room.","100% keeps the original brightness."},
        {"Synchronise presentation with the monitor.","The game's clock runs independently."},
        {"Choose whether this menu opens at launch.","You can always open it again with F11."},
        {"Interactive panning and mouse aiming.","Off restores original panning and aiming."},
        {"Fullscreen, 85% picture size and soft edges.","Gentle colours; adjust any choice to taste."},
        {"Crisp pixels and the original colour palette.","A window with the original 4:3 proportions."},
        {"Settings are saved when you apply them.","F11 pauses play and reopens this menu."}
    };
    int help_row=std::clamp(selected,0,int(Cancel));
    text(568,529,17,help[help_row][0],muted);text(568,554,17,help[help_row][1],muted);
    auto button=[&](int id,float x,float y,float w,const char* label,bool primary=false) {
        box(x,y,w,44,primary?gold:panel);box(x,y,w,44,selected==id?ink:line,true);
        text(x+w/2,y+29,18,label,primary?0xff211a10:ink,true);
    };
    button(Comfort,40,566,240,"4K comfort");button(Original,296,566,240,"Original look");
    SDL_DisplayMode mode{};SDL_GetDesktopDisplayMode(std::max(0,SDL_GetWindowDisplayIndex(window_)),&mode);
    text(568,598,16,"Monitor: "+std::to_string(mode.w)+" x "+std::to_string(mode.h)+" / "+std::to_string(refresh_rate())+" Hz",muted);
    box(40,639,960,1,line);
    button(Cancel,40,660,132,startup?"Quit":"Cancel");button(Apply,800,660,200,startup?"Play":"Apply & resume",true);
    text(206,687,15,"Arrows adjust   Enter confirms   F11 settings",muted);
    if(!message.empty())text(40,627,15,message,0xffffab81);
    else if(s.vsync && !vsync_available_)text(568,624,15,"VSync unavailable; frame pacing is active.",muted);
}
int Presentation::menu_hit(int wx,int wy,bool& left) {
    ui_layout();int px,py;physical_point(wx,wy,px,py);
    float x=(px-ui_x_)/ui_scale_,y=(py-ui_y_)/ui_scale_;left=x>=258 && x<300;
    if(x>=40 && x<536 && y>=154 && y<154+Comfort*44) {
        int row=int(y-154)/44;if(int(y-154)%44<38)return row;
    }
    if(y>=566 && y<610){if(x>=40 && x<280)return Comfort;if(x>=296 && x<536)return Original;}
    if(y>=660 && y<704){if(x>=40 && x<172)return Cancel;if(x>=800 && x<1000)return Apply;}
    return -1;
}
void Presentation::capture(const std::filesystem::path& file) {
    int w,h;SDL_GetRendererOutputSize(renderer_,&w,&h);
    if(!file.parent_path().empty())std::filesystem::create_directories(file.parent_path());
    auto surface=SDL_CreateRGBSurfaceWithFormat(0,w,h,32,SDL_PIXELFORMAT_ARGB8888);
    if(!surface)throw std::runtime_error(SDL_GetError());
    int read=SDL_RenderReadPixels(renderer_,nullptr,SDL_PIXELFORMAT_ARGB8888,surface->pixels,surface->pitch);
    int saved=read<0?-1:SDL_SaveBMP(surface,file.string().c_str());SDL_FreeSurface(surface);
    if(saved<0)throw std::runtime_error(SDL_GetError());
}
void change_setting(DisplaySettings& s,int row,int dir) {
    switch(row) {
    case Display:s.window=cycle(s.window,4,dir);break;
    case PictureSize:{const int sizes[]={70,85,100};int i=s.size==70?0:s.size==85?1:2;s.size=sizes[cycle(i,3,dir)];break;}
    case ScalingFilter:s.scaling=Scaling(cycle(int(s.scaling),3,dir));break;
    case CrtMonitor:s.crt=Crt(cycle(int(s.crt),3,dir));break;
    case ColourProfile:s.colour=Colour(cycle(int(s.colour),4,dir));break;
    case Brightness:s.brightness=80+10*cycle((s.brightness-80)/10,5,dir);break;
    case VSync:s.vsync=!s.vsync;break;
    case Startup:s.startup=!s.startup;break;
    case QualityOfLife:s.qol=!s.qol;break;
    }
}
DisplaySettings comfort_settings(bool startup){DisplaySettings s;s.window=3;s.size=85;s.colour=Colour::Gentle;s.startup=startup;return s;}
DisplaySettings original_settings(bool startup){DisplaySettings s;s.window=0;s.scaling=Scaling::Crisp;s.startup=startup;return s;}
}

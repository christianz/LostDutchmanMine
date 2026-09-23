#define SDL_MAIN_HANDLED
#include <SDL.h>
#include "legacy.h"
#include "image_info.h"
#include <algorithm>
#include <chrono>
#include <fstream>
#include <iostream>
#include <memory>
#include <sstream>

namespace {
void audio_callback(void* userdata,Uint8* stream,int bytes) {
    static_cast<ldm::Audio*>(userdata)->render(reinterpret_cast<float*>(stream),bytes/int(sizeof(float)));
}
struct ScriptEvent {uint64_t time;std::string type;int a=0,b=0;};
uint8_t direction(SDL_Scancode key) {
    switch(key) {
    case SDL_SCANCODE_UP:case SDL_SCANCODE_KP_8:return 1;
    case SDL_SCANCODE_DOWN:case SDL_SCANCODE_KP_2:return 2;
    case SDL_SCANCODE_LEFT:case SDL_SCANCODE_KP_4:return 4;
    case SDL_SCANCODE_RIGHT:case SDL_SCANCODE_KP_6:return 8;
    case SDL_SCANCODE_HOME:case SDL_SCANCODE_KP_7:return 5;
    case SDL_SCANCODE_PAGEUP:case SDL_SCANCODE_KP_9:return 9;
    case SDL_SCANCODE_END:case SDL_SCANCODE_KP_1:return 6;
    case SDL_SCANCODE_PAGEDOWN:case SDL_SCANCODE_KP_3:return 10;
    default:return 0;
    }
}
uint8_t movement(const std::array<bool,SDL_NUM_SCANCODES>& held) {
    uint8_t mask=0;
    for(unsigned i=0;i<held.size();i++)if(held[i])mask|=direction(SDL_Scancode(i));
    if((mask&3)==3)mask&=~3; // Opposite directions cancel, independently per axis.
    if((mask&12)==12)mask&=~12;
    return mask;
}
uint16_t keycode(const SDL_KeyboardEvent& e) {
    uint16_t scan=0;
    switch(e.keysym.sym) {
    case SDLK_ESCAPE: return 0x011b;
    case SDLK_RETURN: return 0x1c0d;
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
void frame(const ldm::State&s,std::array<uint32_t,64000>& pixels) {
    if(s.video_mode==0x13) {
        for(size_t i=0;i<pixels.size();i++)pixels[i]=s.palette[s.memory[0xa0000+i]];
    } else if(s.video_mode==4 || s.video_mode==5) {
        const unsigned cga[]={0,11,13,15};
        const uint32_t canonical[]={0xff000000,0xff0000aa,0xff00aa00,0xff00aaaa,0xffaa0000,0xffaa00aa,0xffaa5500,0xffaaaaaa,0xff555555,0xff5555ff,0xff55ff55,0xff55ffff,0xffff5555,0xffff55ff,0xffffff55,0xffffffff};
        for(unsigned y=0;y<200;y++)for(unsigned x=0;x<320;x++) {
            auto v=s.memory[0xb8000+(y%2)*8192+(y/2)*80+x/4];
            pixels[y*320+x]=canonical[cga[(v>>(6-2*(x%4)))&3]];
        }
    } else pixels.fill(0xff000000);
    if(s.custom_cursor && s.mouse_visibility>=0) {
        for(int y=0;y<16;y++)for(int x=0;x<16;x++) {
            int px=s.mouse_x-s.mouse_hot_x+x,py=s.mouse_y-s.mouse_hot_y+y;
            if(px<0 || px>=320 || py<0 || py>=200)continue;
            auto& c=pixels[py*320+px];
            if(!(s.mouse_mask[y]&(0x8000>>x)))c=0xff000000;
            if(s.mouse_mask[y+16]&(0x8000>>x))c^=0xffffff;
        }
    }
}
void capture(const std::array<uint32_t,64000>& pixels,const std::filesystem::path& file) {
    std::filesystem::create_directories(file.parent_path());
    auto surface=SDL_CreateRGBSurfaceWithFormatFrom(const_cast<uint32_t*>(pixels.data()),320,200,32,320*4,SDL_PIXELFORMAT_ARGB8888);
    if(!surface || SDL_SaveBMP(surface,file.string().c_str())<0)throw std::runtime_error(SDL_GetError());
    SDL_FreeSurface(surface);
}
}
int main(int argc,char**argv) {
    char* base=SDL_GetBasePath();
    auto app_dir=base?std::filesystem::path(base):std::filesystem::absolute(argv[0]).parent_path();
    SDL_free(base);
    std::filesystem::path data=app_dir/"Game",image=data/"port-data.bin",save=app_dir/"Saves",script;
    uint64_t duration=0;std::string keys;
    auto s=std::make_unique<ldm::State>();
    SDL_Window* window=nullptr;SDL_Renderer*renderer=nullptr;SDL_Texture*texture=nullptr;
    SDL_AudioDeviceID audio=0;
    try {
      for(int i=1;i<argc;i++) {
        std::string arg=argv[i];auto value=[&](){if(i+1>=argc)throw std::runtime_error("Missing argument");return std::string(argv[++i]);};
        if(arg=="--data")data=value();else if(arg=="--image")image=value();else if(arg=="--saves")save=value();
        else if(arg=="--seconds")duration=std::stoull(value())*1000;
        else if(arg=="--keys")keys=value();else if(arg=="--script")script=value();
        else {std::cerr<<"Unknown argument "<<arg<<"\n";return 2;}
      }
        std::vector<ScriptEvent> events;size_t event_pos=0;
        if(!script.empty()) {
            std::ifstream f(script);if(!f)throw std::runtime_error("Cannot open input script");
            std::string line;while(std::getline(f,line)){if(line.empty()||line[0]=='#')continue;std::istringstream in(line);ScriptEvent e{};in>>e.time>>e.type>>e.a>>e.b;events.push_back(e);}
            if(!std::is_sorted(events.begin(),events.end(),[](auto&a,auto&b){return a.time<b.time;}))throw std::runtime_error("Input script is not time ordered");
        }
        s->data_dir=std::filesystem::absolute(data);s->save_dir=std::filesystem::absolute(save);
        if(std::filesystem::weakly_canonical(s->data_dir)==std::filesystem::weakly_canonical(s->save_dir))throw std::runtime_error("Save directory must differ from the original game directory");
        s->load(image,image_relocations,entry_cs,entry_ip,stack_ss,stack_sp);
        for(unsigned char c:keys){SDL_KeyboardEvent key{};key.keysym.sym=c;s->keys.push_back(keycode(key));}
        SDL_SetMainReady();
        if(SDL_Init(SDL_INIT_VIDEO|SDL_INIT_TIMER)<0)throw std::runtime_error(SDL_GetError());
        if(SDL_InitSubSystem(SDL_INIT_AUDIO)==0) {
            SDL_AudioSpec wanted{};wanted.freq=48000;wanted.format=AUDIO_F32SYS;wanted.channels=1;wanted.samples=512;wanted.callback=audio_callback;wanted.userdata=&s->audio;
            audio=SDL_OpenAudioDevice(nullptr,0,&wanted,nullptr,0);if(audio)SDL_PauseAudioDevice(audio,0);
        }
        window=SDL_CreateWindow("Lost Dutchman Mine — native port (development)",SDL_WINDOWPOS_CENTERED,SDL_WINDOWPOS_CENTERED,960,720,SDL_WINDOW_RESIZABLE|SDL_WINDOW_ALLOW_HIGHDPI);
        if(!window)throw std::runtime_error(SDL_GetError());
        if(auto icon=SDL_LoadBMP((app_dir/"LostDutchmanMine.bmp").string().c_str())) {
            SDL_SetWindowIcon(window,icon);SDL_FreeSurface(icon);
        }
        renderer=SDL_CreateRenderer(window,-1,SDL_RENDERER_SOFTWARE);
        if(!renderer)throw std::runtime_error(SDL_GetError());
        SDL_RenderSetLogicalSize(renderer,320,240);
        SDL_SetHint(SDL_HINT_RENDER_SCALE_QUALITY,"0");
        texture=SDL_CreateTexture(renderer,SDL_PIXELFORMAT_ARGB8888,SDL_TEXTUREACCESS_STREAMING,320,200);
        if(!texture)throw std::runtime_error(SDL_GetError());
        std::array<uint32_t,64000> pixels{};
        std::array<bool,SDL_NUM_SCANCODES> held{};
        auto keyboard=[&](const SDL_KeyboardEvent& key,bool pressed) {
            auto scan=key.keysym.scancode;
            if(scan>SDL_SCANCODE_UNKNOWN && scan<SDL_NUM_SCANCODES)held[scan]=pressed;
            auto bios=keycode(key);
            if(pressed) {
                if(key.keysym.sym==SDLK_RETURN && (key.keysym.mod&KMOD_ALT)) {
                    if(!key.repeat)SDL_SetWindowFullscreen(window,SDL_GetWindowFlags(window)&SDL_WINDOW_FULLSCREEN_DESKTOP?0:SDL_WINDOW_FULLSCREEN_DESKTOP);
                }else if(bios)s->keys.push_back(bios|((key.repeat && direction(scan))?0x10000u:0));
            }else if(direction(scan)) {
                s->keys.erase(std::remove_if(s->keys.begin(),s->keys.end(),[&](uint32_t pending){
                    return (pending&0x10000) && uint16_t(pending)==bios;
                }),s->keys.end());
            }
        };
        auto release_input=[&](){held.fill(false);s->set_movement(0);s->keys.clear();s->mouse_buttons=0;};
        uint64_t start=SDL_GetPerformanceCounter(),last=start,last_frame=0;
        double frequency=double(SDL_GetPerformanceFrequency()),timer_elapsed=0;
        while(s->running) {
            auto now=SDL_GetPerformanceCounter();uint64_t elapsed=uint64_t((now-start)*1000/frequency);
            if(duration && elapsed>=duration)break;
            timer_elapsed+=(now-last)/frequency;last=now;
            while(timer_elapsed>=((s->pit_divisor?s->pit_divisor:65536)/1193182.0)) {
                timer_elapsed-=(s->pit_divisor?s->pit_divisor:65536)/1193182.0;
                s->timer_interrupt();
            }
            SDL_Event e;
            while(SDL_PollEvent(&e)) {
                if(e.type==SDL_QUIT)s->running=false;
                else if(e.type==SDL_KEYDOWN || e.type==SDL_KEYUP)keyboard(e.key,e.type==SDL_KEYDOWN);
                else if(e.type==SDL_WINDOWEVENT && e.window.event==SDL_WINDOWEVENT_FOCUS_LOST)release_input();
                else if(e.type==SDL_MOUSEMOTION){s->mouse_x=std::clamp(e.motion.x,0,319);s->mouse_y=std::clamp(e.motion.y*200/240,0,199);}
                else if(e.type==SDL_MOUSEBUTTONDOWN || e.type==SDL_MOUSEBUTTONUP){int bit=e.button.button==SDL_BUTTON_LEFT?1:e.button.button==SDL_BUTTON_RIGHT?2:0;if(e.type==SDL_MOUSEBUTTONDOWN)s->mouse_buttons|=bit;else s->mouse_buttons&=~bit;}
            }
            while(event_pos<events.size() && events[event_pos].time<=elapsed) {
                auto event=events[event_pos++];
                if(event.type=="key")s->keys.push_back(uint16_t(event.a));
                else if(event.type=="down" || event.type=="up" || event.type=="repeat") {
                    if(event.a<=0 || event.a>=SDL_NUM_SCANCODES)throw std::runtime_error("Invalid scripted scancode");
                    SDL_KeyboardEvent key{};key.keysym.scancode=SDL_Scancode(event.a);
                    key.keysym.sym=SDL_GetKeyFromScancode(key.keysym.scancode);key.repeat=event.type=="repeat";
                    keyboard(key,event.type!="up");
                }
                else if(event.type=="focuslost")release_input();
                else if(event.type=="ascii") {
                    SDL_KeyboardEvent key{};key.keysym.sym=event.a;
                    if(event.a>='A'&&event.a<='Z'){key.keysym.sym+=32;key.keysym.mod=KMOD_SHIFT;}
                    key.keysym.scancode=SDL_GetScancodeFromKey(key.keysym.sym);s->keys.push_back(keycode(key));
                }
                else if(event.type=="mouse"){s->mouse_x=event.a;s->mouse_y=event.b;}
                else if(event.type=="buttons")s->mouse_buttons=event.a;
                else if(event.type=="capture"){
                    frame(*s,pixels);capture(pixels,std::filesystem::path("captures")/(std::to_string(event.a)+".bmp"));
                    // Recovered game-state fields for reproducible save and movement checks.
                    std::ofstream meta(std::filesystem::path("captures")/(std::to_string(event.a)+".json"));
                    meta<<"{\"x\":"<<s->u16(0x82bd,0x5b4a)<<",\"y\":"<<s->u16(0x82bd,0x5b4c)
                        <<",\"town_page\":"<<s->u16(0x82bd,0x5b5a)<<",\"building\":"<<s->u16(0x82bd,0x5b5e)
                        <<",\"video_mode\":"<<s->video_mode<<",\"held_directions\":"<<unsigned(movement(held))<<"}\n";
                }
                else throw std::runtime_error("Unknown script event");
            }
            s->set_movement(movement(held));
            for(int steps=0;steps<4096 && s->running;steps++) {
                ldm::native_step(*s);
                if(s->waiting)break;
            }
            s->audio.speaker((s->ports[0x61]&3)==3?1193182u/(s->speaker_divisor?s->speaker_divisor:65536):0);
            SDL_ShowCursor(s->custom_cursor?SDL_DISABLE:SDL_ENABLE);
            if(elapsed>=last_frame+16){frame(*s,pixels);SDL_UpdateTexture(texture,nullptr,pixels.data(),320*4);SDL_RenderClear(renderer);SDL_RenderCopy(renderer,texture,nullptr,nullptr);SDL_RenderPresent(renderer);last_frame=elapsed;}
            SDL_Delay(1);
        }
        if(duration || !script.empty()){frame(*s,pixels);capture(pixels,"captures/last-frame.bmp");}
        std::cerr<<"Native runtime: "<<s->boundaries<<" boundaries, "<<s->ticks<<" BIOS ticks, mode "<<s->video_mode
                 <<", PIT="<<s->pit_divisor<<", timer="<<std::hex<<s->u16(0,0x22)<<":"<<s->u16(0,0x20)
                 <<", CS:IP="<<s->cs-ldm::LoadSegment<<":"<<s->ip<<", clock="<<s->u16(0x2b14,0xfbc4)<<":"<<s->u16(0x2b14,0xfbc2)<<std::dec
                 <<", FM writes="<<s->audio.writes()<<", audible samples="<<s->audio.audible_samples()<<"\n";
    } catch(const std::exception&e) {
        std::cerr<<e.what()<<"\n";
        if(duration || !script.empty()) {
            std::filesystem::create_directories(".local");
            std::ofstream(".local/memory.bin",std::ios::binary).write(reinterpret_cast<const char*>(s->memory.data()),s->memory.size());
        }
        if(!duration && script.empty())SDL_ShowSimpleMessageBox(SDL_MESSAGEBOX_ERROR,"Lost Dutchman Mine",e.what(),window);
        if(audio)SDL_CloseAudioDevice(audio);
        SDL_Quit();return 1;
    }
    if(texture)SDL_DestroyTexture(texture);
    if(renderer)SDL_DestroyRenderer(renderer);
    if(window)SDL_DestroyWindow(window);
    if(audio)SDL_CloseAudioDevice(audio);
    SDL_Quit();
    return 0;
}

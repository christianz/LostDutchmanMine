#define SDL_MAIN_HANDLED
#include <SDL.h>
#include "legacy.h"
#include "image_info.h"
#include "session.h"
#include "presentation.h"
#include "keyboard.h"
#include <algorithm>
#include <chrono>
#include <fstream>
#include <iostream>
#include <memory>
#include <sstream>

namespace {
using ldm::input::keycode;
using ldm::input::direction;
using ldm::input::movement;
void audio_callback(void* userdata,Uint8* stream,int bytes) {
    static_cast<ldm::Audio*>(userdata)->render(reinterpret_cast<float*>(stream),bytes/int(sizeof(float)));
}
struct ScriptEvent {uint64_t time;std::string type;int a=0,b=0;};
void capture(const std::array<uint32_t,64000>& pixels,const std::filesystem::path& file) {
    std::filesystem::create_directories(file.parent_path());
    auto surface=SDL_CreateRGBSurfaceWithFormatFrom(const_cast<uint32_t*>(pixels.data()),320,200,32,320*4,SDL_PIXELFORMAT_ARGB8888);
    if(!surface || SDL_SaveBMP(surface,file.string().c_str())<0)throw std::runtime_error(SDL_GetError());
    SDL_FreeSurface(surface);
}
}
int main(int argc,char**argv) {
    char* base=SDL_GetBasePath();
    auto app_dir=base?std::filesystem::path(base):std::filesystem::absolute(argv[0]).parent_path();SDL_free(base);
    std::filesystem::path data=app_dir/"Game",image=data/"port-data.bin",save=app_dir/"Saves",script;
    std::filesystem::path config=app_dir/"display.ini",screenshot;
    uint64_t duration=0;std::string keys;bool force_setup=false,no_setup=false;
    auto s=std::make_unique<ldm::State>();
    SDL_Window* window=nullptr;SDL_Renderer* renderer=nullptr;SDL_AudioDeviceID audio=0;
    std::unique_ptr<ldm::Session> session;
    try {
        for(int i=1;i<argc;i++) {
            std::string arg=argv[i];auto value=[&](){if(i+1>=argc)throw std::runtime_error("Missing argument");return std::string(argv[++i]);};
            if(arg=="--data")data=value();else if(arg=="--image")image=value();else if(arg=="--saves")save=value();
            else if(arg=="--seconds")duration=std::stoull(value())*1000;
            else if(arg=="--keys")keys=value();else if(arg=="--script")script=value();
            else if(arg=="--config")config=value();else if(arg=="--screenshot")screenshot=value();
            else if(arg=="--settings")force_setup=true;else if(arg=="--no-settings")no_setup=true;
            else {std::cerr<<"Unknown argument "<<arg<<"\n";return 2;}
        }
        std::vector<ScriptEvent> events;size_t event_pos=0;
        if(!script.empty()) {
            std::ifstream f(script);if(!f)throw std::runtime_error("Cannot open input script");
            std::string line;while(std::getline(f,line)){if(line.empty()||line[0]=='#')continue;std::istringstream in(line);ScriptEvent e{};in>>e.time>>e.type>>e.a>>e.b;events.push_back(e);}
            if(!std::is_sorted(events.begin(),events.end(),[](auto&a,auto&b){return a.time<b.time;}))throw std::runtime_error("Input script is not time ordered");
        }
        auto settings=ldm::load_settings(config),draft=settings;
        bool menu=force_setup || (!no_setup && !duration && script.empty() && settings.startup);
        bool startup_menu=menu,warmup=menu,quit=false;int selected=ldm::ScalingFilter,mouse_buttons=0,last_window=settings.window==3?1:settings.window;
        std::string message;
        s->data_dir=std::filesystem::absolute(data);s->save_dir=std::filesystem::absolute(save);
        if(std::filesystem::weakly_canonical(s->data_dir)==std::filesystem::weakly_canonical(s->save_dir))throw std::runtime_error("Save directory must differ from the original game directory");
        s->load(image,image_relocations,entry_cs,entry_ip,stack_ss,stack_sp);
        s->qol_improvements=settings.qol;
        for(unsigned char c:keys){SDL_KeyboardEvent key{};key.keysym.sym=c;s->keys.push_back(keycode(key));}
        SDL_SetMainReady();SDL_SetHint(SDL_HINT_WINDOWS_DPI_AWARENESS,"permonitorv2");
        SDL_SetHint(SDL_HINT_MOUSE_FOCUS_CLICKTHROUGH,"1");
        if(SDL_Init(SDL_INIT_VIDEO|SDL_INIT_TIMER)<0)throw std::runtime_error(SDL_GetError());
        if(SDL_InitSubSystem(SDL_INIT_AUDIO)==0) {
            SDL_AudioSpec wanted{};wanted.freq=48000;wanted.format=AUDIO_F32SYS;wanted.channels=1;wanted.samples=512;wanted.callback=audio_callback;wanted.userdata=&s->audio;
            audio=SDL_OpenAudioDevice(nullptr,0,&wanted,nullptr,0);
        }
        window=SDL_CreateWindow("Lost Dutchman Mine",SDL_WINDOWPOS_CENTERED,SDL_WINDOWPOS_CENTERED,1040,780,
            SDL_WINDOW_RESIZABLE|SDL_WINDOW_ALLOW_HIGHDPI|(settings.window==3?SDL_WINDOW_FULLSCREEN_DESKTOP:0));
        if(!window)throw std::runtime_error(SDL_GetError());
        SDL_SetWindowMinimumSize(window,640,480);
        if(auto icon=SDL_LoadBMP((app_dir/"LostDutchmanMine.bmp").string().c_str())){SDL_SetWindowIcon(window,icon);SDL_FreeSurface(icon);}
        renderer=SDL_CreateRenderer(window,-1,SDL_RENDERER_ACCELERATED|SDL_RENDERER_PRESENTVSYNC);
        if(!renderer)renderer=SDL_CreateRenderer(window,-1,SDL_RENDERER_ACCELERATED);
        if(!renderer)renderer=SDL_CreateRenderer(window,-1,SDL_RENDERER_SOFTWARE);
        if(!renderer)throw std::runtime_error(SDL_GetError());
        SDL_RendererInfo info{};SDL_GetRendererInfo(renderer,&info);
        std::cerr<<"Display renderer: "<<info.name<<"\n";
        ldm::Presentation view(window,renderer,app_dir);view.apply(settings,true);
        session=std::make_unique<ldm::Session>(*s);
        if(audio && !menu)SDL_PauseAudioDevice(audio,0);
        std::array<bool,SDL_NUM_SCANCODES> held{};
        auto release_input=[&](){held.fill(false);mouse_buttons=0;session->clear_input();};
        auto open_menu=[&](){
            release_input();session->pause(true);if(audio)SDL_PauseAudioDevice(audio,1);
            menu=true;startup_menu=false;warmup=false;draft=settings;selected=ldm::ScalingFilter;message.clear();
        };
        auto close_menu=[&](){
            menu=false;warmup=false;release_input();session->pause(false);if(audio)SDL_PauseAudioDevice(audio,0);
        };
        auto action=[&](int id,int direction) {
            if(id>=0 && id<ldm::Comfort){ldm::change_setting(draft,id,direction);view.apply(draft,false);message.clear();}
            else if(id==ldm::Comfort){bool qol=draft.qol;draft=ldm::comfort_settings(draft.startup);draft.qol=qol;view.apply(draft,false);}
            else if(id==ldm::Original){bool qol=draft.qol;draft=ldm::original_settings(draft.startup);draft.qol=qol;view.apply(draft,false);}
            else if(id==ldm::Cancel){if(startup_menu)quit=true;else{view.apply(settings,true);close_menu();}}
            else if(id==ldm::Apply) {
                try{ldm::save_settings(config,draft);}
                catch(const std::exception& e){message="Could not save settings. Check that this game folder is writable.";std::cerr<<e.what()<<"\n";return;}
                settings=draft;session->qol(settings.qol);if(settings.window!=3)last_window=settings.window;
                view.apply(settings,true);close_menu();
            }
        };
        auto keyboard=[&](const SDL_KeyboardEvent& key,bool pressed) {
            if(menu) {
                if(!pressed)return;
                switch(ldm::input::menu_key(key)) {
                case SDLK_ESCAPE:action(ldm::Cancel,1);break;
                case SDLK_RETURN:case SDLK_KP_ENTER:if(!key.repeat)action(selected>=ldm::Comfort?selected:ldm::Apply,1);break;
                case SDLK_SPACE:if(selected<ldm::Comfort || !key.repeat)action(selected,1);break;
                case SDLK_TAB:selected=(selected+((key.keysym.mod&KMOD_SHIFT)?ldm::MenuItemCount-1:1))%ldm::MenuItemCount;break;
                case SDLK_UP:selected=(selected+ldm::MenuItemCount-1)%ldm::MenuItemCount;break;
                case SDLK_DOWN:selected=(selected+1)%ldm::MenuItemCount;break;
                case SDLK_LEFT:if(selected<ldm::Comfort)action(selected,-1);else selected=selected==ldm::Comfort?ldm::Apply:selected-1;break;
                case SDLK_RIGHT:if(selected<ldm::Comfort)action(selected,1);else selected=selected==ldm::Apply?ldm::Comfort:selected+1;break;
                default:break;
                }
                return;
            }
            if(pressed && key.keysym.sym==SDLK_F11){if(!key.repeat)open_menu();return;}
            auto scan=key.keysym.scancode;
            if(scan>SDL_SCANCODE_UNKNOWN && scan<SDL_NUM_SCANCODES)held[scan]=pressed;
            if(scan==SDL_SCANCODE_SPACE)session->space(pressed);
            session->directions(movement(held));auto bios=ldm::input::key_event(key);
            if(pressed) {
                if(key.keysym.sym==SDLK_RETURN && (key.keysym.mod&KMOD_ALT)) {
                    if(!key.repeat){settings.window=settings.window==3?last_window:3;view.apply(settings,true);release_input();}
                }else if(bios)session->key(bios);
            }else if(direction(scan) || scan==SDL_SCANCODE_SPACE)session->release_repeat(scan);
        };
        auto pointer=[&](int wx,int wy,bool press,int button,bool motion) {
            if(menu) {
                bool left=false;int hit=view.menu_hit(wx,wy,left);
                if(hit>=0){selected=hit;if(press && button==SDL_BUTTON_LEFT)action(hit,left?-1:1);}
                return;
            }
            int x=0,y=0;bool inside=view.game_point(wx,wy,settings,x,y);
            if(inside || motion)session->mouse(x,y);
            int bit=button==SDL_BUTTON_LEFT?1:button==SDL_BUTTON_RIGHT?2:0;
            if(bit) {
                if(press && inside)mouse_buttons|=bit;else mouse_buttons&=~bit;
                session->buttons(mouse_buttons);
            }
        };
        uint64_t start=SDL_GetPerformanceCounter();double frequency=double(SDL_GetPerformanceFrequency());
        double next_frame=0;auto storage=std::make_unique<ldm::Snapshot>();auto& frame=*storage;
        while(!quit) {
            auto now=SDL_GetPerformanceCounter();double elapsed_ms=(now-start)*1000/frequency;
            uint64_t elapsed=uint64_t(elapsed_ms);
            session->snapshot(frame); // Also propagates precise native diagnostics.
            if((duration && elapsed>=duration) || session->finished())break;
            if(warmup && elapsed>=350 && frame.video_mode==0x13){session->pause(true);warmup=false;}
            SDL_Event e;
            while(SDL_PollEvent(&e)) {
                if(e.type==SDL_QUIT)quit=true;
                else if(e.type==SDL_KEYDOWN || e.type==SDL_KEYUP)keyboard(e.key,e.type==SDL_KEYDOWN);
                else if(e.type==SDL_WINDOWEVENT && e.window.event==SDL_WINDOWEVENT_FOCUS_LOST)release_input();
                else if(e.type==SDL_MOUSEMOTION)pointer(e.motion.x,e.motion.y,false,0,true);
                else if(e.type==SDL_MOUSEBUTTONDOWN || e.type==SDL_MOUSEBUTTONUP)pointer(e.button.x,e.button.y,e.type==SDL_MOUSEBUTTONDOWN,e.button.button,false);
            }
            std::filesystem::path screen_capture;
            while(event_pos<events.size() && events[event_pos].time<=elapsed) {
                auto event=events[event_pos++];
                if(event.type=="key")session->key(uint16_t(event.a));
                else if(event.type=="down" || event.type=="up" || event.type=="repeat") {
                    if(event.a<=0 || event.a>=SDL_NUM_SCANCODES)throw std::runtime_error("Invalid scripted scancode");
                    SDL_KeyboardEvent key{};key.keysym.scancode=SDL_Scancode(event.a);
                    key.keysym.sym=SDL_GetKeyFromScancode(key.keysym.scancode);key.repeat=event.type=="repeat";
                    key.keysym.mod=Uint16(event.b);
                    keyboard(key,event.type!="up");
                }else if(event.type=="focuslost")release_input();
                else if(event.type=="ascii") {
                    SDL_KeyboardEvent key{};key.keysym.sym=event.a;
                    if(event.a>='A'&&event.a<='Z'){key.keysym.sym+=32;key.keysym.mod=KMOD_SHIFT;}
                    session->key(keycode(key));
                }else if(event.type=="mouse")session->mouse(event.a,event.b);
                else if(event.type=="buttons")session->buttons(event.a);
                else if(event.type=="click")pointer(event.a,event.b,true,SDL_BUTTON_LEFT,false);
                else if(event.type=="screen")screen_capture=std::filesystem::path("captures")/(std::to_string(event.a)+"-display.bmp");
                else if(event.type=="capture") {
                    capture(frame.pixels,std::filesystem::path("captures")/(std::to_string(event.a)+".bmp"));
                    std::ofstream meta(std::filesystem::path("captures")/(std::to_string(event.a)+".json"));
                    meta<<"{\"x\":"<<frame.x<<",\"y\":"<<frame.y<<",\"town_page\":"<<frame.town_page<<",\"building\":"<<frame.building
                        <<",\"video_mode\":"<<frame.video_mode<<",\"held_directions\":"<<unsigned(frame.directions)<<",\"boundaries\":"<<frame.boundaries
                        <<",\"mouse_visibility\":"<<frame.mouse_visibility<<",\"mouse_mode\":"<<frame.mouse_mode
                        <<",\"mouse_x\":"<<frame.mouse_x<<",\"mouse_y\":"<<frame.mouse_y
                        <<",\"pointer_visible\":"<<frame.pointer_visible<<",\"desert_view\":"<<frame.desert_view
                        <<",\"map_view\":"<<frame.map_view<<",\"survival_ticks\":"<<frame.survival_ticks
                        <<",\"cave_view\":"<<frame.cave_view<<",\"map_scroll_x\":"<<frame.map_scroll_x<<",\"map_scroll_y\":"<<frame.map_scroll_y
                        <<",\"return_x\":"<<frame.return_x<<",\"return_y\":"<<frame.return_y
                        <<",\"panning_active\":"<<frame.panning_active
                        <<",\"gold_bags\":"<<frame.gold_bags<<",\"qol\":"<<frame.qol_improvements
                        <<",\"cash\":"<<frame.cash<<",\"assay_pounds\":"<<frame.assay_pounds<<",\"assay_grade\":"<<frame.assay_grade
                        <<",\"combat\":"<<frame.combat_active<<",\"bullets\":"<<frame.bullets
                        <<",\"sight_x\":"<<frame.sight_x<<",\"sight_y\":"<<frame.sight_y
                        <<",\"mining_space_held\":"<<frame.mining_space_held<<",\"mining_strokes\":"<<frame.mining_strokes<<"}\n";
                }else throw std::runtime_error("Unknown script event");
            }
            if(elapsed_ms>=next_frame || !screen_capture.empty()) {
                SDL_ShowCursor(menu || !frame.custom_cursor?SDL_ENABLE:SDL_DISABLE);
                if(menu)view.menu(frame.pixels,draft,startup_menu,selected,message);else view.game(frame.pixels,settings);
                if(!screen_capture.empty())view.capture(screen_capture);
                SDL_RenderPresent(renderer);
                double interval=1000.0/std::clamp(view.refresh_rate(),30,240);
                next_frame=std::max(next_frame+interval,elapsed_ms+interval*.1);
            }
            SDL_Delay(1);
        }
        session->stop();session->snapshot(frame);
        if(!screenshot.empty()) {
            if(menu)view.menu(frame.pixels,draft,startup_menu,selected,message);else view.game(frame.pixels,settings);
            view.capture(screenshot);
        }
        if(duration || !script.empty()){ldm::read_frame(*s,frame.pixels);capture(frame.pixels,"captures/last-frame.bmp");}
        std::cerr<<"Native runtime: "<<s->boundaries<<" boundaries, "<<s->ticks<<" BIOS ticks, mode "<<s->video_mode
                 <<", PIT="<<s->pit_divisor<<", timer="<<std::hex<<s->u16(0,0x22)<<":"<<s->u16(0,0x20)
                 <<", CS:IP="<<s->cs-ldm::LoadSegment<<":"<<s->ip<<", clock="<<s->u16(0x2b14,0xfbc4)<<":"<<s->u16(0x2b14,0xfbc2)<<std::dec
                 <<", FM writes="<<s->audio.writes()<<", audible samples="<<s->audio.audible_samples()<<"\n";
    }catch(const std::exception&e) {
        if(session)session->stop();
        std::cerr<<e.what()<<"\n";
        if(duration || !script.empty()) {
            std::filesystem::create_directories(".local");
            std::ofstream(".local/memory.bin",std::ios::binary).write(reinterpret_cast<const char*>(s->memory.data()),s->memory.size());
        }
        if(!duration && script.empty())SDL_ShowSimpleMessageBox(SDL_MESSAGEBOX_ERROR,"Lost Dutchman Mine",e.what(),window);
        if(audio)SDL_CloseAudioDevice(audio);
        SDL_Quit();return 1;
    }
    if(audio)SDL_CloseAudioDevice(audio);
    if(renderer)SDL_DestroyRenderer(renderer);
    if(window)SDL_DestroyWindow(window);
    SDL_Quit();return 0;
}

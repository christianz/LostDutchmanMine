#pragma once
#include "display.h"
#include <SDL.h>
#include <memory>
#include <string>

namespace ldm {
class Presentation {
public:
    Presentation(SDL_Window* window,SDL_Renderer* renderer,const std::filesystem::path& app);
    ~Presentation();
    void apply(const DisplaySettings& settings,bool resize);
    void game(const Pixels& pixels,const DisplaySettings& settings);
    void menu(const Pixels& pixels,const DisplaySettings& draft,bool startup,int selected,const std::string& message);
    // Coordinates from SDL events are window units; the renderer may use more
    // physical pixels on a high-DPI monitor. No automatic SDL logical transform.
    bool game_point(int window_x,int window_y,const DisplaySettings& settings,int& x,int& y);
    int menu_hit(int window_x,int window_y,bool& left);
    void capture(const std::filesystem::path& file);
    int refresh_rate() const;
private:
    SDL_Window* window_;
    SDL_Renderer* renderer_;
    SDL_Texture* texture_=nullptr,*font_=nullptr,*icon_=nullptr;
    int texture_w_=0,texture_h_=0;
    std::vector<uint32_t> processed_;
    std::unique_ptr<Pixels> previous_=std::make_unique<Pixels>();
    int previous_style_=-1;
    float ui_scale_=1,ui_x_=0,ui_y_=0;
    bool vsync_available_=true;
    void upload(const Pixels& pixels,const DisplaySettings& settings);
    SDL_FRect ui_rect(float x,float y,float w,float h) const;
    void box(float x,float y,float w,float h,uint32_t colour,bool outline=false);
    void text(float x,float y,float size,const std::string& value,uint32_t colour,bool centre=false);
    void ui_layout();
    void physical_point(int wx,int wy,int& px,int& py);
};
void change_setting(DisplaySettings& settings,int row,int direction);
DisplaySettings comfort_settings(bool startup);
DisplaySettings original_settings(bool startup);
}

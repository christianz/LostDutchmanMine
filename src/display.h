#pragma once
#include <array>
#include <cstdint>
#include <filesystem>
#include <string>
#include <vector>

namespace ldm {
using Pixels = std::array<uint32_t,320*200>;
enum class Scaling { Crisp, Soft, PixelArt };
enum class Colour { Original, Warm, Vivid, Gentle };
enum class Crt { Off, Soft, Classic };
struct DisplaySettings {
    int window=1; // 960x720, 1280x960, 1600x1200, desktop fullscreen
    int size=100; // Percentage of the available 4:3 display area
    Scaling scaling=Scaling::Soft;
    Colour colour=Colour::Original;
    Crt crt=Crt::Off;
    int brightness=100;
    bool vsync=true, startup=true, qol=true;
};
DisplaySettings load_settings(const std::filesystem::path& file);
void save_settings(const std::filesystem::path& file,const DisplaySettings& settings);
struct Rect {int x=0,y=0,w=0,h=0;};
Rect picture_rect(int width,int height,int percent);
bool picture_point(const Rect& rect,int px,int py,int& x,int& y);
uint32_t colour_pixel(uint32_t pixel,const DisplaySettings& settings);
// Spatial processing only: never changes the game's palette, pixels or timing.
void display_pixels(const Pixels& input,const DisplaySettings& settings,std::vector<uint32_t>& output,int& width,int& height);
// A steady display-space phosphor/scanline mask, cached until size/style change.
// A separate blurred highlight layer supplies the phosphor glow.
void crt_mask(Crt effect,int width,int height,std::vector<uint32_t>& output);
void crt_glow(const std::vector<uint32_t>& picture,int width,int height,Pixels& output);
}

# Panning artwork

`creek-source.png` is new artwork generated with the built-in image-generation tool
on 2026-09-23. `creek.ppm` is the 320 x 112, 24-colour runtime bitmap, reduced with
nearest-neighbour sampling. It is copied to `panning-creek.ppm` beside the executable.
The animated pan, gravel, flakes, water, hands, controls and 5x7 lettering are drawn
as native pixels by `src/panning.cpp` and `src/pixel_art.h`. No original game asset
is changed. See `prompt.txt` for the generation prompt.

Lost Dutchman Mine - native development build

{launch}
Keep Game, Saves and the executable together. On Windows, copy the whole folder
to your PC before running. No installation or administrator access is needed.

Choose your display settings, then Play. The Comfort button selects desktop
fullscreen, an 85% centred picture, soft pixel edges and gentle colours.
Display offers three windows and Fullscreen, Fullscreen 85% or Fullscreen 70%.
F11 reopens display settings during play, pausing the game and music. Settings
are saved in display.ini beside the executable; keep this file when updating.
Set Show at startup to No to skip the menu next time. F11 always remains available.
CRT monitor offers Off, Subtle and Strong: steady scanlines, phosphor texture,
soft glow and gentle edge shading. Preview it in the menu; the choice is saved.

QoL improvements enables clearer menus, an arrow pointer and smooth mouse aiming,
on by default. The six named toolbar buttons fit below the full logo.
Hover for a gold outline and an F1-F6 shortcut hint.
Menus suspend hover on the buttons underneath and restore it when closed.
The black saloon sleep screen also suspends hover until you wake up.
Health and food icons retain their live warning colours and critical-health
flashing. Context buttons also accept clicks on their bevels. Turn QoL improvements off
to restore the original panel and click targets.
The arrow stays visible while walking: keyboard and mouse work together,
and keyboard movement leaves the pointer where you put it.
One click selects a toolbar or context action. Held movement in town,
the saloon and mines is faster; survival time and mine hazard checks keep their
original pace. Space opens a desert close-up; Space, Enter or Escape closes it.
Only the cheapest unowned mule is available to buy. The others show SOLD OUT;
buying the available one unlocks the next. Existing inventory rows are preserved.
Loading inside the saloon preserves the correct street position for Exit,
whether QoL is on or off.
Mouse aiming is ready when an armed fight starts. The original sight follows
mouse motion between encounter ticks. Move to aim and left-click
to fire one shot. The click that entered the fight does not fire or reopen a menu.
Right-click opens the pointer for Run/status/menu choices; a direction key returns
to aiming. WASD, arrows and numpad still aim; Space still fires. A stationary mouse
does not undo keyboard aiming. Ammunition and hit rules stay the same. Turn
QoL improvements off to restore the original keyboard aiming/mouse selection behavior.

At a river, Pan requires a pan in your pack or an owned mule's inventory.
Discarding your last pan prevents further panning. The prospector plays the original
panning animation in the river scene, then puts one gold bag in your pack. The
original sprites, timing and river's gold grade are preserved. F11 pauses the
animation. The restored action applies with QoL on or off; changing display
preferences cannot interrupt it. The custom rock-and-wash minigame is retired.
Hold Space while using the pick to keep mining at the original stroke rate;
release to stop. This works in both modes. Focus loss and settings release it.

Windows launches only the game window. Startup failures still show an error dialog.

VGA starts automatically. Allow the original title/credits sequence to finish.
Hold WASD, cursor keys or the numeric keypad to move; release to stop.
Numpad 8/2/4/6 move up/down/left/right; 7/9/1/3 move diagonally. Num Lock may be
on or off for movement. Combine WASD keys for diagonals; the world map retains
both movement axes while keys repeat. With Num Lock on, the
keypad enters numbers and selects save slots. Numpad Enter confirms. WASD letters
still type normally in save names. WASD/numpad also navigate display settings.
A movement key returns to walking when the hand cursor is active.
Use the mouse for choices,
F1-F6 for the status panel, Space for action, Alt+Enter for fullscreen.
To enter a building, align with its doorway and hold Up to walk inside.
Save and load through F6. New saves go in Saves; the supplied originals in Game
are read only. Back up Saves when moving or updating this build.
Short mouse clicks are now preserved, including the click used to focus the
window. With QoL off, one click restores the hand after keyboard movement.

This is a development build of a faithful port, not a fully validated release.
Windows x64 is cross-compiled; independent Windows 11 testing remains outstanding.
Linux runtime, 4K rendering, display menus, movement and saves have been tested.
See VALIDATION.md for the exact coverage and remaining work.

The original instructions are translated to Rust at build time and compiled to
native machine code. There is no DOSBox, virtual machine, DOS kernel or runtime
CPU interpreter. Original data layouts and game logic are retained. SDL2 handles
desktop services; ymfm synthesizes the original AdLib sound chip.

This private bundle includes your original game data. Original game copyright
remains with its owners. SDL2, ymfm and DejaVu font notices are in licenses.

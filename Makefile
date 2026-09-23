CXX ?= g++
CXXFLAGS ?= -std=c++17 -O1 -Wall -Wextra -pthread
CPPFLAGS += -Isrc -Ibuild/generated -Ithird_party/ymfm
SDL_INCLUDE ?= .local/deps/SDL2-2.32.0/x86_64-w64-mingw32/include/SDL2
SDL_LIBS ?= -l:libSDL2-2.0.so.0
GENERATED = $(wildcard build/generated/*.cpp)
OBJECTS = $(patsubst %.cpp,%.o,$(GENERATED)) build/legacy.o build/audio.o build/ymfm_opl.o build/ymfm_adpcm.o build/ymfm_pcm.o build/probe.o

.PHONY: all recover clean
.DELETE_ON_ERROR:
all: build/ldm-native
recover:
	.venv/bin/python tools/unpack.py $(DATA)/LDM.EXE
	.venv/bin/python tools/analyze.py
	.venv/bin/python tools/translate.py
build/ldm-probe: $(OBJECTS)
	$(CXX) $(CXXFLAGS) $^ -o $@
build/test-assets: $(filter-out build/probe.o,$(OBJECTS)) build/assets.o tests/assets.cpp
	$(CXX) $(CPPFLAGS) $(CXXFLAGS) $^ -o $@
build/test-arithmetic: $(filter-out build/probe.o,$(OBJECTS)) tests/arithmetic.cpp
	$(CXX) $(CPPFLAGS) $(CXXFLAGS) $^ -o $@
build/test-poker: $(filter-out build/probe.o,$(OBJECTS)) tests/poker.cpp
	$(CXX) $(CPPFLAGS) $(CXXFLAGS) $^ -o $@
build/test-quit: $(filter-out build/probe.o,$(OBJECTS)) tests/quit.cpp
	$(CXX) $(CPPFLAGS) $(CXXFLAGS) $^ -o $@
build/test-mouse: $(filter-out build/probe.o,$(OBJECTS)) tests/mouse.cpp
	$(CXX) $(CPPFLAGS) $(CXXFLAGS) $^ -o $@
build/test-keyboard: $(filter-out build/probe.o,$(OBJECTS)) tests/keyboard.cpp src/keyboard.h src/keyboard_event.h
	$(CXX) $(CPPFLAGS) -I$(SDL_INCLUDE) $(CXXFLAGS) $(filter %.o %.cpp,$^) -o $@
build/legacy.o: build/generated/image_info.h
build/ldm-native: $(filter-out build/probe.o,$(OBJECTS)) build/desktop.o build/session.o build/display.o build/presentation.o | build/LostDutchmanMine.bmp build/ui-font.bmp
	$(CXX) $(CXXFLAGS) $^ $(SDL_LIBS) -o $@
build/LostDutchmanMine.bmp: resources/ldm-icon.bmp
	cp $< $@
build/ui-font.bmp: resources/ui-font.bmp
	cp $< $@
build/desktop.o: src/desktop.cpp src/legacy.h
	$(CXX) $(CPPFLAGS) -I$(SDL_INCLUDE) $(CXXFLAGS) -MMD -MP -c $< -o $@
build/presentation.o: src/presentation.cpp src/presentation.h src/ui-font.h
	$(CXX) $(CPPFLAGS) -I$(SDL_INCLUDE) $(CXXFLAGS) -MMD -MP -c $< -o $@
build/test-display: src/display.cpp tests/display.cpp
	$(CXX) $(CPPFLAGS) $(CXXFLAGS) $^ -o $@
build/generated/%.o: build/generated/%.cpp src/legacy.h
	$(CXX) $(CPPFLAGS) $(CXXFLAGS) -MMD -MP -c $< -o $@
build/ymfm_%.o: third_party/ymfm/ymfm_%.cpp
	$(CXX) $(CPPFLAGS) -std=c++17 -O2 -c $< -o $@
build/%.o: src/%.cpp src/legacy.h
	$(CXX) $(CPPFLAGS) $(CXXFLAGS) -MMD -MP -c $< -o $@
-include $(wildcard build/*.d build/generated/*.d)
clean:
	rm -rf build

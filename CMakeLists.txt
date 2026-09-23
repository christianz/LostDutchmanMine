cmake_minimum_required(VERSION 3.16)
project(LostDutchmanMine LANGUAGES CXX)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Generate the translated game first; see README. Python is a build dependency,
# never a runtime dependency. Generated game code stays outside source control.
file(GLOB LDM_GENERATED CONFIGURE_DEPENDS "${CMAKE_SOURCE_DIR}/build/generated/*.cpp")
if(NOT LDM_GENERATED)
    message(FATAL_ERROR "Run tools/unpack.py and tools/translate.py first (see README).")
endif()
find_package(SDL2 REQUIRED)
find_package(Threads REQUIRED)
add_library(ldm-core STATIC ${LDM_GENERATED} src/legacy.cpp src/audio.cpp
    third_party/ymfm/ymfm_opl.cpp third_party/ymfm/ymfm_adpcm.cpp third_party/ymfm/ymfm_pcm.cpp)
target_include_directories(ldm-core PUBLIC src build/generated PRIVATE third_party/ymfm)
target_link_libraries(ldm-core PUBLIC Threads::Threads)
add_executable(LostDutchmanMine src/desktop.cpp)
target_link_libraries(LostDutchmanMine PRIVATE ldm-core SDL2::SDL2)
add_custom_command(TARGET LostDutchmanMine POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E copy_if_different
    "${CMAKE_SOURCE_DIR}/resources/ldm-icon.bmp"
    "$<TARGET_FILE_DIR:LostDutchmanMine>/LostDutchmanMine.bmp")
if(WIN32)
    enable_language(RC)
    target_sources(LostDutchmanMine PRIVATE resources/windows.rc)
    target_include_directories(LostDutchmanMine PRIVATE resources)
    add_custom_command(TARGET LostDutchmanMine POST_BUILD
        COMMAND ${CMAKE_COMMAND} -E copy_if_different
        $<TARGET_FILE:SDL2::SDL2> $<TARGET_FILE_DIR:LostDutchmanMine>)
endif()
add_executable(ldm-probe src/probe.cpp)
target_link_libraries(ldm-probe PRIVATE ldm-core)
add_executable(test-arithmetic tests/arithmetic.cpp)
target_link_libraries(test-arithmetic PRIVATE ldm-core)
add_executable(test-poker tests/poker.cpp)
target_link_libraries(test-poker PRIVATE ldm-core)
add_executable(test-assets tests/assets.cpp src/assets.cpp)
target_link_libraries(test-assets PRIVATE ldm-core)
enable_testing()
add_test(NAME arithmetic COMMAND test-arithmetic)
add_test(NAME poker COMMAND test-poker)
set_tests_properties(poker PROPERTIES WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}")

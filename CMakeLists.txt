cmake_minimum_required(VERSION 3.16)

project(sprite_gen C)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)
set(CMAKE_C_EXTENSIONS OFF)

find_package(SDL2 REQUIRED)

file(GLOB_RECURSE LVGL_SOURCES CONFIGURE_DEPENDS
    lvgl/src/*.c
)

set(LVGL_INCLUDE_DIRS
    ${CMAKE_CURRENT_SOURCE_DIR}
    ${CMAKE_CURRENT_SOURCE_DIR}/lvgl
    ${CMAKE_CURRENT_SOURCE_DIR}/lvgl/src
    ${CMAKE_CURRENT_SOURCE_DIR}/lvgl/src/core
    ${CMAKE_CURRENT_SOURCE_DIR}/lvgl/src/draw
    ${CMAKE_CURRENT_SOURCE_DIR}/lvgl/src/font
    ${CMAKE_CURRENT_SOURCE_DIR}/lvgl/src/libs
    ${CMAKE_CURRENT_SOURCE_DIR}/lvgl/src/misc
    ${CMAKE_CURRENT_SOURCE_DIR}/lvgl/src/osal
    ${CMAKE_CURRENT_SOURCE_DIR}/lvgl/src/layouts
    ${CMAKE_CURRENT_SOURCE_DIR}/lvgl/src/stdlib
    ${CMAKE_CURRENT_SOURCE_DIR}/lvgl/src/display
    ${CMAKE_CURRENT_SOURCE_DIR}/lvgl/src/widgets
    ${CMAKE_CURRENT_SOURCE_DIR}/lvgl/src/indev
    ${CMAKE_CURRENT_SOURCE_DIR}/lvgl/src/tick
    ${CMAKE_CURRENT_SOURCE_DIR}/lvgl/src/draw/sw
    ${CMAKE_CURRENT_SOURCE_DIR}/lvgl/src/draw/sw/blend
)

set(LVGL_WARNING_FLAGS
    -Wall
    -Wextra
    -Wno-unused-parameter
    -Wno-unused-function
)

set(LVGL_LIBS
    SDL2::SDL2
    m
)

add_executable(sprite_gen
    main.c
    ${LVGL_SOURCES}
)

target_include_directories(sprite_gen PRIVATE ${LVGL_INCLUDE_DIRS})
target_compile_definitions(sprite_gen PRIVATE LV_CONF_INCLUDE_SIMPLE)
target_compile_options(sprite_gen PRIVATE ${LVGL_WARNING_FLAGS})
target_link_libraries(sprite_gen PRIVATE ${LVGL_LIBS})

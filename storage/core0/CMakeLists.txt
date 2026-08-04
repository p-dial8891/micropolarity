# == DO NOT EDIT THE FOLLOWING LINES for the Raspberry Pi Pico VS Code Extension to work ==
if(WIN32)
    set(USERHOME $ENV{USERPROFILE})
else()
    set(USERHOME $ENV{HOME})
endif()
set(sdkVersion 2.2.0)
set(toolchainVersion 13_2_Rel1)
set(picotoolVersion 2.2.0-a4)
set(picoVscode ${USERHOME}/.pico-sdk/cmake/pico-vscode.cmake)
if (EXISTS ${picoVscode})
    include(${picoVscode})
endif()
# ====================================================================================
# Generated Cmake Pico project file

cmake_minimum_required(VERSION 3.5)

set(PICO_BOARD pico2_w CACHE STRING "Board type")
set(LV_BUILD_CONF_PATH ./lv_conf.h)
set(CONFIG_LV_BUILD_EXAMPLES true)

# Pull in Pico SDK (must be before project)
include(pico_sdk_import.cmake)

project(core0 C CXX ASM)

# Initialise the Raspberry Pi Pico SDK
pico_sdk_init()

add_subdirectory(../no-OS-FatFS-SD-SDIO-SPI-RPi-Pico/src build)
add_subdirectory(../lvgl lvgl)
include_directories(../)

# Add executable. Default name is the project name, version 0.1
add_executable(core0 main.cpp ring_buffer.cpp messaging.cpp display.cpp runtime.cpp)

# Can leave these off for silent mode:
# add_compile_definitions(USE_PRINTF USE_DBG_PRINTF)
add_compile_definitions(USE_PRINTF)

pico_set_linker_script(core0 ${CMAKE_CURRENT_SOURCE_DIR}/memmap_core0.ld)

target_compile_definitions(core0 PRIVATE
    PICO_FLASH_SIZE_BYTES=\(2*1024*1024\)
)

# Add the standard library, multicore and FatFS/SPI to the build
target_link_libraries(core0 
    pico_stdlib
    pico_multicore
    hardware_sync
    hardware_spi
    no-OS-FatFS-SD-SDIO-SPI-RPi-Pico
    lvgl
)

pico_set_program_name(core0 "core0")
pico_set_program_version(core0 "0.1")

# Choose source and destination for standard input and output:
#   See 4.1. Serial input and output on Raspberry Pi Pico in Getting started with Raspberry Pi Pico (https://datasheets.raspberrypi.org/pico/getting-started-with-pico.pdf)
#   and 2.7.1. Standard Input/Output (stdio) Support in Raspberry Pi Pico C/C++ SDK (https://datasheets.raspberrypi.org/pico/raspberry-pi-pico-c-sdk.pdf):
pico_enable_stdio_uart(core0 1)
pico_enable_stdio_usb(core0 0)

pico_add_extra_outputs(core0)

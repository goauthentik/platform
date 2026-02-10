# Copyright (c) 2014 The Chromium Embedded Framework Authors. All rights
# reserved. Use of this source code is governed by a BSD-style license that
# can be found in the LICENSE file.

# OVERVIEW
#
# CMake is a cross-platform open-source build system that can generate project
# files in many different formats. It can be downloaded from
# http://www.cmake.org or installed via a platform package manager.
#
# CMake-generated project formats that have been tested with this CEF binary
# distribution include:
#
# Linux:      Ninja, GCC 7.5.0+, Unix Makefiles
# MacOS:      Ninja, Xcode 12.2 to 15.0
# Windows:    Ninja, Visual Studio 2022
#
# Ninja is a cross-platform open-source tool for running fast builds using
# pre-installed platform toolchains (GNU, clang, Xcode or MSVC). It can be
# downloaded from http://martine.github.io/ninja/ or installed via a platform
# package manager.
#
# CMAKE STRUCTURE
#
# This CEF binary distribution includes the following CMake files:
#
# CMakeLists.txt              Bootstrap that sets up the CMake environment.
# cmake/*.cmake               CEF configuration files shared by all targets.
# libcef_dll/CMakeLists.txt   Defines the ak_cred_provider target.
# tests/*/CMakeLists.txt      Defines the test application target.
#
# See the "TODO:" comments below for guidance on how to integrate this CEF
# binary distribution into a new or existing CMake project.
#
# BUILD REQUIREMENTS
#
# The below requirements must be met to build this CEF binary distribution.
#
# - CMake version 3.21 or newer.
#
# - Linux requirements:
#   Currently supported distributions include Debian 10 (Buster), Ubuntu 18
#   (Bionic Beaver), and related. Ubuntu 18.04 64-bit with GCC 7.5.0+ is
#   recommended. Newer versions will likely also work but may not have been
#   tested.
#   Required packages include:
#     build-essential
#     libgtk3.0-dev     (required by the cefclient target only)
#
# - MacOS requirements:
#   Xcode 12.2 to 15.4 building on MacOS 11.0 (Big Sur) or newer. The Xcode
#   command-line tools must also be installed. Newer Xcode versions may not have
#   been been tested and are not recommended.
#
# - Windows requirements:
#   Visual Studio 2022 building on Windows 10 or newer. Windows 10/11 64-bit is
#   recommended. Newer versions will likely also work but may not have been
#   tested.
#
# BUILD EXAMPLES
#
# The below commands will generate project files and create a Debug build of all
# CEF targets using CMake and the platform toolchain.
#
# Start by creating and entering the CMake build output directory:
# > cd path/to/cef_binary_*
# > mkdir build && cd build
#
# To perform a Linux build using a 32-bit CEF binary distribution on a 32-bit
# Linux platform or a 64-bit CEF binary distribution on a 64-bit Linux platform:
#   Using Unix Makefiles:
#     > cmake -G "Unix Makefiles" -DCMAKE_BUILD_TYPE=Debug ..
#     > make -j4 cefclient cefsimple
#
#   Using Ninja:
#     > cmake -G "Ninja" -DCMAKE_BUILD_TYPE=Debug ..
#     > ninja cefclient cefsimple
#
# To perform a MacOS build using a 64-bit CEF binary distribution:
#   Using the Xcode IDE:
#     > cmake -G "Xcode" -DPROJECT_ARCH="x86_64" ..
#     Open build\cef.xcodeproj in Xcode and select Product > Build.
#
#   Using Ninja:
#     > cmake -G "Ninja" -DPROJECT_ARCH="x86_64" -DCMAKE_BUILD_TYPE=Debug ..
#     > ninja cefclient cefsimple
#
# To perform a MacOS build using an ARM64 CEF binary distribution:
#   Using the Xcode IDE:
#     > cmake -G "Xcode" -DPROJECT_ARCH="arm64" ..
#     Open build\cef.xcodeproj in Xcode and select Product > Build.
#
#   Using Ninja:
#     > cmake -G "Ninja" -DPROJECT_ARCH="arm64" -DCMAKE_BUILD_TYPE=Debug ..
#     > ninja cefclient cefsimple
#
# To perform a Windows build using a 32-bit CEF binary distribution:
#   Using the Visual Studio 2022 IDE:
#     > cmake -G "Visual Studio 17" -A Win32 ..
#     Open build\cef.sln in Visual Studio and select Build > Build Solution.
#
#   Using Ninja with Visual Studio 2022 command-line tools:
#     (this path may be different depending on your Visual Studio installation)
#     > "C:\Program Files\Microsoft Visual Studio\2022\Professional\VC\Auxiliary\Build\vcvars32.bat"
#     > cmake -G "Ninja" -DCMAKE_BUILD_TYPE=Debug ..
#     > ninja cefclient cefsimple
#
# To perform a Windows build using a 64-bit CEF binary distribution:
#   Using the Visual Studio 2022 IDE:
#     > cmake -G "Visual Studio 17" -A x64 ..
#     Open build\cef.sln in Visual Studio and select Build > Build Solution.
#
#   Using Ninja with Visual Studio 2022 command-line tools:
#     (this path may be different depending on your Visual Studio installation)
#     > "C:\Program Files\Microsoft Visual Studio\2022\Professional\VC\Auxiliary\Build\vcvars64.bat"
#     > cmake -G "Ninja" -DCMAKE_BUILD_TYPE=Debug ..
#     > ninja cefclient cefsimple
#
# To perform a Windows build using an ARM64 CEF binary distribution:
#   Using the Visual Studio 2022 IDE:
#     > cmake -G "Visual Studio 17" -A arm64 ..
#     Open build\cef.sln in Visual Studio and select Build > Build Solution.
#
#   Using Ninja with Visual Studio 2022 command-line tools:
#     (this path may be different depending on your Visual Studio installation)
#     > "C:\Program Files\Microsoft Visual Studio\2022\Professional\VC\Auxiliary\Build\vcvarsamd64_arm64.bat"
#     > cmake -G "Ninja" -DCMAKE_BUILD_TYPE=Debug ..
#     > ninja cefsimple

#
# Global setup.
#

# For VS2022 and Xcode 12+ support.
cmake_minimum_required(VERSION 3.21)

# Only generate Debug and Release configuration types.
set(CMAKE_CONFIGURATION_TYPES Debug Release)

# Project name.
# TODO: Change this line to match your project name when you copy this file.
project(cef)

# Use folders in the resulting project files.
set_property(GLOBAL PROPERTY USE_FOLDERS ON)

#
# CEF_ROOT setup.
# This variable must be set to locate the binary distribution.
# TODO: Choose one of the below examples and comment out the rest.
#

# Example 1: The current directory contains both the complete binary
#            distribution and your project.
# A. Comment in these lines:
#
set(CEF_ROOT "${CMAKE_CURRENT_SOURCE_DIR}")
set(CMAKE_MODULE_PATH ${CMAKE_MODULE_PATH} "${CEF_ROOT}/cmake")

# Example 2: The binary distribution is in a separate directory from your
#            project. Locate the binary distribution using the CEF_ROOT CMake
#            variable.
# A. Create a directory structure for your project like the following:
#    myproject/
#      CMakeLists.txt    <= top-level CMake configuration
#      mytarget/
#        CMakeLists.txt  <= CMake configuration for `mytarget`
#        ... other `mytarget` source files
# B. Copy this file to "myproject/CMakeLists.txt" as the top-level CMake
#    configuration.
# C. Create the target-specific "myproject/mytarget/CMakeLists.txt" file for
#    your application. See the included cefclient and cefsimple CMakeLists.txt
#    files as an example.
# D. Comment in these lines:
#
# set(CEF_ROOT "c:/path/to/cef_binary_3.2704.xxxx.gyyyyyyy_windows32")
# set(CMAKE_MODULE_PATH ${CMAKE_MODULE_PATH} "${CEF_ROOT}/cmake")

# Example 3: The binary distribution is in a separate directory from your
#            project. Locate the binary distribution using the CEF_ROOT
#            environment variable.
# A. Create a directory structure for your project like the following:
#    myproject/
#      CMakeLists.txt    <= top-level CMake configuration
#      cmake/
#        FindCEF.cmake   <= CEF CMake configuration entry point
#      mytarget/
#        CMakeLists.txt  <= CMake configuration for `mytarget`
#        ... other `mytarget` source files
# B. Copy this file to "myproject/CMakeLists.txt" as the top-level CMake
#    configuration.
# C. Copy the cmake/FindCEF.cmake file to "myproject/cmake/FindCEF.cmake".
# D. Create the target-specific "myproject/mytarget/CMakeLists.txt" file for
#    your application. See the included cefclient and cefsimple CMakeLists.txt
#    files as an example.
# E. Set the CEF_ROOT environment variable before executing CMake. For example:
#    > set CEF_ROOT=c:\path\to\cef_binary_3.2704.xxxx.gyyyyyyy_windows32
# F. Comment in these lines:
#
# set(CMAKE_MODULE_PATH ${CMAKE_MODULE_PATH} "${CMAKE_CURRENT_SOURCE_DIR}/cmake")


#
# Load the CEF configuration.
#

# Execute FindCEF.cmake which must exist in CMAKE_MODULE_PATH.
find_package(CEF REQUIRED)

set(CMAKE_CXX_STANDARD 17)

# Download CEF binaries
if (NOT EXISTS "${CMAKE_CURRENT_SOURCE_DIR}/Release")
  message(INFO " > CEF binary `Release` directory is missing.")
  message(INFO " > Downloading CEF binary package...")
  file(DOWNLOAD "https://cef-builds.spotifycdn.com/cef_binary_134.3.2%2Bg615db2f%2Bchromium-134.0.6998.89_windows64.tar.bz2" "${CMAKE_CURRENT_BINARY_DIR}/cef_binary_134.3.2%2Bg615db2f%2Bchromium-134.0.6998.89_windows64.tar.bz2.tar.bz2"
    EXPECTED_HASH SHA256=856cccd8f8b7ebd4cabad7a7ce1bd7596c18bba641bb0f2eae4d3ee51b3c7265
    SHOW_PROGRESS
  )
  message(INFO " > Extracting CEF binary package...")
  file(ARCHIVE_EXTRACT INPUT "cef_binary_134.3.2%2Bg615db2f%2Bchromium-134.0.6998.89_windows64.tar.bz2.tar.bz2")
  message(INFO " > Copying CEF binary `Release` directory...")
  file(COPY "${CMAKE_CURRENT_BINARY_DIR}/cef_binary_134.3.2+g615db2f+chromium-134.0.6998.89_windows64/Release" DESTINATION "${CMAKE_CURRENT_SOURCE_DIR}")
  if (EXISTS "${CMAKE_CURRENT_SOURCE_DIR}/Release")
    message(INFO " > CEF binary `Release` directory setup completed.")
  else()
    message(FATAL_ERROR " > CEF binary `Release` directory could not be setup.")
  endif()
endif()

# Always generate debug symbols
add_compile_options("$<$<NOT:$<CONFIG:Debug>>:/Zi>")
add_link_options("$<$<NOT:$<CONFIG:Debug>>:/DEBUG>")
add_link_options("$<$<NOT:$<CONFIG:Debug>>:/OPT:REF>")
add_link_options("$<$<NOT:$<CONFIG:Debug>>:/OPT:ICF>")

include(FetchContent)

FetchContent_Declare(Corrosion
    GIT_REPOSITORY https://github.com/corrosion-rs/corrosion.git
    GIT_TAG v0.6
)
# Set any global configuration variables such as `Rust_TOOLCHAIN` before this line!
FetchContent_MakeAvailable(Corrosion)

find_program(CXXBRIDGE cxxbridge PATHS "$ENV{HOME}/.cargo/bin/")
if (CXXBRIDGE STREQUAL "CXXBRIDGE-NOTFOUND")
    message("Could not find cxxbridge, trying to install with `cargo install cxxbridge-cmd'")
    find_program(CARGO cargo PATHS "$ENV{HOME}/.cargo/bin/")
    if (CARGO STREQUAL "CARGO-NOTFOUND")
        message(FATAL_ERROR "Requires cargo available in path, install via rustup https://rustup.rs/")
    endif()
    execute_process(COMMAND ${CARGO} install cxxbridge-cmd)
    find_program(CXXBRIDGE cxxbridge PATHS "$ENV{HOME}/.cargo/bin/")
endif()

corrosion_import_crate(
  MANIFEST_PATH ../../Cargo.toml
  CRATES authentik_sys
)
corrosion_add_cxxbridge(
  authentik_sys_bridge
  CRATE authentik_sys
  FILES ffi.rs
)

if(MSVC)
    set_property(TARGET authentik_sys PROPERTY MSVC_RUNTIME_LIBRARY MultiThreaded)
    set_property(TARGET authentik_sys_bridge PROPERTY MSVC_RUNTIME_LIBRARY MultiThreaded)
    corrosion_set_env_vars(authentik_sys "CFLAGS=-MT" "CXXFLAGS=-MT")
    corrosion_set_env_vars(authentik_sys_bridge "CFLAGS=-MT" "CXXFLAGS=-MT")
endif()

FetchContent_Declare(spdlog
  GIT_REPOSITORY https://github.com/gabime/spdlog.git
  GIT_TAG v1.17.0
)
FetchContent_MakeAvailable(spdlog)
set_property(TARGET spdlog PROPERTY MSVC_RUNTIME_LIBRARY MultiThreaded)

FetchContent_Declare(sentry
  GIT_REPOSITORY https://github.com/getsentry/sentry-native.git
  GIT_TAG 0.12.3
)
set(FETCHCONTENT_QUIET OFF CACHE BOOL "" FORCE)
set(SENTRY_BUILD_SHARED_LIBS OFF)
set(SENTRY_BUILD_RUNTIMESTATIC ON)
set(SENTRY_BACKEND inproc)
FetchContent_MakeAvailable(sentry)
set_property(TARGET sentry PROPERTY MSVC_RUNTIME_LIBRARY MultiThreaded)


#
# Define CEF-based targets.
#

add_subdirectory(ak_common)
set_property(TARGET ak_common PROPERTY MSVC_RUNTIME_LIBRARY MultiThreaded)

# Include the ak_cred_provider target.
# Comes from the libcef_dll/CMakeLists.txt file in the binary distribution
# directory.
add_subdirectory(${CEF_LIBCEF_DLL_WRAPPER_PATH} ak_cred_provider)

# Include application targets.
# Comes from the <target>/CMakeLists.txt file in the current directory.
# TODO: Change these lines to match your project target when you copy this file.
if(EXISTS "${CMAKE_CURRENT_SOURCE_DIR}/tests")
  # add_subdirectory(tests/cefsimple)
endif()

# add_subdirectory(ak_cred_provider)

# Set the project output directory for subprojects, such as `cefexe`, to copy their binaries to
set(PROJECT_INSALL_DIR "${CMAKE_CURRENT_BINARY_DIR}/ak_cred_provider/Release")
add_subdirectory(cefexe)
set_property(TARGET ak_cef PROPERTY MSVC_RUNTIME_LIBRARY MultiThreaded)

add_subdirectory(ak_lsa)

# Display configuration settings.
PRINT_CEF_CONFIG()


#
# Define the API documentation target.
#

if(EXISTS "${CMAKE_CURRENT_SOURCE_DIR}/Doxyfile")
  find_package(Doxygen)
  if(DOXYGEN_FOUND)
    add_custom_target(apidocs ALL
      # Generate documentation in the docs/html directory.
      COMMAND "${DOXYGEN_EXECUTABLE}" Doxyfile
      # Write a docs/index.html file.
      COMMAND ${CMAKE_COMMAND} -E echo "<html><head><meta http-equiv=\"refresh\" content=\"0;URL='html/index.html'\"/></head></html>" > docs/index.html
      WORKING_DIRECTORY "${CEF_ROOT}"
      COMMENT "Generating API documentation with Doxygen..."
      VERBATIM )
  else()
    message(WARNING "Doxygen must be installed to generate API documentation.")
  endif()
endif()

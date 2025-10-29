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
# libcef_dll/CMakeLists.txt   Defines the libcef_dll_wrapper target.
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
list(APPEND CMAKE_MODULE_PATH "C:/Strawberry/perl")

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

# --- jwt-cpp and OpenSSL - Begin
find_package(Perl REQUIRED)       # Needed to build OpenSSL

include(FetchContent)

FetchContent_Declare(OpenSSL
  URL https://github.com/openssl/openssl/releases/download/openssl-3.5.0/openssl-3.5.0.tar.gz
  URL_HASH SHA256=344d0a79f1a9b08029b0744e2cc401a43f9c90acd1044d09a530b4885a8e9fc0
  DOWNLOAD_EXTRACT_TIMESTAMP OFF
  # OVERRIDE_FIND_PACKAGE
)
set(FETCHCONTENT_QUIET OFF CACHE BOOL "" FORCE)
FetchContent_MakeAvailable(OpenSSL)

if (NOT EXISTS "${openssl_SOURCE_DIR}/libssl.lib")
  execute_process(COMMAND ${PERL_EXECUTABLE} Configure WORKING_DIRECTORY ${openssl_SOURCE_DIR})
  execute_process(COMMAND nmake WORKING_DIRECTORY ${openssl_SOURCE_DIR})
endif()
if (NOT EXISTS "${openssl_SOURCE_DIR}/libssl.lib")
  message(FATAL_ERROR "OpenSSL library files not found in build. OpenSSL build must succeed first.")
endif()

set(CMAKE_FIND_DEBUG_MODE OFF)

# Set path to OpenSSL library directory for search
set(OPENSSL_ROOT_DIR "${openssl_SOURCE_DIR}")

# Test if the just built OpenSSL package can be found, so that the `jwt-cpp` would also
# find the same package with find_package() call
find_package(OpenSSL REQUIRED)

# Display the set and detected paths
message(INFO " > OPENSSL_ROOT_DIR:         ${OPENSSL_ROOT_DIR}")
message(INFO "")
message(INFO " > OPENSSL_CRYPTO_LIBRARY:   ${OPENSSL_CRYPTO_LIBRARY}")
message(INFO " > OPENSSL_CRYPTO_LIBRARIES: ${OPENSSL_CRYPTO_LIBRARIES}")
message(INFO " > OPENSSL_SSL_LIBRARY:      ${OPENSSL_SSL_LIBRARY}")
message(INFO " > OPENSSL_SSL_LIBRARIES:    ${OPENSSL_SSL_LIBRARIES}")

# Test if the detected OpenSSL package is indeed the one just built and not from another installation such as Strawberry Perl
cmake_path(IS_PREFIX OPENSSL_ROOT_DIR ${OPENSSL_CRYPTO_LIBRARIES} isprefix)
if (NOT isprefix)
  message(FATAL_ERROR "CMake must reference the OpenSSL Crypto library in the project build directory (${OPENSSL_ROOT_DIR}) but it is instead referencing the path (${OPENSSL_CRYPTO_LIBRARIES}).")
endif()
cmake_path(IS_PREFIX OPENSSL_ROOT_DIR ${OPENSSL_SSL_LIBRARIES} isprefix)
if (NOT isprefix)
  message(FATAL_ERROR "CMake must reference the OpenSSL SSL library in the project build directory (${OPENSSL_ROOT_DIR}) but it is instead referencing the path (${OPENSSL_SSL_LIBRARIES}).")
endif()

# If OpenSSL is sorted, fetch and build jwt-cpp that relies on it
FetchContent_Declare(jwtcpp
  URL https://github.com/Thalhammer/jwt-cpp/releases/download/v0.7.1/jwt-cpp-v0.7.1.tar.gz
  # URL_HASH SHA256=
  DOWNLOAD_EXTRACT_TIMESTAMP OFF
)
set(FETCHCONTENT_QUIET OFF CACHE BOOL "" FORCE)
FetchContent_MakeAvailable(jwtcpp)

# --- jwt-cpp and OpenSSL - End

#
# Define CEF-based targets.
#

# Include the libcef_dll_wrapper target.
# Comes from the libcef_dll/CMakeLists.txt file in the binary distribution
# directory.
add_subdirectory(${CEF_LIBCEF_DLL_WRAPPER_PATH} libcef_dll_wrapper)

# Include application targets.
# Comes from the <target>/CMakeLists.txt file in the current directory.
# TODO: Change these lines to match your project target when you copy this file.
if(EXISTS "${CMAKE_CURRENT_SOURCE_DIR}/tests")
  # add_subdirectory(tests/cefsimple)
endif()

# add_subdirectory(AuthentikWindowsCredentialProvider)

# Set the project output directory for subprojects, such as `cefexe`, to copy their binaries to
set(PROJECT_INSALL_DIR "${CMAKE_CURRENT_BINARY_DIR}/libcef_dll_wrapper/Release")
add_subdirectory(cefexe)
# Copy cefexe executable and OpenSSL library file to the common build directory
file(COPY "${OPENSSL_ROOT_DIR}/libcrypto-3-x64.dll" DESTINATION "${PROJECT_INSALL_DIR}")

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


# Copy the Setup (installer) project to the build directory for integration into the
# Visual Studio Solution file
file(COPY "${CMAKE_CURRENT_SOURCE_DIR}/Setup" DESTINATION "${CMAKE_CURRENT_BINARY_DIR}")

# macro(ADD_SETUP_PROJECT)
#   set(PROJECT_SLN_FILE "${CMAKE_CURRENT_BINARY_DIR}/${PROJECT_NAME}.sln")
#   message(WARNING "Execute.")
#   if(EXISTS "${PROJECT_SLN_FILE}")
#     message(WARNING "Exists.")
#     set(SETUP_PROJECT "
#       Project(\"{54435603-DBB4-11D2-8724-00A0C9A8B90C}\") = \"Setup\", \"..\\Setup\\Setup.vdproj\", \"{8818B3B0-D7DD-4294-A5C0-29BA5DE85BEE}\"
#       EndProject
#     ")
#     # Read solution file contents
#     file(READ "${PROJECT_SLN_FILE}" SLN_CONTENTS)

#     # Check if the Setup project has already been added to the solution (i.e., whether this is a re-run of CMake)
#     string(FIND "${SLN_CONTENTS}" "Setup.vdproj" SETUP_POS)
#     if (SETUP_POS GREATER_EQUAL 0)
#       string(FIND "${SLN_CONTENTS}" "EndProject" EP_POS REVERSE)
#       string(SUBSTRING "${SLN_CONTENTS}" 0 ${EP_POS} PRE_STR)
#       string(SUBSTRING "${SLN_CONTENTS}" ${EP_POS} -1 POST_STR)
#       string(CONCAT STR_COMBINED "${PRE_STR}" "${SETUP_PROJECT}" "${POST_STR}")
#       file(WRITE "${PROJECT_SLN_FILE}" "${STR_COMBINED}")
#     endif()
#   endif()
# endmacro()

# cmake_language(DEFER CALL ADD_SETUP_PROJECT)

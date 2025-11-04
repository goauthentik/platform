# Copyright (c) 2014 The Chromium Embedded Framework Authors. All rights
# reserved. Use of this source code is governed by a BSD-style license that
# can be found in the LICENSE file.

project(cefexe)

#
# Source files.
#

set(SRCS
  cefexe.cpp
)

#
# Windows configuration.
#
# find_library(LIBCEF_LIB_PATH libcef.lib PATHS "../../Release")
# find_library(LIBCEF_DLL_WRAPPER_LIB_PATH ak_cred_provider.lib PATHS "../ak_cred_provider/Release")

# Executable target.

add_executable(${PROJECT_NAME} WIN32 ${SRCS})
# target_compile_definitions(${PROJECT_NAME} PUBLIC UNICODE _UNICODE)

target_include_directories(${PROJECT_NAME} PUBLIC ..)

target_link_libraries(${PROJECT_NAME}
  # ${LIBCEF_LIB_PATH}  #//- todo: revise if really needed
  ak_cred_provider
  # ${LIBCEF_DLL_WRAPPER_LIB_PATH} #-
  # ${CEF_STANDARD_LIBS}
)

# Copy binary and resource files to the target output directory.

COPY_FILES("${PROJECT_NAME}" "${PROJECT_NAME}.exe" "${CMAKE_CURRENT_BINARY_DIR}/Release" "${PROJECT_INSALL_DIR}")

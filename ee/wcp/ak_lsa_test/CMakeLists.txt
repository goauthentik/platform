# Copyright (c) 2014 The Chromium Embedded Framework Authors. All rights
# reserved. Use of this source code is governed by a BSD-style license that
# can be found in the LICENSE file.

project(ak_lsa_test)

#
# Source files.
#

set(SRCS
  main.cpp
)

include_directories(
  ../
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

find_library(CREDUI_LIB_PATH Credui.lib)
find_library(SECUR32_LIB_PATH Secur32.lib)
find_library(SHLWAPI_LIB_PATH Shlwapi.lib)
target_link_libraries(${PROJECT_NAME} PUBLIC
	${CREDUI_LIB_PATH}
	${SECUR32_LIB_PATH}
	${SHLWAPI_LIB_PATH}
)

# Copy binary and resource files to the target output directory.
COPY_FILES(
  ${PROJECT_NAME}
  "${PROJECT_NAME}.exe;${PROJECT_NAME}.pdb"
  "${CMAKE_CURRENT_BINARY_DIR}/Release"
  ${PROJECT_INSALL_DIR}
)

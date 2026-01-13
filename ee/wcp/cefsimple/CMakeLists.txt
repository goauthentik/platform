# Copyright (c) 2014 The Chromium Embedded Framework Authors. All rights
# reserved. Use of this source code is governed by a BSD-style license that
# can be found in the LICENSE file.

project(cefsimple)

#
# Source files.
#

# cefsimple sources.
set(CEFSIMPLE_SRCS
  simple_app.cc
  simple_app.h
  simple_handler.cc
  simple_handler.h
  )
set(CEFSIMPLE_SRCS_LINUX
  cefsimple_linux.cc
  simple_handler_linux.cc
  )
set(CEFSIMPLE_SRCS_MAC
  cefsimple_mac.mm
  simple_handler_mac.mm
  )
set(CEFSIMPLE_SRCS_WINDOWS
  # cefsimple_win.h

  cefsimple_win.cc
  resource.h
  simple_handler_win.cc
  )
# APPEND_PLATFORM_SOURCES(CEFSIMPLE_SRCS_WINDOWS)
# source_group(cefsimple FILES ${CEFSIMPLE_SRCS_WINDOWS})



APPEND_PLATFORM_SOURCES(CEFSIMPLE_SRCS)
source_group(cefsimple FILES ${CEFSIMPLE_SRCS})

set(CEFSIMPLE_WIN_SRCS_WINDOWS
  win/cefsimple.rc
  )
APPEND_PLATFORM_SOURCES(CEFSIMPLE_WIN_SRCS)
source_group(cefsimple\\\\win FILES ${CEFSIMPLE_WIN_SRCS})

set(CEFSIMPLE_SRCS
  ${CEFSIMPLE_SRCS}
  ${CEFSIMPLE_WIN_SRCS}
#   ${CEFSIMPLE_SRCS_WINDOWS}
  )

# cefsimple helper sources.
set(CEFSIMPLE_HELPER_SRCS_MAC
  process_helper_mac.cc
  )
APPEND_PLATFORM_SOURCES(CEFSIMPLE_HELPER_SRCS)
source_group(cefsimple FILES ${CEFSIMPLE_HELPER_SRCS})

# cefsimple resources.
set(CEFSIMPLE_RESOURCES_MAC_SRCS_MAC
  mac/Info.plist.in
  mac/cefsimple.icns
  )
APPEND_PLATFORM_SOURCES(CEFSIMPLE_RESOURCES_MAC_SRCS)
source_group(cefsimple\\\\mac FILES ${CEFSIMPLE_RESOURCES_MAC_SRCS})

set(CEFSIMPLE_RESOURCES_MAC_ENGLISH_LPROJ_SRCS_MAC
  mac/English.lproj/InfoPlist.strings
  mac/English.lproj/MainMenu.xib
  )
APPEND_PLATFORM_SOURCES(CEFSIMPLE_RESOURCES_MAC_ENGLISH_LPROJ_SRCS)
source_group(cefsimple\\\\mac\\\\English.lproj FILES ${CEFSIMPLE_RESOURCES_MAC_ENGLISH_LPROJ_SRCS})

set(CEFSIMPLE_RESOURCES_SRCS
  ${CEFSIMPLE_RESOURCES_MAC_SRCS}
  ${CEFSIMPLE_RESOURCES_MAC_ENGLISH_LPROJ_SRCS}
  )


#
# Shared configuration.
#

# Target executable names.
set(CEF_TARGET "cefsimple")
if(OS_MAC)
  set(CEF_HELPER_TARGET "cefsimple_Helper")
  set(CEF_HELPER_OUTPUT_NAME "cefsimple Helper")
else()
  # Logical target used to link the libcef library.
  ADD_LOGICAL_TARGET("libcef_lib" "${CEF_LIB_DEBUG}" "${CEF_LIB_RELEASE}")
endif()

# Determine the target output directory.
SET_CEF_TARGET_OUT_DIR()


#
# Windows configuration.
#

if(OS_WINDOWS)
  # Executable target.
  # add_executable(${CEF_TARGET} WIN32 ${CEFSIMPLE_SRCS})
  add_library(${CEF_TARGET} STATIC ${CEFSIMPLE_SRCS})
  add_dependencies(${CEF_TARGET} ak_cred_provider)
  # SET_EXECUTABLE_TARGET_PROPERTIES(${CEF_TARGET})
  target_include_directories(${CEF_TARGET} PUBLIC ../..)
  # target_include_directories(${CEF_TARGET} PUBLIC
  #   ../..
  #   ${CMAKE_CURRENT_SOURCE_DIR}
  # )
  # target_sources(${CEF_TARGET} PRIVATE
  #   ${CEF_TARGET}.def
  # )
  target_link_libraries(${CEF_TARGET} libcef_lib ak_cred_provider ${CEF_STANDARD_LIBS})

  if(USE_SANDBOX)
    # Logical target used to link the cef_sandbox library.
    ADD_LOGICAL_TARGET("cef_sandbox_lib" "${CEF_SANDBOX_LIB_DEBUG}" "${CEF_SANDBOX_LIB_RELEASE}")
    target_link_libraries(${CEF_TARGET} cef_sandbox_lib ${CEF_SANDBOX_STANDARD_LIBS})
  endif()

  # Add the custom manifest files to the executable.
  # ADD_WINDOWS_MANIFEST("${CMAKE_CURRENT_SOURCE_DIR}/win" "${CEF_TARGET}" "exe")

  # Copy binary and resource files to the target output directory.
  COPY_FILES("${CEF_TARGET}" "${CEF_BINARY_FILES}" "${CEF_BINARY_DIR}" "${CEF_TARGET_OUT_DIR}")
  COPY_FILES("${CEF_TARGET}" "${CEF_RESOURCE_FILES}" "${CEF_RESOURCE_DIR}" "${CEF_TARGET_OUT_DIR}")

  if(USE_SANDBOX)
    # Set LPAC ACLs required for Windows sandbox support.
    SET_LPAC_ACLS("${CEF_TARGET}")
  endif()
endif()

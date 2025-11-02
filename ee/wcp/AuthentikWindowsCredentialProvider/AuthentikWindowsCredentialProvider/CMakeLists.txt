# CMakeList.txt : CMake project for AuthentikWindowsCredentialProvider, include source and define
# project specific logic here.
#

set(CMAKE_BUILD_TYPE Release)

project(
  AuthentikWindowsCredentialProvider
  LANGUAGES CXX
  VERSION 0.16.7
)

# TODO: Add tests and install targets if needed.

set(CMAKE_CXX_STANDARD 11)

# # cefsimple sources.
# set(CEFSIMPLE_SRCS
#   simple_app.cc
#   simple_app.h
#   simple_handler.cc
#   simple_handler.h
#   )
# set(CEFSIMPLE_SRCS_WINDOWS
#   cefsimple_win.cc
#   resource.h
#   simple_handler_win.cc
#   )
# APPEND_PLATFORM_SOURCES(CEFSIMPLE_SRCS)
# source_group(cefsimple FILES ${CEFSIMPLE_SRCS})

# set(CEFSIMPLE_WIN_SRCS_WINDOWS
#   win/cefsimple.rc
#   )
# APPEND_PLATFORM_SOURCES(CEFSIMPLE_WIN_SRCS)
# source_group(cefsimple\\\\win FILES ${CEFSIMPLE_WIN_SRCS})

# set(CEFSIMPLE_SRCS
#   ${CEFSIMPLE_SRCS}
#   ${CEFSIMPLE_WIN_SRCS}
#   )

# # cefsimple helper sources.
# set(CEFSIMPLE_HELPER_SRCS_MAC
#   process_helper_mac.cc
#   )
# APPEND_PLATFORM_SOURCES(CEFSIMPLE_HELPER_SRCS)
# source_group(cefsimple FILES ${CEFSIMPLE_HELPER_SRCS})

# # - Skipped resources -



# #
# # Shared configuration.
# #

# # Target executable names.
# set(CEF_TARGET "cefsimple")
# if(OS_MAC)
#   set(CEF_HELPER_TARGET "cefsimple_Helper")
#   set(CEF_HELPER_OUTPUT_NAME "cefsimple Helper")
# else()
#   # Logical target used to link the libcef library.
#   ADD_LOGICAL_TARGET("libcef_lib" "${CEF_LIB_DEBUG}" "${CEF_LIB_RELEASE}")
# endif()

# # Determine the target output directory.
# SET_CEF_TARGET_OUT_DIR()


# #
# # Windows configuration.
# #

# if(OS_WINDOWS)
#   # Executable target.
#   add_executable(${CEF_TARGET} WIN32 ${CEFSIMPLE_SRCS})
#   add_dependencies(${CEF_TARGET} libcef_dll_wrapper)
#   SET_EXECUTABLE_TARGET_PROPERTIES(${CEF_TARGET})
#   target_link_libraries(${CEF_TARGET} libcef_lib libcef_dll_wrapper ${CEF_STANDARD_LIBS})

#   if(USE_SANDBOX)
#     # Logical target used to link the cef_sandbox library.
#     ADD_LOGICAL_TARGET("cef_sandbox_lib" "${CEF_SANDBOX_LIB_DEBUG}" "${CEF_SANDBOX_LIB_RELEASE}")
#     target_link_libraries(${CEF_TARGET} cef_sandbox_lib ${CEF_SANDBOX_STANDARD_LIBS})
#   endif()

#   # Add the custom manifest files to the executable.
#   ADD_WINDOWS_MANIFEST("${CMAKE_CURRENT_SOURCE_DIR}/win" "${CEF_TARGET}" "exe")

#   # Copy binary and resource files to the target output directory.
#   COPY_FILES("${CEF_TARGET}" "${CEF_BINARY_FILES}" "${CEF_BINARY_DIR}" "${CEF_TARGET_OUT_DIR}")
#   COPY_FILES("${CEF_TARGET}" "${CEF_RESOURCE_FILES}" "${CEF_RESOURCE_DIR}" "${CEF_TARGET_OUT_DIR}")

#   if(USE_SANDBOX)
#     # Set LPAC ACLs required for Windows sandbox support.
#     SET_LPAC_ACLS("${CEF_TARGET}")
#   endif()
# endif()

























# Add source to this project's executable.
set(SRCS
  "ClassFactory.cpp"
  "Credential.cpp"
  "Debug.cpp"
  "Dll.cpp"
  "GUIDs.cpp"
  "Helpers.cpp"
  "pch.cpp"
  "Provider.cpp"
  "res/resource.rc"
)


find_library(CREDUI_LIB_PATH Credui.lib)
find_library(SECUR32_LIB_PATH Secur32.lib)
find_library(SHLWAPI_LIB_PATH Shlwapi.lib)

add_library (${PROJECT_NAME} SHARED
	${SRCS}
)

include_directories(
	include
	res
  ../..
)

if (CMAKE_VERSION VERSION_GREATER 3.12)
	set_property(TARGET ${PROJECT_NAME} PROPERTY CXX_STANDARD 20)
endif()
if (MSVC)
	target_compile_definitions(${PROJECT_NAME} PUBLIC UNICODE _UNICODE)
	target_sources(${PROJECT_NAME} PRIVATE
        ${CMAKE_CURRENT_SOURCE_DIR}/${PROJECT_NAME}.def
    )
endif()

target_link_libraries(${PROJECT_NAME} PUBLIC
	${CREDUI_LIB_PATH}
	${SECUR32_LIB_PATH}
	${SHLWAPI_LIB_PATH}
  #${CEFSIMPLE_LIB_PATH}
  # ${LIBCEF_LIB_PATH}
  cefsimple
  authentik_sys_bridge
  # sentry
  # libcef_lib
  # libcef_dll_wrappe
)

# TODO: Add tests and install targets if needed.

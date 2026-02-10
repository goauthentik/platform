project(ak_lsa)

set(CMAKE_BUILD_TYPE Release)
set(CMAKE_CXX_STANDARD 20)

set(SRCS
  PrepareProfile.cpp
  PrepareToken.cpp
  Main.cpp
)

find_library(CREDUI_LIB_PATH Credui.lib)
find_library(SECUR32_LIB_PATH Secur32.lib)
find_library(SHLWAPI_LIB_PATH Shlwapi.lib)

add_library(${PROJECT_NAME} SHARED
	${SRCS}
)
target_compile_definitions(${PROJECT_NAME} PUBLIC UNICODE _UNICODE SECURITY_WIN32)

include_directories(
	include
)

include_directories(${PROJECT_NAME} PUBLIC ..)

target_link_libraries(${PROJECT_NAME}
  ${CREDUI_LIB_PATH}
  ${SECUR32_LIB_PATH}
  ${SHLWAPI_LIB_PATH}
  authentik_sys_bridge
  ak_common
  spdlog
)
set_property(TARGET ak_lsa PROPERTY MSVC_RUNTIME_LIBRARY MultiThreaded)
set_property(TARGET authentik_sys PROPERTY MSVC_RUNTIME_LIBRARY MultiThreaded)
set_property(TARGET authentik_sys_bridge PROPERTY MSVC_RUNTIME_LIBRARY MultiThreaded)

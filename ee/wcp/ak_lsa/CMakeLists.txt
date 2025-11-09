# CMakeList.txt : CMake project for ak_lsa, include source and define
# project specific logic here.
#

set(CMAKE_BUILD_TYPE Release)

project(
  ak_lsa
  LANGUAGES CXX
  VERSION 0.16.7
)

# TODO: Add tests and install targets if needed.

set(CMAKE_CXX_STANDARD 11)

# Add source to this project's executable.
set(SRCS
  "Dll.cpp"
  "Lsa.cpp"
  "ak.cpp"
)

find_library(CREDUI_LIB_PATH Credui.lib)
find_library(SECUR32_LIB_PATH Secur32.lib)
find_library(SHLWAPI_LIB_PATH Shlwapi.lib)
find_library(ADVAPI32_LIB_PATH advapi32.lib)

add_library (${PROJECT_NAME} SHARED
	${SRCS}
)

include_directories(
	include
	res
  ../
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
  ${ADVAPI32_LIB_PATH}
  authentik_sys_bridge
  spdlog
)

set_property(TARGET ak_lsa PROPERTY MSVC_RUNTIME_LIBRARY MultiThreaded)

COPY_FILES(
  ${PROJECT_NAME}
  "${PROJECT_NAME}.dll;${PROJECT_NAME}.pdb"
  "${CMAKE_CURRENT_BINARY_DIR}/Release"
  ${PROJECT_INSALL_DIR}
)

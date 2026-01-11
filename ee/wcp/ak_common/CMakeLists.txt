project(ak_common)

set(CMAKE_BUILD_TYPE Release)
set(CMAKE_CXX_STANDARD 20)

set(SRCS
  ak_log.cpp
  ak_sentry.cpp
)

add_library(${PROJECT_NAME} STATIC
	${SRCS}
)

# include_directories(${PROJECT_NAME} PUBLIC ..)
target_include_directories(${PROJECT_NAME} PUBLIC
  ${CMAKE_CURRENT_SOURCE_DIR}/include
)

target_link_libraries(${PROJECT_NAME}
  authentik_sys_bridge
  spdlog
  sentry
)
set_property(TARGET authentik_sys PROPERTY MSVC_RUNTIME_LIBRARY MultiThreaded)
set_property(TARGET authentik_sys_bridge PROPERTY MSVC_RUNTIME_LIBRARY MultiThreaded)

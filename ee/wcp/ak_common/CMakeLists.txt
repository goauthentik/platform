project(ak_common)

set(CMAKE_BUILD_TYPE Release)
set(CMAKE_CXX_STANDARD 20)

set(SRCS
  ak_log.cpp
  ak_sentry.cpp
  crypt.cc
)

add_library(${PROJECT_NAME} STATIC
	${SRCS}
)

# include_directories(${PROJECT_NAME} PUBLIC ..)
target_include_directories(${PROJECT_NAME} PUBLIC
  ${CMAKE_CURRENT_SOURCE_DIR}/include
)

target_link_libraries(${PROJECT_NAME}
  ak-ffi-bridge
  spdlog
  sentry
)
set_property(TARGET ak-ffi PROPERTY MSVC_RUNTIME_LIBRARY MultiThreaded)
set_property(TARGET ak-ffi-bridge PROPERTY MSVC_RUNTIME_LIBRARY MultiThreaded)

project(ak_lsa)

set(CMAKE_CXX_STANDARD 20)

set(SRCS
  PrepareProfile.cpp
  PrepareToken.cpp
  Main.cpp
)

add_library (${PROJECT_NAME} SHARED
	${SRCS}
)

add_compile_definitions(SECURITY_WIN32)

target_include_directories(${PROJECT_NAME} PUBLIC ..)

target_link_libraries(${PROJECT_NAME}
  ak_cred_provider
)

# Copy binary and resource files to the target output directory.

COPY_FILES("${PROJECT_NAME}" "${PROJECT_NAME}.dll" "${CMAKE_CURRENT_BINARY_DIR}/Release" "${PROJECT_INSALL_DIR}")

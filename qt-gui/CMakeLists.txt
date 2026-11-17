cmake_minimum_required(VERSION 3.20)
project(poddrop-gui VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)

find_package(Qt6 REQUIRED COMPONENTS Widgets)
find_package(PkgConfig REQUIRED)
pkg_check_modules(GPOD REQUIRED IMPORTED_TARGET libgpod-1.0 glib-2.0 gdk-pixbuf-2.0)

get_filename_component(WORKSPACE_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/.." ABSOLUTE)

if(CMAKE_BUILD_TYPE STREQUAL "Release")
    set(CARGO_PROFILE_FLAG "--release")
    set(RUST_LIB_PATH "${WORKSPACE_ROOT}/target/release/libpoddrop_ffi.a")
else()
    set(CARGO_PROFILE_FLAG "")
    set(RUST_LIB_PATH "${WORKSPACE_ROOT}/target/debug/libpoddrop_ffi.a")
endif()

add_custom_target(poddrop_ffi_build ALL
    COMMAND cargo build -p poddrop-ffi ${CARGO_PROFILE_FLAG}
    WORKING_DIRECTORY "${WORKSPACE_ROOT}"
    COMMENT "Building poddrop-ffi static library"
)

add_executable(poddrop-gui
    src/main.cpp
    src/mainwindow.cpp
    src/dialogs.cpp
    resources/resources.qrc
)

add_dependencies(poddrop-gui poddrop_ffi_build)

target_include_directories(poddrop-gui PRIVATE
    "${CMAKE_CURRENT_SOURCE_DIR}/src"
    "${WORKSPACE_ROOT}/crates/poddrop-ffi/include"
)

target_link_libraries(poddrop-gui PRIVATE
    Qt6::Widgets
    "${RUST_LIB_PATH}"
    PkgConfig::GPOD
    pthread
    dl
    m
)

target_compile_options(poddrop-gui PRIVATE -Wall -Wextra)

include(GNUInstallDirs)
install(TARGETS poddrop-gui RUNTIME DESTINATION "${CMAKE_INSTALL_BINDIR}")
install(FILES "${WORKSPACE_ROOT}/packaging/io.github.m4sa_k1.PodDrop.desktop"
    DESTINATION "${CMAKE_INSTALL_DATADIR}/applications")
install(FILES "${WORKSPACE_ROOT}/packaging/io.github.m4sa_k1.PodDrop.svg"
    DESTINATION "${CMAKE_INSTALL_DATADIR}/icons/hicolor/scalable/apps")

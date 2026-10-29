cmake_minimum_required(VERSION 3.16)
project(ryzora LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTORCC ON)
set(CMAKE_AUTOMOC ON)

find_package(Qt6 REQUIRED COMPONENTS Core Gui Quick Qml Concurrent QuickControls2 Network Multimedia)

qt_standard_project_setup()

set(RUST_WORKSPACE_DIR "${CMAKE_CURRENT_SOURCE_DIR}/..")
set(RUST_LIB_PATH "${RUST_WORKSPACE_DIR}/target/release/libryzora_qt_backend.a")

add_custom_target(ryzora_rust_backend
    COMMAND cargo build --release -p ryzora-qt-backend
    WORKING_DIRECTORY "${RUST_WORKSPACE_DIR}"
    BYPRODUCTS "${RUST_LIB_PATH}"
    COMMENT "Compiling workspace crate ryzora-qt-backend with Cargo..."
    VERBATIM
)

set(PROJECT_SOURCES
    src/main.cpp
    src/ryzora_backend.hpp
    src/preview_stubs.hpp
    src/ryzora_backend.cpp
    qml.qrc
)

add_executable(ryzora ${PROJECT_SOURCES})
add_dependencies(ryzora ryzora_rust_backend)

target_link_libraries(ryzora PRIVATE
    Qt6::Core
    Qt6::Gui
    Qt6::Quick
    Qt6::Qml
    Qt6::Concurrent
    Qt6::QuickControls2
    Qt6::Network
    Qt6::Multimedia
    "${RUST_LIB_PATH}"
    pthread
    dl
    m
)

# ── Installation Rules ──
include(GNUInstallDirs)

# Main Executable
install(TARGETS ryzora
    RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}
)

# Restricted Privileged Helpers
install(PROGRAMS
    "${RUST_WORKSPACE_DIR}/crates/ryzora-core/resources/ryzora-package-helper"
    "${RUST_WORKSPACE_DIR}/crates/ryzora-core/resources/ryzora-sddm-helper"
    DESTINATION ${CMAKE_INSTALL_LIBDIR}/ryzora
)

# Polkit Authorization Actions & Rules
install(FILES
    "${RUST_WORKSPACE_DIR}/crates/ryzora-core/resources/io.ryzora.package.policy"
    "${RUST_WORKSPACE_DIR}/crates/ryzora-core/resources/io.ryzora.sddm.policy"
    DESTINATION ${CMAKE_INSTALL_DATADIR}/polkit-1/actions
)
install(FILES
    "${RUST_WORKSPACE_DIR}/crates/ryzora-core/resources/io.ryzora.package.rules"
    "${RUST_WORKSPACE_DIR}/crates/ryzora-core/resources/io.ryzora.sddm.rules"
    DESTINATION ${CMAKE_INSTALL_DATADIR}/polkit-1/rules.d
)

# Desktop Entry & AppStream Metadata
install(FILES
    "${RUST_WORKSPACE_DIR}/dist-assets/io.ryzora.Ryzora.desktop"
    DESTINATION ${CMAKE_INSTALL_DATADIR}/applications
)
install(FILES
    "${RUST_WORKSPACE_DIR}/dist-assets/io.ryzora.Ryzora.metainfo.xml"
    DESTINATION ${CMAKE_INSTALL_DATADIR}/metainfo
)

# Application Icons
install(DIRECTORY
    "${RUST_WORKSPACE_DIR}/dist-assets/icons/hicolor"
    DESTINATION ${CMAKE_INSTALL_DATADIR}/icons
)

# Shared Data & Package Repository Metadata
install(DIRECTORY
    "${RUST_WORKSPACE_DIR}/repositories"
    DESTINATION ${CMAKE_INSTALL_DATADIR}/ryzora
)
install(DIRECTORY
    "${RUST_WORKSPACE_DIR}/public/assets"
    DESTINATION ${CMAKE_INSTALL_DATADIR}/ryzora/public
)


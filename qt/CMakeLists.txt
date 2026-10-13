cmake_minimum_required(VERSION 3.16)
project(ryzora LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTORCC ON)
set(CMAKE_AUTOMOC ON)

find_package(Qt6 REQUIRED COMPONENTS Core Gui Quick Qml Concurrent QuickControls2)

qt_standard_project_setup()

set(PROJECT_SOURCES
    src/main.cpp
    src/ryzora_backend.hpp
    src/ryzora_backend.cpp
    qml.qrc
)

add_executable(ryzora ${PROJECT_SOURCES})

target_link_libraries(ryzora PRIVATE
    Qt6::Core
    Qt6::Gui
    Qt6::Quick
    Qt6::Qml
    Qt6::Concurrent
    Qt6::QuickControls2
    ${CMAKE_CURRENT_SOURCE_DIR}/../crates/ryzora-qt-backend/target/release/libryzora_qt_backend.a
    pthread
    dl
    m
)

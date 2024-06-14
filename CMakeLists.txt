cmake_minimum_required(VERSION 3.15.0)
project(fxr-ws-reloader LANGUAGES C CXX)

find_package(websocketpp CONFIG REQUIRED)
find_package(nlohmann_json CONFIG REQUIRED)

add_subdirectory("./libER" "libER")

include(FetchContent)
FetchContent_Declare(Corrosion GIT_REPOSITORY https://github.com/corrosion-rs/corrosion.git GIT_TAG v0.5)
FetchContent_MakeAvailable(Corrosion)
corrosion_import_crate(MANIFEST_PATH rust-fxr-reloader/Cargo.toml)

add_definitions(-D_ITERATOR_DEBUG_LEVEL=0)
set(CMAKE_CXX_STANDARD 20)

add_library("fxr-ws-reloader" SHARED "fxr-ws-reloader.cpp")
target_link_libraries("fxr-ws-reloader"
  PUBLIC
    libER_static
  PRIVATE
    websocketpp::websocketpp
    nlohmann_json::nlohmann_json
    rust_fxr_reloader
)

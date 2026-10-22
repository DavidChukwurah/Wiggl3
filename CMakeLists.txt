# wiggl3 (developer build) - works on Windows, macOS and Linux. Ordinary users do not need this; they download a
# finished file (see README.md). Reviewers and developers can build from source like this:
#
#   cmake -S . -B build
#   cmake --build build --config Release
#   ctest --test-dir build -C Release        (optional: runs the unit tests)
#
# CMake only drives cargo here. It installs nothing and downloads nothing itself (cargo fetches the
# Rust libraries pinned in source/Cargo.lock). Everything it makes stays inside the build folder.
cmake_minimum_required(VERSION 3.20)
project(wiggl3 LANGUAGES NONE)

set(WIGGL3_CARGO_ARGS "--locked" CACHE STRING "Extra arguments passed to cargo build/test")
separate_arguments(_cargo_args NATIVE_COMMAND "${WIGGL3_CARGO_ARGS}")

# --- find Rust, and fail early with a plain message ---
find_program(CARGO_EXECUTABLE cargo HINTS "$ENV{USERPROFILE}/.cargo/bin" "$ENV{HOME}/.cargo/bin")
find_program(RUSTC_EXECUTABLE rustc HINTS "$ENV{USERPROFILE}/.cargo/bin" "$ENV{HOME}/.cargo/bin")
if(NOT CARGO_EXECUTABLE OR NOT RUSTC_EXECUTABLE)
  message(FATAL_ERROR
    "Rust (cargo and rustc) 1.89 or newer is required but was not found.\n"
    "Install it from https://rustup.rs or your IT team's software portal, then run CMake again.\n"
    "Windows also needs the Microsoft C++ Build Tools (the MSVC linker); Mac needs the Xcode Command Line Tools.")
endif()
execute_process(COMMAND "${RUSTC_EXECUTABLE}" --version OUTPUT_VARIABLE _rustc_out OUTPUT_STRIP_TRAILING_WHITESPACE)
if(_rustc_out MATCHES "rustc ([0-9]+\\.[0-9]+\\.[0-9]+)")
  set(_rust_ver "${CMAKE_MATCH_1}")
  if(_rust_ver VERSION_LESS "1.89")
    message(FATAL_ERROR "Rust 1.89 or newer is required (found ${_rust_ver}). Update with: rustup update stable")
  endif()
  message(STATUS "Rust ${_rust_ver}")
else()
  message(WARNING "Could not read the Rust version from: ${_rustc_out}")
endif()

# --- build ---
set(CARGO_TARGET_DIR "${CMAKE_BINARY_DIR}/cargo-target")
if(CMAKE_HOST_WIN32)
  set(_exe_suffix ".exe")
else()
  set(_exe_suffix "")
endif()
set(WIGGL3_EXE "${CARGO_TARGET_DIR}/release/wiggl3${_exe_suffix}")
set(_cargo_env ${CMAKE_COMMAND} -E env "CARGO_TARGET_DIR=${CARGO_TARGET_DIR}")

add_custom_target(wiggl3 ALL
  COMMAND ${_cargo_env} "${CARGO_EXECUTABLE}" build --release ${_cargo_args}
  WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}/source"
  BYPRODUCTS "${WIGGL3_EXE}"
  COMMENT "Building wiggl3 with cargo"
  USES_TERMINAL VERBATIM)

# --- tests (ctest) ---
enable_testing()
add_test(NAME wiggl3-unit-tests
  COMMAND ${_cargo_env} "${CARGO_EXECUTABLE}" test ${_cargo_args}
  WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}/source")

# --- macOS: wrap as wiggl3.app (own icon, own Accessibility permission) ---
if(APPLE)
  add_custom_target(mac_app
    COMMAND bash "${CMAKE_SOURCE_DIR}/source/tools/make_mac_app.sh" "${WIGGL3_EXE}" "${CMAKE_BINARY_DIR}/wiggl3.app"
    DEPENDS wiggl3
    COMMENT "Creating wiggl3.app"
    VERBATIM)
endif()

# --- optional: cmake --install build --prefix <dir> ---
install(PROGRAMS "${WIGGL3_EXE}" DESTINATION bin)

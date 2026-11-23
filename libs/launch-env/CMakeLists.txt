# SPDX-License-Identifier: MIT
# The launch-time display hand-off (T309). The compositor writes the Xwayland
# `DISPLAY` to `$XDG_RUNTIME_DIR/<socket>.x11-display` after Xwayland is ready;
# every process that launches a child reads that file at launch time. A
# header-only interface keeps the read in one place without a runtime library.

add_library(dragonfruit-launch-env INTERFACE)
target_link_libraries(dragonfruit-launch-env INTERFACE Qt6::Core)
target_include_directories(dragonfruit-launch-env INTERFACE "${CMAKE_CURRENT_SOURCE_DIR}")

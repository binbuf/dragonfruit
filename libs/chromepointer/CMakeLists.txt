# SPDX-License-Identifier: MIT
# Chrome pointer injection (T-16.12), shared by the shell chrome and the
# Files-owned desktop process (T-19.3). The single constructor for every
# injected offscreen pointer event: the monotonic timestamp is owned here,
# never by the call site, so the T-14.7x `DragHandler` bug cannot reappear at a
# second injection site.

add_library(dragonfruit-chromepointer STATIC
    chromepointer.cpp
    chromepointer.h
)
target_link_libraries(dragonfruit-chromepointer PUBLIC Qt6::Core Qt6::Gui)
target_include_directories(dragonfruit-chromepointer PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}")
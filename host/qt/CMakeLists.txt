cmake_minimum_required(VERSION 3.21)
project(kero_qt_host LANGUAGES CXX)

list(APPEND CMAKE_MODULE_PATH "${CMAKE_CURRENT_SOURCE_DIR}/cmake")

include(KeroConfiguration)
include(KeroApplications)
include(KeroPackaging)
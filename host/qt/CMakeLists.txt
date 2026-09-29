cmake_minimum_required(VERSION 3.21)
project(kero_qt_host VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTOUIC ON)
set(CMAKE_AUTORCC ON)

# Toolchain locations are supplied by CMakePresets.json and environment-backed
# user presets; no platform path is committed here.
set(KERO_QT_PREFIX "$ENV{KERO_QT_PREFIX}" CACHE PATH "Qt kit prefix")
set(KERO_RUNTIME_DIR "$ENV{KERO_RUNTIME_DIR}" CACHE PATH "Compiler runtime directory")
set(KERO_DEPLOY_TOOL "$ENV{KERO_DEPLOY_TOOL}" CACHE FILEPATH "Qt deployment tool")
set(KERO_ICON "$ENV{KERO_ICON}" CACHE FILEPATH "Generated Windows icon")
set(KERO_WINDRES "$ENV{KERO_WINDRES}" CACHE FILEPATH "Windows resource compiler")
set(KERO_RELEASE_DIRECTORY "$ENV{KERO_RELEASE_DIRECTORY}" CACHE PATH "Release package output directory")
set(KERO_HOST "$ENV{KERO_HOST}" CACHE FILEPATH "Built KERO terminal host executable")

if(KERO_QT_PREFIX)
  list(PREPEND CMAKE_PREFIX_PATH "${KERO_QT_PREFIX}")
endif()
find_package(Qt6 6.8 REQUIRED COMPONENTS Network Widgets)
if(NOT DEFINED ENV{KERO_WASM} OR NOT EXISTS "$ENV{KERO_WASM}")
  message(FATAL_ERROR "KERO_WASM must name the freshly built portable core")
endif()
if(NOT KERO_HOST OR NOT EXISTS "${KERO_HOST}")
  message(FATAL_ERROR "KERO_HOST must name the freshly built terminal host executable")
endif()

if(WIN32)
  if(KERO_WINDRES)
    file(TO_CMAKE_PATH "${KERO_WINDRES}" KERO_WINDRES_CMAKE_PATH)
    set(CMAKE_RC_COMPILER "${KERO_WINDRES_CMAKE_PATH}")
  endif()
  enable_language(RC)
  if(NOT KERO_ICON OR NOT EXISTS "${KERO_ICON}")
    message(FATAL_ERROR "Windows packages require KERO_ICON")
  endif()
  if(NOT MSVC AND NOT KERO_WINDRES)
    message(FATAL_ERROR "LLVM/MinGW packages require KERO_WINDRES")
  endif()
  file(TO_CMAKE_PATH "${KERO_ICON}" KERO_ICON_PATH)
  configure_file(resources/kero.rc.in "${CMAKE_CURRENT_BINARY_DIR}/kero.rc" @ONLY)
endif()

add_library(kero_qt_common common/kero_window.cpp common/kero_home.cpp)
target_link_libraries(kero_qt_common PUBLIC Qt6::Widgets)
target_include_directories(kero_qt_common PUBLIC common)

function(add_kero_role role)
  set(options)
  set(one_value MAIN)
  set(multi_value SOURCES LIBRARIES)
  cmake_parse_arguments(ARG "${options}" "${one_value}" "${multi_value}" ${ARGN})
  if(WIN32)
    add_executable(kero-${role} WIN32 ${ARG_MAIN} ${ARG_SOURCES} resources/kero.qrc "${CMAKE_CURRENT_BINARY_DIR}/kero.rc")
  elseif(APPLE)
    add_executable(kero-${role} MACOSX_BUNDLE ${ARG_MAIN} ${ARG_SOURCES} resources/kero.qrc)
  else()
    add_executable(kero-${role} ${ARG_MAIN} ${ARG_SOURCES} resources/kero.qrc)
  endif()
  target_link_libraries(kero-${role} PRIVATE kero_qt_common ${ARG_LIBRARIES})
endfunction()

add_kero_role(install MAIN install/main.cpp
  SOURCES install/kero_installer.cpp app/kero_automation_server.cpp install/aspect_fill_label.cpp install/kero-install.ui
  LIBRARIES Qt6::Network)
target_include_directories(kero-install PRIVATE install)
if(WIN32)
  target_link_libraries(kero-install PRIVATE ole32 shell32)
endif()
add_kero_role(app MAIN app/main.cpp SOURCES app/kero_app.cpp app/kero_service_client.cpp app/kero_automation_server.cpp app/kero-app.ui LIBRARIES Qt6::Network)
set_target_properties(kero-app PROPERTIES OUTPUT_NAME kero)
add_kero_role(uninstall MAIN uninstall/main.cpp)

function(deploy_kero_role role)
  if(WIN32)
    if(NOT KERO_DEPLOY_TOOL)
      message(FATAL_ERROR "Windows packages require KERO_DEPLOY_TOOL")
    endif()
    # Deployment happens against the install stage in kero-package.  That is
    # the directory embedded in the installer, rather than an incidental
    # build-output directory.
  elseif(APPLE)
    if(NOT KERO_DEPLOY_TOOL)
      message(FATAL_ERROR "macOS packages require KERO_DEPLOY_TOOL (macdeployqt)")
    endif()
    add_custom_command(TARGET kero-${role} POST_BUILD
      COMMAND "${KERO_DEPLOY_TOOL}" "$<TARGET_BUNDLE_DIR:kero-${role}>" -always-overwrite)
  endif()
endfunction()
foreach(role IN ITEMS install app uninstall)
  deploy_kero_role(${role})
endforeach()

install(TARGETS kero-install kero-app kero-uninstall
  RUNTIME DESTINATION bin BUNDLE DESTINATION .)
install(FILES "${CMAKE_CURRENT_SOURCE_DIR}/resources/kero.qss" DESTINATION bin)
install(FILES "$ENV{KERO_WASM}" DESTINATION bin RENAME kero.wasm)
install(PROGRAMS "${KERO_HOST}" DESTINATION bin RENAME kero-host${CMAKE_EXECUTABLE_SUFFIX})

set(CPACK_PACKAGE_NAME "KERO")
set(CPACK_PACKAGE_VERSION "${PROJECT_VERSION}")
set(CPACK_PACKAGE_FILE_NAME "kero-${PROJECT_VERSION}-${CMAKE_SYSTEM_NAME}-${CMAKE_SYSTEM_PROCESSOR}")
set(CPACK_PACKAGE_DIRECTORY "${KERO_RELEASE_DIRECTORY}")
if(APPLE)
  set(CPACK_GENERATOR DragNDrop)
elseif(UNIX)
  # linuxdeploy consumes this staged install tree to produce the AppImage.
  set(CPACK_GENERATOR TGZ)
else()
  set(CPACK_GENERATOR ZIP)
endif()
include(CPack)

if(WIN32)
  if(MSVC)
    add_custom_target(kero-package
      COMMAND ${CMAKE_COMMAND} --install "${CMAKE_BINARY_DIR}" --prefix "${CMAKE_BINARY_DIR}/stage"
      COMMAND "${KERO_DEPLOY_TOOL}" --dir "${CMAKE_BINARY_DIR}/stage/bin" --compiler-runtime --no-system-dxc-compiler "${CMAKE_BINARY_DIR}/stage/bin/kero-install.exe"
      COMMAND ${CMAKE_COMMAND} -E copy_if_different "${CMAKE_CURRENT_SOURCE_DIR}/resources/kero.qss" "${CMAKE_BINARY_DIR}/stage/bin/kero.qss"
      COMMAND ${CMAKE_CPACK_COMMAND} --config "${CMAKE_BINARY_DIR}/CPackConfig.cmake"
      DEPENDS kero-install kero-app kero-uninstall
      COMMENT "Staging and deploying the Windows KERO bundle")
  else()
    add_custom_target(kero-package
      COMMAND ${CMAKE_COMMAND} --install "${CMAKE_BINARY_DIR}" --prefix "${CMAKE_BINARY_DIR}/stage"
      COMMAND "${KERO_DEPLOY_TOOL}" --dir "${CMAKE_BINARY_DIR}/stage/bin" --no-system-dxc-compiler "${CMAKE_BINARY_DIR}/stage/bin/kero-install.exe"
      COMMAND ${CMAKE_COMMAND} -E copy_if_different "${CMAKE_CURRENT_SOURCE_DIR}/resources/kero.qss" "${CMAKE_BINARY_DIR}/stage/bin/kero.qss"
      COMMAND ${CMAKE_COMMAND} -E copy_if_different "${KERO_RUNTIME_DIR}/libc++.dll" "${CMAKE_BINARY_DIR}/stage/bin/libc++.dll"
      COMMAND ${CMAKE_COMMAND} -E copy_if_different "${KERO_RUNTIME_DIR}/libunwind.dll" "${CMAKE_BINARY_DIR}/stage/bin/libunwind.dll"
      COMMAND ${CMAKE_COMMAND} -E copy_if_different "${KERO_RUNTIME_DIR}/libwinpthread-1.dll" "${CMAKE_BINARY_DIR}/stage/bin/libwinpthread-1.dll"
      COMMAND ${CMAKE_CPACK_COMMAND} --config "${CMAKE_BINARY_DIR}/CPackConfig.cmake"
      DEPENDS kero-install kero-app kero-uninstall
      COMMENT "Staging and deploying the Windows KERO bundle")
  endif()
elseif(APPLE)
  add_custom_target(kero-package
    COMMAND ${CMAKE_COMMAND} --install "${CMAKE_BINARY_DIR}" --prefix "${CMAKE_BINARY_DIR}/stage"
    COMMAND "${KERO_DEPLOY_TOOL}" "${CMAKE_BINARY_DIR}/stage/kero-install.app" -always-overwrite
    COMMAND "${KERO_DEPLOY_TOOL}" "${CMAKE_BINARY_DIR}/stage/kero-app.app" -always-overwrite
    COMMAND "${KERO_DEPLOY_TOOL}" "${CMAKE_BINARY_DIR}/stage/kero-uninstall.app" -always-overwrite
    COMMAND ${CMAKE_CPACK_COMMAND} --config "${CMAKE_BINARY_DIR}/CPackConfig.cmake"
    DEPENDS kero-install kero-app kero-uninstall
    COMMENT "Staging and deploying the macOS KERO bundle")
else()
  add_custom_target(kero-package
    COMMAND ${CMAKE_COMMAND} --install "${CMAKE_BINARY_DIR}" --prefix "${CMAKE_BINARY_DIR}/stage/usr"
    COMMAND ${CMAKE_CPACK_COMMAND} --config "${CMAKE_BINARY_DIR}/CPackConfig.cmake"
    DEPENDS kero-install kero-app kero-uninstall
    COMMENT "Staging the Linux KERO bundle")
endif()

if(WIN32)
  set(KERO_WINDOWS_PAYLOAD "${KERO_RELEASE_DIRECTORY}/kero-${CMAKE_SYSTEM_PROCESSOR}-payload.zip")
  set(KERO_WINDOWS_RESOURCE "${CMAKE_CURRENT_BINARY_DIR}/kero-installer-payload.rc")
  set(KERO_WINDOWS_RESOURCE_OBJECT "${CMAKE_CURRENT_BINARY_DIR}/kero-installer-payload.res")
  set(KERO_WINDOWS_INSTALLER "${KERO_RELEASE_DIRECTORY}/kero-installer.exe")
  set(KERO_WINDOWS_BOOTSTRAP_STAGE "${CMAKE_CURRENT_BINARY_DIR}/bootstrap")
  configure_file("${CMAKE_CURRENT_SOURCE_DIR}/resources/kero-installer-payload.rc.in" "${KERO_WINDOWS_RESOURCE}" @ONLY)
  if(MSVC)
    # The package script invokes cl.exe directly for the tiny bootstrap.  It
    # can run outside a Visual Studio developer prompt, so provide the SDK
    # roots explicitly instead of relying on INCLUDE/LIB being inherited.
    file(GLOB KERO_WINDOWS_SDK_VERSIONS LIST_DIRECTORIES true
      "C:/Program Files (x86)/Windows Kits/10/Include/*")
    list(SORT KERO_WINDOWS_SDK_VERSIONS COMPARE NATURAL ORDER DESCENDING)
    list(GET KERO_WINDOWS_SDK_VERSIONS 0 KERO_WINDOWS_SDK_INCLUDE)
    get_filename_component(KERO_WINDOWS_SDK_VERSION "${KERO_WINDOWS_SDK_INCLUDE}" NAME)
    set(KERO_WINDOWS_SDK_LIB "C:/Program Files (x86)/Windows Kits/10/Lib/${KERO_WINDOWS_SDK_VERSION}")
    get_filename_component(KERO_MSVC_BIN "${CMAKE_CXX_COMPILER}" DIRECTORY)
    get_filename_component(KERO_MSVC_TOOLS "${KERO_MSVC_BIN}/../../.." ABSOLUTE)
    if(NOT EXISTS "${KERO_WINDOWS_SDK_INCLUDE}/um/windows.h" OR NOT EXISTS "${KERO_WINDOWS_SDK_LIB}/um/${CMAKE_SYSTEM_PROCESSOR}/user32.lib")
      message(FATAL_ERROR "Windows package bootstrap requires matching Windows SDK headers and ${CMAKE_SYSTEM_PROCESSOR} libraries")
    endif()
    set(KERO_BOOTSTRAP_INCLUDE_FLAGS
      "/I${KERO_MSVC_TOOLS}/include"
      "/I${KERO_WINDOWS_SDK_INCLUDE}/shared"
      "/I${KERO_WINDOWS_SDK_INCLUDE}/um"
      "/I${KERO_WINDOWS_SDK_INCLUDE}/ucrt"
      "/I${KERO_WINDOWS_SDK_INCLUDE}/winrt")
    set(KERO_BOOTSTRAP_LINK_FLAGS
      "/LIBPATH:${KERO_MSVC_TOOLS}/lib/${CMAKE_SYSTEM_PROCESSOR}"
      "/LIBPATH:${KERO_WINDOWS_SDK_LIB}/um/${CMAKE_SYSTEM_PROCESSOR}"
      "/LIBPATH:${KERO_WINDOWS_SDK_LIB}/ucrt/${CMAKE_SYSTEM_PROCESSOR}")
    add_custom_target(kero-windows-installer
      COMMAND ${CMAKE_COMMAND} -E make_directory "${KERO_RELEASE_DIRECTORY}"
      COMMAND ${CMAKE_COMMAND} -E rm -rf "${KERO_WINDOWS_BOOTSTRAP_STAGE}"
      COMMAND ${CMAKE_COMMAND} -E make_directory "${KERO_WINDOWS_BOOTSTRAP_STAGE}/payload"
      COMMAND ${CMAKE_COMMAND} -E copy_directory "${CMAKE_BINARY_DIR}/stage/bin" "${KERO_WINDOWS_BOOTSTRAP_STAGE}/payload"
      COMMAND ${CMAKE_COMMAND} -E copy_directory "${CMAKE_BINARY_DIR}/stage/bin" "${KERO_WINDOWS_BOOTSTRAP_STAGE}"
      COMMAND ${CMAKE_COMMAND} -E chdir "${KERO_WINDOWS_BOOTSTRAP_STAGE}" ${CMAKE_COMMAND} -E tar cf "${KERO_WINDOWS_PAYLOAD}" --format=zip -- .
      COMMAND "${CMAKE_RC_COMPILER}" /nologo /fo"${KERO_WINDOWS_RESOURCE_OBJECT}" "${KERO_WINDOWS_RESOURCE}"
      COMMAND "${CMAKE_CXX_COMPILER}" /std:c++20 /EHsc ${KERO_BOOTSTRAP_INCLUDE_FLAGS} "${CMAKE_CURRENT_SOURCE_DIR}/../bootstrap/windows_main.cpp" "${KERO_WINDOWS_RESOURCE_OBJECT}" /link ${KERO_BOOTSTRAP_LINK_FLAGS} /SUBSYSTEM:WINDOWS user32.lib /OUT:"${KERO_WINDOWS_INSTALLER}"
      DEPENDS kero-package WORKING_DIRECTORY "${CMAKE_BINARY_DIR}/stage/bin")
  else()
    add_custom_target(kero-windows-installer
      COMMAND ${CMAKE_COMMAND} -E make_directory "${KERO_RELEASE_DIRECTORY}"
      COMMAND ${CMAKE_COMMAND} -E rm -rf "${KERO_WINDOWS_BOOTSTRAP_STAGE}"
      COMMAND ${CMAKE_COMMAND} -E make_directory "${KERO_WINDOWS_BOOTSTRAP_STAGE}/payload"
      COMMAND ${CMAKE_COMMAND} -E copy_directory "${CMAKE_BINARY_DIR}/stage/bin" "${KERO_WINDOWS_BOOTSTRAP_STAGE}/payload"
      COMMAND ${CMAKE_COMMAND} -E copy_directory "${CMAKE_BINARY_DIR}/stage/bin" "${KERO_WINDOWS_BOOTSTRAP_STAGE}"
      COMMAND ${CMAKE_COMMAND} -E chdir "${KERO_WINDOWS_BOOTSTRAP_STAGE}" ${CMAKE_COMMAND} -E tar cf "${KERO_WINDOWS_PAYLOAD}" --format=zip -- .
      COMMAND "${KERO_WINDRES}" -O coff "${KERO_WINDOWS_RESOURCE}" -o "${KERO_WINDOWS_RESOURCE_OBJECT}"
      COMMAND "${CMAKE_CXX_COMPILER}" "${CMAKE_CURRENT_SOURCE_DIR}/../bootstrap/windows_main.cpp" "${KERO_WINDOWS_RESOURCE_OBJECT}" -std=c++20 -mwindows -municode -static -static-libstdc++ -static-libgcc -o "${KERO_WINDOWS_INSTALLER}"
      DEPENDS kero-package WORKING_DIRECTORY "${CMAKE_BINARY_DIR}/stage/bin")
  endif()
elseif(UNIX AND NOT APPLE)
  add_custom_target(kero-linux-appimage
    COMMAND ${CMAKE_COMMAND} -E make_directory "${KERO_RELEASE_DIRECTORY}"
    COMMAND ${CMAKE_COMMAND} -E env
      "LDAI_OUTPUT=${KERO_RELEASE_DIRECTORY}/kero.AppImage"
      "LDAI_RUNTIME_FILE=$ENV{KERO_APPIMAGE_RUNTIME}"
      "ARCH=$ENV{KERO_LINUX_ARCH}"
      "QMAKE=$ENV{KERO_QT_PREFIX}/bin/qmake"
      "APPIMAGE_EXTRACT_AND_RUN=1"
      "$ENV{KERO_LINUXDEPLOY}" --appdir "${CMAKE_BINARY_DIR}/stage"
      --executable "${CMAKE_BINARY_DIR}/stage/usr/bin/kero-install"
      --desktop-file "${CMAKE_CURRENT_SOURCE_DIR}/resources/kero.desktop"
      --icon-file "$ENV{KERO_LINUX_ICON}"
      --output appimage
    DEPENDS kero-package
    WORKING_DIRECTORY "${KERO_RELEASE_DIRECTORY}"
    COMMENT "Building the Linux AppImage")
endif()

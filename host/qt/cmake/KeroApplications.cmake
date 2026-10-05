add_library(kero_qt_common
  common/kero_window.cpp
  common/kero_home.cpp)
target_compile_features(kero_qt_common PUBLIC cxx_std_20)
target_link_libraries(kero_qt_common PUBLIC Qt6::Widgets)
target_include_directories(kero_qt_common PUBLIC common)

function(add_kero_role role)
  set(options)
  set(one_value MAIN)
  set(multi_value SOURCES LIBRARIES)
  cmake_parse_arguments(ARG "${options}" "${one_value}" "${multi_value}" ${ARGN})

  if(WIN32)
    add_executable(kero-${role} WIN32 "${ARG_MAIN}" ${ARG_SOURCES} resources/kero.qrc "${CMAKE_CURRENT_BINARY_DIR}/kero.rc")
  elseif(APPLE)
    add_executable(kero-${role} MACOSX_BUNDLE "${ARG_MAIN}" ${ARG_SOURCES} resources/kero.qrc)
  else()
    add_executable(kero-${role} "${ARG_MAIN}" ${ARG_SOURCES} resources/kero.qrc)
  endif()

  target_compile_features(kero-${role} PRIVATE cxx_std_20)
  target_link_libraries(kero-${role} PRIVATE kero_qt_common ${ARG_LIBRARIES})
endfunction()

add_kero_role(install MAIN install/main.cpp
  SOURCES install/kero_installer.cpp app/kero_automation_server.cpp install/aspect_fill_label.cpp install/kero-install.ui
  LIBRARIES Qt6::Network)
target_include_directories(kero-install PRIVATE install)

if(WIN32)
  target_link_libraries(kero-install PRIVATE ole32 shell32)
endif()

add_kero_role(app MAIN app/main.cpp
  SOURCES app/kero_app.cpp app/kero_service_client.cpp app/kero_automation_server.cpp app/kero-app.ui
  LIBRARIES Qt6::Network)
set_target_properties(kero-app PROPERTIES OUTPUT_NAME kero)

add_kero_role(uninstall MAIN uninstall/main.cpp)

install(TARGETS kero-install kero-app kero-uninstall
  RUNTIME DESTINATION bin BUNDLE DESTINATION .)
install(FILES "${CMAKE_CURRENT_SOURCE_DIR}/resources/kero.qss" DESTINATION bin)
install(FILES "$ENV{KERO_WASM}" DESTINATION bin RENAME kero.wasm)
install(PROGRAMS "${KERO_HOST}" DESTINATION bin RENAME kero-host${CMAKE_EXECUTABLE_SUFFIX})

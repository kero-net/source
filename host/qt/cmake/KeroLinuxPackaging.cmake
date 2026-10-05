add_custom_target(kero-package
  COMMAND ${CMAKE_COMMAND} --install "${CMAKE_BINARY_DIR}" --prefix "${CMAKE_BINARY_DIR}/stage/usr"
  COMMAND ${CMAKE_CPACK_COMMAND} --config "${CMAKE_BINARY_DIR}/CPackConfig.cmake"
  DEPENDS kero-install kero-app kero-uninstall
  COMMENT "Staging the Linux KERO bundle")

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

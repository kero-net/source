add_custom_target(kero-package
  COMMAND ${CMAKE_COMMAND} --install "${CMAKE_BINARY_DIR}" --prefix "${CMAKE_BINARY_DIR}/stage"
  COMMAND "${KERO_DEPLOY_TOOL}" "${CMAKE_BINARY_DIR}/stage/kero-install.app" -always-overwrite
  COMMAND "${KERO_DEPLOY_TOOL}" "${CMAKE_BINARY_DIR}/stage/kero-app.app" -always-overwrite
  COMMAND "${KERO_DEPLOY_TOOL}" "${CMAKE_BINARY_DIR}/stage/kero-uninstall.app" -always-overwrite
  COMMAND ${CMAKE_CPACK_COMMAND} --config "${CMAKE_BINARY_DIR}/CPackConfig.cmake"
  DEPENDS kero-install kero-app kero-uninstall
  COMMENT "Staging and deploying the macOS KERO bundle")

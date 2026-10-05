set(KERO_WINDOWS_PAYLOAD "${KERO_RELEASE_DIRECTORY}/kero-${CMAKE_SYSTEM_PROCESSOR}-payload.zip")
set(KERO_WINDOWS_RESOURCE "${CMAKE_CURRENT_BINARY_DIR}/kero-installer-payload.rc")
set(KERO_WINDOWS_RESOURCE_OBJECT "${CMAKE_CURRENT_BINARY_DIR}/kero-installer-payload.res")
set(KERO_WINDOWS_INSTALLER "${KERO_RELEASE_DIRECTORY}/kero-installer.exe")
set(KERO_WINDOWS_BOOTSTRAP_STAGE "${CMAKE_CURRENT_BINARY_DIR}/bootstrap")
configure_file("${CMAKE_CURRENT_SOURCE_DIR}/resources/kero-installer-payload.rc.in" "${KERO_WINDOWS_RESOURCE}" @ONLY)

if(MSVC)
  if(CMAKE_SYSTEM_PROCESSOR STREQUAL "ARM64")
    set(KERO_QT_DEPLOY_COMMAND
      COMMAND ${CMAKE_COMMAND} "-DKERO_QT_PREFIX=${Qt6_DIR}/../../.." "-DKERO_STAGE_BIN=${CMAKE_BINARY_DIR}/stage/bin"
        -P "${CMAKE_CURRENT_LIST_DIR}/KeroDeployArm64.cmake")
  else()
    set(KERO_QT_DEPLOY_COMMAND
      COMMAND "${KERO_DEPLOY_TOOL}" --dir "${CMAKE_BINARY_DIR}/stage/bin" --compiler-runtime --no-system-d3d-compiler --no-system-dxc-compiler "${CMAKE_BINARY_DIR}/stage/bin/kero-install.exe")
  endif()
  add_custom_target(kero-package
    COMMAND ${CMAKE_COMMAND} --install "${CMAKE_BINARY_DIR}" --prefix "${CMAKE_BINARY_DIR}/stage"
    ${KERO_QT_DEPLOY_COMMAND}
    COMMAND ${CMAKE_COMMAND} -E copy_if_different "${CMAKE_CURRENT_SOURCE_DIR}/resources/kero.qss" "${CMAKE_BINARY_DIR}/stage/bin/kero.qss"
    COMMAND ${CMAKE_CPACK_COMMAND} --config "${CMAKE_BINARY_DIR}/CPackConfig.cmake"
    DEPENDS kero-install kero-app kero-uninstall
    COMMENT "Staging and deploying the Windows KERO bundle")

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
  set(KERO_BOOTSTRAP_COMPILE_COMMAND
    "${CMAKE_CXX_COMPILER}" /std:c++20 /EHsc ${KERO_BOOTSTRAP_INCLUDE_FLAGS}
    "${CMAKE_CURRENT_SOURCE_DIR}/bootstrap/windows_main.cpp"
    "${KERO_WINDOWS_RESOURCE_OBJECT}" /link ${KERO_BOOTSTRAP_LINK_FLAGS}
    /SUBSYSTEM:WINDOWS user32.lib /OUT:"${KERO_WINDOWS_INSTALLER}")
else()
  add_custom_target(kero-package
    COMMAND ${CMAKE_COMMAND} --install "${CMAKE_BINARY_DIR}" --prefix "${CMAKE_BINARY_DIR}/stage"
    COMMAND "${KERO_DEPLOY_TOOL}" --dir "${CMAKE_BINARY_DIR}/stage/bin" --no-system-d3d-compiler --no-system-dxc-compiler "${CMAKE_BINARY_DIR}/stage/bin/kero-install.exe"
    COMMAND ${CMAKE_COMMAND} -E copy_if_different "${CMAKE_CURRENT_SOURCE_DIR}/resources/kero.qss" "${CMAKE_BINARY_DIR}/stage/bin/kero.qss"
    COMMAND ${CMAKE_COMMAND} -E copy_if_different "${KERO_RUNTIME_DIR}/libc++.dll" "${CMAKE_BINARY_DIR}/stage/bin/libc++.dll"
    COMMAND ${CMAKE_COMMAND} -E copy_if_different "${KERO_RUNTIME_DIR}/libunwind.dll" "${CMAKE_BINARY_DIR}/stage/bin/libunwind.dll"
    COMMAND ${CMAKE_COMMAND} -E copy_if_different "${KERO_RUNTIME_DIR}/libwinpthread-1.dll" "${CMAKE_BINARY_DIR}/stage/bin/libwinpthread-1.dll"
    COMMAND ${CMAKE_CPACK_COMMAND} --config "${CMAKE_BINARY_DIR}/CPackConfig.cmake"
    DEPENDS kero-install kero-app kero-uninstall
    COMMENT "Staging and deploying the Windows KERO bundle")

  set(KERO_BOOTSTRAP_COMPILE_COMMAND
    "${CMAKE_CXX_COMPILER}"
    "${CMAKE_CURRENT_SOURCE_DIR}/bootstrap/windows_main.cpp"
    "${KERO_WINDOWS_RESOURCE_OBJECT}" -std=c++20 -mwindows -municode -static
    -static-libstdc++ -static-libgcc -o "${KERO_WINDOWS_INSTALLER}")
endif()

if(MSVC)
  set(KERO_WINDOWS_RESOURCE_COMMAND "${CMAKE_RC_COMPILER}" /nologo /fo"${KERO_WINDOWS_RESOURCE_OBJECT}" "${KERO_WINDOWS_RESOURCE}")
else()
  set(KERO_WINDOWS_RESOURCE_COMMAND "${KERO_WINDRES}" -O coff "${KERO_WINDOWS_RESOURCE}" -o "${KERO_WINDOWS_RESOURCE_OBJECT}")
endif()

add_custom_target(kero-windows-installer
  COMMAND ${CMAKE_COMMAND} -E make_directory "${KERO_RELEASE_DIRECTORY}"
  COMMAND ${CMAKE_COMMAND} -E rm -rf "${KERO_WINDOWS_BOOTSTRAP_STAGE}"
  COMMAND ${CMAKE_COMMAND} -E make_directory "${KERO_WINDOWS_BOOTSTRAP_STAGE}/payload"
  COMMAND ${CMAKE_COMMAND} -E copy_directory "${CMAKE_BINARY_DIR}/stage/bin" "${KERO_WINDOWS_BOOTSTRAP_STAGE}/payload"
  COMMAND ${CMAKE_COMMAND} -E copy_directory "${CMAKE_BINARY_DIR}/stage/bin" "${KERO_WINDOWS_BOOTSTRAP_STAGE}"
  COMMAND ${CMAKE_COMMAND} -E chdir "${KERO_WINDOWS_BOOTSTRAP_STAGE}" ${CMAKE_COMMAND} -E tar cf "${KERO_WINDOWS_PAYLOAD}" --format=zip -- .
  COMMAND ${KERO_WINDOWS_RESOURCE_COMMAND}
  COMMAND ${KERO_BOOTSTRAP_COMPILE_COMMAND}
  DEPENDS kero-package
  WORKING_DIRECTORY "${CMAKE_BINARY_DIR}/stage/bin")

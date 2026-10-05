# Qt's Windows ARM64 kit has no native windeployqt. Copy the exact runtime
# dependencies from the target kit; the host windeployqt selects x64 DLLs.
get_filename_component(KERO_QT_PREFIX "${KERO_QT_PREFIX}" ABSOLUTE)
foreach(library Qt6Core Qt6Gui Qt6Network Qt6Widgets)
  file(COPY_FILE "${KERO_QT_PREFIX}/bin/${library}.dll" "${KERO_STAGE_BIN}/${library}.dll")
endforeach()
foreach(plugin
    generic/qtuiotouchplugin
    imageformats/qgif
    imageformats/qico
    imageformats/qjpeg
    networkinformation/qnetworklistmanager
    platforms/qwindows
    styles/qmodernwindowsstyle
    tls/qcertonlybackend
    tls/qschannelbackend)
  get_filename_component(directory "${plugin}" DIRECTORY)
  file(MAKE_DIRECTORY "${KERO_STAGE_BIN}/${directory}")
  file(COPY_FILE "${KERO_QT_PREFIX}/plugins/${plugin}.dll" "${KERO_STAGE_BIN}/${plugin}.dll")
endforeach()
if(EXISTS "${KERO_QT_PREFIX}/bin/opengl32sw.dll")
  file(COPY_FILE "${KERO_QT_PREFIX}/bin/opengl32sw.dll" "${KERO_STAGE_BIN}/opengl32sw.dll")
endif()

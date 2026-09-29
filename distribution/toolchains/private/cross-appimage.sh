#!/usr/bin/env bash
set -euo pipefail

appdir= desktop= icon=
while (($#)); do
  case "$1" in
    --appdir) appdir=$2; shift 2 ;;
    --desktop-file) desktop=$2; shift 2 ;;
    --icon-file) icon=$2; shift 2 ;;
    --executable|--output) shift 2 ;;
    *) shift ;;
  esac
done

: "${appdir:?}" "${desktop:?}" "${icon:?}" "${KERO_QT_PREFIX:?}" "${KERO_NATIVE_LINUXDEPLOY:?}" "${LDAI_OUTPUT:?}" "${LDAI_RUNTIME_FILE:?}"
mkdir -p "$appdir/usr/lib" "$appdir/usr/plugins" "$appdir/usr/share/applications" "$appdir/usr/share/icons/hicolor/512x512/apps"
cp -a "$KERO_QT_PREFIX"/lib/libQt6*.so* "$appdir/usr/lib/"
cp -a "$KERO_QT_PREFIX"/lib/libicu*.so* "$appdir/usr/lib/"
for plugin in platforms platforminputcontexts imageformats xcbglintegrations egldeviceintegrations platformthemes tls; do
  [[ -d "$KERO_QT_PREFIX/plugins/$plugin" ]] && cp -a "$KERO_QT_PREFIX/plugins/$plugin" "$appdir/usr/plugins/"
done
cp "$desktop" "$appdir/usr/share/applications/kero.desktop"
cp "$icon" "$appdir/usr/share/icons/hicolor/512x512/apps/kero.png"
cp "$desktop" "$appdir/kero.desktop"
cp "$icon" "$appdir/kero.png"
cat > "$appdir/AppRun" <<'EOF'
#!/usr/bin/env bash
HERE=$(cd "$(dirname "$0")" && pwd)
export LD_LIBRARY_PATH="$HERE/usr/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export QT_PLUGIN_PATH="$HERE/usr/plugins"
exec "$HERE/usr/bin/kero-install" "$@"
EOF
chmod +x "$appdir/AppRun"
extract_dir="${KERO_NATIVE_LINUXDEPLOY%/*}/native-extracted/squashfs-root"
if [[ ! -x "$extract_dir/plugins/linuxdeploy-plugin-appimage/usr/bin/appimagetool" ]]; then
  extract_parent=${extract_dir%/*}
  rm -rf "$extract_parent"
  mkdir -p "$extract_parent"
  (cd "$extract_parent" && "$KERO_NATIVE_LINUXDEPLOY" --appimage-extract >/dev/null)
fi
ARCH=x86_64 "$extract_dir/plugins/linuxdeploy-plugin-appimage/usr/bin/appimagetool" "$appdir" "$LDAI_OUTPUT" --runtime-file "$LDAI_RUNTIME_FILE"

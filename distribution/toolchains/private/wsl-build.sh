#!/usr/bin/env bash
# Private adapter called only by the KERO target coordinator.
set -euo pipefail

workspace=${1:?}
target=${2:?}
qt_version=${3:?}
qt_directory=${4:?}
deploy_arch=${5:?}
rust_target=${6:?}
work_root=${KERO_WSL_WORK_ROOT:?Linux builds require an ext4 work root}
source_root="$work_root/build/source"
test -f "$source_root/src/Cargo.toml"
tools="$work_root/cache/toolchains"
dependencies="$workspace/.heap/cache/downloads"
qt="$tools/qt/$target/$qt_version/$qt_directory"

extract_qt() {
  local name=$1 destination=$2
  if [[ ! -f "$destination/lib/cmake/Qt6/Qt6Config.cmake" ]]; then
    rm -rf "$destination"
    mkdir -p "$destination"
    for archive in "$dependencies/qt/$name"/*.7z; do
      if [[ ${archive##*/} == icu-* ]]; then
        mkdir -p "$destination/lib"
        (cd "$destination/lib" && cmake -E tar xf "$archive")
      else
        (cd "$destination" && cmake -E tar xf "$archive")
      fi
    done
  fi
  test -f "$destination/lib/cmake/Qt6/Qt6Config.cmake"
}

extract_qt "$target" "$qt"
chmod +x "$dependencies/linuxdeploy/linuxdeploy-$deploy_arch.AppImage"
export KERO_QT_PREFIX="$qt"
export KERO_LINUXDEPLOY="$dependencies/linuxdeploy/linuxdeploy-$deploy_arch.AppImage"
export KERO_LINUX_ARCH="$deploy_arch"
export KERO_APPIMAGE_RUNTIME="$dependencies/appimage/runtime-$deploy_arch"
export CARGO_TARGET_DIR="$work_root/build/cargo/$target"
export LD_LIBRARY_PATH="$qt/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
mkdir -p "$work_root/build/assets"
convert "$source_root/assets/images/kero-icon.png" -resize 512x512! "$work_root/build/assets/kero.png"
export KERO_LINUX_ICON="$work_root/build/assets/kero.png"

if [[ "$target" == linux-x64 ]]; then
  arm_qt="$tools/qt/linux-arm64/$qt_version/gcc_arm64"
  extract_qt linux-arm64 "$arm_qt"
  export KERO_ZIG="$tools/zig/zig-aarch64-linux-0.15.2/zig"
  export KERO_C_COMPILER="$source_root/distribution/toolchains/private/zig-cc.sh"
  export KERO_CXX_COMPILER="$source_root/distribution/toolchains/private/zig-cxx.sh"
  export KERO_QT_HOST_PATH="$arm_qt"
  export KERO_LINUX_SYSROOT="$tools/linux-x64/rootfs"
  export KERO_NATIVE_LINUXDEPLOY="$dependencies/linuxdeploy/linuxdeploy-aarch64.AppImage"
  export KERO_LINUXDEPLOY="$source_root/distribution/toolchains/private/cross-appimage.sh"
  export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER="$KERO_C_COMPILER"
  chmod +x "$KERO_C_COMPILER" "$KERO_CXX_COMPILER" "$KERO_NATIVE_LINUXDEPLOY" "$KERO_LINUXDEPLOY"
fi

cd "$source_root"
cargo_command=${KERO_CARGO:-cargo}
rust_toolchain=${KERO_RUST_TOOLCHAIN:-}
toolchain_option=
if [[ -n "$rust_toolchain" ]]; then
  toolchain_option=" --toolchain $rust_toolchain"
fi

rustup target add$toolchain_option "$rust_target"
"$cargo_command" build --manifest-path src/Cargo.toml --release -p kero-cli --target "$rust_target"

build="$work_root/build/qt/$target"
release="$work_root/build/releases/$target"
wasm="$work_root/build/cargo/wasm/wasm32-wasip1/release/kero_core.wasm"
host="$CARGO_TARGET_DIR/$rust_target/release/kero-host"
KERO_WASM="$wasm" KERO_HOST="$host" KERO_DEPLOY_TOOL="$KERO_LINUXDEPLOY" cmake --preset "$target" -S host/qt -B "$build" -D "KERO_RELEASE_DIRECTORY=$release"
cmake --build "$build" --target kero-package
cmake --build "$build" --target kero-linux-appimage
cmake -E sha256sum "$release/kero.AppImage" > "$release/kero.AppImage.sha256"
mkdir -p "$workspace/.heap/cache/build/releases/$target"
cp -f "$release/kero.AppImage" "$release/kero.AppImage.sha256" "$workspace/.heap/cache/build/releases/$target/"

if [[ ${KERO_SIGN_RELEASE:-0} == 1 ]]; then
  : "${KERO_GPG_KEY_ID:?KERO_GPG_KEY_ID is required when signing}"
  gpg_command=${KERO_GPG_COMMAND:-gpg}
  "$gpg_command" --batch --local-user "$KERO_GPG_KEY_ID" --detach-sign --armor "$release/kero.AppImage"
  "$gpg_command" --verify "$release/kero.AppImage.asc" "$release/kero.AppImage"
fi

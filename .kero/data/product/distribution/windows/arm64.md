# Windows ARM64

## Toolchain

The current target uses Qt's `msvc2022_arm64` kit and the matching MSVC ARM64
compiler. Contributors building this target need the appropriate MSVC ARM64
Build Tools and Windows SDK, and the build must run in an ARM64-capable MSVC
developer environment. The portable core target also requires
`wasm32-wasip1` through Rust.

The package validator rejects LLVM/MinGW when paired with this Qt kit.

## Why

This requirement comes from the selected Qt ARM64 binary distribution, which
targets the MSVC toolchain. It is not a semantic KERO-core requirement.
Mixing a MinGW/LLVM runtime with MSVC-built Qt can produce a package with
incompatible runtime assumptions.

## Consequences

Windows ARM64 currently has a Microsoft native-toolchain prerequisite even
though the portable core and other distribution targets do not.

The target requires a local Windows ARM64 host or configured local Windows ARM64 VM.

## Reconsider when

Revisit the compiler requirement if KERO adopts a Qt ARM64 distribution built
for another supported toolchain or replaces the Qt host with a distribution
model that removes this ABI constraint.

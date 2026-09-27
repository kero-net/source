//! Minimal, target-neutral WASM core surface.

/// Version of the host ABI described by this core artifact.
pub const HOST_ABI_VERSION: u32 = 1;

/// Returns the stable ABI version without disclosing host details.
#[unsafe(no_mangle)]
pub extern "C" fn kero_host_abi_version() -> u32 {
    HOST_ABI_VERSION
}

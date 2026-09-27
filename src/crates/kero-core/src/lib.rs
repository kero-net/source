//! Portable core for KERO repository environments.
//!
//! KERO core defines config and the local/mounted data boundary. Filesystem,
//! processes, networking, credentials, installation, and WASM execution are
//! supplied by a host runtime through stable adapters.

pub mod config;
pub mod host;
pub mod knowledge;
pub mod lifecycle;
pub mod platform;
pub mod setup;
pub mod wasm;

/// The current pre-release product direction.
pub const PRODUCT_DIRECTION: &str = "portable repository knowledge environment";

#[cfg(test)]
mod tests {
    #[test]
    fn product_direction_is_knowledge_only() {
        assert_eq!(
            super::PRODUCT_DIRECTION,
            "portable repository knowledge environment"
        );
    }
}

//! Core library for KERO's project-attached knowledge system.
//!
//! The first implementation layer owns project discovery, reviewable mount
//! declarations, source identity, revision observations, regions, provenance,
//! and structured diagnostics.

pub mod knowledge;

/// The current pre-release product direction.
pub const PRODUCT_DIRECTION: &str = "project-attached knowledge system";

#[cfg(test)]
mod tests {
    #[test]
    fn product_direction_is_knowledge_only() {
        assert_eq!(
            super::PRODUCT_DIRECTION,
            "project-attached knowledge system"
        );
    }
}

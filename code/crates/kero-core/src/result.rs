use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Authorization {
    Allow,
    Deny,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Verification {
    Valid,
    Invalid,
    Stale,
    Unverifiable,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Capability {
    Can,
    Cannot,
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Enforcement {
    Advisory,
    Enforced,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Execution {
    Execute,
    Block,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ExecutionResult {
    schema: String,
    authorization: Authorization,
    verification: Verification,
    capability: Capability,
    enforcement: Enforcement,
    execution: Execution,
    reason: String,
}

impl ExecutionResult {
    pub fn reason(&self) -> &str {
        &self.reason
    }

    pub fn blocked(
        authorization: Authorization,
        verification: Verification,
        capability: Capability,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            schema: "kero/execution-result/v1".into(),
            authorization,
            verification,
            capability,
            enforcement: Enforcement::Advisory,
            execution: Execution::Block,
            reason: reason.into(),
        }
    }

    /// Version 1 has no attested deployment wiring, so successful native
    /// operations can only report ADVISORY. `ENFORCED` has no public builder.
    pub fn executed_advisory(
        authorization: Authorization,
        verification: Verification,
        capability: Capability,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            schema: "kero/execution-result/v1".into(),
            authorization,
            verification,
            capability,
            enforcement: Enforcement::Advisory,
            execution: Execution::Execute,
            reason: reason.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimensions_serialize_independently() {
        let result = ExecutionResult::blocked(
            Authorization::Allow,
            Verification::Stale,
            Capability::Can,
            "artifact.expired",
        );
        let json = serde_json::to_value(result).unwrap();
        assert_eq!(json["authorization"], "ALLOW");
        assert_eq!(json["verification"], "STALE");
        assert_eq!(json["capability"], "CAN");
        assert_eq!(json["enforcement"], "ADVISORY");
        assert_eq!(json["execution"], "BLOCK");
    }
}

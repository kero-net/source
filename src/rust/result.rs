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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExecutionResult {
    pub schema: String,
    pub authorization: Authorization,
    pub verification: Verification,
    pub capability: Capability,
    pub enforcement: Enforcement,
    pub execution: Execution,
    pub reason: String,
}

impl ExecutionResult {
    pub fn blocked(
        authorization: Authorization,
        verification: Verification,
        capability: Capability,
        enforcement: Enforcement,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            schema: "scope/execution-result/v1".into(),
            authorization,
            verification,
            capability,
            enforcement,
            execution: Execution::Block,
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
            Enforcement::Enforced,
            "artifact.expired",
        );
        let json = serde_json::to_value(result).unwrap();
        assert_eq!(json["authorization"], "ALLOW");
        assert_eq!(json["verification"], "STALE");
        assert_eq!(json["capability"], "CAN");
        assert_eq!(json["enforcement"], "ENFORCED");
        assert_eq!(json["execution"], "BLOCK");
    }
}

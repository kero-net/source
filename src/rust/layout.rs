use serde::{Deserialize, Serialize};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use thiserror::Error;

pub const DIRECTORY: &str = ".scope";

const PROJECT: &str = r#"schema = "scope/project/v1"
id = "scope.project"
kind = "project"

[paths]
knowledge = "knowledge.toml"
policy_environment = "policy/environment.toml"
policy_records = ["policy/records.toml"]
checklists = "checklists"
tests = "tests"
state = "state"
"#;

const PROJECT_KNOWLEDGE: &str = r#"version = 1

[environment]
contexts = []
baseline = []

[project]
shared_root = "knowledge/shared"
local_root = "knowledge/local"
generated_root = "state/generated"
"#;

const GLOBAL: &str = r#"schema = "scope/environment/v1"
id = "scope.global"
kind = "global"

[paths]
knowledge = "knowledge.toml"
policy_environment = "policy/environment.toml"
policy_records = ["policy/records.toml"]
checklists = "checklists"
tests = "tests"
state = "state"
"#;

const GLOBAL_KNOWLEDGE: &str = r#"version = 1

[environment]
contexts = ["global"]
baseline = []

[global]
root = "knowledge"
"#;

const ENVIRONMENT: &str = r#"schema = "terminal-policy/environment/v1"
environment_id = "environment.project"
context_providers = ["branch", "environment", "project", "revocation", "time"]

[[sources]]
id = "source.project"
kind = "project-policy"
locator = "records.toml"
trust_mode = "origin"
record_actions = ["principal.define", "role.define", "role.assign", "statement.allow", "statement.deny", "delegation.issue"]
operations = ["*"]
scope_universe = ["branch", "command", "environment", "knowledge", "path", "repository", "service"]
"#;

const RECORDS: &str = r#"schema = "terminal-policy/records/v1"
source = "source.project"
"#;

const GLOBAL_ENVIRONMENT: &str = r#"schema = "terminal-policy/environment/v1"
environment_id = "environment.global"
context_providers = ["environment", "revocation", "time"]

[[sources]]
id = "source.global"
kind = "global-policy"
locator = "records.toml"
trust_mode = "origin"
record_actions = ["principal.define", "role.define", "role.assign", "statement.allow", "statement.deny", "delegation.issue"]
operations = ["*"]
scope_universe = ["branch", "command", "environment", "knowledge", "path", "repository", "service"]
"#;

const GLOBAL_RECORDS: &str = r#"schema = "terminal-policy/records/v1"
source = "source.global"
"#;

const TESTS: &str =
    "# SCOPE Project Tests\n\nProject-owned policy and boundary scenarios live here.\n";

#[derive(Debug, Error)]
pub enum LayoutError {
    #[error("layout.io: {0}")]
    Io(#[from] std::io::Error),
    #[error("layout.root-unavailable: {0}")]
    Root(PathBuf),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct InitializedLayout {
    pub schema: String,
    pub root: PathBuf,
    pub created: Vec<PathBuf>,
    pub existing: Vec<PathBuf>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ScopeIdentity {
    pub schema: String,
    pub id: String,
    pub kind: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct UserConfig {
    schema: String,
    global: GlobalLocator,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct GlobalLocator {
    root: PathBuf,
    required_id: String,
}

#[derive(Debug, Error)]
pub enum DiscoveryError {
    #[error("scope.discovery.io: {0}")]
    Io(#[from] std::io::Error),
    #[error("scope.discovery.toml: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("scope.discovery.encode: {0}")]
    Encode(#[from] toml::ser::Error),
    #[error("scope.discovery.home-unavailable")]
    HomeUnavailable,
    #[error("scope.discovery.global-unconfigured: run `scope global init`")]
    GlobalUnconfigured,
    #[error("scope.discovery.project-not-found: {0}")]
    ProjectNotFound(PathBuf),
    #[error("scope.discovery.identity-mismatch: expected {expected}, found {actual} at {path}")]
    IdentityMismatch {
        expected: String,
        actual: String,
        path: PathBuf,
    },
}

fn create_file(
    path: &Path,
    content: &str,
    result: &mut InitializedLayout,
) -> Result<(), LayoutError> {
    match OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(mut file) => {
            file.write_all(content.as_bytes())?;
            file.sync_all()?;
            result.created.push(path.to_path_buf());
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            result.existing.push(path.to_path_buf());
        }
        Err(error) => return Err(error.into()),
    }
    Ok(())
}

pub fn initialize(root: &Path) -> Result<InitializedLayout, LayoutError> {
    if !root.is_dir() {
        return Err(LayoutError::Root(root.to_path_buf()));
    }
    let directory = root.join(DIRECTORY);
    let policy = directory.join("policy");
    let tests = directory.join("tests");
    let state = directory.join("state");
    let checklists = directory.join("checklists");
    let knowledge = directory.join("knowledge");
    fs::create_dir_all(&policy)?;
    fs::create_dir_all(&tests)?;
    fs::create_dir_all(&state)?;
    fs::create_dir_all(&checklists)?;
    fs::create_dir_all(knowledge.join("shared"))?;
    fs::create_dir_all(knowledge.join("local"))?;
    let mut result = InitializedLayout {
        schema: "scope/initialized-layout/v1".into(),
        root: directory.clone(),
        created: Vec::new(),
        existing: Vec::new(),
    };
    create_file(&directory.join("scope.toml"), PROJECT, &mut result)?;
    create_file(
        &directory.join(".gitignore"),
        "/state/\n/knowledge/local/\n",
        &mut result,
    )?;
    create_file(
        &directory.join("knowledge.toml"),
        PROJECT_KNOWLEDGE,
        &mut result,
    )?;
    create_file(&policy.join("environment.toml"), ENVIRONMENT, &mut result)?;
    create_file(&policy.join("records.toml"), RECORDS, &mut result)?;
    create_file(&tests.join("README.md"), TESTS, &mut result)?;
    result.created.sort();
    result.existing.sort();
    Ok(result)
}

pub fn initialize_global(root: &Path) -> Result<InitializedLayout, LayoutError> {
    fs::create_dir_all(root)?;
    let policy = root.join("policy");
    let tests = root.join("tests");
    let state = root.join("state");
    let checklists = root.join("checklists");
    let knowledge = root.join("knowledge");
    fs::create_dir_all(&policy)?;
    fs::create_dir_all(&tests)?;
    fs::create_dir_all(&state)?;
    fs::create_dir_all(&checklists)?;
    fs::create_dir_all(&knowledge)?;
    let mut result = InitializedLayout {
        schema: "scope/initialized-global-layout/v1".into(),
        root: root.to_path_buf(),
        created: Vec::new(),
        existing: Vec::new(),
    };
    create_file(&root.join("scope.toml"), GLOBAL, &mut result)?;
    create_file(&root.join("knowledge.toml"), GLOBAL_KNOWLEDGE, &mut result)?;
    create_file(&root.join(".gitignore"), "/state/\n", &mut result)?;
    create_file(
        &policy.join("environment.toml"),
        GLOBAL_ENVIRONMENT,
        &mut result,
    )?;
    create_file(&policy.join("records.toml"), GLOBAL_RECORDS, &mut result)?;
    create_file(&tests.join("README.md"), TESTS, &mut result)?;
    result.created.sort();
    result.existing.sort();
    Ok(result)
}

fn home() -> Result<PathBuf, DiscoveryError> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or(DiscoveryError::HomeUnavailable)
}

pub fn config_path() -> Result<PathBuf, DiscoveryError> {
    if let Some(path) = env::var_os("SCOPE_CONFIG_HOME") {
        return Ok(PathBuf::from(path).join("config.toml"));
    }
    if let Some(path) = env::var_os("XDG_CONFIG_HOME") {
        return Ok(PathBuf::from(path).join("scope/config.toml"));
    }
    Ok(home()?.join(".config/scope/config.toml"))
}

pub fn configure_global(root: &Path) -> Result<PathBuf, DiscoveryError> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let content = toml::to_string_pretty(&UserConfig {
        schema: "scope/config/v1".into(),
        global: GlobalLocator {
            root: root.to_path_buf(),
            required_id: "scope.global".into(),
        },
    })?;
    fs::write(&path, content)?;
    Ok(path)
}

pub fn read_identity(root: &Path) -> Result<ScopeIdentity, DiscoveryError> {
    let content = fs::read_to_string(root.join("scope.toml"))?;
    Ok(toml::from_str(&content)?)
}

fn require_identity(
    root: PathBuf,
    expected_kind: &str,
    expected_id: Option<&str>,
) -> Result<(PathBuf, ScopeIdentity), DiscoveryError> {
    let root = root.canonicalize()?;
    let identity = read_identity(&root)?;
    if identity.kind != expected_kind {
        return Err(DiscoveryError::IdentityMismatch {
            expected: expected_kind.into(),
            actual: identity.kind,
            path: root,
        });
    }
    if let Some(expected) = expected_id
        && identity.id != expected
    {
        return Err(DiscoveryError::IdentityMismatch {
            expected: expected.into(),
            actual: identity.id,
            path: root,
        });
    }
    Ok((root, identity))
}

pub fn discover_global(
    explicit: Option<&Path>,
) -> Result<(PathBuf, ScopeIdentity), DiscoveryError> {
    if let Some(root) = explicit {
        return require_identity(root.to_path_buf(), "global", None);
    }
    if let Some(root) = env::var_os("SCOPE_GLOBAL_ROOT") {
        return require_identity(PathBuf::from(root), "global", None);
    }
    let path = config_path()?;
    if path.is_file() {
        let config: UserConfig = toml::from_str(&fs::read_to_string(path)?)?;
        return require_identity(
            config.global.root,
            "global",
            Some(&config.global.required_id),
        );
    }
    let fallback = home()?.join(".scope");
    if fallback.join("scope.toml").is_file() {
        return require_identity(fallback, "global", None);
    }
    Err(DiscoveryError::GlobalUnconfigured)
}

pub fn discover_project(start: &Path) -> Result<(PathBuf, ScopeIdentity), DiscoveryError> {
    let start = start.canonicalize()?;
    let mut current = if start.is_dir() {
        start.clone()
    } else {
        start.parent().unwrap_or(&start).to_path_buf()
    };
    loop {
        let candidate = current.join(DIRECTORY);
        if candidate.join("scope.toml").is_file() {
            let identity = read_identity(&candidate)?;
            if identity.kind == "project" {
                return Ok((candidate, identity));
            }
        }
        if !current.pop() {
            break;
        }
    }
    Err(DiscoveryError::ProjectNotFound(start))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn initialization_adds_only_one_consumer_root_and_never_overwrites() {
        let root = tempdir().unwrap();
        let first = initialize(root.path()).unwrap();
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
        assert!(root.path().join(".scope/state").is_dir());
        assert_eq!(first.created.len(), 6);
        assert!(root.path().join(".scope/knowledge/shared").is_dir());
        assert!(root.path().join(".scope/knowledge/local").is_dir());

        let second = initialize(root.path()).unwrap();
        assert!(second.created.is_empty());
        assert_eq!(second.existing.len(), 6);
    }

    #[test]
    fn global_and_project_discovery_use_separate_channels() {
        let root = tempdir().unwrap();
        let global = root.path().join("chosen-global");
        initialize_global(&global).unwrap();
        let repository = root.path().join("repository");
        fs::create_dir(&repository).unwrap();
        initialize(&repository).unwrap();

        let (global_path, global_identity) = discover_global(Some(&global)).unwrap();
        assert_eq!(global_path, global);
        assert_eq!(global_identity.kind, "global");

        let nested = repository.join("src/nested");
        fs::create_dir_all(&nested).unwrap();
        let (project_path, project_identity) = discover_project(&nested).unwrap();
        assert_eq!(project_path, repository.join(".scope"));
        assert_eq!(project_identity.kind, "project");
    }
}

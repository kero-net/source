use kero_core::platform::{
    HiddenAttributeAdapter, IdentityCandidate, IdentityDiscoveryAdapter, IdentityKind,
    PlatformAdapterError, PlatformPathProvider, PrivilegeProvider, PrivilegeRequirement,
};
use kero_core::setup::{Enrollment, HomeLocation, SetupPolicy, plan};
use std::cell::RefCell;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug)]
enum PathSyntax {
    Unix,
    Windows,
}

#[derive(Debug)]
struct SimulatedPlatform {
    syntax: PathSyntax,
    user_home: PathBuf,
    system_home: Option<PathBuf>,
    elevated_paths: Vec<PathBuf>,
    hidden_changes: RefCell<Vec<(PathBuf, bool)>>,
    identities: Vec<IdentityCandidate>,
}

impl SimulatedPlatform {
    fn unix(user: &str, system: Option<&str>) -> Self {
        Self {
            syntax: PathSyntax::Unix,
            user_home: PathBuf::from(user),
            system_home: system.map(PathBuf::from),
            elevated_paths: system.into_iter().map(PathBuf::from).collect(),
            hidden_changes: RefCell::new(Vec::new()),
            identities: Vec::new(),
        }
    }

    fn windows(user: &str, system: Option<&str>) -> Self {
        Self {
            syntax: PathSyntax::Windows,
            user_home: PathBuf::from(user),
            system_home: system.map(PathBuf::from),
            elevated_paths: system.into_iter().map(PathBuf::from).collect(),
            hidden_changes: RefCell::new(Vec::new()),
            identities: Vec::new(),
        }
    }

    fn with_elevated_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.elevated_paths.push(path.into());
        self
    }

    fn with_identity(mut self, reference: &str) -> Self {
        self.identities.push(IdentityCandidate {
            reference: reference.into(),
            kind: IdentityKind::OpenSshEd25519,
        });
        self
    }
}

impl PlatformPathProvider for SimulatedPlatform {
    fn user_home(&self) -> PathBuf {
        self.user_home.clone()
    }

    fn system_home(&self) -> Option<PathBuf> {
        self.system_home.clone()
    }

    fn is_absolute_path(&self, path: &Path) -> bool {
        let value = path.to_string_lossy();
        match self.syntax {
            PathSyntax::Unix => value.starts_with('/'),
            PathSyntax::Windows => {
                let bytes = value.as_bytes();
                value.starts_with("\\\\")
                    || bytes.get(1) == Some(&b':')
                        && bytes
                            .get(2)
                            .is_some_and(|separator| *separator == b'\\' || *separator == b'/')
            }
        }
    }
}

impl PrivilegeProvider for SimulatedPlatform {
    fn privilege_requirement(
        &self,
        path: &Path,
    ) -> Result<PrivilegeRequirement, PlatformAdapterError> {
        Ok(
            if self
                .elevated_paths
                .iter()
                .any(|candidate| candidate == path)
            {
                PrivilegeRequirement::Elevation
            } else {
                PrivilegeRequirement::None
            },
        )
    }
}

impl HiddenAttributeAdapter for SimulatedPlatform {
    fn set_hidden(&self, path: &Path, hidden: bool) -> Result<(), PlatformAdapterError> {
        self.hidden_changes
            .borrow_mut()
            .push((path.to_path_buf(), hidden));
        Ok(())
    }
}

impl IdentityDiscoveryAdapter for SimulatedPlatform {
    fn discover_signing_identities(&self) -> Result<Vec<IdentityCandidate>, PlatformAdapterError> {
        Ok(self.identities.clone())
    }
}

#[test]
fn default_policy_is_user_local_and_asks_before_enrollment() {
    let policy = SetupPolicy::default();
    let platform = SimulatedPlatform::unix("/home/example/.kero", Some("/var/lib/kero"));
    assert_eq!(policy.enrollment, Enrollment::Ask);
    assert!(matches!(policy.home, HomeLocation::UserLocal));
    assert!(!plan(policy, &platform).unwrap().needs_elevation);
}

#[test]
fn none_policy_has_no_home_mutation() {
    let policy = SetupPolicy {
        home: HomeLocation::None,
        ..SetupPolicy::default()
    };
    let platform = SimulatedPlatform::unix("/home/example/.kero", Some("/var/lib/kero"));
    assert_eq!(
        plan(policy, &platform).unwrap().operations,
        vec!["do not create a KERO home"]
    );
}

#[test]
fn relative_custom_home_is_rejected_by_selected_platform_syntax() {
    let policy = SetupPolicy {
        home: HomeLocation::Custom(PathBuf::from("relative")),
        ..SetupPolicy::default()
    };
    let platform = SimulatedPlatform::unix("/home/example/.kero", Some("/var/lib/kero"));
    assert!(plan(policy, &platform).is_err());
}

#[test]
fn windows_absolute_paths_are_interpreted_by_adapter_not_test_host() {
    let custom = PathBuf::from(r"C:\KeroHome");
    let policy = SetupPolicy {
        home: HomeLocation::Custom(custom.clone()),
        ..SetupPolicy::default()
    };
    let platform =
        SimulatedPlatform::windows(r"C:\Users\example\.kero", Some(r"C:\ProgramData\KERO"));
    let setup = plan(policy, &platform).unwrap();
    assert_eq!(setup.home_path, Some(custom));
}

#[test]
fn simulated_platforms_share_the_same_policy_rules() {
    let policy = SetupPolicy::default();
    let windows =
        SimulatedPlatform::windows(r"C:\Users\example\.kero", Some(r"C:\ProgramData\KERO"));
    let linux = SimulatedPlatform::unix("/home/example/.kero", Some("/var/lib/kero"));
    let custom = SimulatedPlatform::unix("/srv/example/kero-home", Some("/srv/kero-system"));

    let windows_plan = plan(policy.clone(), &windows).unwrap();
    let linux_plan = plan(policy.clone(), &linux).unwrap();
    let custom_plan = plan(policy, &custom).unwrap();

    assert_eq!(windows_plan.policy, linux_plan.policy);
    assert_eq!(linux_plan.policy, custom_plan.policy);
    assert!(!windows_plan.needs_elevation);
    assert!(!linux_plan.needs_elevation);
    assert!(!custom_plan.needs_elevation);
}

#[test]
fn privilege_requirement_is_supplied_by_adapter_not_home_kind() {
    let custom_home = PathBuf::from("/secure/custom-kero");
    let platform = SimulatedPlatform::unix("/home/example/.kero", Some("/srv/kero-system"))
        .with_elevated_path(custom_home.clone());
    let custom_policy = SetupPolicy {
        home: HomeLocation::Custom(custom_home),
        ..SetupPolicy::default()
    };
    assert!(plan(custom_policy, &platform).unwrap().needs_elevation);

    let no_elevation_system = SimulatedPlatform {
        elevated_paths: Vec::new(),
        ..SimulatedPlatform::unix("/home/example/.kero", Some("/srv/kero-system"))
    };
    let system_policy = SetupPolicy {
        home: HomeLocation::SystemWide,
        ..SetupPolicy::default()
    };
    assert!(
        !plan(system_policy, &no_elevation_system)
            .unwrap()
            .needs_elevation
    );
}

#[test]
fn hidden_attribute_and_identity_discovery_are_adapter_boundaries() {
    let platform =
        SimulatedPlatform::windows(r"C:\Users\example\.kero", Some(r"C:\ProgramData\KERO"))
            .with_identity("ssh-ed25519:SHA256:example");

    let generated = Path::new(r"C:\repo\.kero\.generated");
    platform.set_hidden(generated, true).unwrap();
    assert_eq!(
        platform.hidden_changes.borrow().as_slice(),
        &[(generated.to_path_buf(), true)]
    );

    let identities = platform.discover_signing_identities().unwrap();
    assert_eq!(
        identities,
        vec![IdentityCandidate {
            reference: "ssh-ed25519:SHA256:example".into(),
            kind: IdentityKind::OpenSshEd25519,
        }]
    );
}

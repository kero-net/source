use kero_core::host::{DataScope, HostPathError, KeroHost, LifecycleHost, ScopedPath};
use kero_core::lifecycle;

#[derive(Default)]
struct SimulatedHost {
    writes: Vec<(DataScope, Vec<String>)>,
    directories: Vec<DataScope>,
    exists: bool,
    discovered: bool,
    hidden: bool,
    removed_runtime: bool,
}

#[derive(Debug)]
struct SimulatedError;
impl std::fmt::Display for SimulatedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("simulated host error")
    }
}
impl std::error::Error for SimulatedError {}
impl KeroHost for SimulatedHost {
    type Error = SimulatedError;
    fn read(&mut self, _: &ScopedPath) -> Result<Vec<u8>, Self::Error> {
        Ok(Vec::new())
    }
    fn write(&mut self, path: &ScopedPath, _: &[u8]) -> Result<(), Self::Error> {
        self.writes
            .push((path.scope().clone(), path.segments().to_vec()));
        Ok(())
    }
    fn create_directory(&mut self, path: &ScopedPath) -> Result<(), Self::Error> {
        self.directories.push(path.scope().clone());
        Ok(())
    }
    fn materialize_mount(&mut self, _: &str) -> Result<(), Self::Error> {
        Ok(())
    }
    fn execute(&mut self, _: &str, _: &[u8]) -> Result<Vec<u8>, Self::Error> {
        Ok(Vec::new())
    }
    fn network(&mut self, _: &str, _: &[u8]) -> Result<Vec<u8>, Self::Error> {
        Ok(Vec::new())
    }
    fn diagnostic(&mut self, _: &str, _: &str) {}
}

impl LifecycleHost for SimulatedHost {
    fn environment_exists(&mut self) -> Result<bool, Self::Error> {
        Ok(self.exists)
    }
    fn discover_nearest_boundary(&mut self) -> Result<bool, Self::Error> {
        Ok(self.discovered)
    }
    fn set_runtime_hidden(&mut self, hidden: bool) -> Result<(), Self::Error> {
        self.hidden = hidden;
        Ok(())
    }
    fn remove_runtime(&mut self) -> Result<(), Self::Error> {
        self.removed_runtime = true;
        Ok(())
    }
}

#[test]
fn host_operations_cannot_mix_local_mount_or_runtime_scopes() {
    let local = ScopedPath::new(DataScope::Local, ["guide.md".into()]).unwrap();
    let mount =
        ScopedPath::new(DataScope::Mount("vendor-docs".into()), ["guide.md".into()]).unwrap();
    let mut host = SimulatedHost::default();
    host.write(&local, b"local").unwrap();
    host.write(&mount, b"mounted").unwrap();
    assert_ne!(host.writes[0].0, host.writes[1].0);
}

#[test]
fn host_boundary_rejects_host_paths_and_traversal() {
    assert!(matches!(
        ScopedPath::new(DataScope::Local, ["..".into()]),
        Err(HostPathError::EscapingSegment(_))
    ));
    assert!(matches!(
        ScopedPath::new(DataScope::Local, ["C:\\host-path".into()]),
        Err(HostPathError::EscapingSegment(_))
    ));
    assert!(matches!(
        ScopedPath::new(DataScope::Mount("Bad Name".into()), Vec::new()),
        Err(HostPathError::InvalidMountName(_))
    ));
}

#[test]
fn host_boundary_keeps_config_and_mount_root_as_distinct_scopes() {
    let config = ScopedPath::new(DataScope::Config, Vec::new()).unwrap();
    let mounts = ScopedPath::new(DataScope::Mounts, Vec::new()).unwrap();
    assert_eq!(config.scope(), &DataScope::Config);
    assert_eq!(mounts.scope(), &DataScope::Mounts);
    assert!(config.segments().is_empty());
    assert!(mounts.segments().is_empty());
}

#[test]
fn lifecycle_is_idempotent_lazy_and_host_scoped() {
    let mut host = SimulatedHost::default();
    lifecycle::initialize(&mut host).unwrap();
    assert_eq!(host.directories, vec![DataScope::Local, DataScope::Mounts]);
    assert_eq!(host.writes[0].0, DataScope::Config);
    assert!(!host.hidden);
    host.exists = true;
    lifecycle::initialize(&mut host).unwrap();
    assert_eq!(host.writes.len(), 1);
    lifecycle::ensure_runtime(&mut host).unwrap();
    assert!(host.hidden);
    assert_eq!(host.directories.last(), Some(&DataScope::Runtime));
    lifecycle::reset_runtime(&mut host).unwrap();
    assert!(host.removed_runtime);
}

#[test]
fn lifecycle_discovery_is_host_owned() {
    let mut host = SimulatedHost::default();
    assert!(lifecycle::discover(&mut host).is_err());
    host.discovered = true;
    lifecycle::discover(&mut host).unwrap();
}

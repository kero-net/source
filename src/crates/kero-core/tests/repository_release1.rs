use ed25519_dalek::{Signer, SigningKey};
use kero_core::config::parse;
use kero_core::host::native_input::{
    capture as capture_input, list as list_input, remove as remove_input,
};
use kero_core::host::native_repository::{
    EnrollmentOutcome, EnvironmentFormat, MountState, OWNED_DIRECTORIES, RepositoryState,
    create_mount, enroll, initialize as initialize_repository, inspect, inspect_environment,
    list_mounts, materialize_global_home, materialize_mount, refresh_mount, remove_mount,
    resolve_local, resolve_mount, sync_mount,
};
use kero_core::setup::Enrollment;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, SystemTime};

#[test]
fn init_creates_separate_local_and_mount_roots() {
    let directory = tempfile::tempdir().unwrap();
    git(directory.path(), ["init"]);
    let boundary = initialize_repository(directory.path()).unwrap();
    assert!(boundary.config.is_file());
    assert!(boundary.data.is_dir());
    assert!(boundary.mounts.is_dir());
    assert!(parse(&std::fs::read_to_string(&boundary.config).unwrap()).is_ok());
    for owned in OWNED_DIRECTORIES {
        assert!(!boundary.directory.join(owned).exists());
    }
}

#[test]
fn local_and_external_paths_cannot_mix_or_escape() {
    let directory = tempfile::tempdir().unwrap();
    git(directory.path(), ["init"]);
    let boundary = initialize_repository(directory.path()).unwrap();
    let external = create_mount(&boundary, "vendor-docs").unwrap();
    assert_eq!(
        resolve_local(&boundary, Path::new("guide.md")).unwrap(),
        boundary.data.join("guide.md")
    );
    assert_eq!(
        resolve_mount(&boundary, "vendor-docs", Path::new("guide.md")).unwrap(),
        external.join("guide.md")
    );
    assert!(resolve_local(&boundary, Path::new("../mnt/vendor-docs/guide.md")).is_err());
    assert!(resolve_mount(&boundary, "../local", Path::new("guide.md")).is_err());
    assert_eq!(list_mounts(&boundary).unwrap().len(), 1);
}

#[test]
fn init_is_idempotent_for_an_existing_boundary() {
    let directory = tempfile::tempdir().unwrap();
    git(directory.path(), ["init"]);
    let first = initialize_repository(directory.path()).unwrap();
    let second = initialize_repository(directory.path()).unwrap();
    assert_eq!(first, second);
}

#[test]
fn init_rejects_a_path_outside_an_explicit_git_worktree() {
    let directory = tempfile::tempdir().unwrap();
    assert!(initialize_repository(directory.path()).is_err());
    assert!(!directory.path().join(".kero").exists());
}

#[test]
fn repository_inspection_and_enrollment_follow_explicit_policy() {
    let directory = tempfile::tempdir().unwrap();
    let repository = directory.path().join("repository");
    std::fs::create_dir(&repository).unwrap();
    git(&repository, ["init"]);
    let nested = repository.join("docs").join("nested");
    std::fs::create_dir_all(&nested).unwrap();

    assert_eq!(inspect(&nested).state, RepositoryState::Eligible);
    let (_, ask) = enroll(&nested, Enrollment::Ask).unwrap();
    assert_eq!(ask, EnrollmentOutcome::Proposed);
    assert!(!repository.join(".kero").exists());
    let (_, manual) = enroll(&nested, Enrollment::Manual).unwrap();
    assert_eq!(manual, EnrollmentOutcome::NotEnrolled);
    assert!(!repository.join(".kero").exists());

    let (_, automatic) = enroll(&nested, Enrollment::Automatic).unwrap();
    assert_eq!(automatic, EnrollmentOutcome::Enrolled);
    assert!(repository.join(".kero").join("config").is_file());
    let inspected = inspect(&nested);
    assert_eq!(inspected.state, RepositoryState::Enrolled);
    assert_eq!(
        inspected.boundary.unwrap().root,
        repository.canonicalize().unwrap()
    );
    let (_, repeated) = enroll(&nested, Enrollment::Automatic).unwrap();
    assert_eq!(repeated, EnrollmentOutcome::AlreadyEnrolled);
}

#[test]
fn discovery_does_not_treat_non_repositories_as_eligible() {
    let directory = tempfile::tempdir().unwrap();
    assert_eq!(
        inspect(directory.path()).state,
        RepositoryState::NotRepository
    );

    let bare = directory.path().join("bare.git");
    let result = Command::new("git")
        .args(["init", "--bare"])
        .arg(&bare)
        .output()
        .unwrap();
    assert!(result.status.success());
    assert_eq!(inspect(&bare).state, RepositoryState::Unsupported);
}

#[test]
fn readonly_worktree_is_unavailable_for_automatic_enrollment() {
    let directory = tempfile::tempdir().unwrap();
    let repository = directory.path().join("readonly-repository");
    std::fs::create_dir(&repository).unwrap();
    git(&repository, ["init"]);
    let nested = repository.join("nested");
    std::fs::create_dir(&nested).unwrap();

    let original = std::fs::metadata(&repository).unwrap().permissions();
    let mut readonly = original.clone();
    readonly.set_readonly(true);
    std::fs::set_permissions(&repository, readonly).unwrap();
    let discovery = inspect(&nested);
    std::fs::set_permissions(&repository, original).unwrap();

    assert_eq!(discovery.state, RepositoryState::Unavailable);
    assert!(!repository.join(".kero").exists());
}

#[test]
fn automatic_enrollment_uses_gits_resolved_exclude_path_for_linked_worktrees() {
    let directory = tempfile::tempdir().unwrap();
    let primary = directory.path().join("primary");
    let linked = directory.path().join("linked");
    std::fs::create_dir(&primary).unwrap();
    git(&primary, ["init"]);
    git(&primary, ["config", "user.email", "kero@example.invalid"]);
    git(&primary, ["config", "user.name", "KERO fixture"]);
    std::fs::write(primary.join("README.md"), "fixture\n").unwrap();
    git(&primary, ["add", "README.md"]);
    git(&primary, ["commit", "-m", "fixture"]);
    git(
        &primary,
        ["worktree", "add", "--detach", linked.to_str().unwrap()],
    );

    let (_, outcome) = enroll(&linked, Enrollment::Automatic).unwrap();
    assert_eq!(outcome, EnrollmentOutcome::Enrolled);
    let exclude = Command::new("git")
        .arg("-C")
        .arg(&linked)
        .args(["rev-parse", "--git-path", "info/exclude"])
        .output()
        .unwrap();
    assert!(exclude.status.success());
    let exclude = String::from_utf8_lossy(&exclude.stdout).trim().to_owned();
    let exclude = Path::new(&exclude);
    let exclude = if exclude.is_absolute() {
        exclude.to_path_buf()
    } else {
        linked.join(exclude)
    };
    let contents = std::fs::read_to_string(exclude).unwrap();
    assert!(contents.lines().any(|line| line == "/.kero/.runtime/"));
}

#[test]
fn materialized_mount_copies_only_external_local_data_and_records_runtime_provenance() {
    let directory = tempfile::tempdir().unwrap();
    let target_root = directory.path().join("target");
    let source_root = directory.path().join("source");
    std::fs::create_dir(&target_root).unwrap();
    std::fs::create_dir(&source_root).unwrap();
    git(&target_root, ["init"]);
    git(&source_root, ["init"]);
    let target = initialize_repository(&target_root).unwrap();
    let source = initialize_repository(&source_root).unwrap();
    std::fs::create_dir(source.data.join("guides")).unwrap();
    std::fs::write(
        source.data.join("guides").join("intro.md"),
        "external knowledge",
    )
    .unwrap();
    std::fs::write(
        source.data.join("guides").join("policy.kst"),
        "# ordinary KERO knowledge\npolicy portable\n",
    )
    .unwrap();
    let expected_modified = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    filetime::set_file_mtime(
        source.data.join("guides").join("policy.kst"),
        filetime::FileTime::from_system_time(expected_modified),
    )
    .unwrap();

    let provenance = materialize_mount(&target, "upstream", &source_root).unwrap();
    assert_eq!(provenance.files, 2);
    assert_eq!(provenance.source_format, "repository");
    assert_eq!(provenance.access, "read-only");
    assert!(
        target
            .mounts
            .join("upstream")
            .join("guides")
            .join("intro.md")
            .is_file()
    );
    assert_eq!(
        std::fs::metadata(
            target
                .mounts
                .join("upstream")
                .join("guides")
                .join("policy.kst")
        )
        .unwrap()
        .modified()
        .unwrap(),
        expected_modified
    );
    assert!(
        target
            .mounts
            .join("upstream")
            .join("guides")
            .join("policy.kst")
            .is_file()
    );
    assert!(!target.data.join("guides").exists());
    assert!(
        target
            .directory
            .join(".runtime/mounts/upstream.kst")
            .is_file()
    );
    assert!(materialize_mount(&target, "upstream", &source_root).is_err());
    let mounts = list_mounts(&target).unwrap();
    assert_eq!(mounts.len(), 1);
    assert_eq!(mounts[0].state, MountState::MaterializedReadOnly);

    remove_mount(&target, "upstream").unwrap();
    assert!(!target.mounts.join("upstream").exists());
    assert!(target.data.is_dir());
}

#[cfg(unix)]
#[test]
fn materialized_mount_rejects_links_and_removes_the_partial_snapshot() {
    use std::os::unix::fs::symlink;

    let directory = tempfile::tempdir().unwrap();
    let target_root = directory.path().join("target");
    let source_root = directory.path().join("source");
    std::fs::create_dir(&target_root).unwrap();
    std::fs::create_dir(&source_root).unwrap();
    git(&target_root, ["init"]);
    git(&source_root, ["init"]);
    let target = initialize_repository(&target_root).unwrap();
    let source = initialize_repository(&source_root).unwrap();
    std::fs::write(source.data.join("safe.txt"), "safe").unwrap();
    symlink(source.data.join("safe.txt"), source.data.join("escape.txt")).unwrap();

    let error = materialize_mount(&target, "unsafe", &source_root).unwrap_err();
    assert!(error.to_string().contains("symbolic link"));
    assert!(!target.mounts.join("unsafe").exists());
    assert!(target.data.is_dir());
}

#[test]
fn explicit_environment_inspection_distinguishes_repository_home_and_invalid_shapes() {
    let directory = tempfile::tempdir().unwrap();
    let repository = directory.path().join("repository");
    let home = directory.path().join("home");
    let invalid = directory.path().join("invalid");
    std::fs::create_dir(&repository).unwrap();
    std::fs::create_dir(&home).unwrap();
    std::fs::create_dir(&invalid).unwrap();
    git(&repository, ["init"]);
    initialize_repository(&repository).unwrap();
    std::fs::write(home.join("config"), "home config\n").unwrap();
    std::fs::create_dir(home.join("data")).unwrap();
    std::fs::create_dir(home.join("mnt")).unwrap();

    assert_eq!(
        inspect_environment(&repository).format,
        EnvironmentFormat::Repository
    );
    assert_eq!(
        inspect_environment(&home).format,
        EnvironmentFormat::GlobalHome
    );
    assert_eq!(
        inspect_environment(&invalid).format,
        EnvironmentFormat::NeedsSetup
    );
}

#[test]
fn schema_neutral_configuration_is_a_valid_repository_without_reinitialization() {
    let directory = tempfile::tempdir().unwrap();
    let repository = directory.path().join("repository");
    std::fs::create_dir(&repository).unwrap();
    git(&repository, ["init"]);
    let boundary = initialize_repository(&repository).unwrap();
    std::fs::write(&boundary.config, "# schema-neutral KERO Structured Text\n").unwrap();

    let inspection = inspect_environment(&repository);
    assert_eq!(inspection.format, EnvironmentFormat::Repository);
    assert!(boundary.data.is_dir());
    assert!(boundary.mounts.is_dir());
    assert_eq!(
        std::fs::read_to_string(&boundary.config).unwrap(),
        "# schema-neutral KERO Structured Text\n"
    );
}

#[test]
fn incomplete_kero_boundary_is_conflicted_and_preserved() {
    let directory = tempfile::tempdir().unwrap();
    let repository = directory.path().join("repository");
    std::fs::create_dir(&repository).unwrap();
    std::fs::create_dir(repository.join(".kero")).unwrap();
    std::fs::write(
        repository.join(".kero").join("config"),
        "environment repository\n",
    )
    .unwrap();

    let inspection = inspect_environment(&repository);
    assert_eq!(inspection.format, EnvironmentFormat::Conflicted);
    assert!(repository.join(".kero").join("config").is_file());
    assert!(!repository.join(".kero").join("data").exists());
}

#[test]
fn global_home_materialization_is_explicit_and_read_only() {
    let directory = tempfile::tempdir().unwrap();
    let target_root = directory.path().join("target");
    let home = directory.path().join("home");
    std::fs::create_dir(&target_root).unwrap();
    std::fs::create_dir(&home).unwrap();
    git(&target_root, ["init"]);
    let target = initialize_repository(&target_root).unwrap();
    std::fs::write(home.join("config"), "home config\n").unwrap();
    std::fs::create_dir(home.join("data")).unwrap();
    std::fs::create_dir(home.join("mnt")).unwrap();
    std::fs::write(home.join("data").join("global.md"), "global knowledge").unwrap();

    let provenance = materialize_global_home(&target, "global", &home).unwrap();
    assert_eq!(provenance.source_format, "global-home");
    assert_eq!(provenance.access, "read-only");
    assert!(
        std::fs::metadata(target.mounts.join("global").join("global.md"))
            .unwrap()
            .permissions()
            .readonly()
    );
    assert_eq!(
        list_mounts(&target).unwrap()[0].state,
        MountState::MaterializedReadOnly
    );
    remove_mount(&target, "global").unwrap();
}

#[test]
fn refresh_replaces_a_snapshot_and_publishes_kst_provenance() {
    let directory = tempfile::tempdir().unwrap();
    let target_root = directory.path().join("target");
    let source_root = directory.path().join("source");
    std::fs::create_dir(&target_root).unwrap();
    std::fs::create_dir(&source_root).unwrap();
    git(&target_root, ["init"]);
    git(&source_root, ["init"]);
    let target = initialize_repository(&target_root).unwrap();
    let source = initialize_repository(&source_root).unwrap();
    std::fs::write(source.data.join("first.txt"), "first").unwrap();
    materialize_mount(&target, "upstream", &source_root).unwrap();
    std::fs::write(source.data.join("second.txt"), "second").unwrap();
    let refreshed = refresh_mount(&target, "upstream").unwrap();
    assert_eq!(refreshed.files, 2);
    assert!(target.mounts.join("upstream/second.txt").is_file());
    assert!(
        target
            .directory
            .join(".runtime/mounts/upstream.kst")
            .is_file()
    );
}

#[test]
fn refresh_repairs_legacy_json_provenance_to_kst() {
    let directory = tempfile::tempdir().unwrap();
    let target_root = directory.path().join("target");
    let source_root = directory.path().join("source");
    std::fs::create_dir(&target_root).unwrap();
    std::fs::create_dir(&source_root).unwrap();
    git(&target_root, ["init"]);
    git(&source_root, ["init"]);
    let target = initialize_repository(&target_root).unwrap();
    let source = initialize_repository(&source_root).unwrap();
    std::fs::write(source.data.join("note.txt"), "one").unwrap();
    let record = materialize_mount(&target, "legacy", &source_root).unwrap();
    let runtime_mounts = target.directory.join(".runtime/mounts");
    std::fs::remove_file(runtime_mounts.join("legacy.kst")).unwrap();
    std::fs::write(runtime_mounts.join("legacy.json"), serde_json::json!({
        "name":"legacy", "source_root":record.source_root, "source_content_sha256":record.source_content_sha256,
        "files":record.files, "source_format":record.source_format, "access":"read-only"
    }).to_string()).unwrap();
    refresh_mount(&target, "legacy").unwrap();
    assert!(runtime_mounts.join("legacy.kst").is_file());
    assert!(!runtime_mounts.join("legacy.json").exists());
}

#[test]
fn signed_pull_grant_refreshes_only_source_changes() {
    let directory = tempfile::tempdir().unwrap();
    let target_root = directory.path().join("target");
    let source_root = directory.path().join("source");
    std::fs::create_dir(&target_root).unwrap();
    std::fs::create_dir(&source_root).unwrap();
    git(&target_root, ["init"]);
    git(&source_root, ["init"]);
    let target = initialize_repository(&target_root).unwrap();
    let source = initialize_repository(&source_root).unwrap();
    std::fs::write(source.data.join("note.txt"), "one").unwrap();
    let record = materialize_mount(&target, "upstream", &source_root).unwrap();
    let signer = SigningKey::from_bytes(&[7; 32]);
    let target_key = hex::encode([9_u8; 32]);
    let expires = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 60;
    let payload = format!(
        "mount=upstream\ntarget={target_key}\ndirection=pull\nbaseline={}\nexpires={expires}\n",
        record.baseline_sha256
    );
    let signature = signer.sign(payload.as_bytes());
    std::fs::write(&source.config, format!("syncGrant upstream\n\ttargetKey {target_key}\n\tdirection pull\n\tbaseline {}\n\texpires {expires}\n\tsourceKey {}\n\tsignature {}\n", record.baseline_sha256, hex::encode(signer.verifying_key().to_bytes()), hex::encode(signature.to_bytes()))).unwrap();
    std::fs::write(source.data.join("note.txt"), "two").unwrap();
    let synchronized = sync_mount(&target, "upstream", &target_key).unwrap();
    assert_eq!(synchronized.status, "ready");
    assert_eq!(
        std::fs::read_to_string(target.mounts.join("upstream/note.txt")).unwrap(),
        "two"
    );
}

#[test]
fn divergent_sync_marks_the_mount_conflicted_without_discarding_either_side() {
    let directory = tempfile::tempdir().unwrap();
    let target_root = directory.path().join("target");
    let source_root = directory.path().join("source");
    std::fs::create_dir(&target_root).unwrap();
    std::fs::create_dir(&source_root).unwrap();
    git(&target_root, ["init"]);
    git(&source_root, ["init"]);
    let target = initialize_repository(&target_root).unwrap();
    let source = initialize_repository(&source_root).unwrap();
    std::fs::write(source.data.join("note.txt"), "baseline").unwrap();
    let record = materialize_mount(&target, "upstream", &source_root).unwrap();
    let signer = SigningKey::from_bytes(&[8; 32]);
    let target_key = hex::encode([10_u8; 32]);
    let expires = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 60;
    let payload = format!(
        "mount=upstream\ntarget={target_key}\ndirection=bidirectional\nbaseline={}\nexpires={expires}\n",
        record.baseline_sha256
    );
    let signature = signer.sign(payload.as_bytes());
    std::fs::write(&source.config, format!("syncGrant upstream\n\ttargetKey {target_key}\n\tdirection bidirectional\n\tbaseline {}\n\texpires {expires}\n\tsourceKey {}\n\tsignature {}\n", record.baseline_sha256, hex::encode(signer.verifying_key().to_bytes()), hex::encode(signature.to_bytes()))).unwrap();
    std::fs::write(source.data.join("note.txt"), "source edit").unwrap();
    let local = target.mounts.join("upstream/note.txt");
    let mut permissions = std::fs::metadata(&local).unwrap().permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        permissions.set_mode(0o600);
    }
    #[cfg(windows)]
    #[allow(clippy::permissions_set_readonly_false)]
    permissions.set_readonly(false);
    std::fs::set_permissions(&local, permissions).unwrap();
    std::fs::write(&local, "local edit").unwrap();
    assert!(
        sync_mount(&target, "upstream", &target_key)
            .unwrap_err()
            .to_string()
            .contains("mount.sync-conflict")
    );
    assert_eq!(
        list_mounts(&target).unwrap()[0].state,
        MountState::Conflicted
    );
    assert_eq!(
        std::fs::read_to_string(source.data.join("note.txt")).unwrap(),
        "source edit"
    );
    assert_eq!(std::fs::read_to_string(local).unwrap(), "local edit");
}

#[test]
fn mount_listing_reports_incomplete_entries_without_hiding_them() {
    let directory = tempfile::tempdir().unwrap();
    git(directory.path(), ["init"]);
    let boundary = initialize_repository(directory.path()).unwrap();
    std::fs::create_dir(boundary.mounts.join("Bad Name")).unwrap();
    std::fs::write(boundary.mounts.join("stray.txt"), "not a mount").unwrap();

    let mounts = list_mounts(&boundary).unwrap();
    assert_eq!(mounts.len(), 2);
    assert!(
        mounts
            .iter()
            .all(|mount| mount.state == MountState::OutOfFormat)
    );
}

#[test]
fn explicit_knowledge_input_is_deterministic_local_and_removable() {
    let directory = tempfile::tempdir().unwrap();
    let repository = directory.path().join("repository");
    let source = directory.path().join("source");
    std::fs::create_dir(&repository).unwrap();
    std::fs::create_dir(&source).unwrap();
    git(&repository, ["init"]);
    std::fs::create_dir(source.join("nested")).unwrap();
    std::fs::write(source.join("z.txt"), "z").unwrap();
    std::fs::write(source.join("nested").join("a.txt"), "a").unwrap();

    let boundary = initialize_repository(&repository).unwrap();
    let first = capture_input(&boundary, &source).unwrap();
    let second = capture_input(&boundary, &source).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.files, 2);
    assert!(
        boundary
            .data
            .join("input")
            .join(&first.id)
            .join("content/nested/a.txt")
            .is_file()
    );
    assert_eq!(list_input(&boundary).unwrap(), vec![first.clone()]);
    let manifest = std::fs::read_to_string(
        boundary
            .data
            .join("input")
            .join(&first.id)
            .join("manifest.json"),
    )
    .unwrap();
    assert!(!manifest.contains(source.to_string_lossy().as_ref()));
    assert!(capture_input(&boundary, &boundary.data).is_err());
    remove_input(&boundary, &first.id).unwrap();
    assert!(list_input(&boundary).unwrap().is_empty());
}

#[test]
fn knowledge_input_identity_covers_empty_directories_and_detects_corruption() {
    let directory = tempfile::tempdir().unwrap();
    let repository = directory.path().join("repository");
    let source = directory.path().join("source");
    std::fs::create_dir(&repository).unwrap();
    std::fs::create_dir(&source).unwrap();
    git(&repository, ["init"]);
    std::fs::create_dir(source.join("empty")).unwrap();
    std::fs::write(source.join("record.txt"), "first").unwrap();

    let boundary = initialize_repository(&repository).unwrap();
    let first = capture_input(&boundary, &source).unwrap();
    assert!(
        boundary
            .data
            .join("input")
            .join(&first.id)
            .join("content/empty")
            .is_dir()
    );

    std::fs::write(source.join("record.txt"), "second").unwrap();
    let second = capture_input(&boundary, &source).unwrap();
    assert_ne!(first.id, second.id);

    std::fs::write(
        boundary
            .data
            .join("input")
            .join(&first.id)
            .join("content/record.txt"),
        "corrupted",
    )
    .unwrap();
    assert!(list_input(&boundary).is_err());
    assert!(remove_input(&boundary, &first.id).is_err());
}

#[cfg(unix)]
#[test]
fn knowledge_input_rejects_symbolic_links() {
    use std::os::unix::fs::symlink;

    let directory = tempfile::tempdir().unwrap();
    let repository = directory.path().join("repository");
    let source = directory.path().join("source");
    std::fs::create_dir(&repository).unwrap();
    std::fs::create_dir(&source).unwrap();
    git(&repository, ["init"]);
    std::fs::write(source.join("outside.txt"), "outside").unwrap();
    symlink(source.join("outside.txt"), source.join("linked.txt")).unwrap();

    let boundary = initialize_repository(&repository).unwrap();
    assert!(capture_input(&boundary, &source).is_err());
    assert!(!boundary.data.join("input").exists());
}

fn git<const N: usize>(directory: &Path, args: [&str; N]) {
    let result = Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(args)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

//! Native host adapter for the portable KERO core.

mod runtime;
mod service;

use clap::{Parser, Subcommand};
use ed25519_dalek::{Signer, SigningKey};
use getrandom::fill;
use kero_core::config;
use kero_core::host::native_input::{
    capture as capture_input, list as list_input, remove as remove_input,
};
use kero_core::host::native_processing::{
    build as build_processed, sign as sign_artifact, verify as verify_processed, verify_signature,
};
use kero_core::host::native_repository::{
    EnrollmentOutcome, EnvironmentFormat, RepositoryDiscovery, RepositoryError, create_mount,
    discover as discover_repository, enroll as enroll_repository, export_mount_conflict,
    initialize as initialize_repository, inspect as inspect_repository, inspect_environment,
    list_mounts, materialize_global_home, materialize_mount, mount_provenance, refresh_mount,
    remove_mount, resolve_mount_conflict, set_mount_refresh_mode as set_runtime_mount_refresh_mode,
    sync_mount,
};
use kero_core::platform::{
    PlatformAdapterError, PlatformPathProvider, PrivilegeProvider, PrivilegeRequirement,
};
use kero_core::setup::{
    self, Enrollment, HomeLocation, Identity, KeroHomeHost, SetupPolicy, Verification,
};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Parser)]
#[command(
    name = "kero-host",
    version,
    about = "KERO terminal host and local service"
)]
struct Cli {
    #[arg(long, global = true)]
    json: bool,
    /// Explicit portable-core artifact; development and embedding override.
    #[arg(long, global = true)]
    runtime: Option<PathBuf>,
    /// Runs a request inside the local service; never user-facing.
    #[arg(long, global = true, hide = true)]
    service_dispatch: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Init {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    Mount {
        #[command(subcommand)]
        command: MountCommand,
    },
    Knowledge {
        #[command(subcommand)]
        command: KnowledgeCommand,
    },
    Process {
        #[command(subcommand)]
        command: ProcessCommand,
    },
    Trust {
        #[command(subcommand)]
        command: TrustCommand,
    },
    Repository {
        #[command(subcommand)]
        command: RepositoryCommand,
    },
    Context {
        #[command(subcommand)]
        command: ContextCommand,
    },
    Setup {
        #[arg(long, default_value = "user")]
        home: String,
        #[arg(long, default_value = "ask")]
        enrollment: String,
        /// Identity policy: reuse creates an identity only during this explicit setup action.
        #[arg(long, default_value = "configure-later", value_parser = ["reuse", "configure-later"])]
        identity: String,
        #[arg(long)]
        dry_run: bool,
    },
    Service {
        #[command(subcommand)]
        command: ServiceCommand,
    },
}

#[derive(Debug, Subcommand)]
enum ServiceCommand {
    Start,
    Status,
    Stop,
    #[command(hide = true)]
    Serve,
}

#[derive(Debug, Subcommand)]
enum MountCommand {
    Create {
        name: String,
        #[arg(long, default_value = ".")]
        repository: PathBuf,
    },
    Add {
        name: String,
        source: PathBuf,
        #[arg(long, default_value = ".")]
        repository: PathBuf,
    },
    AddHome {
        name: String,
        /// KERO global home. Defaults to KERO_HOME or the user-local home.
        #[arg(long)]
        home: Option<PathBuf>,
        #[arg(long, default_value = ".")]
        repository: PathBuf,
    },
    Inspect {
        source: PathBuf,
    },
    List {
        #[arg(long, default_value = ".")]
        repository: PathBuf,
    },
    Remove {
        name: String,
        #[arg(long, default_value = ".")]
        repository: PathBuf,
    },
    Refresh {
        name: String,
        #[arg(long, default_value = ".")]
        repository: PathBuf,
    },
    /// Revalidates a legacy JSON provenance record and rewrites it as KST.
    Repair {
        name: String,
        #[arg(long, default_value = ".")]
        repository: PathBuf,
    },
    Watch {
        #[command(subcommand)]
        command: MountWatchCommand,
    },
    Grant {
        #[command(subcommand)]
        command: MountGrantCommand,
    },
    Sync {
        name: String,
        #[arg(long, default_value = ".")]
        repository: PathBuf,
    },
    Conflict {
        #[command(subcommand)]
        command: MountConflictCommand,
    },
}

#[derive(Debug, Subcommand)]
enum MountConflictCommand {
    Export {
        name: String,
        #[arg(long)]
        output: PathBuf,
        #[arg(long, default_value = ".")]
        repository: PathBuf,
    },
    UseSource {
        name: String,
        #[arg(long, default_value = ".")]
        repository: PathBuf,
    },
    UseLocal {
        name: String,
        #[arg(long, default_value = ".")]
        repository: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
enum MountWatchCommand {
    Enable {
        name: String,
        #[arg(long, default_value = ".")]
        repository: PathBuf,
    },
    Disable {
        name: String,
        #[arg(long, default_value = ".")]
        repository: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
enum MountGrantCommand {
    Create {
        name: String,
        #[arg(long)]
        target_key: String,
        #[arg(long, value_parser = ["pull", "push", "bidirectional"])]
        direction: String,
        #[arg(long)]
        source: PathBuf,
        #[arg(long)]
        expires: u64,
        /// Repository containing the mounted snapshot whose baseline is granted.
        #[arg(long, default_value = ".")]
        repository: PathBuf,
    },
    /// Lists the active and revoked grants recorded by a source environment.
    List {
        #[arg(long)]
        source: PathBuf,
    },
    /// Revokes one collaborator grant without rewriting unrelated KST comments.
    Revoke {
        name: String,
        #[arg(long)]
        target_key: String,
        #[arg(long)]
        source: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
enum KnowledgeCommand {
    /// Lists a validated data scope for the desktop client.
    Browse {
        #[arg(long, default_value = "local")]
        scope: String,
        #[arg(long)]
        repository: Option<PathBuf>,
        #[arg(long, conflicts_with = "repository")]
        home: bool,
    },
    Add {
        source: PathBuf,
        #[arg(long, default_value = ".")]
        repository: PathBuf,
    },
    List {
        #[arg(long, default_value = ".")]
        repository: PathBuf,
    },
    Remove {
        id: String,
        #[arg(long, default_value = ".")]
        repository: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
enum ProcessCommand {
    Build {
        id: String,
        #[arg(long, default_value = ".")]
        repository: PathBuf,
    },
    Verify {
        id: String,
        #[arg(long, default_value = ".")]
        repository: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
enum TrustCommand {
    Sign {
        artifact: PathBuf,
        #[arg(long)]
        key: PathBuf,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    Verify {
        artifact: PathBuf,
        #[arg(long)]
        signature: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum RepositoryCommand {
    Status {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    Enroll {
        #[arg(long, default_value = "ask")]
        policy: String,
        #[arg(default_value = ".")]
        path: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
enum ContextCommand {
    /// Returns the selected global-home state and optional repository classification.
    Status {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
}

#[derive(Serialize)]
struct ResultEnvelope<T: Serialize> {
    ok: bool,
    result: T,
}
#[derive(Serialize)]
struct ErrorEnvelope {
    ok: bool,
    error: String,
}
#[derive(Serialize)]
struct InitResult {
    config: String,
    data: String,
    mounts: String,
}
#[derive(Serialize)]
struct MountResult {
    name: String,
    path: String,
}
#[derive(Serialize)]
struct MaterializedMountResult {
    name: String,
    path: String,
    source_content_sha256: String,
    files: u64,
    source_format: String,
    access: String,
}
#[derive(Serialize)]
struct KnowledgeRemoveResult {
    id: String,
}
#[derive(Serialize)]
struct PathResult {
    path: String,
}
#[derive(Serialize)]
struct EnrollmentResult {
    discovery: RepositoryDiscovery,
    outcome: EnrollmentOutcome,
}
#[derive(Serialize)]
struct SetupResult {
    operations: Vec<String>,
    needs_elevation: bool,
    dry_run: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ContextStatus {
    home: String,
    home_data: String,
    selected_path: String,
    repository: RepositoryDiscovery,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BrowseEntry {
    path: String,
    name: String,
    kind: String,
    size: u64,
    modified_unix_seconds: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BrowseResult {
    scope: String,
    root: String,
    read_only: bool,
    entries: Vec<BrowseEntry>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UiMountStatus {
    name: String,
    destination: String,
    state: String,
    message: String,
    source_format: String,
    access: String,
    refresh_mode: String,
    status: String,
    files: u64,
    baseline_present: bool,
}

fn main() {
    let cli = Cli::parse();
    if !cli.service_dispatch && !matches!(cli.command, Command::Service { .. }) {
        match service::invoke_or_start(std::env::args().skip(1).collect()) {
            Ok(response) => {
                print!("{}", response.stdout);
                eprint!("{}", response.stderr);
                std::process::exit(response.status);
            }
            Err(error) => {
                eprintln!("error: {error}");
                std::process::exit(1);
            }
        }
    }
    let result = run(&cli);
    match result {
        Ok(value) => print(&cli, value),
        Err(error) => {
            if cli.json {
                println!(
                    "{}",
                    serde_json::to_string(&ErrorEnvelope { ok: false, error })
                        .expect("serializable")
                );
            } else {
                eprintln!("error: {error}");
            }
            std::process::exit(1);
        }
    }
}

fn run(cli: &Cli) -> Result<serde_json::Value, String> {
    let artifact = runtime::locate(cli.runtime.as_deref()).map_err(|error| error.to_string())?;
    let runtime_info = runtime::load(&artifact).map_err(|error| error.to_string())?;
    match &cli.command {
        Command::Init { path } => {
            let boundary = initialize_repository(path).map_err(repository_error)?;
            serde_json::to_value(InitResult {
                config: boundary.config.display().to_string(),
                data: boundary.data.display().to_string(),
                mounts: boundary.mounts.display().to_string(),
            })
            .map_err(|error| error.to_string())
        }
        Command::Mount {
            command: MountCommand::Create { name, repository },
        } => {
            let boundary = discover_repository(repository).map_err(repository_error)?;
            let path = create_mount(&boundary, name).map_err(repository_error)?;
            serde_json::to_value(MountResult {
                name: name.clone(),
                path: path.display().to_string(),
            })
            .map_err(|error| error.to_string())
        }
        Command::Mount {
            command:
                MountCommand::Add {
                    name,
                    source,
                    repository,
                },
        } => {
            let boundary = discover_repository(repository).map_err(repository_error)?;
            materialize_mount(&boundary, name, source).map_err(repository_error)?;
            let refresh = global_mount_defaults()?.refresh;
            let provenance = set_runtime_mount_refresh_mode(&boundary, name, &refresh)
                .map_err(repository_error)?;
            serde_json::to_value(MaterializedMountResult {
                name: name.clone(),
                path: boundary.mounts.join(name).display().to_string(),
                source_content_sha256: provenance.source_content_sha256,
                files: provenance.files,
                source_format: provenance.source_format,
                access: provenance.access,
            })
            .map_err(|error| error.to_string())
        }
        Command::Mount {
            command:
                MountCommand::AddHome {
                    name,
                    home,
                    repository,
                },
        } => {
            let boundary = discover_repository(repository).map_err(repository_error)?;
            let home = home.clone().or_else(default_kero_home).ok_or_else(|| {
                "KERO_HOME is unavailable; provide --home with a KERO global home".to_string()
            })?;
            materialize_global_home(&boundary, name, &home).map_err(repository_error)?;
            let refresh = global_mount_defaults()?.refresh;
            let provenance = set_runtime_mount_refresh_mode(&boundary, name, &refresh)
                .map_err(repository_error)?;
            serde_json::to_value(MaterializedMountResult {
                name: name.clone(),
                path: boundary.mounts.join(name).display().to_string(),
                source_content_sha256: provenance.source_content_sha256,
                files: provenance.files,
                source_format: provenance.source_format,
                access: provenance.access,
            })
            .map_err(|error| error.to_string())
        }
        Command::Mount {
            command: MountCommand::Inspect { source },
        } => serde_json::to_value(inspect_environment(source)).map_err(|error| error.to_string()),
        Command::Mount {
            command: MountCommand::List { repository },
        } => {
            let boundary = discover_repository(repository).map_err(repository_error)?;
            refresh_event_mounts(&boundary)?;
            let mounts = list_mounts(&boundary).map_err(repository_error)?;
            let statuses: Vec<UiMountStatus> = mounts
                .into_iter()
                .map(|mount| {
                    let provenance = mount_provenance(&boundary, &mount.name).ok();
                    UiMountStatus {
                        name: mount.name,
                        destination: mount.path.display().to_string(),
                        state: format!("{:?}", mount.state),
                        message: mount.message,
                        source_format: provenance
                            .as_ref()
                            .map(|value| value.source_format.clone())
                            .unwrap_or_else(|| "unknown".into()),
                        access: provenance
                            .as_ref()
                            .map(|value| value.access.clone())
                            .unwrap_or_else(|| "unknown".into()),
                        refresh_mode: provenance
                            .as_ref()
                            .map(|value| value.refresh_mode.clone())
                            .unwrap_or_else(|| "manual".into()),
                        status: provenance
                            .as_ref()
                            .map(|value| value.status.clone())
                            .unwrap_or_else(|| "unhealthy".into()),
                        files: provenance.as_ref().map(|value| value.files).unwrap_or(0),
                        baseline_present: provenance
                            .as_ref()
                            .is_some_and(|value| !value.baseline_sha256.is_empty()),
                    }
                })
                .collect();
            serde_json::to_value(statuses).map_err(|error| error.to_string())
        }
        Command::Mount {
            command: MountCommand::Remove { name, repository },
        } => {
            let boundary = discover_repository(repository).map_err(repository_error)?;
            remove_mount(&boundary, name).map_err(repository_error)?;
            serde_json::to_value(MountResult {
                name: name.clone(),
                path: boundary.mounts.join(name).display().to_string(),
            })
            .map_err(|error| error.to_string())
        }
        Command::Mount {
            command: MountCommand::Refresh { name, repository },
        } => {
            let boundary = discover_repository(repository).map_err(repository_error)?;
            serde_json::to_value(refresh_mount(&boundary, name).map_err(repository_error)?)
                .map_err(|error| error.to_string())
        }
        Command::Mount {
            command: MountCommand::Repair { name, repository },
        } => {
            let boundary = discover_repository(repository).map_err(repository_error)?;
            serde_json::to_value(refresh_mount(&boundary, name).map_err(repository_error)?)
                .map_err(|error| error.to_string())
        }
        Command::Mount {
            command: MountCommand::Watch { command },
        } => {
            let (name, repository, enabled) = match command {
                MountWatchCommand::Enable { name, repository } => (name, repository, true),
                MountWatchCommand::Disable { name, repository } => (name, repository, false),
            };
            let boundary = discover_repository(repository).map_err(repository_error)?;
            set_mount_refresh_mode(&boundary.config, name, enabled)?;
            serde_json::to_value(serde_json::json!({"name":name, "refreshMode":if enabled { "event" } else { "manual" }})).map_err(|error| error.to_string())
        }
        Command::Mount {
            command:
                MountCommand::Grant {
                    command:
                        MountGrantCommand::Create {
                            name,
                            target_key,
                            direction,
                            source,
                            expires,
                            repository,
                        },
                },
        } => {
            let inspection = inspect_environment(source);
            let source_config = match inspection.format {
                EnvironmentFormat::Repository => inspection
                    .root
                    .clone()
                    .map(|root| root.join(".kero").join("config")),
                EnvironmentFormat::GlobalHome => {
                    inspection.root.clone().map(|root| root.join("config"))
                }
                _ => None,
            }
            .ok_or_else(|| {
                "source must be an in-format KERO repository or global home".to_string()
            })?;
            let boundary = discover_repository(repository).map_err(repository_error)?;
            let baseline = mount_provenance(&boundary, name)
                .map_err(repository_error)?
                .baseline_sha256;
            let identity = signing_identity()?;
            let payload = format!(
                "mount={name}\ntarget={target_key}\ndirection={direction}\nbaseline={baseline}\nexpires={expires}\n"
            );
            let signature = identity.sign(payload.as_bytes());
            use std::io::Write;
            let mut config = std::fs::OpenOptions::new()
                .append(true)
                .open(&source_config)
                .map_err(|error| error.to_string())?;
            writeln!(
                config,
                "\n# Signed writable-sync grant. Revoke by removing this complete section."
            )
            .map_err(|error| error.to_string())?;
            writeln!(config, "syncGrant {name}\n\ttargetKey {target_key}\n\tdirection {direction}\n\tbaseline {baseline}\n\texpires {expires}\n\tsourceKey {}\n\tsignature {}", hex::encode(identity.verifying_key().to_bytes()), hex::encode(signature.to_bytes())).map_err(|error| error.to_string())?;
            serde_json::to_value(serde_json::json!({"name":name,"direction":direction,"baseline":baseline,"expires":expires,"source":inspection.root})).map_err(|error| error.to_string())
        }
        Command::Mount {
            command:
                MountCommand::Grant {
                    command: MountGrantCommand::List { source },
                },
        } => {
            let inspection = inspect_environment(source);
            let config = match inspection.format {
                EnvironmentFormat::Repository => inspection
                    .root
                    .map(|root| root.join(".kero").join("config")),
                EnvironmentFormat::GlobalHome => inspection.root.map(|root| root.join("config")),
                _ => None,
            }
            .ok_or_else(|| {
                "source must be an in-format KERO repository or global home".to_string()
            })?;
            let document =
                config::parse(&std::fs::read_to_string(config).map_err(|error| error.to_string())?)
                    .map_err(|error| error.to_string())?;
            let grants: Vec<_> = document.nodes.iter().filter(|node| node.name == "syncGrant" || node.name == "syncGrantRevocation").map(|node| {
                let value = |name: &str| node.children.iter().find(|child| child.name == name).and_then(|child| child.value.clone());
                serde_json::json!({"kind":node.name,"name":node.value,"targetKey":value("targetKey"),"direction":value("direction"),"baseline":value("baseline"),"expires":value("expires")})
            }).collect();
            serde_json::to_value(grants).map_err(|error| error.to_string())
        }
        Command::Mount {
            command:
                MountCommand::Grant {
                    command:
                        MountGrantCommand::Revoke {
                            name,
                            target_key,
                            source,
                        },
                },
        } => {
            let inspection = inspect_environment(source);
            let config = match inspection.format {
                EnvironmentFormat::Repository => inspection
                    .root
                    .map(|root| root.join(".kero").join("config")),
                EnvironmentFormat::GlobalHome => inspection.root.map(|root| root.join("config")),
                _ => None,
            }
            .ok_or_else(|| {
                "source must be an in-format KERO repository or global home".to_string()
            })?;
            use std::io::Write;
            let mut file = std::fs::OpenOptions::new()
                .append(true)
                .open(config)
                .map_err(|error| error.to_string())?;
            writeln!(file, "\n# Revoked writable-sync grant; retained as an audit record.\nsyncGrantRevocation {name}\n\ttargetKey {target_key}").map_err(|error| error.to_string())?;
            serde_json::to_value(
                serde_json::json!({"name":name,"targetKey":target_key,"revoked":true}),
            )
            .map_err(|error| error.to_string())
        }
        Command::Mount {
            command: MountCommand::Sync { name, repository },
        } => {
            let boundary = discover_repository(repository).map_err(repository_error)?;
            let identity = signing_identity()?;
            serde_json::to_value(
                sync_mount(
                    &boundary,
                    name,
                    &hex::encode(identity.verifying_key().to_bytes()),
                )
                .map_err(repository_error)?,
            )
            .map_err(|error| error.to_string())
        }
        Command::Mount {
            command:
                MountCommand::Conflict {
                    command:
                        MountConflictCommand::Export {
                            name,
                            output,
                            repository,
                        },
                },
        } => {
            let boundary = discover_repository(repository).map_err(repository_error)?;
            let path = export_mount_conflict(&boundary, name, output).map_err(repository_error)?;
            serde_json::to_value(PathResult {
                path: path.display().to_string(),
            })
            .map_err(|error| error.to_string())
        }
        Command::Mount {
            command: MountCommand::Conflict { command },
        } => {
            let (name, repository, use_local) = match command {
                MountConflictCommand::UseSource { name, repository } => (name, repository, false),
                MountConflictCommand::UseLocal { name, repository } => (name, repository, true),
                MountConflictCommand::Export { .. } => unreachable!(),
            };
            let boundary = discover_repository(repository).map_err(repository_error)?;
            let identity = signing_identity()?;
            serde_json::to_value(
                resolve_mount_conflict(
                    &boundary,
                    name,
                    &hex::encode(identity.verifying_key().to_bytes()),
                    use_local,
                )
                .map_err(repository_error)?,
            )
            .map_err(|error| error.to_string())
        }
        Command::Knowledge {
            command:
                KnowledgeCommand::Browse {
                    scope,
                    repository,
                    home,
                },
        } => {
            let (root, read_only) = if *home {
                (runtime_info.kero_home.join("data"), false)
            } else {
                let repository = repository.as_ref().ok_or_else(|| {
                    "knowledge browse requires --repository or --home".to_string()
                })?;
                let boundary = discover_repository(repository).map_err(repository_error)?;
                if scope == "local" {
                    (boundary.data, false)
                } else {
                    (boundary.mounts.join(scope), true)
                }
            };
            let entries = browse_data_tree(&root)?;
            serde_json::to_value(BrowseResult {
                scope: scope.clone(),
                root: root.display().to_string(),
                read_only,
                entries,
            })
            .map_err(|error| error.to_string())
        }
        Command::Knowledge {
            command: KnowledgeCommand::Add { source, repository },
        } => {
            let boundary = discover_repository(repository).map_err(repository_error)?;
            serde_json::to_value(
                capture_input(&boundary, source).map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())
        }
        Command::Knowledge {
            command: KnowledgeCommand::List { repository },
        } => {
            let boundary = discover_repository(repository).map_err(repository_error)?;
            serde_json::to_value(list_input(&boundary).map_err(|error| error.to_string())?)
                .map_err(|error| error.to_string())
        }
        Command::Knowledge {
            command: KnowledgeCommand::Remove { id, repository },
        } => {
            let boundary = discover_repository(repository).map_err(repository_error)?;
            remove_input(&boundary, id).map_err(|error| error.to_string())?;
            serde_json::to_value(KnowledgeRemoveResult { id: id.clone() })
                .map_err(|error| error.to_string())
        }
        Command::Process {
            command: ProcessCommand::Build { id, repository },
        } => {
            let boundary = discover_repository(repository).map_err(repository_error)?;
            let path = build_processed(&boundary, id).map_err(|error| error.to_string())?;
            serde_json::to_value(PathResult {
                path: path.display().to_string(),
            })
            .map_err(|error| error.to_string())
        }
        Command::Process {
            command: ProcessCommand::Verify { id, repository },
        } => {
            let boundary = discover_repository(repository).map_err(repository_error)?;
            let path = verify_processed(&boundary, id).map_err(|error| error.to_string())?;
            serde_json::to_value(PathResult {
                path: path.display().to_string(),
            })
            .map_err(|error| error.to_string())
        }
        Command::Trust {
            command:
                TrustCommand::Sign {
                    artifact,
                    key,
                    output,
                },
        } => {
            let path = sign_artifact(artifact, key, output.as_deref())
                .map_err(|error| error.to_string())?;
            serde_json::to_value(PathResult {
                path: path.display().to_string(),
            })
            .map_err(|error| error.to_string())
        }
        Command::Trust {
            command:
                TrustCommand::Verify {
                    artifact,
                    signature,
                },
        } => {
            let path = verify_signature(artifact, signature.as_deref())
                .map_err(|error| error.to_string())?;
            serde_json::to_value(PathResult {
                path: path.display().to_string(),
            })
            .map_err(|error| error.to_string())
        }
        Command::Repository {
            command: RepositoryCommand::Status { path },
        } => serde_json::to_value(inspect_repository(path)).map_err(|error| error.to_string()),
        Command::Repository {
            command: RepositoryCommand::Enroll { policy, path },
        } => {
            let policy = parse_enrollment(policy)?;
            let (discovery, outcome) = enroll_repository(path, policy).map_err(repository_error)?;
            serde_json::to_value(EnrollmentResult { discovery, outcome })
                .map_err(|error| error.to_string())
        }
        Command::Context {
            command: ContextCommand::Status { path },
        } => serde_json::to_value(ContextStatus {
            home: runtime_info.kero_home.display().to_string(),
            home_data: runtime_info.kero_home.join("data").display().to_string(),
            selected_path: path.display().to_string(),
            repository: inspect_repository(path),
        })
        .map_err(|error| error.to_string()),
        Command::Setup {
            home,
            enrollment,
            identity,
            dry_run,
        } => {
            let policy = SetupPolicy {
                home: parse_home(home),
                enrollment: parse_enrollment(enrollment)?,
                identity: if identity == "reuse" {
                    Identity::Reuse
                } else {
                    Identity::ConfigureLater
                },
                verification: Verification::Automatic,
                ..SetupPolicy::default()
            };
            let platform = HostPlatform::from_environment();
            let plan = setup::plan(policy, &platform).map_err(|error| error.to_string())?;
            if !dry_run {
                setup::apply(&plan, &platform).map_err(|error| error.to_string())?;
                if matches!(plan.policy.identity, Identity::Reuse) {
                    if let Some(home) = plan.home_path.as_deref() {
                        create_signing_identity(home)?;
                    }
                }
            }
            serde_json::to_value(SetupResult {
                operations: plan.operations,
                needs_elevation: plan.needs_elevation,
                dry_run: *dry_run,
            })
            .map_err(|error| error.to_string())
        }
        Command::Service { command } => {
            service::control(command, cli.runtime.as_deref()).map_err(|error| error.to_string())
        }
    }
}

fn print(cli: &Cli, value: serde_json::Value) {
    if cli.json {
        println!(
            "{}",
            serde_json::to_string(&ResultEnvelope {
                ok: true,
                result: value
            })
            .expect("serializable")
        );
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(&value).expect("serializable")
        );
    }
}

/// Produces a deterministic, metadata-only view of a service-owned knowledge scope.
fn browse_data_tree(root: &Path) -> Result<Vec<BrowseEntry>, String> {
    if !root.is_dir() {
        return Err(format!("knowledge.scope-unavailable: {}", root.display()));
    }
    fn visit(root: &Path, current: &Path, entries: &mut Vec<BrowseEntry>) -> Result<(), String> {
        let mut children: Vec<_> = std::fs::read_dir(current)
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        children.sort_by_key(|entry| entry.file_name());
        for child in children {
            let path = child.path();
            let metadata = std::fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
            if metadata.file_type().is_symlink() {
                return Err(format!("knowledge.scope-link: {}", path.display()));
            }
            let relative = path
                .strip_prefix(root)
                .map_err(|error| error.to_string())?
                .to_string_lossy()
                .replace('\\', "/");
            let kind = if metadata.is_dir() {
                "directory"
            } else if metadata.is_file() {
                "file"
            } else {
                return Err(format!("knowledge.scope-entry: {}", path.display()));
            };
            let modified_unix_seconds = metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|duration| duration.as_secs())
                .unwrap_or(0);
            entries.push(BrowseEntry {
                name: child.file_name().to_string_lossy().into_owned(),
                path: relative,
                kind: kind.into(),
                size: if metadata.is_file() {
                    metadata.len()
                } else {
                    0
                },
                modified_unix_seconds,
            });
            if metadata.is_dir() {
                visit(root, &path, entries)?;
            }
        }
        Ok(())
    }
    let mut entries = Vec::new();
    visit(root, root, &mut entries)?;
    Ok(entries)
}

fn default_kero_home() -> Option<PathBuf> {
    std::env::var_os("KERO_HOME")
        .map(PathBuf::from)
        .or_else(platform_default_kero_home)
}

fn platform_default_kero_home() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        return std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .map(PathBuf::from)
            .map(|base| base.join(".kero"));
    }
    #[cfg(target_os = "macos")]
    {
        return std::env::var_os("HOME").map(PathBuf::from).map(|base| {
            base.join("Library")
                .join("Application Support")
                .join("KERO")
        });
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        return std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME")
                    .map(|base| PathBuf::from(base).join(".local").join("share"))
            })
            .map(|base| base.join("kero"));
    }
    #[allow(unreachable_code)]
    None
}
fn repository_error(error: RepositoryError) -> String {
    error.to_string()
}
fn parse_home(value: &str) -> HomeLocation {
    match value {
        "user" => HomeLocation::UserLocal,
        "system" => HomeLocation::SystemWide,
        "none" => HomeLocation::None,
        other => HomeLocation::Custom(PathBuf::from(other)),
    }
}
fn parse_enrollment(value: &str) -> Result<Enrollment, String> {
    match value {
        "ask" => Ok(Enrollment::Ask),
        "automatic" => Ok(Enrollment::Automatic),
        "manual" => Ok(Enrollment::Manual),
        _ => Err("enrollment must be ask, automatic, or manual".into()),
    }
}

/// Native host facts; KERO core never interprets these conventions itself.
struct HostPlatform {
    user_home: PathBuf,
    system_home: Option<PathBuf>,
}
impl HostPlatform {
    fn from_environment() -> Self {
        let explicit_home = std::env::var_os("KERO_HOME").map(PathBuf::from);
        Self {
            user_home: explicit_home
                .or_else(platform_default_kero_home)
                .unwrap_or_else(|| PathBuf::from(".")),
            system_home: std::env::var_os("KERO_SYSTEM_HOME").map(PathBuf::from),
        }
    }
}
impl PlatformPathProvider for HostPlatform {
    fn user_home(&self) -> PathBuf {
        self.user_home.clone()
    }
    fn system_home(&self) -> Option<PathBuf> {
        self.system_home.clone()
    }
    fn is_absolute_path(&self, path: &Path) -> bool {
        path.is_absolute()
    }
}
impl PrivilegeProvider for HostPlatform {
    fn privilege_requirement(
        &self,
        path: &Path,
    ) -> Result<PrivilegeRequirement, PlatformAdapterError> {
        Ok(if self.system_home.as_deref() == Some(path) {
            PrivilegeRequirement::Elevation
        } else {
            PrivilegeRequirement::None
        })
    }
}
impl KeroHomeHost for HostPlatform {
    fn create_kero_home(&self, path: &Path, policy: &SetupPolicy) -> Result<(), setup::SetupError> {
        if path.exists() {
            if !path.is_dir() {
                return Err(setup::SetupError::HomeUnrecognized(path.into()));
            }
            std::fs::create_dir_all(path.join("data"))?;
            std::fs::create_dir_all(path.join("mnt"))?;
            if !path.join("config").exists() {
                std::fs::write(path.join("config"), home_config(policy))?;
            }
            return Ok(());
        }
        std::fs::create_dir_all(path.join("data"))?;
        std::fs::create_dir_all(path.join("mnt"))?;
        std::fs::write(path.join("config"), home_config(policy))?;
        Ok(())
    }
}

fn enrollment_name(value: Enrollment) -> &'static str {
    match value {
        Enrollment::Ask => "ask",
        Enrollment::Automatic => "automatic",
        Enrollment::Manual => "manual",
    }
}
fn identity_name(value: Identity) -> &'static str {
    match value {
        Identity::Reuse => "reuse",
        Identity::ConfigureLater => "configure-later",
    }
}

fn home_config(policy: &SetupPolicy) -> String {
    format!(
        "# KERO global-home preferences. These defaults never change repository data.\n\
         # Select one value in each section: uncomment an alternative, then comment\n\
         # the currently active line. KERO_HOME selects this home; it is not set here.\n\
         #\n\
         # Repository enrollment default: ask | automatic | manual\n\
         enrollment {}\n\
         # enrollment ask\n# enrollment automatic\n# enrollment manual\n\
         #\n\
         # Identity default: reuse | configure-later\n\
         identity {}\n\
         # identity reuse\n# identity configure-later\n\
         #\n\
         # Verification default: automatic | warn | strict | permissive\n\
         verification {}\n\
         # verification automatic\n# verification warn\n# verification strict\n# verification permissive\n\
         #\n\
         # Mount defaults. Repository mount entries override refresh only.\n\
         mountDefaults\n\
         \t# Refresh: manual | event\n\
         \trefresh manual\n\
         \t# refresh event\n\
         \t# Event debounce is a positive number of seconds.\n\
         \teventDebounceSeconds 1\n\
         \t# Missed-event audit: disabled (no periodic full-tree scan).\n\
         \tmissedEventAudit disabled\n\
         \t# Access request: readOnly | readWrite. readWrite still requires a grant.\n\
         \taccess readOnly\n\
         \t# access readWrite\n\
         \t# Conflict policy: blockAndAsk.\n\
         \tconflictPolicy blockAndAsk\n\
         #\n\
         # Service endpoint, bearer token, installation paths, and mount sources are\n\
         # intentionally not settings: they are generated or supplied explicitly.\n",
        enrollment_name(policy.enrollment),
        identity_name(policy.identity),
        verification_name(policy.verification),
    )
}
fn verification_name(value: Verification) -> &'static str {
    match value {
        Verification::Automatic => "automatic",
        Verification::Warn => "warn",
        Verification::Strict => "strict",
        Verification::Permissive => "permissive",
    }
}

#[derive(Clone, Debug)]
pub(crate) struct GlobalMountDefaults {
    pub(crate) refresh: String,
    pub(crate) event_debounce_seconds: u64,
    pub(crate) missed_event_audit: String,
    pub(crate) access: String,
    pub(crate) conflict_policy: String,
}

/// Reads the selected home's small, explicit mount-default catalog. Unknown or
/// malformed values fail rather than silently changing synchronization policy.
pub(crate) fn global_mount_defaults() -> Result<GlobalMountDefaults, String> {
    let mut defaults = GlobalMountDefaults {
        refresh: "manual".into(),
        event_debounce_seconds: 1,
        missed_event_audit: "disabled".into(),
        access: "readOnly".into(),
        conflict_policy: "blockAndAsk".into(),
    };
    let Some(home) = default_kero_home() else {
        return Ok(defaults);
    };
    let path = home.join("config");
    if !path.is_file() {
        return Ok(defaults);
    }
    let document =
        config::parse(&std::fs::read_to_string(path).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
    let Some(section) = document
        .nodes
        .iter()
        .rev()
        .find(|node| node.name == "mountDefaults")
    else {
        return Ok(defaults);
    };
    for child in &section.children {
        let Some(value) = child.value.as_deref() else {
            continue;
        };
        match child.name.as_str() {
            "refresh" if matches!(value, "manual" | "event") => defaults.refresh = value.into(),
            "eventDebounceSeconds" => {
                defaults.event_debounce_seconds = value
                    .parse::<u64>()
                    .ok()
                    .filter(|value| *value > 0)
                    .ok_or_else(|| "eventDebounceSeconds must be a positive integer".to_string())?
            }
            "missedEventAudit" if matches!(value, "disabled") => {
                defaults.missed_event_audit = value.into()
            }
            "access" if matches!(value, "readOnly" | "readWrite") => defaults.access = value.into(),
            "conflictPolicy" if matches!(value, "blockAndAsk") => {
                defaults.conflict_policy = value.into()
            }
            "refresh" | "missedEventAudit" | "access" | "conflictPolicy" => {
                return Err(format!(
                    "unsupported global mount default {} {value}",
                    child.name
                ));
            }
            _ => {}
        }
    }
    Ok(defaults)
}

fn set_mount_refresh_mode(config: &Path, name: &str, enabled: bool) -> Result<(), String> {
    if !name.chars().all(|character| {
        character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
    }) {
        return Err("mount names use lower-case letters, digits, and hyphens".into());
    }
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(config)
        .map_err(|error| error.to_string())?;
    writeln!(
        file,
        "\n# Mount refresh override; the last entry for a mount is active."
    )
    .map_err(|error| error.to_string())?;
    writeln!(
        file,
        "mount {name}\n\trefresh {}",
        if enabled { "event" } else { "manual" }
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

/// Revalidates event-refresh mounts at a user-visible status boundary. This
/// catches missed events without adding a background polling loop.
fn refresh_event_mounts(
    boundary: &kero_core::host::native_repository::RepositoryBoundary,
) -> Result<(), String> {
    let document = config::parse(
        &std::fs::read_to_string(&boundary.config).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    let default_refresh = global_mount_defaults()?.refresh;
    let mut modes = std::collections::BTreeMap::new();
    for node in document.nodes.iter().filter(|node| node.name == "mount") {
        if let (Some(name), Some(mode)) = (
            node.value.as_ref(),
            node.children
                .iter()
                .rev()
                .find(|child| child.name == "refresh")
                .and_then(|child| child.value.as_ref()),
        ) {
            modes.insert(name.clone(), mode.clone());
        }
    }
    for mount in list_mounts(boundary).map_err(repository_error)? {
        let name = mount.name;
        let mode = modes
            .get(&name)
            .map(String::as_str)
            .unwrap_or(&default_refresh);
        if mode == "event" {
            refresh_mount(boundary, &name).map_err(repository_error)?;
        }
    }
    Ok(())
}

fn signing_identity() -> Result<SigningKey, String> {
    let home = runtime::ensure_kero_home().map_err(|error| error.to_string())?;
    let path = home.join("identity.ed25519");
    if !path.is_file() {
        return Err("KERO identity is unavailable; run `kero-host setup --identity reuse` with explicit consent first".into());
    }
    let bytes = hex::decode(
        std::fs::read_to_string(path)
            .map_err(|error| error.to_string())?
            .trim(),
    )
    .map_err(|_| "KERO identity is invalid".to_string())?;
    let seed: [u8; 32] = bytes
        .try_into()
        .map_err(|_| "KERO identity is invalid".to_string())?;
    Ok(SigningKey::from_bytes(&seed))
}

fn create_signing_identity(home: &Path) -> Result<SigningKey, String> {
    let path = home.join("identity.ed25519");
    if path.is_file() {
        return signing_identity_at(&path);
    }
    let mut seed = [0_u8; 32];
    fill(&mut seed).map_err(|error| error.to_string())?;
    std::fs::write(&path, format!("{}\n", hex::encode(seed))).map_err(|error| error.to_string())?;
    Ok(SigningKey::from_bytes(&seed))
}

fn signing_identity_at(path: &Path) -> Result<SigningKey, String> {
    let bytes = hex::decode(
        std::fs::read_to_string(path)
            .map_err(|error| error.to_string())?
            .trim(),
    )
    .map_err(|_| "KERO identity is invalid".to_string())?;
    let seed: [u8; 32] = bytes
        .try_into()
        .map_err(|_| "KERO identity is invalid".to_string())?;
    Ok(SigningKey::from_bytes(&seed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setup_initializes_empty_home_and_preserves_existing_home_configuration() {
        let directory = tempfile::tempdir().unwrap();
        let platform = HostPlatform {
            user_home: directory.path().join("user-home"),
            system_home: None,
        };
        let unmarked = directory.path().join("unmarked");
        std::fs::create_dir(&unmarked).unwrap();
        platform
            .create_kero_home(&unmarked, &SetupPolicy::default())
            .unwrap();
        assert!(unmarked.join("config").is_file());

        let marked = directory.path().join("marked");
        platform
            .create_kero_home(&marked, &SetupPolicy::default())
            .unwrap();
        std::fs::write(marked.join("config"), "# contributor note\n").unwrap();
        platform
            .create_kero_home(&marked, &SetupPolicy::default())
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(marked.join("config")).unwrap(),
            "# contributor note\n"
        );
    }

    #[test]
    fn explicit_identity_creation_is_idempotent() {
        let directory = tempfile::tempdir().unwrap();
        let first = create_signing_identity(directory.path()).unwrap();
        let second = create_signing_identity(directory.path()).unwrap();
        assert_eq!(first.verifying_key(), second.verifying_key());
        assert!(directory.path().join("identity.ed25519").is_file());
    }
}

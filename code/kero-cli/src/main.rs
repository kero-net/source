use clap::{Parser, Subcommand, ValueEnum};
use kero_cli::layout;
use kero_cli::policy::{authorize, load_policy, load_request, load_snapshot};
use kero_cli::{
    ArtifactBinding, ArtifactVerifier, CommandSpec, HttpsSpec, PushSpec, WorkspaceWriteBroker,
};
use serde_json::json;
use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "kero",
    version,
    about = "KERO Controls Operations, Policy, and Environment"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Configure KERO for this user; starts the first-run setup experience.
    Init {
        /// Initialize the global environment at this path without prompting.
        #[arg(long)]
        root: Option<PathBuf>,
        /// Skip local storage diagnostics during interactive setup.
        #[arg(long)]
        skip_diagnostics: bool,
        /// Accept the selected location without confirmation.
        #[arg(long)]
        yes: bool,
    },
    /// Create, locate, or inspect the user-selected global KERO environment.
    Global {
        #[command(subcommand)]
        command: GlobalCommand,
    },
    /// Locate the nearest repository-specific .kero/ environment.
    Project {
        #[command(subcommand)]
        command: ProjectCommand,
    },
    /// Locate global or project-specific knowledge storage.
    Knowledge {
        #[command(subcommand)]
        command: KnowledgeCommand,
    },
    /// Report the independently discovered global and project environments.
    Inspect {
        #[arg(long, default_value = ".")]
        start: PathBuf,
    },
    /// Resolve or replay pure Layer 2 authorization decisions.
    Policy {
        #[command(subcommand)]
        command: PolicyCommand,
    },
    /// Verify a signed Layer 2 authorization artifact without re-deciding policy.
    VerifyArtifact {
        #[arg(long)]
        artifact: PathBuf,
        #[arg(long)]
        snapshot: PathBuf,
        #[arg(long)]
        key: PathBuf,
        #[arg(long)]
        audience: String,
        #[arg(long)]
        operation: String,
    },
    /// Perform one exact artifact-bound workspace write through the Linux broker.
    WorkspaceWrite {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        state: PathBuf,
        #[arg(long)]
        artifact: PathBuf,
        #[arg(long)]
        snapshot: PathBuf,
        #[arg(long)]
        key: PathBuf,
        #[arg(long)]
        content: PathBuf,
        /// Broker-owned canonical target expected by the signed artifact.
        #[arg(long)]
        target: String,
        /// Broker-owned principal expected by the signed artifact.
        #[arg(long)]
        principal: String,
        /// Broker-owned session identifier expected by the signed artifact.
        #[arg(long)]
        session: String,
        /// Canonical digest of the trusted request context expected by the broker.
        #[arg(long)]
        context_digest: String,
    },
    /// Run an exact no-network command through Bubblewrap.
    Bubblewrap {
        #[arg(long, default_value = "/usr/bin/bwrap")]
        binary: PathBuf,
        #[arg(long, default_value = "/")]
        working_directory: String,
        #[arg(long, default_value_t = 30)]
        timeout_seconds: u64,
        #[arg(long, default_value_t = 65536)]
        output_limit: usize,
        #[arg(last = true, required = true)]
        argv: Vec<String>,
    },
    /// Push one exact non-force refspec with ambient Git helpers disabled.
    GitPush {
        #[arg(long, default_value = "/usr/bin/git")]
        binary: PathBuf,
        #[arg(long)]
        worktree: PathBuf,
        #[arg(long)]
        remote: String,
        #[arg(long)]
        refspec: String,
        #[arg(long, default_value_t = 30)]
        timeout_seconds: u64,
    },
    /// Invoke one exact HTTPS service destination with DNS bypassed by --resolve.
    HttpsService {
        #[arg(long, default_value = "/usr/bin/curl")]
        binary: PathBuf,
        #[arg(long)]
        url: String,
        #[arg(long)]
        hostname: String,
        #[arg(long)]
        address: std::net::IpAddr,
        #[arg(long)]
        ca_file: PathBuf,
        #[arg(long)]
        body: PathBuf,
        #[arg(long, default_value = "POST")]
        method: String,
        #[arg(long = "header")]
        headers: Vec<String>,
        #[arg(long, default_value_t = 30)]
        timeout_seconds: u64,
        #[arg(long, default_value_t = 65536)]
        response_limit: usize,
    },
}

#[derive(Debug, Subcommand)]
enum GlobalCommand {
    /// Initialize and remember a global environment; defaults to $HOME/.kero.
    Init {
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Print the configured global environment and its declared identity.
    Path {
        #[arg(long)]
        root: Option<PathBuf>,
        #[arg(long)]
        plain: bool,
    },
}

#[derive(Debug, Subcommand)]
enum ProjectCommand {
    /// Initialize the single .kero/ integration surface in a repository.
    Init {
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    /// Print the nearest project environment found by walking upward.
    Path {
        #[arg(long, default_value = ".")]
        start: PathBuf,
        #[arg(long)]
        plain: bool,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum KnowledgeArea {
    Global,
    Shared,
    Local,
}

#[derive(Debug, Subcommand)]
enum KnowledgeCommand {
    /// Print the selected knowledge directory.
    Path {
        #[arg(value_enum)]
        area: KnowledgeArea,
        #[arg(long, default_value = ".")]
        start: PathBuf,
        #[arg(long)]
        plain: bool,
    },
}

#[derive(Debug, Subcommand)]
enum PolicyCommand {
    /// Load trusted TOML policy sources and return ALLOW or DENY.
    Authorize {
        #[arg(long, default_value = ".kero/policy/environment.toml")]
        environment: PathBuf,
        #[arg(long, default_value = ".kero/policy/records.toml")]
        records: Vec<PathBuf>,
        #[arg(long)]
        request: PathBuf,
        #[arg(long, default_value = ".kero/state/policy-snapshots")]
        snapshot_store: PathBuf,
    },
    /// Replay a request solely from an immutable policy snapshot.
    Replay {
        #[arg(long)]
        snapshot: PathBuf,
        #[arg(long)]
        digest: String,
        #[arg(long)]
        request: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();
    let exit = match cli.command {
        Command::Init {
            root,
            skip_diagnostics,
            yes,
        } => run_setup(root, skip_diagnostics, yes),
        Command::Global { command } => run_global(command),
        Command::Project { command } => run_project(command),
        Command::Knowledge { command } => run_knowledge(command),
        Command::Inspect { start } => run_inspect(&start),
        Command::Policy { command } => run_policy(command),
        Command::VerifyArtifact {
            artifact,
            snapshot,
            key,
            audience,
            operation,
        } => match ArtifactVerifier::new(audience, operation, key, snapshot).verify_path(&artifact)
        {
            Ok(verified) => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "schema": "kero/artifact-verification-result/v1",
                        "artifact_id": verified.artifact().artifact_id,
                        "authorization": verified.artifact().authorization.decision,
                        "verification": "VALID",
                        "operation": verified.artifact().operation,
                        "audience": verified.artifact().audience,
                    }))
                    .expect("static result serializes")
                );
                0
            }
            Err(error) => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "schema": "kero/artifact-verification-result/v1",
                        "verification": "INVALID",
                        "error": error.to_string(),
                    }))
                    .expect("static result serializes")
                );
                1
            }
        },
        Command::WorkspaceWrite {
            root,
            state,
            artifact,
            snapshot,
            key,
            content,
            target,
            principal,
            session,
            context_digest,
        } => {
            let result = (|| {
                let content = std::fs::read(content)?;
                let verifier = ArtifactVerifier::new(
                    kero_cli::boundary::workspace_write::BOUNDARY,
                    "filesystem.write",
                    key,
                    snapshot,
                );
                let broker = WorkspaceWriteBroker::new(root, state)
                    .map_err(|error| std::io::Error::other(error.to_string()))?;
                let binding = ArtifactBinding {
                    principal,
                    session_id: session,
                    targets: vec![target.clone()],
                    trusted_context_digest: context_digest,
                    execution_binding: json!({
                        "path": target,
                        "content_digest": kero_cli::canonical::sha256(&content),
                    }),
                    boundary_generation: Some(
                        broker
                            .generation()
                            .map_err(|error| std::io::Error::other(error.to_string()))?,
                    ),
                };
                broker
                    .write(&verifier, &artifact, &binding, &content)
                    .map_err(|error| std::io::Error::other(error.to_string()))
            })();
            match result {
                Ok(result) => {
                    print_json(&result);
                    0
                }
                Err(error) => print_layout_error("kero/workspace-write-error/v1", &error),
            }
        }
        Command::Bubblewrap {
            binary,
            working_directory,
            timeout_seconds,
            output_limit,
            argv,
        } => match kero_cli::boundary::bubblewrap::run(
            binary,
            &CommandSpec {
                argv,
                working_directory,
                environment: Vec::new(),
                timeout: std::time::Duration::from_secs(timeout_seconds),
                output_limit,
            },
        ) {
            Ok(output) => {
                print_json(&json!({
                    "schema": "kero/bubblewrap-result/v1",
                    "capability": "CAN",
                    "enforcement": "ADVISORY",
                    "execution": "EXECUTE",
                    "output": String::from_utf8_lossy(&output),
                }));
                0
            }
            Err(error) => print_layout_error("kero/bubblewrap-error/v1", &error),
        },
        Command::GitPush {
            binary,
            worktree,
            remote,
            refspec,
            timeout_seconds,
        } => match kero_cli::boundary::git_push::push(
            binary,
            &PushSpec {
                worktree,
                remote,
                refspec,
                timeout: std::time::Duration::from_secs(timeout_seconds),
            },
        ) {
            Ok(()) => {
                print_json(&json!({
                    "schema": "kero/git-push-result/v1",
                    "capability": "CAN",
                    "enforcement": "ADVISORY",
                    "execution": "EXECUTE",
                    "reason": "git.push.completed",
                }));
                0
            }
            Err(error) => print_layout_error("kero/git-push-error/v1", &error),
        },
        Command::HttpsService {
            binary,
            url,
            hostname,
            address,
            ca_file,
            body,
            method,
            headers,
            timeout_seconds,
            response_limit,
        } => {
            let result = (|| {
                let body = std::fs::read(body)?;
                kero_cli::boundary::https_service::invoke(
                    binary,
                    &HttpsSpec {
                        url,
                        method,
                        headers,
                        body_digest: kero_cli::canonical::sha256(&body),
                        body,
                        hostname,
                        address,
                        timeout: std::time::Duration::from_secs(timeout_seconds),
                        response_limit,
                        ca_file,
                    },
                )
            })();
            match result {
                Ok(response) => {
                    print_json(&json!({
                        "schema": "kero/https-service-result/v1",
                        "capability": "CAN",
                        "enforcement": "ADVISORY",
                        "execution": "EXECUTE",
                        "response_digest": kero_cli::canonical::sha256(&response),
                        "response_bytes": response.len(),
                    }));
                    0
                }
                Err(error) => print_layout_error("kero/https-service-error/v1", &error),
            }
        }
    };
    if exit != 0 {
        std::process::exit(exit);
    }
}

fn run_knowledge(command: KnowledgeCommand) -> i32 {
    match command {
        KnowledgeCommand::Path { area, start, plain } => {
            let discovered = match area {
                KnowledgeArea::Global => layout::discover_global(None)
                    .map(|(root, identity)| (root.join("knowledge"), identity)),
                KnowledgeArea::Shared => layout::discover_project(&start)
                    .map(|(root, identity)| (root.join("knowledge/shared"), identity)),
                KnowledgeArea::Local => layout::discover_project(&start)
                    .map(|(root, identity)| (root.join("knowledge/local"), identity)),
            };
            match discovered {
                Ok((path, identity)) => {
                    if plain {
                        println!("{}", path.display());
                    } else {
                        print_json(&json!({
                            "schema": "kero/discovered-knowledge/v1",
                            "area": format!("{area:?}").to_lowercase(),
                            "path": path,
                            "environment": identity,
                        }));
                    }
                    0
                }
                Err(error) => print_layout_error("kero/knowledge-error/v1", &error),
            }
        }
    }
}

fn print_json(value: &impl serde::Serialize) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).expect("result serializes")
    );
}

fn print_layout_error(schema: &str, error: &dyn std::fmt::Display) -> i32 {
    print_json(&json!({ "schema": schema, "error": error.to_string() }));
    1
}

fn default_global_root() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(|home| PathBuf::from(home).join(".kero"))
        .ok_or_else(|| "HOME is unavailable; pass --root".to_string())
}

fn writable_mounts() -> Vec<PathBuf> {
    let Ok(mounts) = std::fs::read_to_string("/proc/mounts") else {
        return Vec::new();
    };
    let ignored = [
        "proc", "sysfs", "devtmpfs", "devpts", "cgroup", "cgroup2", "tmpfs", "overlay",
    ];
    let mut paths = mounts
        .lines()
        .filter_map(|line| {
            let fields: Vec<_> = line.split_whitespace().collect();
            (fields.len() >= 4
                && !ignored.contains(&fields[2])
                && fields[3].split(',').any(|flag| flag == "rw"))
            .then(|| PathBuf::from(fields[1].replace("\\040", " ")))
        })
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    paths.sort();
    paths.dedup();
    paths
}

fn initialize_selected_global(root: PathBuf, schema: &str) -> i32 {
    match layout::initialize_global(&root) {
        Ok(initialized) => match layout::discover_global(Some(&root)) {
            Ok((verified_root, _)) => match layout::configure_global(&verified_root) {
                Ok(config) => {
                    print_json(
                        &json!({ "schema": schema, "global": initialized, "config": config }),
                    );
                    0
                }
                Err(error) => print_layout_error("kero/global-error/v1", &error),
            },
            Err(error) => print_layout_error("kero/global-error/v1", &error),
        },
        Err(error) => print_layout_error("kero/global-error/v1", &error),
    }
}

fn run_setup(root: Option<PathBuf>, skip_diagnostics: bool, yes: bool) -> i32 {
    if let Ok((configured, _)) = layout::discover_global(None) {
        print_json(&json!({
            "schema": "kero/setup-status/v1",
            "status": "configured",
            "global_root": configured,
            "next": "Run `kero project init` in a repository to create its .kero/ environment."
        }));
        return 0;
    }

    if let Some(root) = root {
        return initialize_selected_global(root, "kero/setup-initialized/v1");
    }
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return print_layout_error(
            "kero/setup-error/v1",
            &"interactive setup requires a terminal; use `kero init --root PATH`",
        );
    }

    let default = match default_global_root() {
        Ok(path) => path,
        Err(error) => return print_layout_error("kero/setup-error/v1", &error),
    };
    println!("Welcome to KERO. This command will take you through first-time setup.\n");
    println!("KERO keeps shared knowledge and policy in one global environment.");
    println!("Each repository can later opt in with `kero project init`.\n");
    if !skip_diagnostics {
        println!("Checking local storage locations...done.");
        let mounts = writable_mounts();
        if mounts.is_empty() {
            println!("No additional writable mounts were detected.");
        } else {
            println!("Detected writable mounts:");
            for mount in mounts {
                println!("  {}", mount.display());
            }
        }
        println!();
    }
    println!("Choose a global KERO location:");
    println!("  [1] {} (recommended)", default.display());
    println!("  [2] Enter another location");
    print!("Select [1-2]: ");
    let _ = io::stdout().flush();
    let mut choice = String::new();
    if io::stdin().read_line(&mut choice).is_err() {
        return print_layout_error("kero/setup-error/v1", &"could not read setup selection");
    }
    let selected = match choice.trim() {
        "" | "1" => default,
        "2" => {
            print!("Global KERO location: ");
            let _ = io::stdout().flush();
            let mut path = String::new();
            if io::stdin().read_line(&mut path).is_err() || path.trim().is_empty() {
                return print_layout_error("kero/setup-error/v1", &"a global location is required");
            }
            PathBuf::from(path.trim())
        }
        _ => return print_layout_error("kero/setup-error/v1", &"select 1 or 2"),
    };
    if !yes {
        print!("Create and select {}? [Y/n] ", selected.display());
        let _ = io::stdout().flush();
        let mut confirmation = String::new();
        if io::stdin().read_line(&mut confirmation).is_err() {
            return print_layout_error("kero/setup-error/v1", &"could not read setup confirmation");
        }
        if matches!(
            confirmation.trim().to_ascii_lowercase().as_str(),
            "n" | "no"
        ) {
            print_json(&json!({ "schema": "kero/setup-status/v1", "status": "cancelled" }));
            return 0;
        }
    }
    initialize_selected_global(selected, "kero/setup-initialized/v1")
}

fn run_global(command: GlobalCommand) -> i32 {
    match command {
        GlobalCommand::Init { root } => {
            let root = match root {
                Some(root) => root,
                None => match layout::discover_global(None) {
                    Ok((configured, _)) => configured,
                    Err(layout::DiscoveryError::GlobalUnconfigured) => {
                        match std::env::var_os("HOME") {
                            Some(home) => PathBuf::from(home).join(".kero"),
                            None => {
                                return print_layout_error(
                                    "kero/global-error/v1",
                                    &"HOME is unavailable; pass --root",
                                );
                            }
                        }
                    }
                    Err(error) => return print_layout_error("kero/global-error/v1", &error),
                },
            };
            match layout::initialize_global(&root) {
                Ok(initialized) => match layout::discover_global(Some(&root)) {
                    Ok((verified_root, _)) => match layout::configure_global(&verified_root) {
                        Ok(config) => {
                            print_json(&json!({
                                "schema": "kero/initialized-global/v1",
                                "global": initialized,
                                "config": config,
                            }));
                            0
                        }
                        Err(error) => print_layout_error("kero/global-error/v1", &error),
                    },
                    Err(error) => print_layout_error("kero/global-error/v1", &error),
                },
                Err(error) => print_layout_error("kero/global-error/v1", &error),
            }
        }
        GlobalCommand::Path { root, plain } => match layout::discover_global(root.as_deref()) {
            Ok((path, identity)) => {
                if plain {
                    println!("{}", path.display());
                } else {
                    print_json(&json!({
                        "schema": "kero/discovered-environment/v1",
                        "path": path,
                        "identity": identity,
                    }));
                }
                0
            }
            Err(error) => print_layout_error("kero/global-error/v1", &error),
        },
    }
}

fn run_project(command: ProjectCommand) -> i32 {
    match command {
        ProjectCommand::Init { root } => match layout::initialize(&root) {
            Ok(result) => {
                print_json(&result);
                0
            }
            Err(error) => print_layout_error("kero/project-init-error/v1", &error),
        },
        ProjectCommand::Path { start, plain } => match layout::discover_project(&start) {
            Ok((path, identity)) => {
                if plain {
                    println!("{}", path.display());
                } else {
                    print_json(&json!({
                        "schema": "kero/discovered-environment/v1",
                        "path": path,
                        "identity": identity,
                    }));
                }
                0
            }
            Err(error) => print_layout_error("kero/project-error/v1", &error),
        },
    }
}

fn run_inspect(start: &std::path::Path) -> i32 {
    let global = layout::discover_global(None)
        .map(|(path, identity)| json!({ "path": path, "identity": identity }))
        .unwrap_or_else(|error| json!({ "error": error.to_string() }));
    let project = layout::discover_project(start)
        .map(|(path, identity)| json!({ "path": path, "identity": identity }))
        .unwrap_or_else(|error| json!({ "error": error.to_string() }));
    print_json(&json!({
        "schema": "kero/environment-inspection/v1",
        "global": global,
        "project": project,
    }));
    0
}

fn run_policy(command: PolicyCommand) -> i32 {
    let result = match command {
        PolicyCommand::Authorize {
            environment,
            records,
            request,
            snapshot_store,
        } => load_policy(&environment, &records)
            .map_err(|error| error.to_string())
            .and_then(|policy| {
                load_request(&request)
                    .map_err(|error| error.to_string())
                    .and_then(|request| {
                        authorize(&policy, &request, &snapshot_store)
                            .map_err(|error| error.to_string())
                    })
            }),
        PolicyCommand::Replay {
            snapshot,
            digest,
            request,
        } => load_snapshot(&snapshot, Some(&digest))
            .map_err(|error| error.to_string())
            .and_then(|policy| {
                load_request(&request)
                    .map_err(|error| error.to_string())
                    .and_then(|request| {
                        let store = snapshot
                            .parent()
                            .unwrap_or_else(|| std::path::Path::new("."));
                        authorize(&policy, &request, store).map_err(|error| error.to_string())
                    })
            }),
    };
    match result {
        Ok(result) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&result).expect("result serializes")
            );
            if result.decision == "ALLOW" { 0 } else { 2 }
        }
        Err(error) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "schema": "kero/policy-error/v1",
                    "error": error,
                }))
                .expect("error serializes")
            );
            1
        }
    }
}

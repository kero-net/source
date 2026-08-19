use clap::{Parser, Subcommand, ValueEnum};
use scope_cli::ArtifactVerifier;
use scope_cli::layout;
use scope_cli::policy::{authorize, load_policy, load_request, load_snapshot};
use serde_json::json;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "scope",
    version,
    about = "SCOPE Controls Operations, Policy, and Environment"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Initialize the single .scope/ integration surface in a repository.
    Init {
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    /// Create, locate, or inspect the user-selected global SCOPE environment.
    Global {
        #[command(subcommand)]
        command: GlobalCommand,
    },
    /// Locate the nearest repository-specific .scope/ environment.
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
}

#[derive(Debug, Subcommand)]
enum GlobalCommand {
    /// Initialize and remember a global environment; defaults to $HOME/.scope.
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
        #[arg(long, default_value = ".scope/policy/environment.toml")]
        environment: PathBuf,
        #[arg(long, default_value = ".scope/policy/records.toml")]
        records: Vec<PathBuf>,
        #[arg(long)]
        request: PathBuf,
        #[arg(long, default_value = ".scope/state/policy-snapshots")]
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
        Command::Init { root } => match layout::initialize(&root) {
            Ok(result) => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result).expect("layout serializes")
                );
                0
            }
            Err(error) => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "schema": "scope/layout-error/v1",
                        "error": error.to_string(),
                    }))
                    .expect("error serializes")
                );
                1
            }
        },
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
                        "schema": "scope/artifact-verification-result/v1",
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
                        "schema": "scope/artifact-verification-result/v1",
                        "verification": "INVALID",
                        "error": error.to_string(),
                    }))
                    .expect("static result serializes")
                );
                1
            }
        },
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
                            "schema": "scope/discovered-knowledge/v1",
                            "area": format!("{area:?}").to_lowercase(),
                            "path": path,
                            "environment": identity,
                        }));
                    }
                    0
                }
                Err(error) => print_layout_error("scope/knowledge-error/v1", &error),
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

fn run_global(command: GlobalCommand) -> i32 {
    match command {
        GlobalCommand::Init { root } => {
            let root = match root {
                Some(root) => root,
                None => match layout::discover_global(None) {
                    Ok((configured, _)) => configured,
                    Err(layout::DiscoveryError::GlobalUnconfigured) => {
                        match std::env::var_os("HOME") {
                            Some(home) => PathBuf::from(home).join(".scope"),
                            None => {
                                return print_layout_error(
                                    "scope/global-error/v1",
                                    &"HOME is unavailable; pass --root",
                                );
                            }
                        }
                    }
                    Err(error) => return print_layout_error("scope/global-error/v1", &error),
                },
            };
            match layout::initialize_global(&root) {
                Ok(initialized) => match layout::discover_global(Some(&root)) {
                    Ok((verified_root, _)) => match layout::configure_global(&verified_root) {
                        Ok(config) => {
                            print_json(&json!({
                                "schema": "scope/initialized-global/v1",
                                "global": initialized,
                                "config": config,
                            }));
                            0
                        }
                        Err(error) => print_layout_error("scope/global-error/v1", &error),
                    },
                    Err(error) => print_layout_error("scope/global-error/v1", &error),
                },
                Err(error) => print_layout_error("scope/global-error/v1", &error),
            }
        }
        GlobalCommand::Path { root, plain } => match layout::discover_global(root.as_deref()) {
            Ok((path, identity)) => {
                if plain {
                    println!("{}", path.display());
                } else {
                    print_json(&json!({
                        "schema": "scope/discovered-environment/v1",
                        "path": path,
                        "identity": identity,
                    }));
                }
                0
            }
            Err(error) => print_layout_error("scope/global-error/v1", &error),
        },
    }
}

fn run_project(command: ProjectCommand) -> i32 {
    match command {
        ProjectCommand::Path { start, plain } => match layout::discover_project(&start) {
            Ok((path, identity)) => {
                if plain {
                    println!("{}", path.display());
                } else {
                    print_json(&json!({
                        "schema": "scope/discovered-environment/v1",
                        "path": path,
                        "identity": identity,
                    }));
                }
                0
            }
            Err(error) => print_layout_error("scope/project-error/v1", &error),
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
        "schema": "scope/environment-inspection/v1",
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
                    "schema": "scope/policy-error/v1",
                    "error": error,
                }))
                .expect("error serializes")
            );
            1
        }
    }
}

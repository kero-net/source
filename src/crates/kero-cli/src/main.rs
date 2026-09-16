use clap::{Parser, Subcommand};
use kero_core::knowledge::*;
use serde::Serialize;
use std::path::{Path, PathBuf};

const CLI_ENVELOPE: &str = "kero/cli-result/v1alpha1";
#[derive(Debug, Parser)]
#[command(
    name = "kero",
    version,
    about = "A persistent, project-attached knowledge environment"
)]
struct Cli {
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Command,
}
#[derive(Debug, Subcommand)]
enum Command {
    Init {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        id: Option<String>,
    },
    Add {
        source: PathBuf,
        #[arg(long)]
        project: Option<PathBuf>,
    },
    Remove {
        mount: String,
        #[arg(long)]
        project: Option<PathBuf>,
    },
    Status {
        #[arg(default_value = ".")]
        project: PathBuf,
        #[arg(long)]
        compiled: Option<PathBuf>,
    },
    Knowledge {
        #[command(subcommand)]
        command: Knowledge,
    },
}
#[derive(Debug, Subcommand)]
enum Knowledge {
    Validate {
        project: Option<PathBuf>,
    },
    Compile {
        project: Option<PathBuf>,
        #[arg(long)]
        output: PathBuf,
    },
    Inspect {
        compiled: PathBuf,
    },
}
#[derive(Serialize)]
struct Envelope<T: Serialize> {
    schema: &'static str,
    ok: bool,
    result: T,
}
#[derive(Serialize)]
struct ErrorEnvelope {
    schema: &'static str,
    ok: bool,
    error: ErrorBody,
}
#[derive(Serialize)]
struct ErrorBody {
    code: String,
    message: String,
}
#[derive(Serialize)]
struct InitResult {
    project: String,
    suggestions: Vec<String>,
}
#[derive(Serialize)]
struct MountResult {
    project: Project,
}

fn main() {
    let cli = Cli::parse();
    if let Err((code, message)) = run(&cli) {
        if cli.json {
            println!(
                "{}",
                serde_json::to_string(&ErrorEnvelope {
                    schema: CLI_ENVELOPE,
                    ok: false,
                    error: ErrorBody { code, message }
                })
                .unwrap()
            );
        } else {
            eprintln!("error: {message}");
        }
        std::process::exit(1);
    }
}
fn run(cli: &Cli) -> Result<(), (String, String)> {
    match &cli.command {
        Command::Init { path, id } => {
            let root = absolute(path).map_err(ioerr)?;
            let suggested = source_candidates(&root).map_err(perr)?;
            let default_id = root
                .file_name()
                .and_then(|v| v.to_str())
                .map(logical_name)
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "project".into());
            let id = KnowledgeSetId::new(id.clone().unwrap_or(default_id)).map_err(anyerr)?;
            let boundary = initialize(&root, id).map_err(perr)?;
            output(
                cli,
                InitResult {
                    project: boundary.declaration.display().to_string(),
                    suggestions: suggested
                        .into_iter()
                        .map(|p| p.to_string_lossy().into())
                        .collect(),
                },
                "initialized KERO project",
            )
        }
        Command::Add { source, project } => {
            let boundary = boundary(project.as_deref())?;
            let mut value = load(&boundary).map_err(perr)?;
            let locator = source
                .strip_prefix(&boundary.root)
                .unwrap_or(source)
                .to_string_lossy()
                .replace('\\', "/");
            let stem = source
                .file_stem()
                .and_then(|v| v.to_str())
                .map(logical_name)
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "source".into());
            let mut name = stem.clone();
            let mut n = 2;
            while value.mounts.iter().any(|m| m.id.as_str() == name) {
                name = format!("{stem}-{n}");
                n += 1;
            }
            let mut mount = Mount::new(
                MountId::new(&name).map_err(anyerr)?,
                SourceId::new(format!("source.{name}")).map_err(anyerr)?,
                locator,
                MountKind::LocalFile,
                0,
            );
            mount.media_type = Some("text/markdown".into());
            mount.importer = Some("kero.markdown".into());
            value.add_mount(mount).map_err(perr)?;
            save(&boundary, &value).map_err(perr)?;
            output(cli, MountResult { project: value }, "source added")
        }
        Command::Remove { mount, project } => {
            let boundary = boundary(project.as_deref())?;
            let mut value = load(&boundary).map_err(perr)?;
            let id = MountId::new(mount).map_err(anyerr)?;
            value.remove_mount(&id).map_err(perr)?;
            save(&boundary, &value).map_err(perr)?;
            output(cli, MountResult { project: value }, "source removed")
        }
        Command::Status { project, compiled } => {
            let boundary = discover(project).map_err(perr)?;
            let artifact = compiled
                .as_ref()
                .map(|p| read_artifact(p))
                .transpose()
                .map_err(cerr)?;
            let status =
                project_status(&boundary, artifact.as_ref().map(|a| &a.state)).map_err(perr)?;
            output(cli, status, "status computed")
        }
        Command::Knowledge { command } => match command {
            Knowledge::Validate { project } => {
                let boundary = boundary(project.as_deref())?;
                let report = CompilerService::validate(&boundary).map_err(cerr)?;
                output(cli, report, "project is valid")
            }
            Knowledge::Compile {
                project,
                output: target,
            } => {
                let boundary = boundary(project.as_deref())?;
                let artifact = CompilerService::compile(
                    &boundary,
                    target,
                    &CancellationToken::default(),
                    &mut |_| {},
                )
                .map_err(cerr)?;
                output(cli, artifact.report, "knowledge compiled")
            }
            Knowledge::Inspect { compiled } => {
                let artifact = read_artifact(compiled).map_err(cerr)?;
                output(cli, artifact, "compiled artifact")
            }
        },
    }
}
fn output<T: Serialize>(cli: &Cli, value: T, human: &str) -> Result<(), (String, String)> {
    if cli.json {
        println!(
            "{}",
            serde_json::to_string(&Envelope {
                schema: CLI_ENVELOPE,
                ok: true,
                result: value
            })
            .unwrap()
        )
    } else {
        println!("{human}")
    }
    Ok(())
}
fn boundary(path: Option<&Path>) -> Result<ProjectBoundary, (String, String)> {
    discover(path.unwrap_or(Path::new("."))).map_err(perr)
}
fn absolute(path: &Path) -> std::io::Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.into())
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}
fn logical_name(value: &str) -> String {
    let mut out = String::new();
    for c in value.to_ascii_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c)
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-')
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        out.insert_str(0, "project-")
    }
    out
}
fn perr(e: ProjectError) -> (String, String) {
    (e.diagnostic().code, e.to_string())
}
fn cerr(e: CompilerError) -> (String, String) {
    (e.code().into(), e.to_string())
}
fn ioerr(e: std::io::Error) -> (String, String) {
    ("cli.io".into(), e.to_string())
}
fn anyerr<E: std::fmt::Display>(e: E) -> (String, String) {
    ("knowledge.id.invalid".into(), e.to_string())
}

//! Native loading and ABI validation for the portable KERO core artifact.

use std::path::{Path, PathBuf};
use thiserror::Error;
use wasmi::{Engine, Linker, Module, Store};

const REQUIRED_ABI_VERSION: u32 = 1;

/// Facts established after the host loads the portable core.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeInfo {
    pub artifact: PathBuf,
    pub abi_version: u32,
    pub kero_home: PathBuf,
}

/// Runtime loading failures are host-delivery failures, not repository errors.
#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("runtime.artifact-not-found: expected kero.wasm at {0}")]
    ArtifactNotFound(PathBuf),
    #[error("runtime.io: {0}")]
    Io(#[from] std::io::Error),
    #[error("runtime.module: {0}")]
    Module(#[from] wasmi::Error),
    #[error("runtime.abi-mismatch: host requires ABI {expected}, core provides ABI {actual}")]
    AbiMismatch { expected: u32, actual: u32 },
    #[error("runtime.home-unrecognized: {0} exists but is not a marked KERO home")]
    HomeUnrecognized(PathBuf),
}

/// Finds the portable core beside the installed host, or at an explicit host
/// runtime path for development and embedding.
pub fn locate(explicit: Option<&Path>) -> Result<PathBuf, RuntimeError> {
    let artifact = explicit
        .map(Path::to_path_buf)
        .or_else(|| std::env::var_os("KERO_RUNTIME").map(PathBuf::from))
        .or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|host| host.parent().map(|parent| parent.join("kero.wasm")))
        })
        .unwrap_or_else(|| PathBuf::from("kero.wasm"));
    if !artifact.is_file() {
        return Err(RuntimeError::ArtifactNotFound(artifact));
    }
    Ok(artifact)
}

/// Instantiates the core with no ambient imports and checks its stable ABI.
pub fn load(artifact: &Path) -> Result<RuntimeInfo, RuntimeError> {
    let bytes = std::fs::read(artifact)?;
    let engine = Engine::default();
    let module = Module::new(&engine, &bytes[..])?;
    let mut store = Store::new(&engine, ());
    let linker = Linker::new(&engine);
    let instance = linker.instantiate(&mut store, &module)?.start(&mut store)?;
    let abi = instance
        .get_typed_func::<(), u32>(&store, "kero_host_abi_version")?
        .call(&mut store, ())?;
    if abi != REQUIRED_ABI_VERSION {
        return Err(RuntimeError::AbiMismatch {
            expected: REQUIRED_ABI_VERSION,
            actual: abi,
        });
    }
    let kero_home = ensure_kero_home()?;
    Ok(RuntimeInfo {
        artifact: artifact.to_path_buf(),
        abi_version: abi,
        kero_home,
    })
}

/// Resolves the selected KERO home and provisions its fixed global layout.
///
pub fn ensure_kero_home() -> Result<PathBuf, RuntimeError> {
    let home = std::env::var_os("KERO_HOME")
        .map(PathBuf::from)
        .or_else(default_kero_home)
        .ok_or_else(|| RuntimeError::HomeUnrecognized(PathBuf::from("KERO_HOME")))?;
    if home.exists() && !home.is_dir() {
        return Err(RuntimeError::HomeUnrecognized(home));
    }
    std::fs::create_dir_all(home.join("data"))?;
    std::fs::create_dir_all(home.join("mnt"))?;
    if !home.join("config").exists() {
        std::fs::write(home.join("config"), default_home_config())?;
    }
    Ok(home)
}

fn default_home_config() -> &'static str {
    "# KERO global-home preferences. These defaults never change repository data.\n\
# Select one value in each section: uncomment an alternative, then comment\n\
# the currently active line. KERO_HOME selects this home; it is not set here.\n\
#\n\
# Repository enrollment default: ask | automatic | manual\n\
enrollment ask\n\
# enrollment automatic\n# enrollment manual\n\
#\n\
# Identity default: reuse | configure-later\n\
identity configure-later\n\
# identity reuse\n\
#\n\
# Verification default: automatic | warn | strict | permissive\n\
verification automatic\n\
# verification warn\n# verification strict\n# verification permissive\n\
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
\tconflictPolicy blockAndAsk\n"
}

/// Resolves the platform-owned default shared by the installed native hosts.
fn default_kero_home() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let root = std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .map(PathBuf::from)
            .map(|base| base.join(".kero"))?;
        return Some(root);
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

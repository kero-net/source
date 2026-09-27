//! Local-only service transport for the KERO command-line host.

use crate::{ServiceCommand, global_mount_defaults, runtime};
use getrandom::fill;
use kero_core::{
    config,
    host::native_repository::{
        discover as discover_repository, list_mounts, mount_source_data, refresh_mount,
    },
};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use thiserror::Error;

const SERVICE_DIRECTORY: &str = ".runtime";
const SERVICE_STATE: &str = "service.kst";
const LEGACY_SERVICE_STATE: &str = "service.json";
const SERVICE_PROTOCOL_VERSION: u32 = 3;

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("service.io: {0}")]
    Io(#[from] std::io::Error),
    #[error("service.protocol: {0}")]
    Protocol(#[from] serde_json::Error),
    #[error("service.token: {0}")]
    Token(String),
    #[error("service.runtime: {0}")]
    Runtime(#[from] runtime::RuntimeError),
    #[error("service.unavailable: {0}")]
    Unavailable(String),
}

#[derive(Clone, Debug)]
struct ServiceState {
    transport: String,
    endpoint: String,
    process_id: u32,
    created_at: u64,
    protocol_version: u32,
    token: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum Request {
    Ping {
        token: String,
    },
    Run {
        token: String,
        arguments: Vec<String>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
struct Response {
    status: i32,
    stdout: String,
    stderr: String,
}

struct MountWatcher {
    key: String,
    _watcher: RecommendedWatcher,
}

/// Captured result returned by the service for one terminal invocation.
pub struct InvocationResult {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

/// Runs an explicit service lifecycle command.
pub fn control(
    command: &ServiceCommand,
    runtime_path: Option<&Path>,
) -> Result<serde_json::Value, ServiceError> {
    match command {
        ServiceCommand::Start => {
            let state = start(runtime_path)?;
            Ok(serde_json::json!({"state":"running", "transport":state.transport}))
        }
        ServiceCommand::Status => match state_if_running()? {
            Some(state) => Ok(serde_json::json!({"state":"running", "transport":state.transport})),
            None => Ok(serde_json::json!({"state":"stopped"})),
        },
        ServiceCommand::Stop => {
            // KERO's installed host is a user-session service. Allowing a
            // normal terminal command to stop it would strand refresh watchers
            // and make a successful install appear inactive.
            let state = start(runtime_path)?;
            Ok(
                serde_json::json!({"state":"running", "transport":state.transport,
                "message":"the persistent KERO service cannot be stopped from the terminal"}),
            )
        }
        ServiceCommand::Serve => {
            serve(runtime_path).map(|()| serde_json::json!({"state":"stopped"}))
        }
    }
}

/// Sends an ordinary CLI invocation through the local service, starting it on
/// demand. Arguments deliberately remain opaque to this transport.
pub fn invoke_or_start(arguments: Vec<String>) -> Result<InvocationResult, ServiceError> {
    let state = match state_if_running()? {
        Some(state) => state,
        None => start(runtime_argument(&arguments).as_deref())?,
    };
    let response = request(
        &state,
        Request::Run {
            token: state.token.clone(),
            arguments,
        },
    )?;
    Ok(InvocationResult {
        status: response.status,
        stdout: response.stdout,
        stderr: response.stderr,
    })
}

fn start(runtime_path: Option<&Path>) -> Result<ServiceState, ServiceError> {
    if let Some(state) = state_if_running()? {
        if state.protocol_version == SERVICE_PROTOCOL_VERSION {
            return Ok(state);
        }
        replace_obsolete_service(&state)?;
    }
    let (executable, runtime_artifact) = prepare_service_host(runtime_path)?;
    let mut command = Command::new(executable);
    command.arg("--service-dispatch");
    command.arg("--runtime").arg(runtime_artifact);
    command.arg("service").arg("serve");
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command.spawn()?;

    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        if let Some(state) = state_if_running()? {
            return Ok(state);
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    Err(ServiceError::Unavailable(
        "service did not publish its local endpoint within three seconds".into(),
    ))
}

/// Replaces only a live, token-authenticated earlier KERO service. This is an
/// update operation, not a user-visible stop: the replacement is launched
/// immediately with a new endpoint and token.
fn replace_obsolete_service(state: &ServiceState) -> Result<(), ServiceError> {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::Threading::{
            OpenProcess, PROCESS_TERMINATE, TerminateProcess,
        };
        if state.process_id == std::process::id() {
            return Err(ServiceError::Unavailable(
                "refusing to replace the current host process".into(),
            ));
        }
        let process = OpenProcess(PROCESS_TERMINATE, 0, state.process_id);
        if process.is_null() {
            return Err(ServiceError::Unavailable(
                "Windows could not open the obsolete KERO service for replacement".into(),
            ));
        }
        let terminated = TerminateProcess(process, 0);
        CloseHandle(process);
        if terminated == 0 {
            return Err(ServiceError::Unavailable(
                "Windows could not replace the obsolete KERO service".into(),
            ));
        }
    }
    #[cfg(not(windows))]
    {
        let _ = state;
        return Err(ServiceError::Unavailable(
            "service protocol replacement is not yet implemented on this platform".into(),
        ));
    }
    let _ = fs::remove_file(state_path()?);
    Ok(())
}

/// Runs the long-lived host from disposable global runtime state. Windows
/// cannot replace an executable held by a running process, so keeping the
/// service out of the installed/development path makes updates safe.
fn prepare_service_host(runtime_path: Option<&Path>) -> Result<(PathBuf, PathBuf), ServiceError> {
    let home = runtime::ensure_kero_home()?;
    let directory = home.join(SERVICE_DIRECTORY).join("service-host");
    fs::create_dir_all(&directory)?;
    let executable_source = std::env::current_exe()?;
    let runtime_source = runtime::locate(runtime_path)?;
    let suffix = format!("{}-{}", std::process::id(), unix_seconds()?);
    // Each instance owns a private directory, while its executable and portable
    // core keep their installed sibling names. Ordinary dispatched requests can
    // therefore resolve kero.wasm without receiving a client runtime argument.
    let instance = directory.join(suffix);
    fs::create_dir_all(&instance)?;
    let executable = instance.join(format!("kero-host{}", service_executable_suffix()));
    let artifact = instance.join("kero.wasm");
    fs::copy(executable_source, &executable)?;
    fs::copy(runtime_source, &artifact)?;
    Ok((executable, artifact))
}

#[cfg(windows)]
fn service_executable_suffix() -> &'static str {
    ".exe"
}
#[cfg(not(windows))]
fn service_executable_suffix() -> &'static str {
    ""
}

fn serve(runtime_path: Option<&Path>) -> Result<(), ServiceError> {
    let _ = runtime::load(&runtime::locate(runtime_path)?)?;
    let path = state_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut endpoint_random = [0_u8; 32];
    let mut token_random = [0_u8; 32];
    fill(&mut endpoint_random).map_err(|error| ServiceError::Token(error.to_string()))?;
    fill(&mut token_random).map_err(|error| ServiceError::Token(error.to_string()))?;
    let mut listener = LocalListener::bind(&hex::encode(endpoint_random))?;
    let state = ServiceState {
        transport: LocalListener::transport_name().into(),
        endpoint: listener.endpoint().into(),
        process_id: std::process::id(),
        created_at: unix_seconds()?,
        protocol_version: SERVICE_PROTOCOL_VERSION,
        token: hex::encode(token_random),
    };
    write_state(&path, &state)?;
    let mut watchers = Vec::<MountWatcher>::new();

    let result = (|| -> Result<(), ServiceError> {
        loop {
            let mut stream = listener.accept()?;
            let request: Request = read_message(&mut stream)?;
            let response = match request {
                Request::Ping { token } if token == state.token => Response {
                    status: 0,
                    stdout: String::new(),
                    stderr: String::new(),
                },
                Request::Run { token, arguments } if token == state.token => {
                    let response = run_child(arguments.clone())?;
                    if response.status == 0 {
                        configure_watcher(&arguments, &mut watchers);
                        reconcile_watches(&arguments, &mut watchers);
                    }
                    response
                }
                _ => Response {
                    status: 1,
                    stdout: String::new(),
                    stderr: "service authentication failed".into(),
                },
            };
            write_message(&mut stream, &response)?;
        }
        #[allow(unreachable_code)]
        Ok(())
    })();
    let _ = fs::remove_file(path);
    result
}

fn configure_watcher(arguments: &[String], watchers: &mut Vec<MountWatcher>) {
    let Some(position) = arguments.windows(3).position(|values| {
        values == ["mount", "watch", "enable"] || values == ["mount", "watch", "disable"]
    }) else {
        return;
    };
    let enabled = arguments[position + 2] == "enable";
    let Some(name) = arguments.get(position + 3) else {
        return;
    };
    let repository = arguments
        .windows(2)
        .find(|values| values[0] == "--repository")
        .map(|values| PathBuf::from(&values[1]))
        .unwrap_or_else(|| PathBuf::from("."));
    let Ok(boundary) = discover_repository(&repository) else {
        return;
    };
    let key = format!("{}:{name}", boundary.root.display());
    watchers.retain(|watcher| watcher.key != key);
    if !enabled {
        return;
    }
    let Ok(source) = mount_source_data(&boundary, name) else {
        return;
    };
    let pending = Arc::new(AtomicBool::new(false));
    let debounce = Arc::clone(&pending);
    let mount = name.clone();
    let watcher_boundary = boundary.clone();
    let callback = move |event: notify::Result<notify::Event>| {
        if event.is_err() || debounce.swap(true, Ordering::AcqRel) {
            return;
        }
        let pending = Arc::clone(&debounce);
        let boundary = watcher_boundary.clone();
        let mount = mount.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(1));
            let _ = refresh_mount(&boundary, &mount);
            pending.store(false, Ordering::Release);
        });
    };
    let Ok(mut watcher) = RecommendedWatcher::new(callback, notify::Config::default()) else {
        return;
    };
    if watcher.watch(&source, RecursiveMode::Recursive).is_ok() {
        watchers.push(MountWatcher {
            key,
            _watcher: watcher,
        });
    }
}

/// Restores durable event-watch policy whenever the service handles a command
/// for a repository. This deliberately avoids searching the machine for
/// repositories; only an explicitly selected repository is ever watched.
fn reconcile_watches(arguments: &[String], watchers: &mut Vec<MountWatcher>) {
    let repository = arguments
        .windows(2)
        .find(|values| values[0] == "--repository")
        .map(|values| PathBuf::from(&values[1]))
        .unwrap_or_else(|| PathBuf::from("."));
    let Ok(boundary) = discover_repository(&repository) else {
        return;
    };
    let Ok(document) = config::parse(&fs::read_to_string(&boundary.config).unwrap_or_default())
    else {
        return;
    };
    let defaults = global_mount_defaults().unwrap_or_else(|_| crate::GlobalMountDefaults {
        refresh: "manual".into(),
        event_debounce_seconds: 1,
        missed_event_audit: "disabled".into(),
        access: "readOnly".into(),
        conflict_policy: "blockAndAsk".into(),
    });
    let mut overrides = std::collections::BTreeMap::new();
    for node in document.nodes.iter().filter(|node| node.name == "mount") {
        if let (Some(name), Some(refresh)) = (
            node.value.as_ref(),
            node.children
                .iter()
                .rev()
                .find(|child| child.name == "refresh")
                .and_then(|child| child.value.as_ref()),
        ) {
            overrides.insert(name.clone(), refresh.clone());
        }
    }
    let Ok(mounts) = list_mounts(&boundary) else {
        return;
    };
    for info in mounts {
        let name = info.name;
        let enabled = overrides
            .get(&name)
            .map(String::as_str)
            .unwrap_or(&defaults.refresh)
            == "event";
        let key = format!("{}:{name}", boundary.root.display());
        watchers.retain(|watcher| watcher.key != key);
        if !enabled {
            continue;
        }
        let Ok(source) = mount_source_data(&boundary, &name) else {
            continue;
        };
        let pending = Arc::new(AtomicBool::new(false));
        let debounce = Arc::clone(&pending);
        let debounce_seconds = defaults.event_debounce_seconds;
        let watcher_boundary = boundary.clone();
        let mount = name.to_owned();
        let callback = move |event: notify::Result<notify::Event>| {
            if event.is_err() || debounce.swap(true, Ordering::AcqRel) {
                return;
            }
            let pending = Arc::clone(&debounce);
            let boundary = watcher_boundary.clone();
            let mount = mount.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_secs(debounce_seconds));
                let _ = refresh_mount(&boundary, &mount);
                pending.store(false, Ordering::Release);
            });
        };
        let Ok(mut watcher) = RecommendedWatcher::new(callback, notify::Config::default()) else {
            continue;
        };
        if watcher.watch(&source, RecursiveMode::Recursive).is_ok() {
            watchers.push(MountWatcher {
                key,
                _watcher: watcher,
            });
        }
    }
}

fn run_child(arguments: Vec<String>) -> Result<Response, ServiceError> {
    let executable = std::env::current_exe()?;
    let output = Command::new(executable)
        .arg("--service-dispatch")
        .args(arguments)
        .output()?;
    Ok(Response {
        status: output.status.code().unwrap_or(1),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

fn state_if_running() -> Result<Option<ServiceState>, ServiceError> {
    let path = state_path()?;
    // TCP state from unreleased hosts is intentionally never interpreted as a
    // fallback endpoint. It is disposable and cannot keep a port-based service alive.
    let legacy = path
        .parent()
        .unwrap_or(Path::new("."))
        .join(LEGACY_SERVICE_STATE);
    if legacy.is_file() {
        let _ = fs::remove_file(legacy);
    }
    let Ok(bytes) = fs::read(&path) else {
        return Ok(None);
    };
    let Ok(state) = decode_state(&String::from_utf8_lossy(&bytes)) else {
        let _ = fs::remove_file(path);
        return Ok(None);
    };
    match request(
        &state,
        Request::Ping {
            token: state.token.clone(),
        },
    ) {
        Ok(response) if response.status == 0 => Ok(Some(state)),
        _ => {
            let _ = fs::remove_file(state_path()?);
            Ok(None)
        }
    }
}

fn runtime_argument(arguments: &[String]) -> Option<PathBuf> {
    arguments
        .windows(2)
        .find(|pair| pair[0] == "--runtime")
        .map(|pair| PathBuf::from(&pair[1]))
}

fn state_path() -> Result<PathBuf, ServiceError> {
    Ok(runtime::ensure_kero_home()?
        .join(SERVICE_DIRECTORY)
        .join(SERVICE_STATE))
}

fn write_state(path: &Path, state: &ServiceState) -> Result<(), ServiceError> {
    let temporary = path.with_extension("kst.tmp");
    fs::write(&temporary, encode_state(state))?;
    fs::rename(temporary, path)?;
    Ok(())
}

fn request(state: &ServiceState, message: Request) -> Result<Response, ServiceError> {
    let mut stream = LocalListener::connect(&state.endpoint)?;
    write_message(&mut stream, &message)?;
    read_message(&mut stream)
}

fn write_message<T: Serialize, S: Write>(stream: &mut S, value: &T) -> Result<(), ServiceError> {
    serde_json::to_writer(&mut *stream, value)?;
    stream.write_all(b"\n")?;
    stream.flush()?;
    Ok(())
}

fn read_message<T: for<'a> Deserialize<'a>, S: Read>(stream: &mut S) -> Result<T, ServiceError> {
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line)?;
    if line.is_empty() {
        return Err(ServiceError::Unavailable(
            "service closed the local connection".into(),
        ));
    }
    Ok(serde_json::from_str(&line)?)
}

fn unix_seconds() -> Result<u64, ServiceError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|time| time.as_secs())
        .map_err(|_| ServiceError::Unavailable("system clock is before Unix epoch".into()))
}

fn encode_state(state: &ServiceState) -> String {
    format!(
        "# Generated KERO local-service state. Runtime state is disposable.\ntransport {}\nendpoint {:?}\nprocessId {}\ncreatedAt {}\nprotocolVersion {}\ntoken {}\n",
        state.transport,
        state.endpoint,
        state.process_id,
        state.created_at,
        state.protocol_version,
        state.token
    )
}

fn decode_state(input: &str) -> Result<ServiceState, ServiceError> {
    let document = config::parse(input).map_err(|error| {
        ServiceError::Unavailable(format!("invalid local-service state: {error}"))
    })?;
    let value = |name: &str| {
        document
            .nodes
            .iter()
            .find(|node| node.name == name)
            .and_then(|node| node.value.clone())
            .ok_or_else(|| {
                ServiceError::Unavailable(format!("local-service state is missing {name}"))
            })
    };
    let transport = value("transport")?;
    if !matches!(transport.as_str(), "named-pipe" | "unix-socket") {
        return Err(ServiceError::Unavailable(
            "local-service state has unsupported transport".into(),
        ));
    }
    let protocol_version = document
        .nodes
        .iter()
        .find(|node| node.name == "protocolVersion")
        .and_then(|node| node.value.clone())
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    Ok(ServiceState {
        transport,
        endpoint: value("endpoint")?,
        process_id: value("processId")?.parse().map_err(|_| {
            ServiceError::Unavailable("local-service state has invalid processId".into())
        })?,
        created_at: value("createdAt")?.parse().map_err(|_| {
            ServiceError::Unavailable("local-service state has invalid createdAt".into())
        })?,
        protocol_version,
        token: value("token")?,
    })
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod transport_tests {
    use super::*;

    #[test]
    fn service_state_is_kst_and_never_contains_a_port() {
        let state = ServiceState {
            transport: "named-pipe".into(),
            endpoint: r"\\.\pipe\KERO-test".into(),
            process_id: 42,
            created_at: 7,
            protocol_version: SERVICE_PROTOCOL_VERSION,
            token: "token".into(),
        };
        let encoded = encode_state(&state);
        assert!(encoded.contains("transport named-pipe"));
        assert!(!encoded.lines().any(|line| line.starts_with("port ")));
        assert_eq!(decode_state(&encoded).unwrap().endpoint, state.endpoint);
    }

    #[test]
    fn tcp_state_is_not_a_valid_local_ipc_record() {
        assert!(decode_state(r#"{"port":54019,"token":"old"}"#).is_err());
    }
}

#[cfg(windows)]
struct LocalListener {
    endpoint: String,
    pending: std::fs::File,
}

#[cfg(windows)]
impl LocalListener {
    fn bind(random: &str) -> Result<Self, ServiceError> {
        let endpoint = format!(r"\\.\pipe\KERO-{random}");
        Ok(Self {
            pending: create_named_pipe(&endpoint, true)?,
            endpoint,
        })
    }
    fn endpoint(&self) -> &str {
        &self.endpoint
    }
    fn transport_name() -> &'static str {
        "named-pipe"
    }
    fn accept(&mut self) -> Result<std::fs::File, ServiceError> {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::Foundation::{ERROR_PIPE_CONNECTED, GetLastError};
        use windows_sys::Win32::System::Pipes::ConnectNamedPipe;
        let handle = self.pending.as_raw_handle();
        let connected = unsafe { ConnectNamedPipe(handle, std::ptr::null_mut()) } != 0;
        if !connected && unsafe { GetLastError() } != ERROR_PIPE_CONNECTED {
            return Err(ServiceError::Io(std::io::Error::last_os_error()));
        }
        // Keep one server instance pending before processing the connected client.
        let next = create_named_pipe(&self.endpoint, false)?;
        Ok(std::mem::replace(&mut self.pending, next))
    }
    fn connect(endpoint: &str) -> Result<std::fs::File, ServiceError> {
        use std::os::windows::io::FromRawHandle;
        use windows_sys::Win32::Foundation::{GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE};
        use windows_sys::Win32::Storage::FileSystem::{
            CreateFileW, FILE_ATTRIBUTE_NORMAL, OPEN_EXISTING,
        };
        let wide = wide(endpoint);
        let handle = unsafe {
            CreateFileW(
                wide.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                0,
                std::ptr::null(),
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                std::ptr::null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            return Err(ServiceError::Io(std::io::Error::last_os_error()));
        }
        Ok(unsafe { std::fs::File::from_raw_handle(handle as *mut _) })
    }
}

#[cfg(windows)]
fn create_named_pipe(endpoint: &str, first_instance: bool) -> Result<std::fs::File, ServiceError> {
    use std::os::windows::io::FromRawHandle;
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_FLAG_FIRST_PIPE_INSTANCE, PIPE_ACCESS_DUPLEX,
    };
    use windows_sys::Win32::System::Pipes::{
        CreateNamedPipeW, PIPE_READMODE_MESSAGE, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_MESSAGE,
        PIPE_WAIT,
    };
    let wide = wide(endpoint);
    // Owner-only DACL: the service process is created in the signed-in user's
    // token, so no other account can open the endpoint even before token auth.
    let security = named_pipe_security()?;
    let mode = PIPE_ACCESS_DUPLEX
        | if first_instance {
            FILE_FLAG_FIRST_PIPE_INSTANCE
        } else {
            0
        };
    let handle = unsafe {
        CreateNamedPipeW(
            wide.as_ptr(),
            mode,
            PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
            2,
            64 * 1024,
            64 * 1024,
            0,
            &security.attributes,
        )
    };
    security.release();
    if handle == INVALID_HANDLE_VALUE {
        return Err(ServiceError::Io(std::io::Error::last_os_error()));
    }
    Ok(unsafe { std::fs::File::from_raw_handle(handle as *mut _) })
}

#[cfg(windows)]
struct PipeSecurity {
    attributes: windows_sys::Win32::Security::SECURITY_ATTRIBUTES,
    descriptor: *mut core::ffi::c_void,
}
#[cfg(windows)]
impl PipeSecurity {
    fn release(self) {
        unsafe {
            windows_sys::Win32::Foundation::LocalFree(self.descriptor);
        }
    }
}
#[cfg(windows)]
fn named_pipe_security() -> Result<PipeSecurity, ServiceError> {
    use windows_sys::Win32::Security::Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW;
    use windows_sys::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};
    let sddl = wide("D:P(A;;GA;;;OW)");
    let mut descriptor: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
    if unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            1,
            &mut descriptor,
            std::ptr::null_mut(),
        )
    } == 0
    {
        return Err(ServiceError::Io(std::io::Error::last_os_error()));
    }
    Ok(PipeSecurity {
        attributes: SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: descriptor,
            bInheritHandle: 0,
        },
        descriptor: descriptor.cast(),
    })
}
#[cfg(windows)]
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(unix)]
struct LocalListener {
    endpoint: PathBuf,
    listener: std::os::unix::net::UnixListener,
}
#[cfg(unix)]
impl LocalListener {
    fn bind(_: &str) -> Result<Self, ServiceError> {
        use std::os::unix::fs::PermissionsExt;
        let endpoint = runtime::ensure_kero_home()?
            .join(SERVICE_DIRECTORY)
            .join("service.sock");
        if endpoint.exists() {
            if std::os::unix::net::UnixStream::connect(&endpoint).is_ok() {
                return Err(ServiceError::Unavailable(
                    "an existing local Unix socket is still live".into(),
                ));
            }
            fs::remove_file(&endpoint)?;
        }
        let listener = std::os::unix::net::UnixListener::bind(&endpoint)?;
        fs::set_permissions(&endpoint, fs::Permissions::from_mode(0o600))?;
        Ok(Self { endpoint, listener })
    }
    fn endpoint(&self) -> &str {
        self.endpoint.to_str().unwrap_or_default()
    }
    fn transport_name() -> &'static str {
        "unix-socket"
    }
    fn accept(&mut self) -> Result<std::os::unix::net::UnixStream, ServiceError> {
        self.listener
            .accept()
            .map(|(stream, _)| stream)
            .map_err(ServiceError::Io)
    }
    fn connect(endpoint: &str) -> Result<std::os::unix::net::UnixStream, ServiceError> {
        std::os::unix::net::UnixStream::connect(endpoint).map_err(ServiceError::Io)
    }
}

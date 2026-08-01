#![cfg(feature = "serve")]

#[path = "support/shim_contract.rs"]
mod shim_contract;

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

static NEXT_TEMP_DIRECTORY: AtomicU64 = AtomicU64::new(0);
static SERVER_START_LOCK: Mutex<()> = Mutex::new(());

struct TestDirectory {
    path: PathBuf,
}

impl TestDirectory {
    fn new(label: &str) -> Self {
        let sequence = NEXT_TEMP_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock must follow the Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "Mech serve ü path # % spaces {label} {} {sequence} {nanos}",
            std::process::id(),
        ));
        std::fs::create_dir_all(&path).expect("serve fixture directory must be created");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

#[derive(Debug)]
struct HttpResponse {
    status: u16,
    headers: BTreeMap<String, String>,
    body: Vec<u8>,
}

struct RunningServer {
    child: Child,
    port: u16,
    stdout_path: PathBuf,
    stderr_path: PathBuf,
    stopped: bool,
}

impl RunningServer {
    fn spawn(current_dir: &Path, input: &Path, no_config: bool) -> Self {
        Self::spawn_with_web_resources(current_dir, input, no_config, None, None)
    }

    fn spawn_with_web_resources(
        current_dir: &Path,
        input: &Path,
        no_config: bool,
        shim: Option<&Path>,
        stylesheet: Option<&Path>,
    ) -> Self {
        Self::spawn_inputs_with_web_resources(current_dir, &[input], no_config, shim, stylesheet)
    }

    fn spawn_inputs_with_web_resources(
        current_dir: &Path,
        inputs: &[&Path],
        no_config: bool,
        shim: Option<&Path>,
        stylesheet: Option<&Path>,
    ) -> Self {
        // Selecting an ephemeral port and starting the child cannot be atomic
        // across processes. Serialize that handoff inside this test binary so
        // parallel serve tests cannot both select the same just-released port
        // before either child has bound it (which Windows detects reliably).
        let _start_guard = SERVER_START_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let port = available_port();
        let stdout_path = current_dir.join(format!("serve-{port}.stdout.log"));
        let stderr_path = current_dir.join(format!("serve-{port}.stderr.log"));
        let wasm_pkg = current_dir.join("test-wasm-pkg");
        std::fs::create_dir_all(&wasm_pkg).expect("test WASM package directory must be created");
        std::fs::write(wasm_pkg.join("mech_wasm_bg.wasm"), b"\0asm\x01\0\0\0")
            .expect("test WASM module must be written");
        std::fs::write(
            wasm_pkg.join("mech_wasm.js"),
            "export default async function init() {}\n",
        )
        .expect("test WASM wrapper must be written");
        let stdout = File::create(&stdout_path).expect("server stdout log must be created");
        let stderr = File::create(&stderr_path).expect("server stderr log must be created");

        let mut command = Command::new(env!("CARGO_BIN_EXE_mech"));
        command.current_dir(current_dir);
        configure_child_process_group(&mut command);
        if no_config {
            command.arg("--no-config");
        }
        command
            .arg("serve")
            .args(inputs)
            .arg("--address")
            .arg("127.0.0.1")
            .arg("--port")
            .arg(port.to_string())
            .arg("--wasm")
            .arg(&wasm_pkg)
            .stdin(Stdio::null())
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr));
        if let Some(shim) = shim {
            command.arg("--shim").arg(shim);
        }
        if let Some(stylesheet) = stylesheet {
            command.arg("--stylesheet").arg(stylesheet);
        }
        let child = command.spawn().expect("Cargo-built mech server must start");
        let mut server = Self {
            child,
            port,
            stdout_path,
            stderr_path,
            stopped: false,
        };
        server.wait_until_ready();
        server
    }

    fn request(&mut self, path: &str) -> HttpResponse {
        match http_request(self.port, path, true) {
            Ok(response) => response,
            Err(error) => self.fail(&format!("GET {path} failed: {error}")),
        }
    }

    fn request_headers(&mut self, path: &str) -> HttpResponse {
        match http_request(self.port, path, false) {
            Ok(response) => response,
            Err(error) => self.fail(&format!("header request for {path} failed: {error}")),
        }
    }

    fn assert_route(&mut self, path: &str, status: u16, content_type: &str) -> HttpResponse {
        let response = self.request(path);
        if response.status != status {
            self.fail(&format!(
                "GET {path} returned {}, expected {status}",
                response.status,
            ));
        }
        let actual = response
            .headers
            .get("content-type")
            .map(String::as_str)
            .unwrap_or("");
        if !actual.starts_with(content_type) {
            self.fail(&format!(
                "GET {path} returned content-type `{actual}`, expected `{content_type}`",
            ));
        }
        response
    }

    fn assert_body_contains(&mut self, path: &str, response: &HttpResponse, expected: &str) {
        let body = String::from_utf8_lossy(&response.body);
        if !body.contains(expected) {
            self.fail(&format!(
                "GET {path} body did not contain `{expected}`\nbody:\n{body}",
            ));
        }
    }

    fn wait_until_ready(&mut self) {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            if let Some(status) = self
                .child
                .try_wait()
                .expect("server process status must be readable")
            {
                self.stopped = true;
                self.fail(&format!("server exited before becoming ready: {status}"));
            }
            if TcpStream::connect(("127.0.0.1", self.port)).is_ok() {
                return;
            }
            if Instant::now() >= deadline {
                self.fail("server did not become ready within 30 seconds");
            }
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    fn stop(&mut self) {
        if self.stopped {
            return;
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
        self.stopped = true;
    }

    fn interrupt_and_wait(&mut self, timeout: Duration) {
        if self.stopped {
            return;
        }
        send_interrupt(&self.child).unwrap_or_else(|error| {
            self.fail(&format!("failed to interrupt server: {error}"));
        });
        let deadline = Instant::now() + timeout;
        loop {
            match self.child.try_wait() {
                Ok(Some(status)) => {
                    self.stopped = true;
                    if !status.success() {
                        self.fail(&format!(
                            "interrupted server exited unsuccessfully: {status}"
                        ));
                    }
                    return;
                }
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(25));
                }
                Ok(None) => self.fail("interrupted server did not exit before the deadline"),
                Err(error) => self.fail(&format!("failed to wait for interrupted server: {error}")),
            }
        }
    }

    fn logs(&self) -> String {
        format!(
            "stdout:\n{}\nstderr:\n{}",
            std::fs::read_to_string(&self.stdout_path).unwrap_or_default(),
            std::fs::read_to_string(&self.stderr_path).unwrap_or_default(),
        )
    }

    fn fail(&mut self, message: &str) -> ! {
        self.stop();
        panic!("{message}\n{}", self.logs());
    }
}

#[cfg(unix)]
fn configure_child_process_group(_command: &mut Command) {}

#[cfg(windows)]
fn configure_child_process_group(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    command.creation_flags(windows_sys::Win32::System::Threading::CREATE_NEW_PROCESS_GROUP);
}

#[cfg(unix)]
fn send_interrupt(child: &Child) -> std::io::Result<()> {
    let status = Command::new("kill")
        .arg("-INT")
        .arg(child.id().to_string())
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("kill -INT exited with {status}"),
        ))
    }
}

#[cfg(windows)]
fn send_interrupt(child: &Child) -> std::io::Result<()> {
    let script = format!(
        r#"
$source = @'
using System;
using System.Runtime.InteropServices;
public static class ConsoleSignal {{
    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern bool GenerateConsoleCtrlEvent(
        uint ctrlEvent,
        uint processGroupId
    );
}}
'@
Add-Type -TypeDefinition $source
if (-not [ConsoleSignal]::GenerateConsoleCtrlEvent(1, {})) {{
    exit 1
}}
"#,
        child.id(),
    );
    let status = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("GenerateConsoleCtrlEvent helper exited with {status}"),
        ))
    }
}

impl Drop for RunningServer {
    fn drop(&mut self) {
        self.stop();
    }
}

fn available_port() -> u16 {
    TcpListener::bind(("127.0.0.1", 0))
        .expect("ephemeral localhost port must bind")
        .local_addr()
        .expect("ephemeral localhost address must exist")
        .port()
}

fn http_request(port: u16, path: &str, read_body: bool) -> std::io::Result<HttpResponse> {
    let mut stream = TcpStream::connect(("127.0.0.1", port))?;
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    stream.set_write_timeout(Some(Duration::from_secs(10)))?;
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n",
    )?;
    stream.flush()?;

    let mut bytes = Vec::new();
    let header_end = loop {
        let mut chunk = [0_u8; 4096];
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            break find_header_end(&bytes).ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "HTTP headers were incomplete",
                )
            })?;
        }
        bytes.extend_from_slice(&chunk[..read]);
        if let Some(header_end) = find_header_end(&bytes) {
            break header_end;
        }
    };

    if read_body {
        stream.read_to_end(&mut bytes)?;
    } else {
        bytes.truncate(header_end);
    }
    parse_response(bytes, header_end)
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|position| position + 4)
}

fn parse_response(bytes: Vec<u8>, header_end: usize) -> std::io::Result<HttpResponse> {
    let header_text = std::str::from_utf8(&bytes[..header_end])
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string()))?;
    let mut lines = header_text.split("\r\n");
    let status = lines
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|status| status.parse::<u16>().ok())
        .ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "HTTP status was invalid")
        })?;
    let mut headers = BTreeMap::new();
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
    }
    Ok(HttpResponse {
        status,
        headers,
        body: bytes[header_end..].to_vec(),
    })
}

fn shim_fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("shims")
        .join(name)
}

fn shipped_include_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("include")
        .join(name)
}

fn copy_resource(directory: &TestDirectory, resource: &Path) -> PathBuf {
    let file_name = resource
        .file_name()
        .expect("web resource must have a file name");
    let destination = directory.path().join(file_name);
    std::fs::copy(resource, &destination).expect("web resource fixture must be copied");
    destination
}

fn copy_slot_fixture(directory: &TestDirectory, name: &str) -> PathBuf {
    let source = directory.path().join(name);
    std::fs::copy(shim_fixture_path("all-slots.mec"), &source)
        .expect("slot-contract source fixture must be copied");
    std::fs::copy(
        shim_fixture_path("hero.svg"),
        directory.path().join("hero.svg"),
    )
    .expect("slot-contract hero fixture must be copied");
    source
}

#[cfg(all(feature = "formatter", has_file_wasm))]
fn format_rich_fixture(
    directory: &TestDirectory,
    source: &Path,
    shim: &Path,
    stylesheet: &Path,
    output: &Path,
) {
    let output_result = Command::new(env!("CARGO_BIN_EXE_mech"))
        .current_dir(directory.path())
        .arg("format")
        .arg(source)
        .arg("--html")
        .arg("--shim")
        .arg(shim)
        .arg("--stylesheet")
        .arg(stylesheet)
        .arg("--out")
        .arg(output)
        .output()
        .expect("Cargo-built mech formatter must start");
    assert!(
        output_result.status.success(),
        "mech format failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output_result.stdout),
        String::from_utf8_lossy(&output_result.stderr),
    );
}

fn assert_served_rich_shell(server: &mut RunningServer, selectors: &[&str]) {
    let root = server.assert_route("/", 200, "text/html");
    let html = String::from_utf8_lossy(&root.body);
    shim_contract::assert_rich_shell(&html, selectors);
    assert!(
        html.contains("data-mech-document-status=\"loading\""),
        "rich shell did not retain its deterministic startup state"
    );
    assert!(
        html.contains("data-mech-source-url-key="),
        "served document does not identify its source route"
    );
}

#[test]
fn mech_serve_process_routes_work_for_sources_directories_and_projects() {
    let fixture = TestDirectory::new("real HTTP");

    {
        let source_dir = fixture.path().join("single source").join("nested");
        std::fs::create_dir_all(&source_dir).unwrap();
        let source = source_dir.join("bubble sort # %.mec");
        let raw = "answer := 42\n";
        std::fs::write(&source, raw).unwrap();
        let mut server = RunningServer::spawn(fixture.path(), &source, true);
        let encoded = "bubble%20sort%20%23%20%25";

        for route in [
            "/",
            "/index.html",
            &format!("/{encoded}.mec"),
            &format!("/{encoded}.html"),
            &format!("/{encoded}"),
        ] {
            let response = server.assert_route(route, 200, "text/html");
            if route.ends_with(".mec") && response.body == raw.as_bytes() {
                server.fail("public .mec route returned raw source");
            }
        }
        let root = server.assert_route("/", 200, "text/html");
        server.assert_body_contains("/", &root, "WasmDocument");
        server.assert_body_contains("/", &root, "/_mech/pkg/mech_wasm.js");
        if String::from_utf8_lossy(&root.body).contains("/_mech/project.js") {
            server.fail("standalone default shim unexpectedly required project.js");
        }
        let raw_route = format!("/source/{encoded}.mec");
        let raw_response = server.assert_route(&raw_route, 200, "text/x-mech");
        if raw_response.body != raw.as_bytes() {
            server.fail("raw source route did not return the source bytes");
        }
        server.assert_route(&format!("/code/{encoded}.mec"), 200, "text/plain");
        server.assert_route("/_mech/pkg/mech_wasm.js", 200, "application/javascript");
        let wasm = server.assert_route("/_mech/pkg/mech_wasm_bg.wasm", 200, "application/wasm");
        if !wasm.body.starts_with(b"\0asm") {
            server.fail("served WASM body did not begin with raw WebAssembly magic");
        }
        server.assert_route("/mech.mcfg", 404, "text/html");
    }

    {
        let serve_dir = fixture.path().join("directory workspace");
        let nested = serve_dir.join("café # %");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(serve_dir.join("alpha.mec"), "alpha := 1\n").unwrap();
        std::fs::write(nested.join("beta file.mec"), "beta := 2\n").unwrap();
        std::fs::write(serve_dir.join("style.css"), "body { color: teal; }\n").unwrap();
        std::fs::write(fixture.path().join("sibling.mec"), "secret := 9\n").unwrap();
        let mut server = RunningServer::spawn(fixture.path(), &serve_dir, true);
        let nested_key = "caf%C3%A9%20%23%20%25/beta%20file";

        let listing = server.assert_route("/", 200, "text/html");
        server.assert_body_contains("/", &listing, "/alpha.mec");
        server.assert_body_contains("/", &listing, &format!("/{nested_key}.mec"));
        for stem in ["alpha", nested_key] {
            server.assert_route(&format!("/{stem}.mec"), 200, "text/html");
            server.assert_route(&format!("/{stem}.html"), 200, "text/html");
            server.assert_route(&format!("/{stem}"), 200, "text/html");
            server.assert_route(&format!("/source/{stem}.mec"), 200, "text/x-mech");
            server.assert_route(&format!("/code/{stem}.mec"), 200, "text/plain");
        }
        server.assert_route("/style.css", 200, "text/css");
        server.assert_route("/sibling.mec", 404, "text/html");
    }

    {
        let project = fixture.path().join("configured project");
        std::fs::create_dir_all(project.join("lib")).unwrap();
        std::fs::write(project.join("main.mec"), "answer := 42\n").unwrap();
        std::fs::write(project.join("lib/support.mec"), "support := 1\n").unwrap();
        std::fs::write(
            project.join("index.html"),
            "<!doctype html><p>project root</p>",
        )
        .unwrap();
        std::fs::write(
            project.join("mech.mcfg"),
            "config := { hosts: [] serve: { paths: [\"lib\"] } run: { paths: [\"main.mec\"] grants: [] } }\n",
        )
        .unwrap();
        let mut server = RunningServer::spawn(fixture.path(), &project, false);

        let root = server.assert_route("/", 200, "text/html");
        server.assert_body_contains("/", &root, "project root");
        for stem in ["main", "lib/support"] {
            let formatted = server.assert_route(&format!("/{stem}.mec"), 200, "text/html");
            if formatted.body == std::fs::read(project.join(format!("{stem}.mec"))).unwrap() {
                server.fail("configured public .mec route returned raw source");
            }
            server.assert_route(&format!("/{stem}.html"), 200, "text/html");
            server.assert_route(&format!("/{stem}"), 200, "text/html");
            server.assert_route(&format!("/source/{stem}.mec"), 200, "text/x-mech");
        }
        server.assert_route("/mech.mcfg", 200, "text/x-mech");
        let manifest = server.assert_route("/_mech/project-sources.json", 200, "application/json");
        server.assert_body_contains(
            "/_mech/project-sources.json",
            &manifest,
            "source/lib/support.mec",
        );
        server.assert_route("/_mech/project.js", 200, "application/javascript");
        server.assert_route("/_mech/pkg/mech_wasm.js", 200, "application/javascript");
        let wasm = server.request_headers("/_mech/pkg/mech_wasm_bg.wasm");
        if wasm.status != 200
            || !wasm
                .headers
                .get("content-type")
                .is_some_and(|value| value.starts_with("application/wasm"))
        {
            server.fail("configured WASM route did not return a successful WASM response");
        }
    }
}

#[test]
fn mech_serve_custom_shim_renders_all_supported_slots() {
    let fixture = TestDirectory::new("all slots custom shim");
    let source = copy_slot_fixture(&fixture, "all slots # %.mec");
    let source_key = "all%20slots%20%23%20%25.mec";
    let shim = copy_resource(&fixture, &shim_fixture_path("all-slots.html"));
    let stylesheet = copy_resource(&fixture, &shim_fixture_path("all-slots.css"));
    let mut server = RunningServer::spawn_with_web_resources(
        fixture.path(),
        &source,
        true,
        Some(&shim),
        Some(&stylesheet),
    );

    let root = server.assert_route("/", 200, "text/html");
    let html = String::from_utf8_lossy(&root.body);
    shim_contract::assert_complete_slot_contract(&html, source_key);
    assert!(
        html.contains(&format!("data-mech-source-url-key=\"{source_key}\"")),
        "served page did not point its controller at the encoded source route"
    );
    server.assert_route(&format!("/code/{source_key}"), 200, "text/plain");
}

#[test]
fn mech_serve_default_shim_restores_rich_shell() {
    let fixture = TestDirectory::new("default rich shell");
    let source = copy_slot_fixture(&fixture, "default source.mec");
    let mut server = RunningServer::spawn(fixture.path(), &source, true);

    assert_served_rich_shell(
        &mut server,
        &[
            "id=\"header\"",
            "id=\"logo\"",
            "id=\"nav\"",
            "id=\"github\"",
            "class=\"mech-toc\"",
            "id=\"resizer\"",
            "id=\"toggle-repl\"",
        ],
    );
}

#[test]
fn mech_serve_blog_shim_restores_rich_shell() {
    let fixture = TestDirectory::new("blog rich shell");
    let source = copy_slot_fixture(&fixture, "blog source.mec");
    let shim = copy_resource(&fixture, &shipped_include_path("blog.html"));
    let stylesheet = copy_resource(&fixture, &shipped_include_path("blog.css"));
    let mut server = RunningServer::spawn_with_web_resources(
        fixture.path(),
        &source,
        true,
        Some(&shim),
        Some(&stylesheet),
    );

    assert_served_rich_shell(
        &mut server,
        &[
            "site-header",
            "contentShell",
            "articleIntro",
            "articleLayout",
            "hero-panel",
            "console-pane",
            "footer",
        ],
    );
}

#[test]
fn mech_serve_docs_shim_restores_rich_shell() {
    let fixture = TestDirectory::new("docs rich shell");
    let source = copy_slot_fixture(&fixture, "docs source.mec");
    let shim = copy_resource(&fixture, &shipped_include_path("docs.html"));
    let stylesheet = copy_resource(&fixture, &shipped_include_path("docs.css"));
    let mut server = RunningServer::spawn_with_web_resources(
        fixture.path(),
        &source,
        true,
        Some(&shim),
        Some(&stylesheet),
    );

    assert_served_rich_shell(
        &mut server,
        &[
            "site-header",
            "contentShell",
            "articleIntro",
            "articleLayout",
            "docs-content",
            "console-pane",
            "footer",
        ],
    );
}

#[cfg(all(feature = "formatter", has_file_wasm))]
fn assert_formatted_rich_page_is_served(shim: &str, stylesheet: &str) {
    let fixture = TestDirectory::new(&format!("formatted {shim}"));
    let source = copy_slot_fixture(&fixture, "all-slots.mec");
    let output = fixture.path().join("index.html");
    let shim = copy_resource(&fixture, &shipped_include_path(shim));
    let stylesheet = copy_resource(&fixture, &shipped_include_path(stylesheet));
    format_rich_fixture(&fixture, &source, &shim, &stylesheet, &output);

    let mut server = RunningServer::spawn_inputs_with_web_resources(
        fixture.path(),
        &[&output, &source],
        true,
        None,
        None,
    );
    let root = server.assert_route("/", 200, "text/html");
    let html = String::from_utf8_lossy(&root.body);
    shim_contract::assert_rich_shell(
        &html,
        &[
            "site-header",
            "contentShell",
            "articleLayout",
            "console-pane",
        ],
    );
    assert!(
        html.contains("data-mech-source-url-key=\"\""),
        "formatted static root should use its embedded document code"
    );
    assert!(
        html.contains("data-mech-document-code"),
        "formatted static root did not retain its executable document code"
    );
    assert!(!html.contains("{{CODE}}"));
    server.assert_route("/code/all-slots.mec", 200, "text/plain");
}

#[cfg(all(feature = "formatter", has_file_wasm))]
#[test]
fn formatted_blog_page_executes_when_served() {
    assert_formatted_rich_page_is_served("blog.html", "blog.css");
}

#[cfg(all(feature = "formatter", has_file_wasm))]
#[test]
fn formatted_docs_page_executes_when_served() {
    assert_formatted_rich_page_is_served("docs.html", "docs.css");
}

#[test]
fn mech_serve_interrupt_closes_keep_alive_server_once() {
    let fixture = TestDirectory::new("shutdown");
    let source = fixture.path().join("main.mec");
    std::fs::write(&source, "answer := 42\n").unwrap();
    let mut server = RunningServer::spawn(fixture.path(), &source, true);

    let mut keep_alive =
        TcpStream::connect(("127.0.0.1", server.port)).expect("keep-alive connection must open");
    write!(
        keep_alive,
        "GET / HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nConnection: keep-alive\r\n\r\n",
        server.port,
    )
    .unwrap();
    keep_alive.flush().unwrap();

    server.interrupt_and_wait(Duration::from_secs(5));
    drop(keep_alive);
    let logs = server.logs();
    assert_eq!(
        logs.matches("Server received shutdown signal.").count(),
        1,
        "{logs}",
    );
    assert!(
        logs.matches("Graceful shutdown timed out; forcing server close.")
            .count()
            <= 1,
        "{logs}",
    );
    assert_eq!(logs.matches("Closing server.").count(), 1, "{logs}");
}

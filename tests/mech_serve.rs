#![cfg(feature = "serve")]

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

static NEXT_TEMP_DIRECTORY: AtomicU64 = AtomicU64::new(0);

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
        let port = available_port();
        let stdout_path = current_dir.join(format!("serve-{port}.stdout.log"));
        let stderr_path = current_dir.join(format!("serve-{port}.stderr.log"));
        let stdout = File::create(&stdout_path).expect("server stdout log must be created");
        let stderr = File::create(&stderr_path).expect("server stderr log must be created");

        let mut command = Command::new(env!("CARGO_BIN_EXE_mech"));
        command.current_dir(current_dir);
        if no_config {
            command.arg("--no-config");
        }
        command
            .arg("serve")
            .arg(input)
            .arg("--address")
            .arg("127.0.0.1")
            .arg("--port")
            .arg(port.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr));
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
        let raw_route = format!("/source/{encoded}.mec");
        let raw_response = server.assert_route(&raw_route, 200, "text/x-mech");
        if raw_response.body != raw.as_bytes() {
            server.fail("raw source route did not return the source bytes");
        }
        server.assert_route(&format!("/code/{encoded}.mec"), 200, "text/plain");
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

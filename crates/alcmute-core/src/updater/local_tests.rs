use super::super::{API, RELEASES};
use super::*;
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};
use winreg::{enums::HKEY_CURRENT_USER, RegKey};

struct Server {
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = self.worker.take().unwrap().join();
    }
}

#[test]
#[ignore = "Run scripts/test-update-local.ps1 with isolated Inno Setup fixtures"]
fn local_update_e2e() {
    let root = fs::canonicalize(std::env::var("ALCMUTE_LOCAL_UPDATE_ROOT").unwrap()).unwrap();
    let id = std::env::var("ALCMUTE_LOCAL_UPDATE_ID").unwrap();
    assert_eq!(id.len(), 36);
    assert!(id.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-'));
    assert_eq!(fs::read_to_string(root.join("fixture.marker")).unwrap(), id);
    let install = root.join("install");
    let data = root.join("data");
    fs::create_dir_all(&install).unwrap();
    fs::create_dir_all(&data).unwrap();
    let settings = b"# preserved user settings\nautostart = false\n";
    fs::write(data.join("settings.toml"), settings).unwrap();
    fs::copy(root.join("old.exe"), install.join("alcmute.exe")).unwrap();
    assert_eq!(
        windows::executable_version(&install.join("alcmute.exe")).unwrap(),
        "0.0.1"
    );
    let registry = RegKey::predef(HKEY_CURRENT_USER)
        .create_subkey(format!("Software\\AlcMute\\Testing\\LocalUpdater\\{id}"))
        .unwrap()
        .0;
    registry.set_value("Version", &"0.0.1").unwrap();
    let installed = || {
        registry
            .get_value::<String, _>("Version")
            .map_err(|e| e.to_string())
    };
    let setup = fs::read(root.join("AlcMute-0.0.2-Setup.exe")).unwrap();
    let hash = format!("{:x}", Sha256::digest(&setup));
    let manifest = format!("{hash}  AlcMute-0.0.2-Setup.exe\n");
    let metadata = serde_json::json!({"tag_name":"v0.0.2", "html_url":format!("{RELEASES}/tag/v0.0.2"), "draft":false,"prerelease":false,"assets":[
        {"name":"AlcMute-0.0.2-Setup.exe","browser_download_url":format!("{RELEASES}/download/v0.0.2/AlcMute-0.0.2-Setup.exe"),"size":setup.len(),"state":"uploaded","digest":format!("sha256:{hash}")},
        {"name":"SHA256SUMS.txt","browser_download_url":format!("{RELEASES}/download/v0.0.2/SHA256SUMS.txt"),"size":manifest.len(),"state":"uploaded"}
    ]});
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let client = Client {
        local: Some(format!("http://{}", listener.local_addr().unwrap())),
    };
    let stop = Arc::new(AtomicBool::new(false));
    let mode = Arc::new(AtomicU8::new(0));
    let worker_stop = stop.clone();
    let worker_mode = mode.clone();
    let worker = thread::spawn(move || {
        while !worker_stop.load(Ordering::Relaxed) {
            let (mut stream, _) = match listener.accept() {
                Ok(v) => v,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(5));
                    continue;
                }
                Err(e) => panic!("{e}"),
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = [0; 8192];
            let count = match stream.read(&mut request) {
                Ok(0) | Err(_) => continue,
                Ok(count) => count,
            };
            let request = String::from_utf8_lossy(&request[..count]);
            let path = request.split_whitespace().nth(1).unwrap();
            let m = worker_mode.load(Ordering::Relaxed);
            let (status, body) = if path.starts_with("/api/") {
                if m == 1 {
                    (404, Vec::new())
                } else if m == 2 {
                    (500, Vec::new())
                } else {
                    let mut json = metadata.clone();
                    if m == 3 {
                        json["assets"][0]["browser_download_url"] =
                            "https://invalid.example/setup.exe".into();
                    }
                    (200, serde_json::to_vec(&json).unwrap())
                }
            } else if path.ends_with("/SHA256SUMS.txt") {
                (200, manifest.as_bytes().to_vec())
            } else if path.ends_with("/AlcMute-0.0.2-Setup.exe") {
                let mut bytes = setup.clone();
                if m == 4 {
                    bytes[128] ^= 1;
                }
                (200, bytes)
            } else {
                (404, Vec::new())
            };
            if write!(
                stream,
                "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .is_ok()
            {
                let _ = stream.write_all(&body);
            }
        }
    });
    let _server = Server {
        stop,
        worker: Some(worker),
    };
    let latest = format!("{API}/latest");
    mode.store(1, Ordering::Relaxed);
    assert!(client.fetch(&latest).unwrap().is_none());
    mode.store(2, Ordering::Relaxed);
    assert!(client.fetch(&latest).is_err());
    mode.store(3, Ordering::Relaxed);
    assert!(client
        .stage(
            &client.fetch(&latest).unwrap().unwrap(),
            &root.join("invalid")
        )
        .is_err());
    mode.store(4, Ordering::Relaxed);
    assert!(client
        .stage(
            &client.fetch(&latest).unwrap().unwrap(),
            &root.join("corrupt")
        )
        .is_err());
    mode.store(0, Ordering::Relaxed);
    let release = client.fetch(&latest).unwrap().unwrap();
    assert!(release.newer_than("0.0.1").unwrap());
    assert!(!release.newer_than("0.0.2").unwrap());
    let version = Version::parse("0.0.2").unwrap();
    for fail in [true, false] {
        let stage = root.join(if fail { "failure" } else { "success" });
        let setup = client.stage(&release, &stage).unwrap();
        let exit = root.join(if fail { "exit-failure" } else { "exit-success" });
        let mut child = Command::new(install.join("alcmute.exe"))
            .arg("--wait")
            .arg(&exit)
            .spawn()
            .unwrap();
        fs::write(&exit, "exit").unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while child.try_wait().unwrap().is_none() {
            if Instant::now() > deadline {
                child.kill().unwrap();
                panic!("Fixture did not exit");
            }
            thread::sleep(Duration::from_millis(10));
        }
        // Retain the exited process handle: reproduces the original os error 31.
        if fail {
            std::env::set_var("ALCMUTE_FIXTURE_FAIL", "1");
        }
        let result = perform(
            &client,
            &setup,
            &version,
            child.id(),
            &install,
            &data,
            &installed,
        );
        std::env::remove_var("ALCMUTE_FIXTURE_FAIL");
        assert_eq!(fs::read(data.join("settings.toml")).unwrap(), settings);
        assert_eq!(
            fs::read(stage.join("previous-alcmute.exe")).unwrap(),
            fs::read(root.join("old.exe")).unwrap()
        );
        if fail {
            assert!(result.is_err(), "{result:?}");
            assert_eq!(installed().unwrap(), "0.0.1");
            assert_eq!(
                windows::executable_version(&install.join("alcmute.exe")).unwrap(),
                "0.0.1"
            );
            assert!(!data.join("restarted-version.txt").exists());
        } else {
            result.unwrap();
            let deadline = Instant::now() + Duration::from_secs(10);
            while !data.join("restarted-version.txt").exists() {
                assert!(Instant::now() < deadline, "Updated fixture did not restart");
                thread::sleep(Duration::from_millis(10));
            }
            assert_eq!(
                fs::read_to_string(data.join("restarted-version.txt")).unwrap(),
                "0.0.2"
            );
            assert_eq!(installed().unwrap(), "0.0.2");
        }
    }
    fs::write(root.join("report.json"), serde_json::to_vec_pretty(&serde_json::json!({"passed":true,"cases":["API 404","API 500","untrusted asset","corrupt download","same-version rejection","installer failure preserves old executable/settings","exited parent handle","install verification","background restart","settings preservation"],"scope":"Loopback transport, shared production transaction, isolated Inno fixture; no real AlcMute installation or public Release"})).unwrap()).unwrap();
}

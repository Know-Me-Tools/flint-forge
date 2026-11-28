//! Operator commands exercised through the installed binary interface.
use std::process::{Command, Output};

use base64::Engine as _;
use serde_json::Value;
use sha2::{Digest, Sha256};
use tempfile::tempdir;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

fn forge() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_forge"));
    cmd.env_remove("FORGE_CONTAINER")
        .env_remove("FORGE_CONTAINERIZED")
        .env_remove("FLINT_JWT_SECRET")
        .env("RUST_LOG", "off");
    cmd
}

fn stdout(result: &Output) -> String {
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    String::from_utf8(result.stdout.clone()).unwrap()
}

#[test]
fn version_and_smoke_token_have_the_requested_contract() {
    assert_eq!(
        stdout(&forge().arg("version").output().unwrap()).trim(),
        env!("CARGO_PKG_VERSION")
    );
    let key = "synthetic-fixture-key-for-testing-commands-only";
    let result = forge()
        .args([
            "token",
            "mint",
            "--subject",
            "fixture",
            "--role",
            "authenticated",
            "--expiry-seconds",
            "75",
        ])
        .env("FLINT_JWT_SECRET", key)
        .output()
        .unwrap();
    let claims = jsonwebtoken::decode::<Value>(
        stdout(&result).trim(),
        &jsonwebtoken::DecodingKey::from_secret(key.as_bytes()),
        &jsonwebtoken::Validation::default(),
    )
    .unwrap()
    .claims;
    assert_eq!(claims["sub"], "fixture");
    assert_eq!(claims["role"], "authenticated");
    assert_eq!(
        claims["exp"].as_i64().unwrap() - claims["iat"].as_i64().unwrap(),
        75
    );
    let missing = forge().args(["token", "mint"]).output().unwrap();
    assert!(!missing.status.success());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn registers_a_manifest_and_handles_admin_errors() {
    let server = MockServer::start().await;
    let dir = tempdir().unwrap();
    let component = dir.path().join("sample.wasm");
    let bytes = b"synthetic component bytes";
    std::fs::write(&component, bytes).unwrap();
    let endpoint = server.uri();
    let register = |caps: &str| {
        forge()
            .args(["fn", "register"])
            .arg(&component)
            .args(["--admin-url", &endpoint, "--capabilities", caps])
            .output()
            .unwrap()
    };
    Mock::given(method("POST"))
        .and(path("/admin/functions"))
        .respond_with(ResponseTemplate::new(201).set_body_string("registered"))
        .expect(1)
        .mount(&server)
        .await;
    assert_eq!(
        stdout(&register("db,llm,kv,identity,secrets,http")).trim(),
        "registered"
    );
    let requests = server.received_requests().await.unwrap();
    let data: Value = requests[0].body_json().unwrap();
    assert_eq!(data["name"], "sample");
    assert_eq!(
        data["wasm_base64"],
        base64::engine::general_purpose::STANDARD.encode(bytes)
    );
    assert_eq!(
        data["manifest"]["content_digest"],
        format!("{:x}", Sha256::digest(bytes))
    );
    assert_eq!(
        data["manifest"]["capabilities"].as_array().unwrap().len(),
        6
    );
    let before = data["manifest"]["not_before"]
        .as_str()
        .unwrap()
        .parse::<jiff::Timestamp>()
        .unwrap();
    let after = data["manifest"]["not_after"]
        .as_str()
        .unwrap()
        .parse::<jiff::Timestamp>()
        .unwrap();
    assert!((after.as_second() - before.as_second() - 365 * 24 * 3600).abs() <= 1);
    assert!(!register("unknown").status.success());
    server.reset().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(400))
        .mount(&server)
        .await;
    assert!(!register("kv").status.success());
}

#[cfg(unix)]
#[test]
fn container_registration_passes_the_working_directory_and_component_path() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempdir().unwrap();
    let docker = dir.path().join("docker");
    std::fs::write(&docker, "#!/bin/sh\nprintf '%s\\n' \"$@\"\n").unwrap();
    std::fs::set_permissions(&docker, std::fs::Permissions::from_mode(0o755)).unwrap();
    let path = dir.path().join("fixtures/component.wasm");
    let result = forge()
        .current_dir(dir.path())
        .env("PATH", dir.path())
        .args(["--container", "fn", "register"])
        .arg(&path)
        .output()
        .unwrap();
    let args = stdout(&result);
    let args = args.lines().collect::<Vec<_>>();
    assert!(args.starts_with(&["run", "--rm", "-i", "-v"]));
    assert!(!args.contains(&"--container"));
    assert!(args.ends_with(&[
        "flint-forge-cli",
        "forge",
        "fn",
        "register",
        path.to_str().unwrap()
    ]));
}

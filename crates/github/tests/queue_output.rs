use std::{fs, process::Command};

#[test]
fn queue_stdout_is_json_even_with_tracing_enabled() {
    let directory =
        std::env::temp_dir().join(format!("review-radar-queue-output-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    let database = directory.join("capture.sqlite3");
    let connection = rusqlite::Connection::open(&database).unwrap();
    connection
        .execute_batch(include_str!("../src/schema.sql"))
        .unwrap();
    connection.execute(
        "INSERT INTO captures VALUES (1, '2026-09-22T00:00:00Z', 'example', 10, 4, 20, 1, 1, 4999, '2026-09-22T01:00:00Z')",
        [],
    ).unwrap();
    drop(connection);
    let output = Command::new(env!("CARGO_BIN_EXE_review-radar-queue"))
        .args(["--database", database.to_str().unwrap(), "--state-database"])
        .arg(directory.join("state.sqlite3"))
        .env("RUST_LOG", "info")
        .output()
        .unwrap();
    fs::remove_dir_all(directory).unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let response: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(response["captureId"], 1);
    assert!(response["pullRequests"].as_array().unwrap().is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("projection completed"));
}

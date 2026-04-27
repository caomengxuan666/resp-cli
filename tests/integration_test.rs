use std::process::Command;

#[test]
fn test_basic_connection() {
    // Test basic connection to Redis server
    let output = Command::new("cargo")
        .arg("run")
        .arg("--")
        .arg("PING")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("PONG"),
        "Expected PONG response, got: {}",
        stdout
    );
}

#[test]
fn test_set_get_command() {
    // Test SET and GET commands
    let set_output = Command::new("cargo")
        .arg("run")
        .arg("--")
        .arg("SET")
        .arg("test_key")
        .arg("test_value")
        .output()
        .expect("Failed to execute SET command");

    let set_stdout = String::from_utf8_lossy(&set_output.stdout);
    assert!(
        set_stdout.contains("OK"),
        "Expected OK response for SET, got: {}",
        set_stdout
    );

    let get_output = Command::new("cargo")
        .arg("run")
        .arg("--")
        .arg("GET")
        .arg("test_key")
        .output()
        .expect("Failed to execute GET command");

    let get_stdout = String::from_utf8_lossy(&get_output.stdout);
    assert!(
        get_stdout.contains("test_value"),
        "Expected test_value response for GET, got: {}",
        get_stdout
    );
}

#[test]
fn test_config_loading() {
    // Test that config loading works
    let output = Command::new("cargo")
        .arg("run")
        .arg("--")
        .arg("CONFIG")
        .arg("GET")
        .arg("*maxmemory*")
        .output()
        .expect("Failed to execute CONFIG command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.is_empty(),
        "Expected non-empty response for CONFIG GET"
    );
}

#[test]
fn test_error_handling() {
    // Test error handling for unknown command
    let output = Command::new("cargo")
        .arg("run")
        .arg("--")
        .arg("UNKNOWN_COMMAND")
        .output()
        .expect("Failed to execute unknown command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Error"),
        "Expected error response for unknown command, got: {}",
        stdout
    );
}

#[test]
fn test_alias_command() {
    // Test ALIAS command
    let output = Command::new("cargo")
        .arg("run")
        .arg("--")
        .arg("ALIAS")
        .output()
        .expect("Failed to execute ALIAS command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.is_empty(),
        "Expected non-empty response for ALIAS command"
    );
}

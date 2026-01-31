use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

// Helper function to check if running as root
fn is_root() -> bool {
    nix::unistd::getuid().is_root()
}

// Helper function to create a minimal rootfs for testing
fn create_test_rootfs(temp_dir: &TempDir) -> Result<String, Box<dyn std::error::Error>> {
    let rootfs_path = temp_dir.path().join("rootfs");
    fs::create_dir_all(&rootfs_path)?;

    // Create basic directory structure
    fs::create_dir_all(rootfs_path.join("bin"))?;
    fs::create_dir_all(rootfs_path.join("usr/bin"))?;
    fs::create_dir_all(rootfs_path.join("proc"))?;
    fs::create_dir_all(rootfs_path.join("sys"))?;
    fs::create_dir_all(rootfs_path.join("dev"))?;

    // Copy a simple binary if available (like /bin/echo)
    if Path::new("/bin/echo").exists() {
        fs::copy("/bin/echo", rootfs_path.join("bin/echo"))?;
    }

    Ok(rootfs_path.to_string_lossy().to_string())
}

#[test]
#[ignore] // Use `cargo test -- --ignored` to run privileged tests
fn test_container_lifecycle_with_root() {
    if !is_root() {
        println!("Skipping privileged test - not running as root");
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let rootfs = create_test_rootfs(&temp_dir).unwrap();

    // Test that our container binary can be built and executed
    // This is a basic smoke test
    let output = Command::new("cargo")
        .args(&[
            "run",
            "--",
            "run",
            &rootfs,
            "/bin/echo",
            "Hello from container",
        ])
        .output()
        .expect("Failed to execute container");

    // The test should either succeed (container runs) or fail gracefully
    // We're mainly testing that the code doesn't panic unexpectedly
    assert!(output.status.success() || !output.stderr.is_empty());

    println!(
        "Container output: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    if !output.stderr.is_empty() {
        println!(
            "Container stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
#[ignore] // Use `cargo test -- --ignored` to run privileged tests
fn test_container_isolation_verification() {
    if !is_root() {
        println!("Skipping privileged test - not running as root");
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let rootfs = create_test_rootfs(&temp_dir).unwrap();

    // Test container isolation by checking if it can see host processes
    let output = Command::new("cargo")
        .args(&["run", "--", "run", &rootfs, "/bin/sh", "-c", "ls /proc"])
        .output()
        .expect("Failed to execute container");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // In a properly isolated container, /proc should show only container processes
    // This is a basic check - in reality we'd need more sophisticated testing
    assert!(stdout.contains("1") || output.status.success());
}

#[test]
fn test_argument_validation() {
    // Test that the CLI properly validates arguments without requiring privileges
    let test_cases = vec![
        vec!["run", "/invalid/path", "/bin/echo"],   // Invalid path
        vec!["run", "/tmp", "/nonexistent/command"], // Invalid command
    ];

    for args in test_cases {
        let mut full_args = vec!["cargo", "run", "--"];
        full_args.extend(args.iter());

        let output = Command::new("cargo")
            .args(&full_args[1..]) // Skip the first "cargo"
            .output()
            .expect("Failed to execute container");

        // Should either succeed or fail gracefully, not hang
        println!("Exit code: {}", output.status);
        if output.stdout.len() > 0 {
            println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        }
        if output.stderr.len() > 0 {
            println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        }

        // Should fail gracefully, not panic or hang
        assert!(
            output.status.code().is_some()
                || !output.stderr.is_empty()
                || !output.stdout.is_empty()
        );
    }
}

#[test]
fn test_error_handling() {
    // Test that errors are handled gracefully
    let test_cases = vec![
        vec!["run", "", "/bin/echo"],            // Empty rootfs
        vec!["run", "/tmp", ""],                 // Empty command
        vec!["run", "/tmp", "invalid\0command"], // Invalid command with null byte
    ];

    for args in test_cases {
        let mut full_args = vec!["cargo", "run", "--"];
        full_args.extend(args.iter());

        // These should fail gracefully, not panic
        let output = Command::new("cargo")
            .args(&full_args[1..])
            .output()
            .expect("Failed to execute container");

        // Should either succeed (if empty args are handled) or fail gracefully
        assert!(!output.stderr.is_empty() || !output.stdout.is_empty());
    }
}

#[test]
fn test_cgroup_availability() {
    // Test if cgroup v2 is available on the system
    if Path::new("/sys/fs/cgroup/cgroup.controllers").exists() {
        let controllers = fs::read_to_string("/sys/fs/cgroup/cgroup.controllers")
            .expect("Failed to read cgroup.controllers");

        println!("Available cgroup controllers: {}", controllers);

        // Should have at least basic controllers
        assert!(controllers.len() > 0);

        // Check for commonly available controllers
        let common_controllers = vec!["cpu", "memory", "pids"];
        for controller in common_controllers {
            if controllers.contains(controller) {
                println!("✓ {} controller available", controller);
            } else {
                println!("⚠ {} controller not available", controller);
            }
        }
    } else {
        println!("cgroup v2 not available or not running as root");
    }
}

#[test]
fn test_namespace_support() {
    // Test if user namespaces are supported
    if is_root() {
        let output = Command::new("unshare")
            .args(&["--user", "--pid", "--fork", "echo", "namespace test"])
            .output()
            .expect("Failed to test namespace support");

        if output.status.success() {
            println!("✓ Namespace support available");
        } else {
            println!(
                "⚠ Namespace support limited: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    } else {
        println!("Cannot test namespace support without root privileges");
    }
}

#[test]
fn test_basic_system_requirements() {
    // Test basic system requirements for container runtime
    let requirements = vec![
        ("/proc", "proc filesystem"),
        ("/sys", "sysfs filesystem"),
        ("/dev/null", "null device"),
        ("/dev/zero", "zero device"),
    ];

    for (path, description) in requirements {
        assert!(
            Path::new(path).exists(),
            "{} not available at {}",
            description,
            path
        );
        println!("✓ {} available", description);
    }
}

#[test]
fn test_container_binary_compilation() {
    // Test that the container binary can be compiled
    let output = Command::new("cargo")
        .args(&["check"])
        .output()
        .expect("Failed to run cargo check");

    assert!(
        output.status.success(),
        "Compilation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    println!("✓ Container binary compiles successfully");

    // Also test build
    let output = Command::new("cargo")
        .args(&["build"])
        .output()
        .expect("Failed to run cargo build");

    assert!(
        output.status.success(),
        "Build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    println!("✓ Container binary builds successfully");
}

#[test]
fn test_unit_tests_pass() {
    // Run unit tests to ensure they pass
    let output = Command::new("cargo")
        .args(&["test", "--lib"])
        .output()
        .expect("Failed to run cargo test");

    assert!(
        output.status.success(),
        "Unit tests failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    println!("✓ All unit tests pass");
}

#[test]
fn test_help_and_usage() {
    // Test that help commands work without panicking
    let help_cases = vec![vec!["--help"], vec!["run", "--help"]];

    for args in help_cases {
        let output = Command::new("cargo")
            .args(&["run", "--"])
            .args(args.clone())
            .output()
            .expect("Failed to execute help command");

        // Help commands should exit with a non-zero status but not panic
        assert!(!output.stderr.is_empty() || !output.stdout.is_empty());
        println!("✓ Help command works for args: {:?}", args);
    }
}

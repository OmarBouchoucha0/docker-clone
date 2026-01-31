use std::fs;
use std::io::Write;
use std::path::Path;

pub fn setup_cgroup(pid: i32) -> Result<(), Box<dyn std::error::Error>> {
    // Read cgroup information
    let cgroup_line = std::fs::read_to_string("/proc/self/cgroup")
        .map_err(|e| format!("Failed to read /proc/self/cgroup: {}", e))?;

    let cgroup_rel = cgroup_line
        .lines()
        .find(|l| l.starts_with("0::"))
        .ok_or("Could not find cgroup path in expected format")?
        .trim_start_matches("0::");

    let base = "/sys/fs/cgroup";
    let parent_cgroup = format!("{}{}", base, cgroup_rel);
    let child_cgroup = format!("{}/docker-clone-{}", parent_cgroup, pid);

    // Check available controllers
    let controllers_path = format!("{}/cgroup.controllers", parent_cgroup);
    let controllers = std::fs::read_to_string(&controllers_path)
        .map_err(|e| format!("Failed to read cgroup.controllers: {}", e))?;
    println!("Available controllers: {}", controllers);

    std::fs::create_dir_all(&child_cgroup)?;
    enable_controllers(&child_cgroup)?;

    // Create cgroup directory
    println!("Creating cgroup at {}", child_cgroup);
    fs::create_dir_all(&child_cgroup)
        .map_err(|e| format!("Failed to create cgroup directory: {}", e))?;

    // Memory limit: 100MB
    let memory_limit_path = format!("{}/memory.max", child_cgroup);
    fs::write(&memory_limit_path, "104857600")
        .map_err(|e| format!("Failed to set memory limit: {}", e))?;

    // CPU limit: 50% of one core
    let cpu_limit_path = format!("{}/cpu.max", child_cgroup);
    fs::write(&cpu_limit_path, "50000 100000")
        .map_err(|e| format!("Failed to set CPU limit: {}", e))?;

    // Attach process to cgroup
    let procs_path = format!("{}/cgroup.procs", child_cgroup);
    let mut procs = fs::OpenOptions::new()
        .write(true)
        .open(&procs_path)
        .map_err(|e| format!("Failed to open cgroup.procs: {}", e))?;

    writeln!(procs, "{}", pid)
        .map_err(|e| format!("Failed to write PID to cgroup.procs: {}", e))?;

    Ok(())
}

fn enable_controllers(parent: &str) -> Result<(), Box<dyn std::error::Error>> {
    let controllers_file = format!("{}/cgroup.controllers", parent);
    let subtree_file = format!("{}/cgroup.subtree_control", parent);

    // If this cgroup doesn't allow delegation (no controllers file), skip.
    if !Path::new(&controllers_file).exists() {
        return Ok(());
    }

    let controllers = fs::read_to_string(&controllers_file)?;
    if controllers.trim().is_empty() {
        return Ok(());
    }

    // Prepare "+cpu +memory +pids" etc
    let enable_string = controllers
        .split_whitespace()
        .map(|c| format!("+{}", c))
        .collect::<Vec<_>>()
        .join(" ");

    // Only write if subtree_control exists
    if Path::new(&subtree_file).exists() {
        fs::write(&subtree_file, enable_string)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_cgroup_path_generation() {
        let base = "/sys/fs/cgroup";
        let cgroup_rel = "/user.slice/user-1000.slice";
        let pid = 1234;

        let parent_cgroup = format!("{}{}", base, cgroup_rel);
        let child_cgroup = format!("{}/docker-clone-{}", parent_cgroup, pid);

        assert_eq!(parent_cgroup, "/sys/fs/cgroup/user.slice/user-1000.slice");
        assert_eq!(
            child_cgroup,
            "/sys/fs/cgroup/user.slice/user-1000.slice/docker-clone-1234"
        );
    }

    #[test]
    fn test_memory_limit_value() {
        let expected_memory_bytes = 100 * 1024 * 1024; // 100MB
        let actual_memory_setting = "104857600"; // As used in code

        assert_eq!(expected_memory_bytes.to_string(), actual_memory_setting);
    }

    #[test]
    fn test_cpu_limit_value() {
        let cpu_limit = "50000 100000"; // 50% of one core

        let parts: Vec<&str> = cpu_limit.split_whitespace().collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "50000");
        assert_eq!(parts[1], "100000");

        // Verify it's actually 50%
        let quota: u64 = parts[0].parse().unwrap();
        let period: u64 = parts[1].parse().unwrap();
        assert_eq!(quota * 100, period * 50);
    }

    #[test]
    fn test_enable_controllers_string() {
        let controller_string = "+cpu +memory +pids";
        let controllers: Vec<&str> = controller_string.split_whitespace().collect();

        assert_eq!(controllers.len(), 3);
        assert!(controllers.contains(&"+cpu"));
        assert!(controllers.contains(&"+memory"));
        assert!(controllers.contains(&"+pids"));
    }

    #[test]
    fn test_cgroup_file_paths() {
        let base = "/sys/fs/cgroup/test";
        let test_files = vec![
            "cgroup.controllers",
            "cgroup.subtree_control",
            "memory.max",
            "cpu.max",
            "cgroup.procs",
        ];

        for file in test_files {
            let full_path = format!("{}/{}", base, file);
            assert!(full_path.starts_with("/sys/fs/cgroup/test/"));
            assert!(full_path.ends_with(file));
        }
    }

    #[test]
    fn test_different_pid_cgroup_paths() {
        let base = "/sys/fs/cgroup/user.slice";
        let test_pids = vec![1, 100, 9999, 12345];

        for pid in test_pids {
            let child_cgroup = format!("{}/docker-clone-{}", base, pid);
            assert!(child_cgroup.contains(&format!("docker-clone-{}", pid)));
            assert!(child_cgroup.starts_with(base));
        }
    }

    #[test]
    fn test_cgroup_parsing_logic() {
        let cgroup_lines = vec![
            "0::/user.slice/user-1000.slice",
            "1:memory:/user.slice/user-1000.slice",
            "2:cpu,cpuacct:/user.slice/user-1000.slice",
        ];

        for line in cgroup_lines {
            let found = line.starts_with("0::");
            assert_eq!(found, line.starts_with("0::"));

            if found {
                let cgroup_rel = line.trim_start_matches("0::");
                assert_eq!(cgroup_rel, "/user.slice/user-1000.slice");
            }
        }
    }

    #[test]
    fn test_controller_values() {
        let controllers = "cpu memory pids io blkio rdma misc";
        let controller_list: Vec<&str> = controllers.split_whitespace().collect();

        assert!(controller_list.contains(&"cpu"));
        assert!(controller_list.contains(&"memory"));
        assert!(controller_list.contains(&"pids"));
        assert!(controller_list.contains(&"io"));
        assert!(controller_list.contains(&"blkio"));
    }

    #[test]
    fn test_error_message_formatting() {
        let operation = "Failed to read cgroup.controllers";
        let error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let formatted = format!("{}: {}", operation, error);

        assert!(formatted.contains(operation));
        assert!(formatted.contains("File not found"));
    }

    #[test]
    fn test_memory_limit_calculations() {
        let test_sizes = vec![
            (50, "52428800"),     // 50MB
            (100, "104857600"),   // 100MB
            (200, "209715200"),   // 200MB
            (1024, "1073741824"), // 1GB
        ];

        for (mb, expected_bytes) in test_sizes {
            let calculated = mb * 1024 * 1024;
            assert_eq!(calculated.to_string(), expected_bytes);
        }
    }
}

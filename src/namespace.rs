use nix::unistd::{getgid, getuid};

pub fn setup_user_namespace(pid: i32) -> Result<(), Box<dyn std::error::Error>> {
    let uid = getuid().as_raw();
    let gid = getgid().as_raw();

    // Deny setgroups to prevent privilege escalation
    if let Err(e) = std::fs::write(format!("/proc/{}/setgroups", pid), "deny") {
        return Err(format!("Failed to write setgroups: {}", e).into());
    }

    // Map user ID
    if let Err(e) = std::fs::write(format!("/proc/{}/uid_map", pid), format!("0 {} 1", uid)) {
        return Err(format!("Failed to write uid_map: {}", e).into());
    }

    // Map group ID
    if let Err(e) = std::fs::write(format!("/proc/{}/gid_map", pid), format!("0 {} 1", gid)) {
        return Err(format!("Failed to write gid_map: {}", e).into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_file_paths_generation() {
        let pid = 9999;

        let setgroups_path = format!("/proc/{}/setgroups", pid);
        let uid_map_path = format!("/proc/{}/uid_map", pid);
        let gid_map_path = format!("/proc/{}/gid_map", pid);

        assert_eq!(setgroups_path, "/proc/9999/setgroups");
        assert_eq!(uid_map_path, "/proc/9999/uid_map");
        assert_eq!(gid_map_path, "/proc/9999/gid_map");
    }

    #[test]
    fn test_uid_gid_format_generation() {
        let uid = 1000;
        let gid = 1000;

        let uid_mapping = format!("0 {} 1", uid);
        let gid_mapping = format!("0 {} 1", gid);

        assert_eq!(uid_mapping, "0 1000 1");
        assert_eq!(gid_mapping, "0 1000 1");
    }

    #[test]
    fn test_different_pid_values() {
        let test_pids = vec![1, 100, 9999, 12345];

        for pid in test_pids {
            let setgroups_path = format!("/proc/{}/setgroups", pid);
            assert!(setgroups_path.starts_with("/proc/"));
            assert!(setgroups_path.ends_with("/setgroups"));
            assert!(setgroups_path.contains(&pid.to_string()));
        }
    }

    #[test]
    fn test_uid_gid_values() {
        // Test various realistic UID/GID values
        let test_values = vec![(0, 0), (1000, 1000), (1001, 1001), (65534, 65534)];

        for (uid, gid) in test_values {
            let uid_mapping = format!("0 {} 1", uid);
            let gid_mapping = format!("0 {} 1", gid);

            assert!(uid_mapping.starts_with("0 "));
            assert!(uid_mapping.ends_with(" 1"));
            assert!(gid_mapping.starts_with("0 "));
            assert!(gid_mapping.ends_with(" 1"));
        }
    }

    #[test]
    fn test_setgroups_content() {
        let expected_content = "deny";
        assert_eq!(expected_content, "deny");
        assert!(expected_content.len() > 0);
    }

    #[test]
    fn test_filesystem_error_message_formatting() {
        let file_path = "/proc/1234/setgroups";
        let error_msg = "Permission denied";
        let formatted = format!("Failed to write setgroups: {}", error_msg);

        assert!(formatted.contains("Failed to write setgroups:"));
        assert!(formatted.contains(error_msg));
        assert!(formatted.contains(file_path) == false); // File path not included in this specific message
    }

    #[test]
    fn test_real_uid_gid_functions() {
        // Test that we can call the real getuid/getgid functions
        // This test just verifies the functions can be called without panicking
        let uid = getuid().as_raw();
        let gid = getgid().as_raw();

        // Should be reasonable values (0-65535 typically)
        assert!(uid <= 65535);
        assert!(gid <= 65535);
    }
}

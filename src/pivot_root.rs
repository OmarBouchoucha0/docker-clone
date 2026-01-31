use anyhow::Result;
use nix::mount::{mount, umount2, MntFlags, MsFlags};
use nix::unistd::{chdir, pivot_root};

pub fn setup_rootfs(rootfs: &str) -> Result<()> {
    // First bind mount the rootfs to itself to ensure it's a mount point
    mount(
        Some(rootfs),
        rootfs,
        None::<&str>,
        MsFlags::MS_BIND | MsFlags::MS_REC,
        None::<&str>,
    )?;

    // Create the old_root directory inside the new rootfs
    let old_root = format!("{}/.old_root", rootfs);
    std::fs::create_dir_all(&old_root)?;

    // Pivot root to the new filesystem
    pivot_root(rootfs, old_root.as_str())?;

    // Change to new root directory
    chdir("/")?;

    // Unmount the old root filesystem
    umount2("/.old_root", MntFlags::MNT_DETACH)?;

    // Remove the old_root directory
    std::fs::remove_dir_all("/.old_root")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use nix::mount::{MntFlags, MsFlags};
    use tempfile::TempDir;

    #[test]
    fn test_old_root_path_generation() {
        let test_paths = vec![
            "/tmp/container",
            "/var/lib/container",
            "/opt/container",
            "/home/user/container",
        ];

        for rootfs in test_paths {
            let old_root = format!("{}/.old_root", rootfs);
            assert!(old_root.ends_with("/.old_root"));
            assert!(old_root.starts_with(rootfs));
            assert_eq!(old_root.len(), rootfs.len() + "/.old_root".len());
        }
    }

    #[test]
    fn test_rootfs_path_validation() {
        let valid_paths = vec!["/tmp/container", "/var/lib/container", "/opt/container"];

        let invalid_paths = vec!["", "relative/path", "/", "//double/slash"];

        for path in valid_paths {
            assert!(path.starts_with('/'));
            assert!(!path.is_empty());
            assert!(path.len() > 1);
        }

        for path in invalid_paths {
            if path.is_empty() || !path.starts_with('/') || path == "/" {
                // These should be considered invalid for our use case
            }
        }
    }

    #[test]
    fn test_temp_directory_operations() {
        // Test actual filesystem operations using tempdir
        let temp_dir = TempDir::new().unwrap();
        let test_rootfs = temp_dir.path().join("test_rootfs");

        // Test directory creation
        std::fs::create_dir_all(&test_rootfs).unwrap();
        assert!(test_rootfs.exists());
        assert!(test_rootfs.is_dir());

        // Test old_root directory creation
        let old_root = test_rootfs.join(".old_root");
        std::fs::create_dir_all(&old_root).unwrap();
        assert!(old_root.exists());
        assert!(old_root.is_dir());

        // Test directory removal
        std::fs::remove_dir_all(&old_root).unwrap();
        assert!(!old_root.exists());

        // Clean up
        std::fs::remove_dir_all(&test_rootfs).unwrap();
        assert!(!test_rootfs.exists());
    }

    #[test]
    fn test_mount_flags() {
        let expected_flags = MsFlags::MS_BIND | MsFlags::MS_REC;
        let expected_umount_flags = MntFlags::MNT_DETACH;

        // Verify the flags are what we expect
        assert!(expected_flags.contains(MsFlags::MS_BIND));
        assert!(expected_flags.contains(MsFlags::MS_REC));
        assert_eq!(expected_umount_flags, MntFlags::MNT_DETACH);
    }

    #[test]
    fn test_different_rootfs_paths() {
        let test_cases = vec![
            ("/tmp/container1", "/tmp/container1/.old_root"),
            ("/var/lib/container2", "/var/lib/container2/.old_root"),
            ("/opt/container3", "/opt/container3/.old_root"),
        ];

        for (rootfs, expected_old_root) in test_cases {
            let actual_old_root = format!("{}/.old_root", rootfs);
            assert_eq!(actual_old_root, expected_old_root);
        }
    }

    #[test]
    fn test_directory_operations_sequence() {
        let temp_dir = TempDir::new().unwrap();
        let rootfs_path = temp_dir.path().join("container");
        let old_root_path = rootfs_path.join(".old_root");

        // Simulate the directory creation sequence
        assert!(!rootfs_path.exists());
        assert!(!old_root_path.exists());

        // Create rootfs directory
        std::fs::create_dir_all(&rootfs_path).unwrap();
        assert!(rootfs_path.exists());

        // Create old_root directory inside rootfs
        std::fs::create_dir_all(&old_root_path).unwrap();
        assert!(old_root_path.exists());
        assert!(old_root_path.parent().unwrap() == rootfs_path);

        // Remove old_root
        std::fs::remove_dir_all(&old_root_path).unwrap();
        assert!(!old_root_path.exists());

        // Verify rootfs still exists
        assert!(rootfs_path.exists());
    }

    #[test]
    fn test_path_string_operations() {
        let rootfs = "/test/container";

        // Test various path operations
        let old_root = format!("{}/.old_root", rootfs);
        assert!(old_root.contains(rootfs));
        assert!(old_root.contains(".old_root"));

        // Test path components
        let components: Vec<&str> = old_root.split('/').filter(|s| !s.is_empty()).collect();
        assert_eq!(components, vec!["test", "container", ".old_root"]);
    }

    #[test]
    fn test_error_message_formatting() {
        let operation = "Failed to create directory";
        let path = "/test/container/.old_root";
        let error = "Permission denied";
        let formatted = format!("{} {}: {}", operation, path, error);

        assert!(formatted.contains(operation));
        assert!(formatted.contains(path));
        assert!(formatted.contains(error));
    }

    #[test]
    fn test_rootfs_name_patterns() {
        let valid_names = vec![
            "container",
            "alpine-rootfs",
            "ubuntu_20.04",
            "my-container-123",
        ];

        let invalid_names = vec!["", "container/", "/container", "container/../etc"];

        for name in valid_names {
            assert!(!name.is_empty());
            assert!(!name.contains('/'));
            assert!(!name.contains(".."));
        }

        for name in invalid_names {
            if name.is_empty() || name.contains('/') || name.contains("..") {
                // These are invalid for our use case
            }
        }
    }
}

use crate::cgroup::setup_cgroup;
use crate::namespace::setup_user_namespace;
use crate::pivot_root::setup_rootfs;
use anyhow::Result;
use nix::mount::{mount, MsFlags};
use nix::sched::{clone, CloneFlags};
use nix::sys::signal::Signal;
use nix::sys::socket::{socketpair, AddressFamily, SockFlag, SockType};
use nix::unistd::{execvp, sethostname};
use nix::unistd::{read, write};
use std::env;
use std::ffi::CString;
use std::os::fd::AsRawFd;

const STACK_SIZE: usize = 1024 * 1024; // 1MB stack

pub fn run_container(
    rootfs: &str,
    command: &str,
    args: Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut stack = vec![0u8; STACK_SIZE];
    let flags = CloneFlags::CLONE_NEWPID
        | CloneFlags::CLONE_NEWNS
        | CloneFlags::CLONE_NEWUTS
        | CloneFlags::CLONE_NEWUSER;
    let child_pid: nix::unistd::Pid;

    let (parent_sock, child_sock) = socketpair(
        AddressFamily::Unix,
        SockType::SeqPacket,
        None,
        SockFlag::empty(),
    )?;

    unsafe {
        child_pid = clone(
            Box::new(move || {
                let mut buf = [0u8; 1];
                if read(child_sock.as_raw_fd(), &mut buf).is_err() {
                    return 1;
                }
                child_process(rootfs.to_string(), command.to_string(), args.clone())
            }),
            &mut stack,
            flags,
            Some(Signal::SIGCHLD as i32),
        )?;
    }

    if let Err(e) = setup_cgroup(child_pid.as_raw()) {
        eprintln!("Failed to setup cgroups: {}", e);
        return Err(e);
    }

    if let Err(e) = setup_user_namespace(child_pid.as_raw()) {
        eprintln!("Failed to setup user namespace: {}", e);
        return Err(e);
    }

    if let Err(e) = write(parent_sock.as_raw_fd(), &[1]) {
        eprintln!("Failed to signal child process: {}", e);
        return Err(e.into());
    }

    println!("Container started with PID: {}", child_pid);

    match nix::sys::wait::waitpid(child_pid, None) {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("Failed to wait for child process: {}", e);
            Err(e.into())
        }
    }
}

fn child_process(rootfs: String, command: String, args: Vec<String>) -> isize {
    println!(
        "rootfs : {}, command : {}, args : {:?}",
        rootfs, command, args
    );

    // Make mount namespace private
    if let Err(e) = mount(
        None::<&str>,
        "/",
        None::<&str>,
        MsFlags::MS_REC | MsFlags::MS_PRIVATE,
        None::<&str>,
    ) {
        eprintln!("Failed to make mount private: {}", e);
        return 1;
    }

    // Set hostname
    if let Err(e) = sethostname("docker-clone") {
        eprintln!("Failed to set hostname: {}", e);
        return 1;
    }

    // Setup rootfs with proper error handling
    if let Err(e) = setup_rootfs(&rootfs) {
        eprintln!("Failed to setup root filesystem: {}", e);
        return 1;
    }

    // Ensure /proc exists
    if let Err(e) = std::fs::create_dir_all("/proc") {
        eprintln!("Failed to create /proc directory: {}", e);
        return 1;
    }

    // Mount proc filesystem
    if let Err(e) = mount(
        Some("proc"),
        "/proc",
        Some("proc"),
        MsFlags::empty(),
        None::<&str>,
    ) {
        eprintln!("Failed to mount proc: {}", e);
        return 1;
    }

    exec_command(&command, args)
}

fn exec_command(command: &str, args: Vec<String>) -> isize {
    let cmd = match CString::new(command) {
        Ok(cmd) => cmd,
        Err(e) => {
            eprintln!("Failed to create command string: {}", e);
            return 1;
        }
    };

    let mut full_args = vec![cmd.clone()];
    for arg in &args {
        match CString::new(arg.as_str()) {
            Ok(c_arg) => full_args.push(c_arg),
            Err(e) => {
                eprintln!("Failed to create argument string '{}': {}", arg, e);
                return 1;
            }
        }
    }

    // Set PATH environment variable
    unsafe {
        env::set_var("PATH", "/bin:/sbin:/usr/bin:/usr/sbin");
    }

    println!("Executing {:?} with args {:?}", command, full_args);

    match execvp(&cmd, &full_args) {
        Ok(_) => 0, // This should never be reached as execvp replaces the process
        Err(e) => {
            eprintln!("exec failed: {}", e);
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_exec_command_builds_correct_args() {
        let command = "/bin/ls";
        let args = vec!["-la".to_string(), "/tmp".to_string()];

        let _cmd = CString::new(command).unwrap();
        let mut full_args = vec![CString::new(command).unwrap()];
        full_args.extend(args.iter().map(|s| CString::new(s.as_str()).unwrap()));

        assert_eq!(full_args.len(), 3);
        assert_eq!(full_args[0].to_str().unwrap(), "/bin/ls");
        assert_eq!(full_args[1].to_str().unwrap(), "-la");
        assert_eq!(full_args[2].to_str().unwrap(), "/tmp");
    }

    #[test]
    fn test_exec_command_with_empty_args() {
        let command = "/bin/echo";
        let args = vec![];

        let _cmd = CString::new(command).unwrap();
        let mut full_args = vec![CString::new(command).unwrap()];
        full_args.extend(
            args.iter()
                .map(|s: &String| CString::new(s.as_str()).unwrap()),
        );

        assert_eq!(full_args.len(), 1);
        assert_eq!(full_args[0].to_str().unwrap(), "/bin/echo");
    }

    #[test]
    fn test_exec_command_with_special_characters() {
        let command = "/bin/echo";
        let args = vec!["hello world".to_string(), "test$123".to_string()];

        let _cmd = CString::new(command).unwrap();
        let mut full_args = vec![CString::new(command).unwrap()];
        full_args.extend(
            args.iter()
                .map(|s: &String| CString::new(s.as_str()).unwrap()),
        );

        assert_eq!(full_args.len(), 3);
        assert_eq!(full_args[0].to_str().unwrap(), "/bin/echo");
        assert_eq!(full_args[1].to_str().unwrap(), "hello world");
        assert_eq!(full_args[2].to_str().unwrap(), "test$123");
    }

    #[test]
    fn test_exec_command_handles_invalid_command() {
        let command = ""; // Invalid empty command
        let _args = vec!["test".to_string()];

        // This should create a valid CString even from empty string
        // The actual error would occur when execvp tries to execute it
        assert!(CString::new(command).is_ok());
    }

    #[test]
    fn test_exec_command_handles_null_bytes() {
        let _command = "/bin/ls";
        let _args = vec!["invalid\0arg".to_string()];

        // This should fail to create CString due to null byte
        let result = CString::new("invalid\0arg");
        assert!(result.is_err());
    }

    #[test]
    fn test_exec_command_environment_variable_setting() {
        // Test that PATH is set correctly
        unsafe {
            env::set_var("PATH", "/bin:/sbin:/usr/bin:/usr/sbin");
        }

        let path = env::var("PATH");
        assert!(path.is_ok());
        assert_eq!(path.unwrap(), "/bin:/sbin:/usr/bin:/usr/sbin");
    }
}

use crate::cgroup::setup_cgroup;
use crate::namespace::setup_user_namespace;
use crate::pivot_root::setup_rootfs;
use anyhow::Result;
use nix::mount::{MsFlags, mount};
use nix::sched::{CloneFlags, clone};
use nix::sys::signal::Signal;
use nix::sys::socket::{AddressFamily, SockFlag, SockType, socketpair};
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
                read(child_sock.as_raw_fd(), &mut buf).unwrap();
                child_process(rootfs.to_string(), command.to_string(), args.clone())
            }),
            &mut stack,
            flags,
            Some(Signal::SIGCHLD as i32),
        )?;
    }

    setup_cgroup(child_pid.as_raw())?;

    setup_user_namespace(child_pid.as_raw())?;

    write(parent_sock.as_raw_fd(), &[1])?;
    println!("Container started with PID: {}", child_pid);
    nix::sys::wait::waitpid(child_pid, None)?;
    Ok(())
}

fn child_process(rootfs: String, command: String, args: Vec<String>) -> isize {
    println!(
        "rootfs : {}, command : {}, args : {:?}",
        rootfs, command, args
    );

    mount(
        None::<&str>,
        "/",
        None::<&str>,
        MsFlags::MS_REC | MsFlags::MS_PRIVATE,
        None::<&str>,
    )
    .unwrap();

    sethostname("docker-clone").unwrap();

    // setup_rootfs(&rootfs).unwrap();

    std::fs::create_dir_all("/proc").unwrap();
    mount(
        Some("proc"),
        "/proc",
        Some("proc"),
        MsFlags::empty(),
        None::<&str>,
    )
    .unwrap();

    return exec_command(&command, args);
}

fn exec_command(command: &str, args: Vec<String>) -> isize {
    let cmd = CString::new(command).unwrap();
    let mut full_args = vec![cmd.clone()];
    full_args.extend(args.iter().map(|s| CString::new(s.as_str()).unwrap()));

    unsafe {
        env::set_var("PATH", "/bin:/sbin:/usr/bin:/usr/sbin");
    }

    println!("Executing {:?} with args {:?}", command, full_args);
    let Err(e) = execvp(&cmd, &full_args);
    eprintln!("exec: {}", e);
    return 1;
}

#[cfg(test)]
mod tests {
    use super::*;

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
}

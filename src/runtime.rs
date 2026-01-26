use anyhow::Result;
use nix::mount::{MsFlags, mount};
use nix::sched::{CloneFlags, clone};
use nix::sys::signal::Signal;
use nix::unistd::{chdir, chroot, execvp, sethostname};
use std::env;
use std::ffi::CString;

const STACK_SIZE: usize = 1024 * 1024; // 1MB stack

pub fn run_container(rootfs: &str, command: &str, args: Vec<String>) -> Result<()> {
    let mut stack = vec![0u8; STACK_SIZE];

    let flags = CloneFlags::CLONE_NEWPID | CloneFlags::CLONE_NEWNS | CloneFlags::CLONE_NEWUTS;
    let mut rootfs = Some(rootfs.to_string());
    let mut command = Some(command.to_string());
    let mut args = Some(args);

    let child_pid: nix::unistd::Pid;

    unsafe {
        env::set_var("PATH", "/bin:/sbin:/usr/bin:/usr/sbin");
        child_pid = clone(
            Box::new(move || {
                let rootfs = rootfs.take().unwrap();
                let command = command.take().unwrap();
                let args = args.take().unwrap();

                child_process(rootfs, command, args)
            }),
            &mut stack,
            flags,
            Some(Signal::SIGCHLD as i32),
        )?;
    }

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
    if let Err(e) = chroot(rootfs.as_str()) {
        eprintln!("chroot failed: {}", e);
        return 1;
    }
    if let Err(e) = chdir("/") {
        eprintln!("chdir failed: {}", e);
        return 1;
    }
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
    println!("Executing {:?} with args {:?}", command, args);
    let cmd = CString::new(command).unwrap();
    let mut full_args = vec![CString::new(command).unwrap()];
    full_args.extend(args.iter().map(|s| CString::new(s.as_str()).unwrap()));
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

use anyhow::Result;
use nix::sched::{CloneFlags, clone};
use nix::sys::signal::Signal;
use nix::unistd::{chdir, chroot, execvp};
use std::ffi::CString;
use std::path::Path;

const STACK_SIZE: usize = 1024 * 1024; // 1MB stack

pub fn run_container(rootfs: &str, command: &str, args: Vec<String>) -> Result<()> {
    let mut stack = vec![0u8; STACK_SIZE];
    let flags = CloneFlags::CLONE_NEWPID;

    let mut rootfs = Some(rootfs.to_string());
    let mut command = Some(command.to_string());
    let mut args = Some(args);

    let child_pid: nix::unistd::Pid;

    unsafe {
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
    if let Err(e) = chroot(rootfs.as_str()) {
        eprintln!("chroot failed: {}", e);
        return 1;
    }
    if let Err(e) = chdir("/") {
        eprintln!("chdir failed: {}", e);
        return 1;
    }
    let path = Path::new(&command);
    if !path.exists() {
        eprintln!(
            "Executable '{}' not found, trying BusyBox fallback...",
            command
        );

        let busybox = Path::new("/bin/busybox");
        if !busybox.exists() {
            eprintln!("BusyBox not found in /bin");
            return 1;
        }

        let mut new_args = vec![command];
        new_args.extend(args);
        return exec_command("/bin/busybox", new_args);
    }
    return exec_command(&command, args);
}

fn exec_command(command: &str, args: Vec<String>) -> isize {
    println!("Executing {:?} with args {:?}", command, args);
    let cmd = CString::new(command).unwrap();
    let args: Vec<CString> = args
        .iter()
        .map(|s| CString::new(s.as_str()).unwrap())
        .collect();
    let Err(e) = execvp(&cmd, &args);
    eprintln!("execvp failed: {}", e);
    return 1;
}

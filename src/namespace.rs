use nix::unistd::{getgid, getuid};

pub fn setup_user_namespace(pid: i32) -> std::io::Result<()> {
    let uid = getuid().as_raw();
    let gid = getgid().as_raw();

    std::fs::write(format!("/proc/{}/setgroups", pid), "deny")?;
    std::fs::write(format!("/proc/{}/uid_map", pid), format!("0 {} 1", uid))?;
    std::fs::write(format!("/proc/{}/gid_map", pid), format!("0 {} 1", gid))?;
    Ok(())
}

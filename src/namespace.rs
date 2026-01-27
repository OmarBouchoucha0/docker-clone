use nix::unistd::{Gid, Uid};

pub fn setup_userns(pid: i32) -> std::io::Result<()> {
    let uid = Uid::current().as_raw();
    let gid = Gid::current().as_raw();

    std::fs::write(format!("/proc/{}/setgroups", pid), "deny")?;
    std::fs::write(format!("/proc/{}/uid_map", pid), format!("0 {} 1", uid))?;
    std::fs::write(format!("/proc/{}/gid_map", pid), format!("0 {} 1", gid))?;

    Ok(())
}

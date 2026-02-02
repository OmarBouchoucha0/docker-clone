use anyhow::Result;
use nix::mount::{MntFlags, MsFlags, mount, umount2};
use nix::unistd::{chdir, pivot_root};

pub fn setup_rootfs(rootfs: &str) -> Result<()> {
    let rootfs = std::fs::canonicalize(rootfs)?;

    mount(
        None::<&str>,
        "/",
        None::<&str>,
        MsFlags::MS_PRIVATE | MsFlags::MS_REC,
        None::<&str>,
    )?;

    mount(
        Some(&rootfs),
        &rootfs,
        None::<&str>,
        MsFlags::MS_BIND | MsFlags::MS_REC,
        None::<&str>,
    )?;

    let old_root = rootfs.join(".old_root");
    std::fs::create_dir_all(&old_root)?;

    pivot_root(&rootfs, &old_root)?;

    chdir("/")?;

    umount2("/.old_root", MntFlags::MNT_DETACH)?;

    std::fs::remove_dir_all("/.old_root")?;

    Ok(())
}


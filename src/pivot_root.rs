use anyhow::{Context, Result};
use nix::mount::{MntFlags, MsFlags, mount, umount2};
use nix::unistd::chdir;
use std::path::Path;

pub fn setup_rootfs(rootfs: impl AsRef<Path>) -> Result<()> {
    let rootfs = rootfs.as_ref();

    let rootfs = if rootfs.is_absolute() {
        rootfs.to_path_buf()
    } else {
        std::env::current_dir()
            .context("Failed to get current directory")?
            .join(rootfs)
    };

    if !rootfs.exists() {
        anyhow::bail!("Rootfs path does not exist: {:?}", rootfs);
    }

    println!("Setting up rootfs at: {:?}", rootfs);

    mount(
        None::<&str>,
        "/",
        None::<&str>,
        MsFlags::MS_PRIVATE | MsFlags::MS_REC,
        None::<&str>,
    )
    .context("Failed to make / a private mount")?;

    mount(
        Some(rootfs.as_path()),
        rootfs.as_path(),
        None::<&str>,
        MsFlags::MS_BIND | MsFlags::MS_REC,
        None::<&str>,
    )
    .with_context(|| format!("Failed to bind mount at {:?}", rootfs))?;

    let old_root = rootfs.join(".old_root");
    std::fs::create_dir_all(&old_root)
        .with_context(|| format!("Failed to create old_root at {:?}", old_root))?;

    chdir(&rootfs).with_context(|| format!("Failed to chdir to {:?}", rootfs))?;

    pivot_root(".", ".old_root").context("Failed to pivot_root")?;

    chdir("/").context("Failed to chdir to /")?;

    umount2("/.old_root", MntFlags::MNT_DETACH).context("Failed to unmount old root")?;

    std::fs::remove_dir_all("/.old_root").context("Failed to remove old_root directory")?;

    Ok(())
}

fn pivot_root<P1: ?Sized + nix::NixPath, P2: ?Sized + nix::NixPath>(
    new_root: &P1,
    put_old: &P2,
) -> nix::Result<()> {
    use nix::errno::Errno;
    let res = new_root.with_nix_path(|new_root| {
        put_old.with_nix_path(|put_old| unsafe {
            libc::syscall(libc::SYS_pivot_root, new_root.as_ptr(), put_old.as_ptr())
        })
    })??;

    Errno::result(res).map(drop)
}


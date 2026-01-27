use nix::mount::{MntFlags, MsFlags, mount, umount2};
use nix::unistd::{chdir, pivot_root};

pub fn setup_rootfs(rootfs: &str) -> nix::Result<()> {
    mount(
        Some(rootfs),
        rootfs,
        None::<&str>,
        MsFlags::MS_BIND | MsFlags::MS_REC,
        None::<&str>,
    )?;

    let old_root = format!("{}/.old_root", rootfs);
    std::fs::create_dir_all(&old_root).unwrap();

    pivot_root(rootfs, old_root.as_str())?;

    chdir("/")?;

    umount2("/.old_root", MntFlags::MNT_DETACH)?;

    std::fs::remove_dir_all("/.old_root").unwrap();

    Ok(())
}

use std::fs;
use std::io::Write;

pub fn setup_cgroup(pid: i32) -> Result<(), Box<dyn std::error::Error>> {
    let cgroup_path = format!("/sys/fs/cgroup/docker-clone/{}", pid);
    println!("Creating cgroup at {}", cgroup_path);
    fs::create_dir_all(&cgroup_path)?;

    // Memory limit: 100MB
    fs::write(format!("{}/memory.max", cgroup_path), "104857600")?;

    // CPU limit: 50% of one core
    fs::write(format!("{}/cpu.max", cgroup_path), "50000 100000")?;

    // Attach process
    let mut procs = fs::OpenOptions::new()
        .write(true)
        .open(format!("{}/cgroup.procs", cgroup_path))?;

    writeln!(procs, "{}", pid)?;

    Ok(())
}

use std::fs;
use std::io::Write;

pub fn setup_cgroup(pid: i32) -> Result<(), Box<dyn std::error::Error>> {
    let cgroup_line = std::fs::read_to_string("/proc/self/cgroup")?;
    let cgroup_rel = cgroup_line
        .lines()
        .find(|l| l.starts_with("0::"))
        .unwrap()
        .trim_start_matches("0::");

    let base = "/sys/fs/cgroup";
    let parent_cgroup = format!("{}{}", base, cgroup_rel);
    let child_cgroup = format!("{}/docker-clone-{}", parent_cgroup, pid);

    let controllers = std::fs::read_to_string(format!("{}/cgroup.controllers", parent_cgroup))?;
    println!("Available controllers: {}", controllers);
    enable_controllers(&parent_cgroup)?;

    println!("Creating cgroup at {}", child_cgroup);
    fs::create_dir_all(&child_cgroup)?;

    // Memory limit: 100MB
    fs::write(format!("{}/memory.max", child_cgroup), "104857600")?;

    // CPU limit: 50% of one core
    fs::write(format!("{}/cpu.max", child_cgroup), "50000 100000")?;

    // Attach process
    let mut procs = fs::OpenOptions::new()
        .write(true)
        .open(format!("{}/cgroup.procs", child_cgroup))?;

    writeln!(procs, "{}", pid)?;

    Ok(())
}

fn enable_controllers(parent: &str) -> std::io::Result<()> {
    std::fs::write(
        format!("{}/cgroup.subtree_control", parent),
        "+cpu +memory +pids",
    )?;
    Ok(())
}

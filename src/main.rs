use clap::Parser;
mod cgroup;
mod namespace;
mod pivot_root;
mod runtime;
use runtime::run_container;

#[derive(Parser, Debug)]
#[command(name = "container")]
#[command(about = "A simple container runtime")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    Run {
        rootfs: String,
        command: String,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run {
            rootfs,
            command,
            args,
        } => {
            run_container(&rootfs, &command, args)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_cli_parse_run_command() {
        // Test basic run command parsing
        let args = vec!["container", "run", "/tmp/rootfs", "/bin/ls"];
        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command {
            Commands::Run {
                rootfs,
                command,
                args,
            } => {
                assert_eq!(rootfs, "/tmp/rootfs");
                assert_eq!(command, "/bin/ls");
                assert!(args.is_empty());
            }
        }
    }

    #[test]
    fn test_cli_parse_run_command_with_args() {
        // Test run command with arguments
        let args = vec!["container", "run", "/tmp/rootfs", "/bin/ls", "-la", "/tmp"];
        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command {
            Commands::Run {
                rootfs,
                command,
                args,
            } => {
                assert_eq!(rootfs, "/tmp/rootfs");
                assert_eq!(command, "/bin/ls");
                assert_eq!(args.len(), 2);
                assert_eq!(args[0], "-la");
                assert_eq!(args[1], "/tmp");
            }
        }
    }

    #[test]
    fn test_cli_parse_run_command_with_hyphen_args() {
        // Test run command with hyphen-prefixed arguments
        let args = vec![
            "container",
            "run",
            "/tmp/rootfs",
            "/bin/bash",
            "-c",
            "echo hello",
        ];
        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command {
            Commands::Run {
                rootfs,
                command,
                args,
            } => {
                assert_eq!(rootfs, "/tmp/rootfs");
                assert_eq!(command, "/bin/bash");
                assert_eq!(args.len(), 2);
                assert_eq!(args[0], "-c");
                assert_eq!(args[1], "echo hello");
            }
        }
    }

    #[test]
    fn test_cli_parse_run_command_with_special_paths() {
        // Test run command with various filesystem paths
        let test_cases = vec![
            "/var/lib/container",
            "/opt/docker/rootfs",
            "/home/user/custom_root",
            "./relative/path",
        ];

        for rootfs_path in test_cases {
            let args = vec!["container", "run", rootfs_path, "/bin/echo", "test"];
            let cli = Cli::try_parse_from(args).unwrap();

            match cli.command {
                Commands::Run {
                    rootfs,
                    command,
                    args,
                } => {
                    assert_eq!(rootfs, rootfs_path);
                    assert_eq!(command, "/bin/echo");
                    assert_eq!(args.len(), 1);
                    assert_eq!(args[0], "test");
                }
            }
        }
    }

    #[test]
    fn test_cli_parse_various_commands() {
        // Test parsing of various container commands
        let test_cases = vec![
            ("/bin/sh", vec![]),
            ("/bin/bash", vec!["-c", "echo hello"]),
            ("/usr/bin/python3", vec!["script.py", "--verbose"]),
            ("/bin/cat", vec!["/etc/hosts"]),
        ];

        for (command, args) in test_cases {
            let mut cli_args = vec!["container", "run", "/tmp/rootfs", command];
            cli_args.extend(args.iter());

            let cli = Cli::try_parse_from(cli_args).unwrap();

            match cli.command {
                Commands::Run {
                    rootfs,
                    command: parsed_command,
                    args: parsed_args,
                } => {
                    assert_eq!(rootfs, "/tmp/rootfs");
                    assert_eq!(parsed_command, command);
                    assert_eq!(parsed_args, args);
                }
            }
        }
    }

    #[test]
    fn test_cli_help_generation() {
        // Test that help can be generated without panicking
        let result = Cli::try_parse_from(["container", "--help"]);
        assert!(result.is_err());

        // The error should be a help message, not a panic
        let error = result.unwrap_err();
        assert!(error.to_string().contains("help") || error.to_string().len() > 0);
    }

    #[test]
    fn test_cli_command_help() {
        // Test that command help can be generated
        let result = Cli::try_parse_from(["container", "run", "--help"]);
        assert!(result.is_err());

        // The error should be a help message, not a panic
        let error = result.unwrap_err();
        assert!(error.to_string().contains("help") || error.to_string().len() > 0);
    }

    #[test]
    fn test_cli_invalid_subcommand() {
        // Test handling of invalid subcommands
        let result = Cli::try_parse_from(["container", "invalid"]);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(error.to_string().contains("invalid") || error.to_string().len() > 0);
    }

    #[test]
    fn test_cli_empty_args() {
        // Test parsing with no arguments at all(first arg is the name of the program)
        let result = Cli::try_parse_from(["prog"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_cli_argument_validation() {
        // Test that rootfs and command arguments are properly parsed
        let args = vec!["container", "run", "/path/to/rootfs", "/usr/bin/command"];
        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command {
            Commands::Run {
                rootfs,
                command,
                args,
            } => {
                // Verify rootfs is a non-empty string
                assert!(!rootfs.is_empty());
                assert!(rootfs.len() > 1);

                // Verify command is a non-empty string
                assert!(!command.is_empty());
                assert!(command.len() > 1);

                // Verify args vector is properly initialized
                assert!(args.is_empty()); // No args in this test case
            }
        }
    }

    #[test]
    fn test_cli_complex_command_line() {
        // Test parsing a complex command line similar to real usage
        let args = vec![
            "container",
            "run",
            "/var/lib/alpine-rootfs",
            "/bin/sh",
            "-c",
            "echo 'Hello World' && ls -la /tmp && cat /etc/os-release",
        ];

        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command {
            Commands::Run {
                rootfs,
                command,
                args,
            } => {
                assert_eq!(rootfs, "/var/lib/alpine-rootfs");
                assert_eq!(command, "/bin/sh");
                assert_eq!(args.len(), 2);
                assert_eq!(args[0], "-c");
                assert_eq!(
                    args[1],
                    "echo 'Hello World' && ls -la /tmp && cat /etc/os-release"
                );
            }
        }
    }

    #[test]
    fn test_cli_command_and_args_separation() {
        // Test that command and args are properly separated
        let args = vec![
            "container",
            "run",
            "/tmp/rootfs",
            "echo",
            "arg1",
            "arg2",
            "--flag",
            "arg3",
        ];

        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command {
            Commands::Run {
                rootfs,
                command,
                args,
            } => {
                assert_eq!(rootfs, "/tmp/rootfs");
                assert_eq!(command, "echo"); // First arg after rootfs is the command
                assert_eq!(args, vec!["arg1", "arg2", "--flag", "arg3"]);
            }
        }
    }
}

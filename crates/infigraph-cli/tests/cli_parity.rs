use std::collections::HashSet;
use std::fs;
use std::process::{Command, Output};

fn process_diagnostics(command: &str, output: &Output) -> String {
    #[cfg(unix)]
    let platform_status = {
        use std::os::unix::process::ExitStatusExt;
        format!(
            "unix_signal: {:?}\ncore_dumped: {}\n",
            output.status.signal(),
            output.status.core_dumped()
        )
    };
    #[cfg(not(unix))]
    let platform_status = "";

    format!(
        "command: {command}\nstatus: {}\nexit_code: {:?}\n{platform_status}stdout:\n{}\nstderr:\n{}",
        output.status,
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    )
}

#[cfg(unix)]
#[test]
fn failed_process_diagnostics_preserve_status_signal_and_both_streams() {
    use std::os::unix::process::ExitStatusExt;
    use std::process::ExitStatus;

    let mut output = Output {
        status: ExitStatus::from_raw(7 << 8),
        stdout: b"fixture stdout".to_vec(),
        stderr: b"fixture stderr".to_vec(),
    };
    let failure = process_diagnostics("fixture command", &output);
    for expected in [
        "command: fixture command",
        "exit_code: Some(7)",
        "unix_signal: None",
        "stdout:\nfixture stdout",
        "stderr:\nfixture stderr",
    ] {
        assert!(failure.contains(expected), "{failure}");
    }

    output.status = ExitStatus::from_raw(4);
    let signal = process_diagnostics("fixture command", &output);
    assert!(signal.contains("exit_code: None"), "{signal}");
    assert!(signal.contains("unix_signal: Some(4)"), "{signal}");
}

fn get_cli_subcommands() -> HashSet<String> {
    let output = Command::new(env!("CARGO_BIN_EXE_infigraph"))
        .arg("--help")
        .output()
        .expect("failed to run infigraph --help");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let text = format!("{stdout}{stderr}");

    let mut commands = HashSet::new();
    let mut in_commands = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Commands:") || trimmed.starts_with("Subcommands:") {
            in_commands = true;
            continue;
        }
        if in_commands {
            if trimmed.is_empty() || trimmed.starts_with("Options:") {
                break;
            }
            if let Some(cmd) = trimmed.split_whitespace().next() {
                commands.insert(cmd.to_lowercase());
            }
        }
    }
    commands
}

#[test]
fn all_mapped_cli_commands_exist_in_binary() {
    let output = Command::new(env!("CARGO_BIN_EXE_infigraph"))
        .arg("--help")
        .output()
        .expect("failed to run infigraph --help");

    if !output.status.success() && output.stdout.is_empty() {
        eprintln!(
            "infigraph --help exited with {:?}, stderr: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        );
        // Binary failed to run (e.g. missing DLLs on Windows CI) — skip rather than fail
        eprintln!("Skipping cli_parity test: binary could not execute");
        return;
    }

    let available = get_cli_subcommands();
    if available.is_empty() {
        panic!(
            "Could not parse any subcommands from `infigraph --help`. \
             Check that the binary builds and --help output is parseable.\n\
             stdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }

    let mut missing = Vec::new();
    for (_mcp_tool, cli_cmd) in infigraph_mcp::MCP_TO_CLI_MAP {
        let top_cmd = cli_cmd.split_whitespace().next().unwrap();
        if !available.contains(&top_cmd.to_lowercase()) {
            missing.push(format!("{_mcp_tool} → `infigraph {cli_cmd}`"));
        }
    }

    assert!(
        missing.is_empty(),
        "MCP tools mapped to CLI commands that don't exist in `infigraph --help`:\n  {}",
        missing.join("\n  ")
    );
}

#[test]
fn allowlist_covers_all_mcp_tools() {
    let allowed = infigraph_mcp::allowed_tools_from_names();
    let allowed_set: HashSet<&str> = allowed.iter().map(|s| s.as_str()).collect();

    let mut missing = Vec::new();
    for tool in infigraph_mcp::MCP_TOOL_NAMES {
        let prefixed = format!("mcp__infigraph__{tool}");
        if !allowed_set.contains(prefixed.as_str()) {
            missing.push(*tool);
        }
    }

    assert!(
        missing.is_empty(),
        "MCP tools not in Claude Code allowlist: {missing:?}"
    );
}

#[test]
fn test_context_runs_in_the_cli_process() {
    let project = tempfile::tempdir().expect("create temporary project");
    let source_dir = project.path().join("src");
    fs::create_dir_all(&source_dir).expect("create source directory");
    fs::write(
        source_dir.join("calculator.py"),
        "def add(left, right):\n    return left + right\n",
    )
    .expect("write source file");

    let cli = env!("CARGO_BIN_EXE_infigraph");
    let index = Command::new(cli)
        .args([
            "--root",
            project.path().to_str().unwrap(),
            "index",
            "--no-embed",
        ])
        .output()
        .expect("run index");
    assert!(
        index.status.success(),
        "index failed:\n{}",
        process_diagnostics("infigraph --root <fixture> index --no-embed", &index)
    );

    let test_context = Command::new(cli)
        .args([
            "--root",
            project.path().to_str().unwrap(),
            "test-context",
            "--file",
            "src/calculator.py",
            "--limit",
            "1",
        ])
        .output()
        .expect("run test-context");
    assert!(
        test_context.status.success(),
        "test-context failed:\n{}",
        process_diagnostics(
            "infigraph --root <fixture> test-context --file src/calculator.py --limit 1",
            &test_context
        )
    );
    assert!(
        String::from_utf8_lossy(&test_context.stdout).contains("Test Generation Context"),
        "unexpected test-context output:\n{}",
        process_diagnostics(
            "infigraph --root <fixture> test-context --file src/calculator.py --limit 1",
            &test_context
        )
    );
}

// Copyright 2024, Giordano Salvador
// SPDX-License-Identifier: BSD-3-Clause

use crate::exit_code;
use exit_code::exit;
use exit_code::ExitCode;

use std::io::Write;
use std::process;
use std::process::Stdio;

pub struct CommandResult {
    pub success: bool,
    pub stdout: Option<String>,
}

impl CommandResult {
    pub fn new(success: bool, stdout: Option<String>) -> Self {
        CommandResult{success, stdout}
    }
}

pub struct Command {}

impl Command {
    pub fn run(bin: &str, args: &[&str]) -> CommandResult {
        let output = match process::Command::new(bin).args(args.iter()).output() {
            Ok(output)  => output,
            Err(msg)    => {
                eprintln!("Failed to run with {}: {}", bin, msg);
                exit(ExitCode::CommandError);
            },
        };
        let stderr: &[u8] = output.stderr.as_slice();
        let stdout: &[u8] = output.stdout.as_slice();
        if !stderr.is_empty() {
            eprintln!("{} stderr:\n{}", bin, std::str::from_utf8(stderr).unwrap());
        }
        let stdout_opt = if !stdout.is_empty() {
            Some(std::str::from_utf8(stdout).unwrap().to_string())
        } else {
            None
        };
        CommandResult::new(output.status.success(), stdout_opt)
    }

    pub fn run_with_input(bin: &str, args: &[&str], input: &str) -> CommandResult {
        let mut proc = match process::Command::new(bin)
            .args(args.iter())
            .stdin(Stdio::piped())
            .spawn() {
            Ok(proc)    => proc,
            Err(msg)    => {
                eprintln!("Failed to spawn {} child process: {}", bin, msg);
                exit(ExitCode::CommandError);
            },
        };
        let proc_stdin = match proc.stdin.as_mut() {
            Some(stdin) => stdin,
            None        => {
                eprintln!("Failed to get {} process stdin handle", bin);
                exit(ExitCode::CommandError);
            },
        };
        match proc_stdin.write_all(input.as_bytes()) {
            Ok(())      => (),
            Err(msg)    => {
                eprintln!("Failed to write input to {} process stdin: {}", bin, msg);
                exit(ExitCode::CommandError);
            },
        };
        let output = match proc.wait_with_output() {
            Ok(output)  => output,
            Err(msg)    => {
                eprintln!("Failed to run with {}: {}", bin, msg);
                exit(ExitCode::CommandError);
            },
        };
        let stderr: &[u8] = output.stderr.as_slice();
        let stdout: &[u8] = output.stdout.as_slice();
        if !stderr.is_empty() {
            eprintln!("{} stderr:\n{}", bin, std::str::from_utf8(stderr).unwrap());
        }
        let stdout_opt = if !stdout.is_empty() {
            Some(std::str::from_utf8(stdout).unwrap().to_string())
        } else {
            None
        };
        CommandResult::new(output.status.success(), stdout_opt)
    }
}

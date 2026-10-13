// SPDX-License-Identifier: GPL-3.0-only
use otp_core::{native::Target, Options};
use std::{
    io::{self, BufRead, BufReader, Write},
    net::TcpListener,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};
fn main() -> ExitCode {
    match run() {
        Ok(n) => ExitCode::from(n),
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
fn run() -> Result<u8, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args.iter().any(|s| s == "--help" || s == "-h") {
        println!("ODIN Turbo Pascal 2.0.0-dev.2 (Rust)\nGPL-3.0-only\n\notpc check FILE.pas\notpc build FILE.pas [-o OUTPUT] [--debug] [--range-check] [--overflow-check]\notpc run FILE.pas [options] [-- PROGRAM_ARGS...]\notpc debug FILE.pas\notpc emit-asm FILE.pas --target linux-x64|windows-x64|macos-x64|macos-arm64 [-o FILE.s]\notpc doctor\n\nDevelopment release. The supported Pascal subset is documented in COMPATIBILITY.md.\nNative linking currently requires a C toolchain (OTP_CC) and bundled otp-rtl.\n");
        return Ok(0);
    }
    if args[0] == "--version" {
        println!("ODIN Turbo Pascal 2.0.0-dev.2");
        return Ok(0);
    }
    if args[0] == "doctor" {
        println!(
            "Host: {} / {}",
            std::env::consts::OS,
            std::env::consts::ARCH
        );
        println!("Target: {:?}", Target::host());
        let runtime = otp_core::build::runtime_path();
        let runtime_ok = runtime.is_ok();
        println!("Runtime: {:?}", runtime);
        let cc = std::env::var("OTP_CC").unwrap_or_else(|_| {
            if cfg!(windows) {
                "clang".into()
            } else {
                "cc".into()
            }
        });
        let ok = Command::new(&cc)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        println!("Linker {cc}: {}", if ok { "available" } else { "missing" });
        return Ok(if ok && runtime_ok { 0 } else { 1 });
    }
    let verb = &args[0];
    if !["check", "build", "run", "debug", "emit-asm"].contains(&verb.as_str()) {
        return Err(format!("Unknown command {verb}. Use --help."));
    }
    let file = PathBuf::from(args.get(1).ok_or("Pascal source file required")?);
    let mut opts = Options::default();
    opts.debug = verb == "debug";
    let mut output = None;
    let mut target = Target::host().map_err(|e| e.to_string())?;
    let mut program_args = vec![];
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "-o" => {
                i += 1;
                output = Some(PathBuf::from(args.get(i).ok_or("-o requires a filename")?));
            }
            "--target" => {
                i += 1;
                target = Target::parse(args.get(i).ok_or("--target requires a target")?)
                    .ok_or("Unknown target")?;
            }
            "--debug" => opts.debug = true,
            "--range-check" => opts.range = true,
            "--overflow-check" => opts.overflow = true,
            "--" => {
                program_args = args[i + 1..].to_vec();
                break;
            }
            s => return Err(format!("Unknown option {s}")),
        }
        i += 1;
    }
    if verb == "check" || verb == "emit-asm" {
        let asm = otp_core::compile_file(&file, target, &opts).map_err(|e| diagnostic(&file, e))?;
        if verb == "check" {
            println!(
                "{}: semantic checks passed for implemented subset",
                file.display()
            )
        } else {
            let p = output.unwrap_or_else(|| file.with_extension("s"));
            if std::fs::canonicalize(&p).ok() == std::fs::canonicalize(&file).ok() {
                return Err("Assembly output would overwrite the Pascal source".into());
            }
            std::fs::write(&p, asm).map_err(|e| e.to_string())?;
            println!("Assembly: {} ({})", p.display(), target.name());
        }
        return Ok(0);
    }
    if target != Target::host().unwrap() {
        return Err(
            "Cross-linking is not implemented. emit-asm supports selecting another target.".into(),
        );
    }
    let output = output.unwrap_or_else(|| {
        if cfg!(windows) {
            file.with_extension("exe")
        } else {
            file.with_extension("")
        }
    });
    let result = otp_core::build::build(&file, &output, &opts).map_err(|e| diagnostic(&file, e))?;
    eprintln!(
        "Built native {}: {}",
        target.name(),
        result.executable.display()
    );
    if verb == "debug" {
        return debug(&result.executable, &program_args);
    }
    if verb == "run" {
        let status = Command::new(&result.executable)
            .args(program_args)
            .status()
            .map_err(|e| e.to_string())?;
        return Ok(status.code().unwrap_or(1).clamp(0, 255) as u8);
    }
    Ok(0)
}
fn diagnostic(path: &Path, e: otp_core::Error) -> String {
    format!("{}:{}", path.display(), e)
}
fn debug(exe: &Path, args: &[String]) -> Result<u8, String> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let mut child = Command::new(exe)
        .args(args)
        .env("OTP_DEBUG_ADDR", listener.local_addr().unwrap().to_string())
        .spawn()
        .map_err(|e| e.to_string())?;
    let start = std::time::Instant::now();
    let mut stream = loop {
        match listener.accept() {
            Ok((s, _)) => break s,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                if let Some(s) = child.try_wait().map_err(|e| e.to_string())? {
                    return Err(format!("Debuggee exited before connecting: {s}"));
                }
                if start.elapsed().as_secs() > 15 {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err("Debugger connection timeout".into());
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            Err(e) => return Err(e.to_string()),
        }
    };
    let mut reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);
    loop {
        let mut event = String::new();
        if reader.read_line(&mut event).map_err(|e| e.to_string())? == 0 {
            break;
        }
        eprintln!("{}", event.trim_end());
        loop {
            eprint!("debug [s=step n=next c=continue b LINE q=quit]> ");
            io::stderr().flush().map_err(|e| e.to_string())?;
            let mut line = String::new();
            if io::stdin()
                .read_line(&mut line)
                .map_err(|e| e.to_string())?
                == 0
            {
                line = "q".into();
            }
            let a: Vec<_> = line.split_whitespace().collect();
            let cmd = match a.first().copied() {
                Some("s") => "STEP\n".into(),
                Some("n") => "NEXT\n".into(),
                Some("c") => "CONT\n".into(),
                Some("q") => "QUIT\n".into(),
                Some("b") => format!("BREAK {}\n", a.get(1).ok_or("Breakpoint needs line")?),
                _ => {
                    eprintln!("Unknown debug command");
                    continue;
                }
            };
            stream
                .write_all(cmd.as_bytes())
                .map_err(|e| e.to_string())?;
            if !cmd.starts_with("BREAK") {
                break;
            }
        }
    }
    Ok(child
        .wait()
        .map_err(|e| e.to_string())?
        .code()
        .unwrap_or(1)
        .clamp(0, 255) as u8)
}

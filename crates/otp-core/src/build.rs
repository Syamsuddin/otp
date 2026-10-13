// SPDX-License-Identifier: GPL-3.0-only
use crate::{native::Target, Error, Options};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
#[derive(Debug)]
pub struct BuildResult {
    pub executable: PathBuf,
    pub assembly: PathBuf,
    pub linker: String,
}
pub fn runtime_path() -> Result<PathBuf, String> {
    if let Some(p) = std::env::var_os("OTP_RUNTIME") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return Ok(p);
        }
        return Err("OTP_RUNTIME does not point to a file".into());
    }
    let name = if cfg!(all(windows, target_env = "msvc")) {
        "otp_rtl.lib"
    } else {
        "libotp_rtl.a"
    };
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let dir = exe.parent().unwrap();
    for p in [
        dir.join(name),
        dir.join("../lib").join(name),
        dir.join("../Resources/lib").join(name),
    ] {
        if p.is_file() {
            return Ok(p);
        }
    }
    Err(format!("Runtime {name} missing. Build otp-rtl and place the library beside otpc, or set OTP_RUNTIME."))
}
pub fn build(input: &Path, output: &Path, opts: &Options) -> Result<BuildResult, Error> {
    let target = Target::host().map_err(|e| Error::new(1, 1, e))?;
    let asm = crate::compile_file(input, target, opts)?;
    let rtl = runtime_path().map_err(|e| Error::new(1, 1, e))?;
    let abs = if output.is_absolute() {
        output.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| Error::new(1, 1, e.to_string()))?
            .join(output)
    };
    if fs::canonicalize(&abs).unwrap_or_else(|_| abs.clone())
        == fs::canonicalize(input).unwrap_or_default()
    {
        return Err(Error::new(1, 1, "Output would overwrite Pascal source"));
    }
    if let Some(p) = abs.parent() {
        fs::create_dir_all(p).map_err(|e| Error::new(1, 1, e.to_string()))?;
    }
    let build_dir = abs.with_extension(format!("otp-build-{}", std::process::id()));
    fs::create_dir_all(&build_dir).map_err(|e| Error::new(1, 1, e.to_string()))?;
    let assembly = build_dir.join("program.s");
    fs::write(&assembly, asm).map_err(|e| Error::new(1, 1, e.to_string()))?;
    let tmp = build_dir.join(if cfg!(windows) {
        "program.exe"
    } else {
        "program"
    });
    let cc = std::env::var("OTP_CC").unwrap_or_else(|_| {
        if cfg!(windows) {
            "clang".into()
        } else {
            "cc".into()
        }
    });
    let mut c = Command::new(&cc);
    c.arg(&assembly).arg(&rtl).arg("-o").arg(&tmp);
    match target {
        Target::LinuxX64 => {
            c.args(["-ldl", "-lpthread", "-lm", "-lrt", "-lutil"]);
        }
        Target::MacX64 | Target::MacArm64 => {
            c.args([
                "-liconv",
                "-lSystem",
                "-framework",
                "Security",
                "-framework",
                "CoreFoundation",
            ]);
        }
        Target::WindowsX64 => {
            c.args([
                "-ladvapi32",
                "-luserenv",
                "-lws2_32",
                "-lbcrypt",
                "-lntdll",
                "-lkernel32",
            ]);
        }
    }
    if opts.debug {
        c.arg("-g");
    }
    let out=c.output().map_err(|e|Error::new(1,1,format!("Linker '{cc}' unavailable: {e}. This development build requires a native C toolchain.")))?;
    if !out.status.success() {
        return Err(Error::new(
            1,
            1,
            format!(
                "Native linking failed:\n{}",
                String::from_utf8_lossy(&out.stderr)
            ),
        ));
    }
    if cfg!(windows) && abs.exists() {
        fs::remove_file(&abs).map_err(|e| Error::new(1, 1, e.to_string()))?;
    }
    fs::rename(&tmp, &abs).map_err(|e| Error::new(1, 1, e.to_string()))?;
    Ok(BuildResult {
        executable: abs,
        assembly,
        linker: cc,
    })
}

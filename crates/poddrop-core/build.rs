use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=csrc/poddrop_gpod.c");
    println!("cargo:rerun-if-env-changed=PKG_CONFIG_PATH");

    let cflags_out = Command::new("pkg-config")
        .args(["--cflags", "libgpod-1.0"])
        .output()
        .expect("Failed to execute pkg-config for libgpod-1.0");
    assert!(
        cflags_out.status.success(),
        "pkg-config --cflags libgpod-1.0 failed: {}",
        String::from_utf8_lossy(&cflags_out.stderr)
    );

    let libs_out = Command::new("pkg-config")
        .args(["--libs", "libgpod-1.0", "glib-2.0"])
        .output()
        .expect("Failed to execute pkg-config --libs libgpod-1.0 glib-2.0");
    assert!(
        libs_out.status.success(),
        "pkg-config --libs libgpod-1.0 failed: {}",
        String::from_utf8_lossy(&libs_out.stderr)
    );

    let cflags = String::from_utf8(cflags_out.stdout).expect("Invalid UTF-8 in cflags");
    let libs = String::from_utf8(libs_out.stdout).expect("Invalid UTF-8 in libs");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    let obj_path = out_dir.join("poddrop_gpod.o");
    let lib_path = out_dir.join("libpoddrop_gpod.a");

    let cc = env::var("CC").unwrap_or_else(|_| "cc".to_string());
    let mut compile_cmd = Command::new(&cc);
    compile_cmd
        .arg("-c")
        .arg("-O2")
        .arg("-fPIC")
        .arg("-Wall")
        .arg("-Wextra")
        .arg("csrc/poddrop_gpod.c")
        .arg("-o")
        .arg(&obj_path);

    for flag in cflags.split_whitespace() {
        compile_cmd.arg(flag);
    }

    let compile_status = compile_cmd
        .status()
        .expect("Failed to invoke C compiler for poddrop_gpod.c");
    assert!(compile_status.success(), "C compilation failed");

    let ar = env::var("AR").unwrap_or_else(|_| "ar".to_string());
    let ar_status = Command::new(&ar)
        .args(["rcs"])
        .arg(&lib_path)
        .arg(&obj_path)
        .status()
        .expect("Failed to invoke ar");
    assert!(ar_status.success(), "ar failed");

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=poddrop_gpod");

    for token in libs.split_whitespace() {
        if let Some(lib) = token.strip_prefix("-l") {
            println!("cargo:rustc-link-lib={lib}");
        } else if let Some(dir) = token.strip_prefix("-L") {
            println!("cargo:rustc-link-search=native={dir}");
        }
    }
}

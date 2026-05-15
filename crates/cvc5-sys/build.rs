fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=cvc5_wrapper.h");
    println!("cargo:rerun-if-changed=cvc5_wrapper.cc");
    println!("cargo:rerun-if-env-changed=SOTER_CVC5_ROOT");
    println!("cargo:rerun-if-env-changed=CVC5_CONFIGURE_FLAGS");

    if std::env::var("CARGO_FEATURE_LINK").is_ok() {
        build_with_cvc5();
    }
}

fn build_with_cvc5() {
    use std::{env, path::PathBuf};

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = manifest_dir.parent().unwrap().parent().unwrap();
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let external_root = env::var("SOTER_CVC5_ROOT").ok();
    let cvc5_root = external_root.as_deref().map_or_else(
        || workspace_root.join("vendor/cvc5/build/install"),
        PathBuf::from,
    );
    let cvc5_src = if external_root.is_some() {
        cvc5_root.clone()
    } else {
        workspace_root.join("vendor/cvc5")
    };
    emit_identity_metadata(external_root.is_some(), &cvc5_src);
    let include_dir = cvc5_root.join("include");
    let lib_dir = find_lib_dir(&cvc5_root);

    assert!(
        include_dir.join("cvc5/c/cvc5.h").exists(),
        "CVC5 headers not found at {}.\nRun `make cvc5` from the soter workspace root, or set SOTER_CVC5_ROOT to an installed CVC5 prefix.",
        include_dir.display()
    );
    assert!(
        lib_dir.exists(),
        "CVC5 library directory not found under {}.\nRun `make cvc5` from the soter workspace root, or set SOTER_CVC5_ROOT.",
        cvc5_root.display()
    );

    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .file("cvc5_wrapper.cc")
        .include(&include_dir)
        .flag_if_supported("-Wno-unused-parameter")
        .compile("soter_cvc5_wrapper");

    copy_runtime_libraries(&lib_dir, &out_dir);
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:LIB_DIR={}", lib_dir.display());
    println!("cargo:rustc-link-lib=dylib=cvc5parser");
    println!("cargo:rustc-link-lib=dylib=cvc5");
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir.display());
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", out_dir.display());

    #[cfg(target_os = "macos")]
    println!("cargo:rustc-link-lib=c++");
    #[cfg(target_os = "linux")]
    println!("cargo:rustc-link-lib=stdc++");
}

fn find_lib_dir(root: &std::path::Path) -> std::path::PathBuf {
    let lib = root.join("lib");
    if lib.exists() {
        return lib;
    }
    root.join("lib64")
}

fn copy_runtime_libraries(lib_dir: &std::path::Path, out_dir: &std::path::Path) {
    let Ok(entries) = std::fs::read_dir(lib_dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };

        let is_dylib = path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("dylib"));
        let is_runtime = name.starts_with("libcvc5") && (is_dylib || name.contains(".so"));
        if is_runtime {
            let _ = std::fs::copy(&path, out_dir.join(name));
        }
    }
}

fn emit_identity_metadata(external_root: bool, source_dir: &std::path::Path) {
    let source_mode = if external_root {
        "external-root"
    } else {
        "vendored"
    };
    let source_commit = git_head(source_dir).unwrap_or_else(|| "unavailable".to_string());
    let configure_flags =
        std::env::var("CVC5_CONFIGURE_FLAGS").unwrap_or_else(|_| "--no-poly".to_string());
    println!("cargo:rustc-env=SOTER_CVC5_SOURCE_MODE={source_mode}");
    println!("cargo:rustc-env=SOTER_CVC5_SOURCE_COMMIT={source_commit}");
    println!("cargo:rustc-env=SOTER_CVC5_CONFIGURE_FLAGS={configure_flags}");
}

fn git_head(source_dir: &std::path::Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(source_dir)
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let commit = String::from_utf8(output.stdout).ok()?;
    let commit = commit.trim();
    (!commit.is_empty()).then(|| commit.to_string())
}

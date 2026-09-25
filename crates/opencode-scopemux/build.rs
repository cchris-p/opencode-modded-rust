//! Build script for the native `scopemux-core` FFI provider.
//!
//! When the `native` feature is enabled this configures and builds the
//! `scopemux-core` C library (`parser_core`) and links it, along with the
//! vendored Tree-sitter static libraries, into the crate.
//!
//! The source tree is taken from `SCOPEMUX_CORE_DIR`, falling back to
//! `third_party/scopemux-core` in this repository. Use
//! `scripts/fetch-scopemux-core.sh` to populate that directory at the pinned
//! revision.
//!
//! Without the `native` feature this script does nothing, so default builds
//! never require CMake or a C toolchain.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=SCOPEMUX_CORE_DIR");
    println!("cargo:rerun-if-env-changed=SCOPEMUX_SKIP_NATIVE_BUILD");

    if env::var_os("CARGO_FEATURE_NATIVE").is_none() {
        return;
    }

    // Dev aid: compile the Rust FFI surface without building the C library
    // (`cargo check` does not link, so unresolved symbols are not an issue).
    if env::var_os("SCOPEMUX_SKIP_NATIVE_BUILD").is_some() {
        println!("cargo:warning=SCOPEMUX_SKIP_NATIVE_BUILD set; skipping scopemux-core build");
        return;
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .expect("crate is nested under the workspace root")
        .to_path_buf();

    let source_dir = env::var("SCOPEMUX_CORE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| workspace_root.join("third_party/scopemux-core"));

    if !source_dir.join("CMakeLists.txt").is_file() {
        panic!(
            "scopemux-core sources not found at {}.\n\
             Set SCOPEMUX_CORE_DIR or run scripts/fetch-scopemux-core.sh.",
            source_dir.display()
        );
    }

    println!("cargo:rerun-if-changed={}", source_dir.display());

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let build_dir = out_dir.join("scopemux-core-build");

    // scopemux-core is developed and CI-tested with GCC, where implicit
    // function declarations and some pointer-type mismatches are warnings.
    // AppleClang treats them as errors, so relax just those diagnostics for the
    // native C build.
    let relaxed_c_flags = "-Wno-implicit-function-declaration \
         -Wno-incompatible-function-pointer-types \
         -Wno-incompatible-pointer-types \
         -Wno-int-conversion";

    let configure = Command::new("cmake")
        .arg("-S")
        .arg(&source_dir)
        .arg("-B")
        .arg(&build_dir)
        .arg("-DSCOPEMUX_BUILD_TESTS=OFF")
        .arg("-DCMAKE_BUILD_TYPE=Release")
        .arg(format!("-DCMAKE_C_FLAGS={relaxed_c_flags}"))
        .status()
        .expect("failed to run cmake (is it installed?)");
    assert!(
        configure.success(),
        "scopemux-core CMake configuration failed"
    );

    let build = Command::new("cmake")
        .arg("--build")
        .arg(&build_dir)
        .arg("--target")
        .arg("parser_core")
        .status()
        .expect("failed to run cmake --build");
    assert!(build.success(), "scopemux-core parser_core build failed");

    // scopemux-core resolves Tree-sitter query files from SCMU_QUERIES_DIR at
    // runtime. Bake the built source's queries path so the provider can set it.
    println!(
        "cargo:rustc-env=SCOPEMUX_QUERIES_DIR={}",
        source_dir.join("queries").display()
    );

    // Static libraries live in the build tree and the Tree-sitter library dir.
    println!(
        "cargo:rustc-link-search=native={}",
        build_dir.join("core").display()
    );
    println!(
        "cargo:rustc-link-search=native={}",
        build_dir.join("tree-sitter-libs").display()
    );

    for lib in [
        "parser_core",
        "tree-sitter",
        "tree-sitter-c",
        "tree-sitter-cpp",
        "tree-sitter-python",
        "tree-sitter-javascript",
        "tree-sitter-typescript",
        "tree-sitter-rust",
    ] {
        println!("cargo:rustc-link-lib=static={lib}");
    }

    // System libraries the C core depends on.
    if cfg!(target_os = "linux") {
        println!("cargo:rustc-link-lib=dylib=dl");
        println!("cargo:rustc-link-lib=dylib=m");
    }
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-lib=dylib=c++");
    }
}

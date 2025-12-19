use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn push_file(list: &mut Vec<PathBuf>, path: PathBuf) {
    println!("cargo:rerun-if-changed={}", path.display());
    list.push(path);
}

fn collect_c_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_c_files(&path, out);
        } else if let Some(ext) = path.extension() {
            if ext == "c" {
                out.push(path);
            }
        }
    }
}

fn main() {
    println!("cargo:rerun-if-env-changed=MARS_SRC_DIR");

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let mars_dir = env::var("MARS_SRC_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| manifest_dir.join("../../third_party/mars/mars"));

    if !mars_dir.exists() {
        panic!("mars source dir not found: {} (set MARS_SRC_DIR to override)", mars_dir.display());
    }

    let mars_parent = mars_dir
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target = env::var("TARGET").unwrap_or_default();
    let is_android_armv7 = target_os == "android" && target.contains("armv7");

    let native_dir = manifest_dir.join("native");
    let mut cpp_files: Vec<PathBuf> = Vec::new();
    let mut c_files: Vec<PathBuf> = Vec::new();

    // wrapper + stubs
    push_file(&mut cpp_files, native_dir.join("mars_xlog_wrapper.cc"));
    push_file(&mut cpp_files, native_dir.join("strutil_stub.cc"));
    println!("cargo:rerun-if-changed={}", native_dir.join("mars_xlog_wrapper.h").display());

    // xlog core
    push_file(&mut cpp_files, mars_dir.join("xlog/src/appender.cc"));
    push_file(&mut cpp_files, mars_dir.join("xlog/src/formater.cc"));
    push_file(&mut cpp_files, mars_dir.join("xlog/src/log_base_buffer.cc"));
    push_file(&mut cpp_files, mars_dir.join("xlog/src/log_zlib_buffer.cc"));
    push_file(&mut cpp_files, mars_dir.join("xlog/src/log_zstd_buffer.cc"));
    push_file(&mut cpp_files, mars_dir.join("xlog/src/xlogger_interface.cc"));
    push_file(&mut cpp_files, mars_dir.join("xlog/crypt/log_crypt.cc"));

    // xlogger
    push_file(&mut cpp_files, mars_dir.join("comm/xlogger/xlogger.cc"));
    push_file(&mut cpp_files, mars_dir.join("comm/xlogger/xlogger_category.cc"));

    // comm basics
    push_file(&mut cpp_files, mars_dir.join("comm/autobuffer.cc"));
    push_file(&mut cpp_files, mars_dir.join("comm/boost_exception.cc"));
    push_file(&mut cpp_files, mars_dir.join("comm/mmap_util.cc"));
    push_file(&mut cpp_files, mars_dir.join("comm/ptrbuffer.cc"));
    push_file(&mut cpp_files, mars_dir.join("comm/tickcount.cc"));

    // boost libs (filesystem/system/iostreams)
    for entry in [
        mars_dir.join("boost/libs/filesystem/src"),
        mars_dir.join("boost/libs/system/src"),
        mars_dir.join("boost/libs/iostreams/src"),
    ] {
        if let Ok(entries) = fs::read_dir(&entry) {
            for ent in entries.flatten() {
                let path = ent.path();
                if path.extension().map(|e| e == "cpp").unwrap_or(false) {
                    if target_os != "windows" {
                        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            if name.contains("windows") {
                                continue;
                            }
                        }
                    }
                    push_file(&mut cpp_files, path);
                }
            }
        }
    }

    // platform-specific console + threadinfo
    if target_os == "android" {
        push_file(&mut cpp_files, mars_dir.join("xlog/jni/ConsoleLog.cc"));
        push_file(&mut cpp_files, mars_dir.join("comm/unix/xlogger_threadinfo.cc"));
    } else if target_os == "ios" || target_os == "macos" || target_os == "tvos" || target_os == "watchos" {
        push_file(&mut cpp_files, mars_dir.join("xlog/objc/objc_console.mm"));
        push_file(&mut cpp_files, mars_dir.join("comm/objc/data_protect_attr.mm"));
        push_file(&mut cpp_files, mars_dir.join("comm/objc/scope_autoreleasepool.mm"));
        push_file(&mut cpp_files, mars_dir.join("comm/objc/xlogger_threadinfo.mm"));
    } else if target_os == "ohos" || target_os == "harmony" || target_os == "harmonyos" || target.contains("ohos") {
        push_file(&mut cpp_files, mars_dir.join("xlog/ohos/ConsoleLog.cc"));
        push_file(&mut cpp_files, mars_dir.join("comm/unix/xlogger_threadinfo.cc"));
    } else {
        push_file(&mut cpp_files, mars_dir.join("xlog/unix/ConsoleLog.cc"));
        push_file(&mut cpp_files, mars_dir.join("comm/unix/xlogger_threadinfo.cc"));
    }

    // C sources
    push_file(&mut c_files, mars_dir.join("comm/xlogger/xloggerbase.c"));
    push_file(&mut c_files, mars_dir.join("comm/xlogger/loginfo_extract.c"));
    push_file(&mut c_files, mars_dir.join("comm/assert/__assert.c"));
    push_file(&mut c_files, mars_dir.join("comm/time_utils.c"));

    // micro-ecc
    if let Ok(entries) = fs::read_dir(mars_dir.join("xlog/crypt/micro-ecc-master")) {
        for ent in entries.flatten() {
            let path = ent.path();
            if path.extension().map(|e| e == "c").unwrap_or(false) {
                push_file(&mut c_files, path);
            }
        }
    }

    // zstd (only lib/*.c)
    collect_c_files(&mars_dir.join("zstd/lib"), &mut c_files);
    for path in &c_files {
        println!("cargo:rerun-if-changed={}", path.display());
    }

    // C++ build
    let mut cpp_build = cc::Build::new();
    cpp_build.cpp(true);
    cpp_build.flag_if_supported("-std=gnu++14");
    cpp_build.flag_if_supported("-fno-exceptions");

    if target_os == "android" {
        cpp_build.define("ANDROID", None);
    }

    // include paths
    cpp_build
        .include(&mars_parent)
        .include(&mars_dir)
        .include(mars_dir.join("comm"))
        .include(mars_dir.join("comm/xlogger"))
        .include(mars_dir.join("xlog"))
        .include(mars_dir.join("xlog/src"))
        .include(mars_dir.join("xlog/crypt"))
        .include(mars_dir.join("xlog/crypt/micro-ecc-master"))
        .include(mars_dir.join("boost"))
        .include(mars_dir.join("zstd/lib"))
        .include(mars_dir.join("zstd/lib/common"))
        .include(mars_dir.join("zstd/lib/compress"))
        .include(mars_dir.join("zstd/lib/decompress"))
        .include(mars_dir.join("zstd/lib/dictBuilder"))
        .include(mars_dir.join("zstd/lib/deprecated"))
        .include(mars_dir.join("zstd/lib/legacy"));

    for file in &cpp_files {
        cpp_build.file(file);
    }

    cpp_build.compile("mars_xlog_cpp");

    // C build
    let mut c_build = cc::Build::new();
    if target_os == "android" {
        c_build.define("ANDROID", None);
    }
    // clang's integrated assembler doesn't support .syntax divided in micro-ecc ARM asm.
    // Disable ARM asm on armv7 Android by forcing the generic C implementation.
    if is_android_armv7 {
        c_build.define("uECC_PLATFORM", Some("uECC_arch_other"));
    }

    c_build
        .include(&mars_parent)
        .include(&mars_dir)
        .include(mars_dir.join("comm"))
        .include(mars_dir.join("comm/xlogger"))
        .include(mars_dir.join("xlog"))
        .include(mars_dir.join("xlog/crypt"))
        .include(mars_dir.join("zstd/lib"))
        .include(mars_dir.join("zstd/lib/common"))
        .include(mars_dir.join("zstd/lib/compress"))
        .include(mars_dir.join("zstd/lib/decompress"))
        .include(mars_dir.join("zstd/lib/dictBuilder"))
        .include(mars_dir.join("zstd/lib/deprecated"))
        .include(mars_dir.join("zstd/lib/legacy"));

    for file in &c_files {
        c_build.file(file);
    }
    c_build.compile("mars_xlog_c");

    // link stdlib
    if target_os == "ios" || target_os == "macos" || target_os == "tvos" || target_os == "watchos" {
        println!("cargo:rustc-link-lib=c++");
        println!("cargo:rustc-link-lib=objc");
        println!("cargo:rustc-link-lib=framework=Foundation");
    } else {
        println!("cargo:rustc-link-lib=stdc++");
    }

    // system libs
    println!("cargo:rustc-link-lib=z");
    if target_os == "android" {
        println!("cargo:rustc-link-lib=log");
        println!("cargo:rustc-link-lib=android");
    }
    if target_os == "ohos" || target_os == "harmony" || target_os == "harmonyos" || target.contains("ohos") {
        println!("cargo:rustc-link-lib=hilog");
    }

    if target_os == "linux" || target_os == "ohos" || target_os == "harmony" || target_os == "harmonyos" {
        println!("cargo:rustc-link-lib=pthread");
    }
}

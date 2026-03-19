// MIT License
//
// Copyright (c) 2026 worksoup <https://github.com/worksoup/>
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use cmake::Config;
use std::{
    env, fs,
    path::{Path, PathBuf},
};

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    if let Ok(lib) = pkg_config::Config::new().probe("uchardet")
        // 目前与已发布版本不兼容。
        && false
    {
        let include_args: Vec<String> = lib
            .include_paths
            .iter()
            .map(|p| format!("-I{}", p.display()))
            .collect();
        let header = find_header(&lib.include_paths).unwrap_or_else(|| PathBuf::from("uchardet.h"));
        generate_bindings(out_dir, header, include_args);
    } else {
        let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
        select_cxx_stdlib(target_os);
        let patched_src = patch_src(&out_dir);
        let dst = Config::new(patched_src)
            .define("BUILD_BINARY", "OFF")
            .define("BUILD_STATIC", "ON")
            .define("BUILD_SHARED_LIBS", "OFF")
            .build();
        // 输出链接指令
        println!("cargo:rustc-link-search=native={}/lib", dst.display());
        println!("cargo:rustc-link-search=native={}/lib64", dst.display());
        println!("cargo:rustc-link-lib=static=uchardet");

        let header = PathBuf::from("uchardet/src/uchardet.h");
        let include_args = vec!["-Iuchardet/src".to_string()];
        generate_bindings(out_dir, header, include_args);
    }
}

fn patch_src(out_dir: impl AsRef<Path>) -> PathBuf {
    let original_src = Path::new("uchardet");
    if !original_src.exists() {
        panic!(
            "Original source directory '{}' not found.",
            original_src.display()
        );
    }
    if cfg!(feature = "source_no_patch") {
        return original_src.to_path_buf();
    }
    let patched_src = out_dir.as_ref().join("uchardet_patched");
    if patched_src.exists() {
        // out_dir.
        fs::remove_dir_all(&patched_src).expect("Failed to remove old patched source");
    }
    copy_dir_all(original_src, &patched_src).expect("Failed to copy source");
    let cmakelists_path = patched_src.join("CMakeLists.txt");
    let content = fs::read_to_string(&cmakelists_path).expect("Failed to read CMakeLists.txt");

    let new_content: Vec<&str> = content
        .lines()
        .filter(|line| !line.contains("-fsanitize=address"))
        .collect();

    fs::write(&cmakelists_path, new_content.join("\n")).expect("Failed to write CMakeLists.txt");
    patched_src
}

fn select_cxx_stdlib(target_os: String) {
    let cpp_stdlib = 'cpp_stdlib: {
        Some(match target_os.as_str() {
            "macos" => "c++",
            "windows" => {
                let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap();
                if target_env.as_str() == "gnu" {
                    "stdc++"
                } else {
                    break 'cpp_stdlib None;
                }
            }
            _ => "stdc++",
        })
    };
    if let Some(cpp_stdlib) = cpp_stdlib {
        println!("cargo:rustc-link-lib={}", cpp_stdlib)
    }
}

fn find_header(include_paths: &[PathBuf]) -> Option<PathBuf> {
    for dir in include_paths {
        let candidate = dir.join("uchardet.h");
        if candidate.exists() {
            return Some(candidate);
        }
        let candidate = dir.join("uchardet").join("uchardet.h");
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

fn generate_bindings(out_path: impl AsRef<Path>, header: PathBuf, include_args: Vec<String>) {
    let bindings = bindgen::Builder::default()
        .header(header.to_str().expect("header path not valid UTF-8"))
        .use_core()
        .allowlist_function("uchardet_.*")
        .allowlist_type("uchardet_t")
        .clang_args(include_args)
        .size_t_is_usize(true)
        .generate()
        .expect("Unable to generate bindings");

    bindings
        .write_to_file(out_path.as_ref().join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.as_ref().join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

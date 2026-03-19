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
use std::{env, path::PathBuf};

fn main() {
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
        generate_bindings(header, include_args);
    } else {
        let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
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
        let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
        let cmake_build_type = if profile == "release" {
            Some("Release")
        } else if target_os.as_str() != "windows" {
            None
        } else {
            Some("RelWithDebInfo")
        };
        if let Some(cpp_stdlib) = cpp_stdlib {
            println!("cargo:rustc-link-lib={}", cpp_stdlib)
        }
        eprintln!("pkg-config failed, building uchardet from source");
        let mut config = Config::new("uchardet");
        config
            .define("BUILD_BINARY", "OFF")
            .define("BUILD_STATIC", "ON")
            .define("BUILD_SHARED_LIBS", "OFF");
        if let Some(cmake_build_type) = cmake_build_type {
            config.define("CMAKE_BUILD_TYPE", cmake_build_type);
        }
        let dst = config.build();
        // 输出链接指令
        println!("cargo:rustc-link-search=native={}/lib", dst.display());
        println!("cargo:rustc-link-search=native={}/lib64", dst.display());
        println!("cargo:rustc-link-lib=static=uchardet");

        let header = PathBuf::from("uchardet/src/uchardet.h");
        let include_args = vec!["-Iuchardet/src".to_string()];
        generate_bindings(header, include_args);
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

fn generate_bindings(header: PathBuf, include_args: Vec<String>) {
    let bindings = bindgen::Builder::default()
        .header(header.to_str().expect("header path not valid UTF-8"))
        .use_core()
        .allowlist_function("uchardet_.*")
        .allowlist_type("uchardet_t")
        .clang_args(include_args)
        .size_t_is_usize(true)
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

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

use std::{
    fs,
    path::{Path, PathBuf},
};
use uchardet_git::{CharsetDetector, Error};

// 原仓库中跳过的测试。
const SKIP_TESTS: &[(&str, &str)] = &[
    ("da", "iso-8859-1"),
    ("es", "iso-8859-15"),
    ("he", "iso-8859-8"),
    ("ja", "utf-16be"),
    ("ja", "utf-16le"),
    ("zh", "gb18030"), // 额外跳过的测试：因 uchardet 未发布的 git 版本中添加了 windows-1251 编码支持，影响了测试结果。已在本仓库中额外添加了测试样本，以弥补该测试。
];

const NO_LANG_ENCODINGS: &[&str] = &["ascii", "utf-16", "utf-32"];

fn test_data_dir() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.join("uchardet-git-sys/uchardet/test")
}

fn extra_test_data_dir() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.join("tests/data")
}

fn run_test_file(
    file_path: &Path,
    expected_lang: &str,
    expected_charset: &str,
) -> Result<(), Error> {
    let data = fs::read(file_path).expect("无法读取测试文件");
    let mut detector = CharsetDetector::new();
    detector.feed_data(&data)?;
    let candidates = detector.detect();
    let candidate = candidates.best().expect("检测结果为空");

    let detected_charset = candidate.encoding_name()?;
    let detected_lang = candidate.language()?;

    assert_eq!(
        detected_charset.to_lowercase(),
        expected_charset.to_lowercase(),
        "编码不匹配: {:?}",
        file_path
    );

    if NO_LANG_ENCODINGS.contains(&expected_charset) {
        assert!(
            detected_lang.is_none(),
            "编码 {} 不应有语言检测，但得到 {:?}",
            expected_charset,
            detected_lang
        );
    } else {
        assert_eq!(
            detected_lang.as_ref().map(|s| s.to_lowercase()),
            Some(expected_lang.to_lowercase()),
            "语言不匹配: {:?}",
            file_path
        );
    }
    Ok(())
}

/// 运行 AutoEncodingReader 的解码测试（验证解码后的 UTF-8 内容是否与预期一致）
#[cfg(feature = "auto_encoding_reader")]
fn test_auto_encoding_reader_for_file(
    file_path: &Path,
    expected_encoding_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Read;
    use uchardet_git::auto_encoding_reader::AutoEncodingReader;

    let Some(expected_encoding) = uchardet_git::encoding::to_standard(expected_encoding_name)
    else {
        eprintln!(
            "警告: 编码 {} 不被 encoding_rs 支持，跳过测试",
            expected_encoding_name
        );
        return Ok(());
    };

    // 使用预期编码解码得到参考 UTF-8 字符串
    let raw_bytes = fs::read(file_path)?;
    let (expected_str, _, had_errors) = expected_encoding.decode(&raw_bytes);
    if had_errors {
        eprintln!(
            "警告: 预期编码 {} 解码 {} 时出现替换字符",
            expected_encoding_name,
            file_path.display()
        );
    }

    // 使用 AutoEncodingReader 解码（将预期编码作为后备，确保即使检测失败也能解码）
    let file = fs::File::open(file_path)?;
    let fallbacks = &[expected_encoding];
    let mut reader = AutoEncodingReader::new_with_fallbacks_default(file, fallbacks)
        .map_err(|e| format!("创建 AutoEncodingReader 失败: {}", e))?;

    let mut decoded_bytes = Vec::new();
    reader.read_to_end(&mut decoded_bytes)?;
    let decoded_str = String::from_utf8(decoded_bytes)?;

    if decoded_str != expected_str {
        return Err(format!("解码内容不匹配，文件: {}", file_path.display()).into());
    }

    Ok(())
}

#[test]
fn test_uchardet_data() {
    let test_dir = test_data_dir();
    let extra_test_dir = extra_test_data_dir();
    assert!(test_dir.exists(), "测试数据目录不存在: {:?}", test_dir);
    assert!(
        extra_test_dir.exists(),
        "额外测试数据目录不存在: {:?}",
        extra_test_dir
    );

    for (is_extra, entry) in fs::read_dir(test_dir)
        .expect("读取测试目录失败")
        .map(|x| (false, x))
        .chain(
            fs::read_dir(extra_test_dir)
                .expect("读取额外测试目录失败")
                .map(|x| (true, x)),
        )
    {
        let entry = entry.expect("无法读取目录项");
        let lang_dir = entry.path();
        if !lang_dir.is_dir() {
            continue;
        }
        let lang = lang_dir.file_name().unwrap().to_str().unwrap();
        if lang.len() != 2 {
            continue;
        }

        for file_entry in fs::read_dir(&lang_dir).expect("读取语言目录失败") {
            let file_entry = file_entry.expect("无法读取文件项");
            let file_path = file_entry.path();
            if !file_path.is_file() {
                continue;
            }

            let file_name = file_path.file_prefix().unwrap().to_str().unwrap();
            let expected_charset = file_name;

            if SKIP_TESTS.contains(&(lang, expected_charset)) && !is_extra {
                eprintln!("跳过已知失败的测试: {}/{}", lang, expected_charset);
                continue;
            }

            if is_extra {
                println!("正在运行额外的测试: {}/{}", lang, expected_charset);
            }

            // 运行原始的编码检测测试
            if let Err(e) = run_test_file(&file_path, lang, expected_charset) {
                panic!("编码检测测试失败 {:?}: {}", file_path, e);
            }

            #[cfg(feature = "auto_encoding_reader")]
            if let Err(e) = test_auto_encoding_reader_for_file(&file_path, expected_charset) {
                panic!("AutoEncodingReader 解码测试失败 {:?}: {}", file_path, e);
            }
        }
    }
}

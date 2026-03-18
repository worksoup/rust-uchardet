use std::fs;
use std::path::{Path, PathBuf};
use uchardet::{Error, UCharsetDetector};

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
    let mut detector = UCharsetDetector::new();
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

#[test]
fn test_uchardet_data() {
    let test_dir = test_data_dir();
    let extra_test_dir = extra_test_data_dir();
    assert!(test_dir.exists(), "测试数据目录不存在: {:?}", test_dir);
    assert!(
        extra_test_dir.exists(),
        "额外测试数据目录不存在: {:?}",
        test_dir
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

            if let Err(e) = run_test_file(&file_path, lang, expected_charset) {
                panic!("测试失败 {:?}: {}", file_path, e);
            }
        }
    }
}

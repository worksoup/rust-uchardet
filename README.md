# uchardet-git

**本 README 由 AI 撰写。**

[![crates.io](https://img.shields.io/crates/v/uchardet-git.svg)](https://crates.io/crates/uchardet-git)
[![docs.rs](https://docs.rs/uchardet-git/badge.svg)](https://docs.rs/uchardet-git)

**uchardet** 是一个用于检测未知字符编码的 Rust 库，它简单封装了 [uchardet](https://www.freedesktop.org/wiki/Software/uchardet/) C++ 库。
该库能够分析字节流，并返回可能的编码名称及置信度，同时可选支持将结果映射到 [encoding_rs](https://crates.io/crates/encoding_rs) 中定义的 Web 兼容编码。

## 特性

- 检测字节流的编码，返回编码名称（如 `"UTF-8"`, `"GB18030"`）；
- 获取多个候选编码及其置信度；
- （可选）通过 `encoding` 特性支持将检测结果转换为 `encoding_rs::Encoding`，便于后续编解码；
  > [!IMPORTANT] 注意
  >
  > `encoding_rs` 遵循 WHATWG 标准，而 `uhardet` 返回的编码名称实际上是 GNU `libiconv` 兼容的。大部分情况下，这没有问题，但有以下两点需注意：
  > 1. 将检测结果转换为 `encoding_rs::Encoding` 时，可能会得到 `None`;
  > 2. 一些编码（据我所知，它们是 `ISO-8859-1` 和 `EUC-KR`）可以转换为 `encoding_rs::Encoding`, 但使用 `encoding_rs` 解码可能会得到错误结果。
  >
  >       只考虑 GNU `libiconv` 能以 `uchardet` 的检测结果正确解码字节流的情况：
  >
  >       | 检测结果     | 对应 `Encoding` | 差异说明                                                      |
  >       | ------------ | --------------- | ------------------------------------------------------------- |
  >       | `ISO-8859-1` | `windows-1252`  | 在 GNU `libiconv` 定义下，两者并不等价。                      |
  >       | `EUC-KR`     | `euc-kr`        | 名称相同，但 `encoding_rs` 可能与 GNU `libiconv` 定义有差异。 |

- （可选）通过 `auto_encoding_reader` 特性提供自动检测并转换为 UTF-8 的读取器 `AutoEncodingReader`;
  > [!IMPORTANT] 它不会处理如上所述的问题。
- 从捆绑源码编译 `uchardet` 库（需 CMake 和 C++ 编译器）。
  > [!IMPORTANT] 编译过程未经广泛测试。

## 使用示例

```rust
use uchardet_git::detect_encoding_name;

let data = &[
 0x46, 0x93, 0x72, 0x61, 0x6e, 0xe7, 0x6f, 0x69, 0x73, 0xe9, 0x94,
];
let encoding = detect_encoding_name(data).expect("检测失败");
assert_eq!(encoding, "WINDOWS-1252");

// 启用 `encoding` 特性后，可返回 `encoding_rs::Encoding`
#[cfg(feature = "encoding")]
{
    use uchardet_git::detect_encoding;
    let enc = detect_encoding(data).expect("检测失败");
    assert_eq!(enc.name(), "windows-1252");
}
```

### 自动转码 Reader（需启用 `auto_encoding_reader` 特性）

```rust
use std::io::Read;
use uchardet_git::auto_encoding_reader::AutoEncodingReader;

let data = b"\x93\x72\x61\x6e\xe7\x6f\x69\x73\xe9";
let cursor = std::io::Cursor::new(data);
let mut reader = AutoEncodingReader::new(cursor).expect("创建读取器失败");
// assert_eq!(reader.encoding_name(), &Some("".to_owned()));
assert_eq!(reader.decoder().encoding().name(), "gb18030");

let mut utf8_string = String::new();
reader.read_to_string(&mut utf8_string).unwrap();
println!("解码后的文本: {}", utf8_string);
```

### 高级用法

```rust
use uchardet_git::CharsetDetector;

let mut detector = CharsetDetector::new();
detector.feed_data(b"some data").unwrap();
detector.feed_data(b" more data").unwrap();
let candidates = detector.detect();

for cand in &candidates {
    println!("编码: {}", cand.encoding_name().unwrap());
    println!("置信度: {}", cand.confidence());
    if let Some(lang) = cand.language().unwrap() {
        println!("语言: {}", lang);
    }
}
```

## 安装

在 `Cargo.toml` 中添加：

```toml
[dependencies]
uchardet-git = "0.0.6"
```

默认开启 `encoding` 和 `auto_encoding_reader` 特性，如需禁用：

```toml
uchardet-git = { version = "0.0.6", default-features = false }
```

构建脚本会自动编译项目内 `uchardet-git-sys/uchardet` 子模块中的源码。编译需要以下工具：

- Rust 和 Cargo
- C++ 编译器（如 `g++`, `clang++` 或 MSVC）
- CMake ≥ 3.5

**编译过程未经广泛测试。**

## 许可证

本项目采用 **MIT 许可证**。详情参见 [LICENSE](LICENSE) 文件。

## 鸣谢

- 原始 C++ 库 [uchardet](https://www.freedesktop.org/wiki/Software/uchardet/) 的开发者。
- 本 Rust 包装受到 [rust-uchardet](https://github.com/emk/rust-uchardet) 早期版本的启发。

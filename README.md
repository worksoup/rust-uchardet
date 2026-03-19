## uchardet-git

**本 README 由 AI 撰写。**

[![crates.io](https://img.shields.io/crates/v/uchardet.svg)](https://crates.io/crates/uchardet)
[![docs.rs](https://docs.rs/uchardet/badge.svg)](https://docs.rs/uchardet)

**uchardet** 是一个用于检测未知字符编码的 Rust 库，它简单封装了 [uchardet](https://www.freedesktop.org/wiki/Software/uchardet/) C++ 库。
该库能够分析字节流，并返回可能的编码名称及置信度，同时可选支持将结果映射到 [encoding_rs](https://crates.io/crates/encoding_rs) 中定义的 Web 兼容编码。

### 特性
- 检测字节流的编码，返回编码名称（如 `"UTF-8"`, `"GB18030"`）；
- 获取多个候选编码及其置信度；
- （可选）通过 `encoding` 特性支持将检测结果转换为 `encoding_rs::Encoding`，便于后续编解码；
- （可选）通过 `auto_encoding_reader` 特性提供自动检测并转换为 UTF-8 的读取器 `AutoEncodingReader`;
- 自动链接系统预装的 `uchardet` 库（目前不支持），若无则从源码编译（需 CMake 和 C++ 编译器）。

### 使用示例

```rust
use uchardet::detect_encoding_name;

let data = &[
	0x46, 0x93, 0x72, 0x61, 0x6e, 0xe7, 0x6f, 0x69, 0x73, 0xe9, 0x94,
];
let encoding = detect_encoding_name(data).expect("检测失败");
assert_eq!(encoding, "WINDOWS-1252");

// 启用 `encoding` 特性后，可返回 `encoding_rs::Encoding`
#[cfg(feature = "encoding")]
{
    use uchardet::detect_encoding;
    let enc = detect_encoding(data).expect("检测失败");
    assert_eq!(enc.name(), "windows-1252");
}
```

### 自动转码读取器（需启用 `auto_encoding_reader` 特性）

```rust
use std::io::Read;
use uchardet::auto_encoding_reader::AutoEncodingReader;

let data = b"\x93\x72\x61\x6e\xe7\x6f\x69\x73\xe9";
let cursor = std::io::Cursor::new(data);
let mut reader = AutoEncodingReader::new(cursor).expect("创建读取器失败");

let mut utf8_string = String::new();
reader.read_to_string(&mut utf8_string).unwrap();
println!("解码后的文本: {}", utf8_string);
```

### 安装

在 `Cargo.toml` 中添加：

```toml
[dependencies]
uchardet = "0.0.3"
```

默认开启 `encoding` 和 `auto_encoding_reader` 特性，如需禁用：

```toml
uchardet = { version = "0.0.3", default-features = false }
```

### 获取 uchardet 库

#### 使用系统库（目前不支持）
**本库与 uchardet 最新 git 版本兼容，而与已发布版本不兼容。**

#### 自动编译捆绑源码
构建脚本会自动编译项目内 `uchardet-git-sys/uchardet` 子模块中的源码。编译需要以下工具：
- Rust 和 Cargo
- C++ 编译器（如 `g++`, `clang++` 或 MSVC）
- CMake ≥ 3.5

此过程未经测试。

### API 概览

#### 主要类型
- `CharsetDetector`：核心检测器，支持分块喂入数据。
- `Candidates`：检测结果候选列表，支持迭代和索引访问。
- `Candidate`：单个候选，包含编码名称、置信度和语言（若有）。
- `AutoEncodingReader`：自动检测编码并实时转换为 UTF-8 的读取器。

#### 便捷函数
- `detect_encoding_name(data: impl AsRef<[u8]>) -> Result<String, Error>`  
  返回最可能的编码名称（字符串形式）。
- `detect_encoding(data: impl AsRef<[u8]>) -> Result<&'static encoding_rs::Encoding, Error>`  
  返回 `encoding_rs::Encoding` 引用（需启用 `encoding` 特性）。

#### 高级用法
```rust
use uchardet::CharsetDetector;

let mut detector = CharsetDetector::new();
detector.feed_data(b"some data")?;
detector.feed_data(b" more data")?;
let candidates = detector.detect();

for cand in &candidates {
    println!("编码: {}", cand.encoding_name()?);
    println!("置信度: {}", cand.confidence());
    if let Some(lang) = cand.language()? {
        println!("语言: {}", lang);
    }
}
```

### 许可证
本项目采用 **MIT 许可证**。详情参见 [LICENSE](LICENSE) 文件。

### 鸣谢
- 原始 C++ 库 [uchardet](https://www.freedesktop.org/wiki/Software/uchardet/) 的开发者。
- 本 Rust 包装受到 [rust-uchardet](https://github.com/emk/rust-uchardet) 早期版本的启发。

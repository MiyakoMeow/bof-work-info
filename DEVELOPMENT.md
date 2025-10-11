# 开发环境设置

本项目使用 Nix Flakes 提供可重现的开发环境。

## 前置要求

- [Nix](https://nixos.org/download.html) (推荐使用 [Nix Flakes](https://nixos.wiki/wiki/Flakes))
- [direnv](https://direnv.net/) (可选，用于自动加载环境)

## 设置开发环境

### 方法 1: 使用 direnv (推荐)

1. 安装 direnv
2. 在项目根目录运行 `direnv allow`
3. 每次进入项目目录时，环境会自动加载

### 方法 2: 手动加载

```bash
nix develop
```

## 环境特性

- 使用 `nixpkgs-unstable` 分支
- 集成 `rust-overlay` 提供最新 Rust 工具链
- 包含 Rust 开发所需的所有工具：
  - rustc, cargo
  - rust-analyzer
  - clippy
  - cargo-watch, cargo-expand, cargo-udeps 等
- 自动设置环境变量：
  - `RUST_BACKTRACE=1`
  - `RUST_LOG=debug`
  - `RUST_SRC_PATH` 指向 rust-src

## 项目结构

这是一个 Rust 工作区项目，包含以下成员：

- `fetcher/` - 数据获取工具
- `downloader/` - 数据下载工具

所有依赖都在根目录的 `Cargo.toml` 中统一管理。

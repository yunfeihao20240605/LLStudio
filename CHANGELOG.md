# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [v0.1.2] - 2026-08-25

### 修复 (Bug Fixes)
- 修复 macOS / Windows 下 `mpv_opengl_init_params` 结构体初始化方式不兼容旧版 mpv 头文件导致的编译错误
- 修复 `media_bridge.rs` 中 `use std::process::Command` 缺失导致的 `E0433` 编译错误
- 修复 Windows 链接器 `LNK1181: cannot open input file 'mpv.lib'` 错误——`build.rs` 新增对 `MPV_PREFIX` 根目录的 `rustc-link-search`，兼容 shinchiro dev 包的目录结构

### 优化 (Improvements)
- 重命名所有发布安装包为 `LLStudio` 前缀，文件名更简洁统一
  - `LLStudio-Setup.exe`（Windows）
  - `LLStudio_amd64.deb`（Linux）
  - `LLStudio-macOS-arm64.dmg`（macOS Apple Silicon）
  - `LLStudio-macOS-x86_64.dmg`（macOS Intel）
- CI 矩阵新增 `macos-15` 与 `macos-15-intel` runner，覆盖 Apple Silicon 与 Intel 双架构

---

## [v0.1.1] - 2026-08-24

### 新增 (Features)
- 初始多平台 CI/CD 发布流程（Linux `.deb`、macOS `.dmg`、Windows `.exe`）
- 基于 mpv 的视频播放后端（`mpv_video_item.cpp`）
- QML 与 Rust 混合架构（cxx-qt 桥接层）

---

## [v0.1.0] - 2026-08-01

### 新增 (Features)
- 项目初始化
- 英语听力学习核心功能框架
- 字幕、波形、录音、笔记、AI 辅导等模块骨架


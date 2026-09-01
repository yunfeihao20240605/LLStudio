# 发布新版本

本文说明 LLStudio 从准备版本到 GitHub Release 发布的完整流程。

当前项目使用 Cargo workspace 统一管理版本号，GitHub Actions 通过推送版本标签自动构建多平台安装包。

## 发布前准备

在仓库根目录执行以下命令，确认当前分支和工作区状态：

```bash
git status
git branch --show-current
git log -1 --oneline
```

发布前应确认：

- 代码已经合并到准备发布的分支，通常是 `main`
- 没有未确认的临时修改
- 本次版本的功能和已知限制已经确认
- 版本号、CHANGELOG 章节和 Git 标签名称保持一致

发布前不要求在本地构建安装包。正式构建由推送版本标签后触发的 GitHub Actions 完成。

## 更新版本号

编辑仓库根目录的 `Cargo.toml`，修改 workspace 版本号：

```toml
[workspace.package]
version = "0.2.0"
```

所有 crate 使用 `version.workspace = true`，因此通常只需要修改根目录的 `Cargo.toml`。

应用界面中的版本号通过 `CARGO_PKG_VERSION` 和 `Qt.application.version` 获取，不需要再单独修改 QML 文本。

如果 `Cargo.lock` 中的本地 package 版本发生变化，将 `Cargo.lock` 一并提交；不要手动修改锁文件内容。

## 更新 CHANGELOG

在 `CHANGELOG.md` 顶部增加对应版本章节。章节标题必须与 Git 标签完全一致：

```markdown
## [v0.2.0] - 2026-08-30
```

建议按照以下分类记录变更：

- `新增 (Features)`：新功能和用户可见能力
- `优化 (Improvements)`：交互、性能、界面和内部流程改进
- `修复 (Bug Fixes)`：已解决的问题
- `已知限制 (Known Limitations)`：发布时仍然存在的限制

发布工作流会从 `CHANGELOG.md` 中提取 `## [v0.2.0]` 到下一个版本章节之间的内容，作为 GitHub Release 的发布说明。

如果没有对应章节，工作流只能生成默认说明：

```text
Release v0.2.0
```

## 可选本地检查

本地检查不是发布的前置条件。如果希望在推送前快速确认差异，可以运行：

```bash
git diff --check
```

如果希望提前发现 Rust 编译或测试问题，可以额外运行：

```bash
cargo check
```

如果要运行测试：

```bash
cargo test --workspace
```

在 macOS 上也可以选择运行本地开发脚本确认 Qt、FFmpeg、mpv 和 QML 资源正常：

```bash
chmod +x scripts/run-macos-dev.sh
./scripts/run-macos-dev.sh
```

如果需要在本地预览 macOS 安装包，可以手动执行：

```bash
chmod +x scripts/build-macos-dmg.sh
./scripts/build-macos-dmg.sh
```

以上本地构建和测试均为可选步骤，不影响后续创建标签和触发远程发布。

## 提交版本修改

确认版本号和更新日志无误后提交：

```bash
git add Cargo.toml Cargo.lock CHANGELOG.md
git diff --cached --check
git commit -m "Release v0.2.0"
```

如果 `Cargo.lock` 没有变化，可以只添加实际修改的文件：

```bash
git add Cargo.toml CHANGELOG.md
```

提交后确认版本已经写入构建元数据：

```bash
cargo metadata --no-deps --format-version 1 | rg '"name":"els-app"|"version":"0.2.0"'
```

## 创建版本标签

使用带说明的 annotated tag：

```bash
git tag -a v0.2.0 -m "Release v0.2.0"
```

检查标签指向的提交：

```bash
git show --stat --oneline v0.2.0
git tag --list "v0.2.0"
```

标签必须写成 `v0.2.0`，不能只写 `0.2.0`。当前发布工作流监听的是：

```yaml
on:
  push:
    tags:
      - 'v*'
```

## 推送并触发自动发布

先推送代码提交，再推送版本标签：

```bash
git push origin main
git push origin v0.2.0
```

推送 `v0.2.0` 后，`.github/workflows/release.yml` 会自动启动发布流程。

## 自动构建内容

工作流会构建以下平台：

- Windows x64：`LLStudio-Setup-<version>.exe`
- Linux x64：`english-learning-studio_<version>_amd64.deb`
- macOS Apple Silicon：`LLStudio-macOS-arm64-<version>.dmg`
- macOS Intel：`LLStudio-macOS-x86_64-<version>.dmg`

构建流程包括：

1. 检出带版本标签的代码
2. 安装 Rust、Qt、FFmpeg 和 mpv 依赖
3. 编译 `els-app` release 版本
4. 生成对应平台的安装包
5. 执行平台安装包验证脚本
6. 从 `CHANGELOG.md` 提取发布说明
7. 创建 GitHub Release 并上传安装包

## 检查 GitHub Actions

打开 GitHub 仓库的 `Actions` 页面，找到 `Build and Release Multi-platform Packages` 工作流。

需要确认：

- Windows、Linux、macOS Apple Silicon 和 macOS Intel 任务均完成
- 安装包验证步骤通过
- GitHub Release 已创建
- Release 页面包含 `v0.2.0` 的发布说明
- 四个平台的安装包均已上传

如果某个平台失败，不要立即删除标签重新发布。先查看失败任务日志，修复问题后使用新的补丁版本，例如 `v0.2.1`，避免复用已经推送过的版本标签。

## 发布后的验证

下载各平台安装包并至少验证以下功能：

- 应用可以正常启动
- “文件”菜单可以打开视频和音频
- 音频可以播放、暂停、定位和调节速度
- 波形可以正常加载，高倍率下 bin 不会被错误连接
- 字幕、选区、片段、笔记和训练流程正常
- 关于 LLStudio 面板显示 `LLStudio · v0.2.0`
- 浅色和深色主题下主要界面显示正常

## 版本发布示例

以 `v0.3.0` 为例，完整发布命令如下。这里不包含本地构建命令：

```bash
git status

# 修改 Cargo.toml 和 CHANGELOG.md 后
git diff --check

git add Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "Release v0.3.0"

git tag -a v0.3.0 -m "Release v0.3.0"
git push origin main
git push origin v0.3.0
```

发布完成后，版本号、Git 标签、GitHub Release 和安装包名称应保持一致。

# 在 Windows 本地构建安装包

在 PowerShell 中进入仓库根目录，准备以下工具并加入 `PATH`：

Windows 构建使用 `scripts/windows-dependencies.json` 中固定的 FFmpeg/mpv 版本和 SHA-256，构建脚本不会再自动选择第三方仓库的 `latest` 资产。修改依赖版本时，应同时更新资产名称、发布标签和哈希，并先在隔离环境中完成 Defender/VirusTotal 检查。

- Rust/Cargo 和 `rustup`
- Qt 6（需要 `windeployqt`）
- 7-Zip（命令名为 `7z`）
- NSIS（命令名为 `makensis.exe`）

执行：

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\scripts\build-windows-installer.ps1
```

脚本会自动下载 x64 shared FFmpeg，以及 mpv 的 x64 development/runtime 压缩包；随后使用 MSVC 构建 `x86_64-pc-windows-msvc`，由 mpv 导出定义生成 MSVC `mpv.lib`，收集 Qt/FFmpeg/mpv DLL、运行依赖校验并生成带版本号的 `LLStudio-Setup-<version>.exe`。发布构建同时保留 `els-app.pdb` 作为私有崩溃分析符号，不会打入安装包。

本地执行前请在 **x64 Native Tools Command Prompt for VS** 中运行，或先加载 Visual Studio 的 MSVC 开发环境，使 `cl.exe`、`lib.exe` 和 Windows SDK 可用。`scripts/mpv-msvc.def` 用于把 mpv DLL 的公开 C API 生成 MSVC 导入库；macOS 和 Linux 不会执行这段逻辑。

如果依赖已经安装在自定义目录，可指定：

```powershell
.\scripts\build-windows-installer.ps1 `
  -FfmpegDir D:\ffmpeg `
  -MpvDevDir D:\mpv-dev `
  -MpvRuntimeDir D:\mpv-runtime
```

Qt 的 `windeployqt` 必须能从 `PATH` 找到；如果 Qt 的 CMake 目录不在默认位置，可先设置：

```powershell
$env:Qt6_DIR = "C:\Qt\6.6.3\mingw_64\lib\cmake\Qt6"
```

输出安装程序为仓库根目录下的 `LLStudio-Setup-<version>.exe`，其中版本号由 `cargo metadata` 自动读取。

安装程序版本由 `cargo metadata` 自动读取 `els-app` 的版本，不需要修改 `installer.nsi`。
发布时应先同步修改根目录 `Cargo.toml` 的 workspace 版本号，再创建相同版本的 `v*` Git 标签；如果两者不一致，GitHub Actions 会直接失败。

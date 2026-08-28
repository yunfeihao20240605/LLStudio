# 在 macOS 本地构建 DMG

在仓库根目录执行：

```bash
chmod +x scripts/build-macos-dmg.sh
./scripts/build-macos-dmg.sh
```

脚本会自动识别 Apple Silicon 或 Intel：

- Apple Silicon 使用 `aarch64-apple-darwin` 和 Homebrew FFmpeg。
- Intel 使用 `x86_64-apple-darwin` 和 FFmpeg 8.1.2，以匹配 `ffmpeg-next 8.1.0`。

需要预先安装 Homebrew 和 Rust。脚本会自动安装 Qt、mpv、FFmpeg、`pkg-config` 和 `create-dmg`。如果 Qt 安装在非 Homebrew 路径，可指定：

```bash
QT_PREFIX=/path/to/Qt/6.6.3/macos ./scripts/build-macos-dmg.sh
```

输出文件位于 `dist/LLStudio-macOS-arm64.dmg` 或 `dist/LLStudio-macOS-x86_64.dmg`。

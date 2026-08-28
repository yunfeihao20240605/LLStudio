# macOS 本地开发运行

不要直接运行可能由旧 Homebrew FFmpeg 版本链接生成的 `target/debug/els-app`。使用：

```bash
chmod +x scripts/run-macos-dev.sh
./scripts/run-macos-dev.sh
```

项目使用 `ffmpeg-next 8.1.0`，本地开发需要 FFmpeg 8.1.2 和 Homebrew mpv。脚本会设置 `PKG_CONFIG_PATH`，检测旧的 FFmpeg 链接并在必要时清理后重新构建。

如果 FFmpeg 或 mpv 安装在自定义目录：

```bash
FFMPEG_PREFIX=/path/to/ffmpeg@8.1.2 \
MPV_PREFIX=/path/to/mpv \
./scripts/run-macos-dev.sh
```

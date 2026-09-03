# LLStudio 项目协作规则

## 发布规则

执行任何版本发布前，必须先阅读并以以下文档为准：

[docs/release.md](docs/release.md)

发布流程至少必须确认以下事项：

- 发布代码已合并到准备发布的分支，通常为 `main`；
- 工作区中没有未确认的临时修改；
- 根目录 `Cargo.toml` 中的 workspace 版本号已更新；
- `Cargo.lock` 中的本地 package 版本发生变化时已一并提交，且未手动修改锁文件内容；
- `CHANGELOG.md` 顶部存在与 Git 标签完全一致的版本章节；
- 版本修改已经提交；
- 使用 annotated tag，格式为 `v<版本号>`；
- 先推送 `main`，再推送版本标签；
- GitHub Actions 已完成 Windows、Ubuntu 22.04、Ubuntu 24.04、macOS Apple Silicon 和 macOS Intel 五个平台的构建与验证；
- GitHub Release 已包含五个平台的安装包和对应发布说明；
- 发布失败时先检查失败日志，修复后使用新的补丁版本，不复用已经推送过的版本标签；
- 发布后验证应用启动、媒体播放、波形、字幕、片段、笔记、训练流程及主题显示。

发布前应执行或确认：

```bash
sed -n '1,260p' docs/release.md
git status
git branch --show-current
git log -1 --oneline
```

详细命令、版本命名和安装包名称以 `docs/release.md` 为唯一准则。

# CI/DevOps Journal — Release Workflow 与文档门禁

- **agent**: agent-ci-release
- **date**: 2026-10-12
- **scope**: `.github/workflows/release.yml`（新建）、`.github/workflows/quality.yml`（追加 step）、
  `docs/refactor/journal-ci-release.md`（本文件）。**仅允许修改/新建 `.github/**` + 本 journal**，未触碰其他文件。

## 变更

### 1. 新建 `.github/workflows/release.yml`
- **触发**：
  - `push.tags: ['v*']` —— 正式发布。**故意不加 `paths` 过滤**：tag push 事件下 paths/diff
    语义不明确（与既有 commit 对比，容易漏触发）。与 `main.yml` 无冲突：其 push 触发已
    `paths-ignore: '.github/**'`，不会对同一 tag 重复跑构建。
  - `workflow_dispatch`（限 main）：手动预发布，input `prerelease`（默认 `true`）→
    `gh release create --prerelease`。tag push 一律按正式发布。
- **职责边界（防冲突）**：本 workflow **不构建**固件/IPK/APK/Rust 二进制。构建分别归属
  `main.yml`（OpenWrt SDK）与 `rust-build.yml`。release 只做两件事：生成 notes、创建 Release。
- **release notes 生成**：内联 python（heredoc，`set -eu`）解析 `CHANGELOG.md`：
  - 匹配 `## [<version>]` 段落（Keep a Changelog 风格，允许标题带后缀）；
  - tag 规范化 `v3.4.0_rc3` → `3.4.0_rc3`，并兼容连字符变体 `3.4.0-rc3`；
  - 版本段落缺失时**回退 `[Unreleased]`**（header 标注"段落待补充"）；再缺失用 tag 名兜底。
  - 已对真实 CHANGELOG 冒烟测试三种输入（存在/不存在版本段落）。
- **artifact 提示**：**不跨 workflow 拉取产物**（Actions artifact 有过期时间、需 token，
  且 rust-build 与本 workflow 是独立 workflow）。仅在 notes 追加下载链接模板：
  - SDK 固件 → `.../actions/workflows/main.yml`
  - Rust 二进制 → `.../actions/workflows/rust-build.yml`（musl 双目标）
  - 并说明 `publish-existing-release.yml` 仍是从指定构建 run 手动打包上传的通道。
- **创建 Release**：`gh CLI`（`GH_TOKEN: github.token`，`permissions.contents: write`）。
  幂等：`gh release view` 已存在则 `gh release edit` 更新 notes/prerelease 标记，否则 create
  （`--verify-tag` 确认 tag 已存在）。POSIX 兼容写法处理空数组展开
  (`"${ARR[@]+${ARR[@]}}"`)，`bash -n` 通过。

### 2. `quality.yml` 追加 markdownlint（可选）
- `check-all` job 内、shell 回归测试前追加 step：
  `npx --yes markdownlint-cli '*.md' 'docs/**/*.md' --ignore node_modules`，
  **`continue-on-error: true`**——文档 lint 告警不阻塞质量门禁（与 lua best-effort step 同策略）。
- 未采纳 ruff：仓库无 Python 源码包（仅 scripts 内联 python），无意义。

## 验证记录
| 项目 | 结果 |
| --- | --- |
| 全部 8 个 `.github/workflows/*.yml` `yaml.safe_load` | PASS |
| release.yml + quality.yml 全部 `run:` 块 `bash -n` | PASS |
| CHANGELOG 提取逻辑冒烟（`v3.4.0_rc3` / `v9.9.9` / `v3.4.0`） | PASS（均正确回退 Unreleased，notes_len=6468） |
| `main.yml` 触发面复核 | push/pull_request 均 `paths-ignore: .github/**`，无冲突 |

## 已知边界
- 现阶段 CHANGELOG 只有 `[Unreleased]` / `[Base]` 段落，正式发版前应把内容切到
  `## [3.4.0_rc3]` 段落，否则 notes 会整体取 Unreleased 并标注"待补充"。
- dispatch 触发时 checkout 的是默认分支；tag push 触发时 checkout 的是 tag——符合各自语义。
- release.yml 未做任何构建，因此 tag push 时 main.yml（若触发）与本 workflow 并行，
  Release 可能早于产物打包出现——产物走 Actions 页链接，属预期行为。

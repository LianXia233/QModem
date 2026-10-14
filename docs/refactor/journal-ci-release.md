# CI/DevOps Journal — Release Workflow 与文档门禁

- **agent**: agent-ci-release
- **date**: 2026-10-12
- **scope**: `.github/workflows/release.yml`（新建）、`.github/workflows/quality.yml`（追加 step）、
  `docs/refactor/journal-ci-release.md`（本文件）。**仅允许修改/新建 `.github/**` + 本 journal**，未触碰其他文件。

## 变更

### -1. 修复自动发版链路断链（2026-10-14，v1.1.1）

**问题一（手动触发不上传 Release）**：`main.yml` 的 `job_release_artifacts` 要求
`push_type == 'tag'`，而 `workflow_dispatch` 在 `job_prepare` 一律被判为 `branch`，
导致手动编译即使全部成功，「发布产物」job 也永远被 skipped（实测 run 38504467881
三个构建变体全部 success、发布产物 skipped）。

**修复**：`workflow_dispatch` 增加可选 `tag` 输入；填写后 `determine_push` 将本次
运行判为 `tag` 发布，构建产物上传到该已存在 Release（用于修复后补发资产）；
留空保持只编译不发布。

**问题二（tagger 自动打的标签不触发编译——更深层的断链）**：v1.1.1 发版时发现
tagger.yml 自动推送的 tag（github-actions[bot] + GITHUB_TOKEN）**不会触发任何
push 事件工作流**——GitHub 出于防递归触发的安全设计，主动抑制内置 GITHUB_TOKEN
产生的 push 事件。即 `main.yml` 的 `push: tags: ['v*']` 对自动标签永远失效。
v1.1.0 当时成功纯属人工推送 tag 的巧合。

**修复**：
- `main.yml` 增加 `workflow_call` 触发器（input `tag` 必填）；
- `tagger.yml` 打完标签后 `uses: ./.github/workflows/main.yml` 直接调用编译+发布，
  绕开 push 事件抑制问题；
- `determine_push` 统一用 `inputs.tag`（dispatch 与 call 共用）判定发布。

**手动补发通道**：`gh workflow run main.yml -f tag=vX.Y.Z`（需要 PAT/经典 token，
内置 GITHUB_TOKEN 无法创建 workflow dispatch，HTTP 403）。

### 0. 触发策略：按需编译，不再"每次推送就编译"（2026-10-14）

**问题**：`main.yml` 原先监听 `push: branches: ['*']`，任何分支/任何文件改动都会
跑满 arm64 三个变体的全量构建（约 15 分钟 × 3），日常提交代价过高。

**现状（拆成两个工作流）**：

| 工作流 | 触发条件 | 职责 |
| --- | --- | --- |
| `main.yml` | ① 推送 `v*` 标签 ② 手动 `workflow_dispatch` ③ PR 到 main（忽略 docs/README/.github） | 全量编译 + 产物上传 + 打 tag 时创建 Release |
| `tagger.yml` | `push` 到 main/stable **且变更文件包含 `version.mk`**，或手动触发 | 只做版本号判断与打标签（秒级，不编译） |

**发版链路**：改 `version.mk` → `tagger.yml` 打出 `vX.Y.Z` 标签 → 标签推送触发
`main.yml` 全量编译 → 自动创建 Release。日常推代码不触发任何编译。

**手动编译**：Actions → "OpenWrt SDK 自动编译" → Run workflow。

**其他**：两个工作流都保留 `concurrency`（同一 ref 只保留最新一次运行），SDK/Rust
缓存键不含 commit sha，因此即便不看编译也不会丢缓存收益。

### 0.1 Rust 目标：优先 Cortex-A53（2026-10-14）

`build-rust.sh` 对 `aarch64*` 目标显式加 `-C target-cpu=cortex-a53`。Rust 的
`aarch64-unknown-linux-musl` 默认 `target-cpu=generic`，实测特性集只有
`crt-static`/`neon`；指定 cortex-a53 后追加 `aes`/`crc`/`sha2`/`pmuv3`
（`rustc --print cfg` 实测）。主力设备 MT7981 的 A53 在 `/proc/cpuinfo` 中确认带有
`asimd aes pmull sha1 sha2 crc32`，因此安全可用。

未实现 ARMv8 Crypto 扩展的 A53 设备需回退：导出 `QMODEM_RUST_TARGET_CPU=generic`
即可（无需改脚本）。C 侧由 OpenWrt SDK 自身的 `-mcpu=cortex-a53` 保证一致。


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

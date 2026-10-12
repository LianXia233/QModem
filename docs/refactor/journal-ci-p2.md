# CI/DevOps Journal — P2 Rust CI 与门禁扩展

- **agent**: agent-ci-p2
- **date**: 2026-10-12
- **scope**: `.github/workflows/**`、`scripts/check_all.sh`（追加）；`main.yml` 未触碰（SDK 构建属上游 feed 流程）

## 变更

### 1. 新增 `.github/workflows/rust-build.yml`
- 触发：push / pull_request，路径过滤 `rust/**`、`scripts/cross-build-rust.sh`、自身。
- `cross-build` job：矩阵 `[x86_64-unknown-linux-musl, aarch64-unknown-linux-musl]`，
  ubuntu-latest 安装 `musl-tools`，`dtolnay/rust-toolchain@stable`（targets 矩阵注入），
  调用 `bash scripts/cross-build-rust.sh --target <t>`。
  - 接口约定（写在 workflow 头注释）：脚本支持 `--target <triple>`、失败非零退出、产物位于
    `scripts/dist/<target>/`。并行代理创建脚本期间该步骤 `continue-on-error: true`（已注释说明，
    脚本稳定后应移除）。
  - 产物收集：优先 `scripts/dist/<target>/`，回退 `rust/target/<target>/release/`，仅拷贝 ELF 可执行文件，
    生成 `SHA256SUMS`，上传 artifact `rust-binaries-<target>`（`if-no-files-found: error`）。
- `test` job：native `cargo test --workspace`（working-directory: rust）。

### 2. `scripts/check_all.sh` 追加 stage（不改既有 stage 逻辑）
- `rust-cross`：默认 **SKIP**（打印原因：避免拖慢本地门禁）；仅当
  `scripts/cross-build-rust.sh` 存在且设置 `FAST=1`（或 CLI `--fast`，已加入参数解析与 help）时，
  以 `rustc -vV` 的 host triple 跑一次 native 构建冒烟。
- `lua-smoke`：`tests/lua/smoke.sh` 存在才跑，退出码 0=通过（含脚本内部 SKIP），1=FAIL。
- STAGES 数组追加两 stage；`--only` 可单独触发。

### 3. `.github/workflows/quality.yml`
- `check-all` job 内追加 step：`bash tests/sh/run_tests.sh`，`hashFiles` 守卫（存在才跑）。

## 调用约定假设（cross-build-rust.sh，本任务未修改该文件）
```bash
bash scripts/cross-build-rust.sh --target <rust-triple>
# 失败 → 非零退出；产物 → scripts/dist/<triple>/qmodem-atd
```
执行期间该脚本由并行代理合入，实测接口与假设一致，`--fast` 冒烟真实 PASS
（host `x86_64-unknown-linux-gnu`）。

## 验证记录
| 项目 | 结果 |
| --- | --- |
| `bash -n scripts/check_all.sh` | PASS |
| `bash scripts/check_all.sh` 全量 | EXIT=0：shell PASS(69)、node PASS(27)、js-test PASS、rust PASS(test+clippy)、c PASS(86 因缺头跳过)、crlf PASS、rust-cross SKIP(未设 FAST)、lua-smoke PASS |
| `--only rust-cross --fast` | PASS（脚本已存在，native 冒烟真实通过） |
| `yaml.safe_load`：rust-build.yml / quality.yml / main.yml | 均通过（jobs: [cross-build, test] / [check-all] / 原有 4 job 未变） |

## 遗留 / 注意
- 脚本合入稳定后，移除 rust-build.yml 构建步的 `continue-on-error: true`。
- musl 目标需 CI runner 有 musl 工具链（已装 musl-tools；aarch64 依赖 rustup target 自带 linker 配置由脚本处理）。

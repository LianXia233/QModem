# journal-rust-cross — Rust 交叉编译与打包脚本

> 维护者：agent-rust-cross（初版）+ 编排者（收尾：crt-static 修正、本机验证、文档补齐）
> 对应工作包：W19（Phase 2）。接口被 `scripts/check_all.sh`（rust-cross stage）与
> `.github/workflows/rust-build.yml` 消费。

## 1. 交付物

| 文件 | 说明 |
|------|------|
| `scripts/cross-build-rust.sh` | 一键交叉/原生构建 qmodem-atd：rustup target 自动安装、musl linker 自动获取（musl-tools / musl.cc 兜底）、产物收集到 `scripts/dist/<target>/qmodem-atd`（含 sha256 与 `file` 自检） |
| `rust/Cargo.toml [profile.release]` | `opt-level="z"` + `lto=true` + `strip=true` + `codegen-units=1` |
| `rust/README.md` | 追加 Cross compilation 章节（用法、两条集成路线） |

## 2. 验证记录（本机实测）

- `bash scripts/cross-build-rust.sh`（原生 x86_64-unknown-linux-gnu）：通过。
- 产物：**450,544 B**，stripped；sha256 `3e7e7d0f...8719dcf`。
- `cargo test --workspace` 37/37 保持全绿；`bash -n` 通过。

## 3. 编排者收尾修正（重要）

初版脚本对**所有** target 无条件 `export RUSTFLAGS="-C target-feature=+crt-static"`。
对原生 gnu target 这会强制 static-pie 链接，产物从 450KB 膨胀到 1,416,224 B 且
属非标准链接形态。已修正为仅对 `*-musl` target 注入（musl 下 crt-static 本就是
默认语义，此处显式化仅作加固）。修正后 dist 产物与直接 `cargo build --release`
体积一致（450,544 B）。

## 4. 与 OpenWrt 包的集成路线（两条，README 详述）

1. **SDK 源码构建**：feed 侧包 Makefile 直接 `cargo build --release`（SDK 自带
   rust 工具链时优先，产物随 ipk/apk 构建可复现）。
2. **预编译注入**：CI（rust-build.yml）产出 aarch64/x86_64 musl 静态二进制
   artifact，`qmodem-suite` 打包时按 ARCH 选择注入 —— 适合 SDK 无 rust 工具链的
   代际（与 OpenWRT-CI 现有 arm64/amd64-only 矩阵一致）。

## 5. 已知边界

- aarch64 musl 链接器：发行版通常无现成包，脚本按 musl-tools → musl.cc 预编译
  顺序兜底；两者皆不可用时退出码 2 并给出手动步骤（CI 由 rust-build.yml 的
  ubuntu runner 自行解决）。
- `scripts/dist/` 为构建产物目录，已加入 `.gitignore`，不入库。

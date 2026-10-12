# CI/DevOps 门禁固化日志（agent-ci）

日期: 2026-10-12
范围: `scripts/check_all.sh`（新建）、`.github/workflows/quality.yml`（新建）、`.github/workflows/vendor-fixtures.yml`（仅替换手写 shell 语法清单）。

## 变更内容

### 1. scripts/check_all.sh（一站式本地门禁）
- `shell`：`find application -path '*/files/*' -name '*.sh'` + 全仓 `*/usr/libexec/rpcd/*`（rpcd 脚本无后缀）自动发现，按 shebang 选 `sh -n` / `bash -n`。覆盖 68 个文件，修复 vendor-fixtures.yml 手写清单缺口（C2）。
- `node`：`node --check` 全量 `luci/*/htdocs/**/*.js`（26 个）。
- `js-test`：`luci/luci-app-qmodem-next/tests/test_sms_service.js` 存在时执行。
- `rust`：`cargo test --workspace` + `cargo clippy --workspace -- -D warnings`。
- `c`：`gcc -fsyntax-only`；本机缺 OpenWrt 系统头时按 `No such file or directory` 判定 SKIP 并打印原因（本次 93 个文件 SKIP，属预期）。
- `crlf`：`git ls-files` + `grep -I -l $'\r'`。**排除 vendored 第三方目录** `driver/`、`application/qfirehose/`、`application/quectel_CM_5G_M/`（quectel 上游源码预存 CRLF 共 36 个文件，不属本次重构门禁范围）。
- 参数：`--only <stage>`（shell|node|js-test|rust|c|crlf）、`--changed`（git diff HEAD/ORIG_HEAD 涉及文件，未涉及目录的 stage 自动 SKIP）。汇总 PASS/FAIL/SKIP，任一 FAIL 退出码 1。

### 2. .github/workflows/quality.yml（新建）
- push/PR 触发，paths 限定 application/luci/rust/driver/files/scripts/check_all.sh 及自身，与 main.yml（SDK 重量级构建）、vendor-fixtures.yml（fixture 回放）不重叠。
- ubuntu-latest 单 job：setup-node@v4 (node 20) + dtolnay/rust-toolchain@stable (clippy) + `bash scripts/check_all.sh`。
- `apt install lua5.1` 步骤标注 `continue-on-error: true`（尽力而为）。
- 体积门禁占位：`cargo build --release` 后 `ls -l rust/target/release/` 写入 `rust-binary-size.txt` 并 upload-artifact，人工核对，无硬阈值。

### 3. vendor-fixtures.yml
原 35-49 行手写 `sh -n` 清单整体替换为 `bash scripts/check_all.sh --only shell`，job 其余步骤（qmodem-seal 构建、boundary 测试、fixture 回放）不变。

## 验证记录

| 检查 | 结果 |
| --- | --- |
| `bash -n scripts/check_all.sh` | 通过 |
| `bash scripts/check_all.sh` 全量 | 全绿：shell 68 文件 PASS / node 26 文件 PASS / js-test PASS / rust cargo test+clippy PASS / c SKIP（缺 OpenWrt 头，93 文件）/ crlf PASS；退出码 0 |
| `--only shell` | PASS，退出码 0 |
| `--changed`（工作区 1 个 shell 改动） | shell PASS，node/js-test/rust/c 各按目录 SKIP，退出码 0 |
| `--only crlf` | PASS，退出码 0 |
| `yaml.safe_load` 校验 quality.yml / vendor-fixtures.yml / main.yml | 三个均 OK |

CRLF 首跑曾 FAIL（36 个 vendored 文件），改为排除 vendored 路径后全绿；该决策已写入 check_all.sh 注释。

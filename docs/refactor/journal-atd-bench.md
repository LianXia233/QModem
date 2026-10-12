# qmodem-atd 微基准日志（journal-atd-bench）

> 本日志由 agent-atd-bench 维护。范围：`rust/qmodem-atd/tests/**`（仅新增集成测试，不改 Cargo.toml/src）。结果报告：[bench-atd.md](bench-atd.md)。

## 任务

为 qmodem-atd 补 std-only 微基准（不用 criterion，零外部依赖）：

1. 单端口 sendat 往返延迟（LoopbackPort，1000 次，p50/p95/max，毫秒输出）；
2. 并发 8 任务 × 500 命令排队总吞吐；
3. broadcast 32 订阅者分发耗时；
4. JSON 1KB 载荷 parse/serialize 各 1000 次。

## 实现方式（tests/bench_atd.rs）

- 每个 `#[test]` 用 `std::time::Instant` 计时固定负载，打印 p50/p95/max/mean 与总耗时；百分位用排序取下标的手写 helper（约 10 行），无任何依赖。
- 计时前预热（bench1 预热 20 次、bench4 预热 50 次）压制首次冷启动离群值；JSON 循环用 `std::hint::black_box` 防优化剔除。
- 正确性内嵌断言（非计时敏感）：响应 status/end_flag、4000 条全 OK、32 订阅者各收满 1000 条零丢失、JSON round-trip 等值。
- CI 稳定性：只做宽松上界 sanity（单次往返 <100ms、4000 条 <30s、广播 <5s、JSON <5s），不设下界、不做相对比较，慢机不 flaky。
- 1KB 载荷用确定前缀 + `"x".repeat(pad)` 精确补齐到 1024 字节，`assert_eq!(payload.len(), 1024)` 自验证。

## 踩坑记录

- raw string 定界符：JSON 内容以 `"note":"` 收尾时，`r#"..."#` 的 `"#` 终止符与内容引号歧义，改 `r##"..."##`；另漏了 `String::from(` 的右括号，首次编译失败两处，均已修复。
- 运行目录注意：workspace 根是 `rust/`，须在 `/workspace/QModem/rust` 下执行 cargo。

## 结果摘要（debug 构建，2026-10-12）

| 基准 | 关键数值 |
|---|---|
| sendat 往返 ×1000 | p50=50.1ms / p95=50.2ms / max=61.0ms（由 actor `IDLE_SLICE=50ms` 待机拾取上界主导，port.rs:288-294；压载态吞吐见下行，非回归） |
| 并发 8×500 吞吐 | 总 127.4ms ≈ 31397 cmd/s |
| 广播 32 订阅 ×1000 | p50=6.84µs / p95=10.61µs / max=236.8µs，总 8.0ms |
| JSON 1KB ×1000 | parse p50=24.7µs / serialize p50=21.8µs |

## 验证

`cargo test -p qmodem-atd`：lib 14 + atd_integration 8 + bench_atd 4 + zero_dep 1，全部通过（51.3s，其中 bench1 占 ~50s，为 IDLE_SLICE 语义下的预期）。

## 遗留候选（本次不改 src）

- `send_at`/actor 间加唤醒通道压缩待机拾取延迟（可把 bench1 p50 从 50ms 降到 µs 级），改动属 src 范围，留待后续 agent。
- bench 目前固定 debug 数据；若需 release 口径可在 CI 加 `--release` 步骤复用同一测试。

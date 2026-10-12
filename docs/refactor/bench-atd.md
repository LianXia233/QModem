# qmodem-atd std-only 微基准报告（bench-atd）

> 生成：agent-atd-bench（2026-10-12）。实现：`rust/qmodem-atd/tests/bench_atd.rs`（集成测试形式，`#[test]` 内 `Instant` 计时，零外部依赖，不用 criterion）。
> 运行：`cargo test -p qmodem-atd --test bench_atd -- --nocapture --test-threads=1`（debug 构建）。
> 断言只做宽松上界 sanity（单次往返 <100ms 等），CI 慢机不 flaky。

## 结果（Linux CI 同级机器，debug 构建，2026-10-12）

| # | 基准 | 负载 | p50 | p95 | max | mean | 总耗时 | 结论 |
|---|---|---|---|---|---|---|---|---|
| 1 | 单端口 sendat 往返延迟（LoopbackPort） | 1000 次 + 20 次预热 | 50.119 ms | 50.227 ms | 60.970 ms | 50.156 ms | 50.16 s | 分布由 actor 50ms `IDLE_SLICE` 主导（见下） |
| 2 | 并发 sendat 排队吞吐（8 任务 × 500） | 4000 条命令过单端口 actor | — | — | — | — | 127.4 ms | ≈ 31397 cmd/s |
| 3 | broadcast 32 订阅者分发（`LineHub::send`） | 1000 次发布 × 32 订阅 | 6.84 µs | 10.61 µs | 236.84 µs | 8.03 µs | 8.0 ms | 纯进程内锁 + clone，µs 级 |
| 4 | JSON 1KB parse | 1000 次（载荷恰 1024B） | 24.71 µs | 44.35 µs | 114.65 µs | 30.57 µs | 30.6 ms | 手写 parser，µs 级 |
| 4' | JSON 1KB serialize（`Display`） | 1000 次 | 21.84 µs | 23.00 µs | 38.41 µs | 22.19 µs | 22.2 ms | round-trip 等值断言通过 |

## 观察与解读

- **基准 1 的 p50≈50ms 不是回归**：actor 空闲时阻塞在 `read_chunk(IDLE_SLICE=50ms)`，命令经 mpsc 入队不会唤醒该等待，拾取必须等本片超时（port.rs:288-294），因此待机拾取延迟上界即 50ms。持续压载下（基准 2，队列非空，走 `try_recv` 立即拾取）单条命令吞吐为 µs 级（4000 条仅 127ms）。若需压缩待机拾取延迟，后续可在 `send_at` 侧或 actor 侧加唤醒通道（记录为优化候选，本次不改 src）。
- **基准 2**：单端口 actor 串行执行下 8×500 并发排队总吞吐 ≈31k cmd/s（debug），release 预期更高；无响应错配。
- **基准 3**：32 订阅 fan-out 单次均值 8µs；max 236µs 为偶发调度抖动。投递正确性断言：32 订阅者各收满 1000 条，零丢失。
- **基准 4**：1KB 协议形状载荷 parse ≈30µs、serialize ≈22µs（debug），对应 Unix-socket JSON 行协议单请求开销可忽略。

## Sanity 上界（CI 稳定性设计）

| 断言 | 阈值 | 依据 |
|---|---|---|
| 单次 sendat 往返 | < 100 ms | `IDLE_SLICE` 50ms 上界 + 调度余量 |
| 4000 条并发总耗时 | < 30 s | 实测 0.13s，百倍余量 |
| 1000 次广播总耗时 | < 5 s | 实测 8ms |
| 2000 次 JSON 操作总耗时 | < 5 s | 实测 53ms |

## 复现

```sh
cd rust && cargo test -p qmodem-atd --test bench_atd -- --nocapture --test-threads=1
```

日志：`journal-atd-bench.md`（agent-atd-bench）。

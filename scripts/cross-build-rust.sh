#!/usr/bin/env bash
# cross-build-rust.sh — 交叉/原生构建 qmodem-atd（musl 静态二进制）
#
# 用法:
#   bash scripts/cross-build-rust.sh                     # 原生 target（本机直接跑通）
#   bash scripts/cross-build-rust.sh --target x86_64-unknown-linux-musl
#   bash scripts/cross-build-rust.sh --target aarch64-unknown-linux-musl
#
# 产物: scripts/dist/<target-or-native>/qmodem-atd
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RUST_DIR="$SCRIPT_DIR/../rust"
DIST_DIR="$SCRIPT_DIR/dist"

TARGET=""
while [[ $# -gt 0 ]]; do
    case "$1" in
        --target)
            [[ $# -ge 2 ]] || { echo "error: --target 需要参数" >&2; exit 1; }
            TARGET="$2"; shift 2 ;;
        -h|--help)
            sed -n '2,9p' "$0"; exit 0 ;;
        *)
            echo "error: 未知参数 $1（支持 --target <triple>）" >&2; exit 1 ;;
    esac
done

HOST_TARGET="$(rustc -vV | awk '/^host:/ {print $2}')"
# 无 --target 时使用原生 target（仍可跑通，产物路径按 target 命名）
[[ -z "$TARGET" ]] && TARGET="$HOST_TARGET"

case "$TARGET" in
    x86_64-unknown-linux-musl) LINKER="x86_64-linux-musl-gcc" ;;
    aarch64-unknown-linux-musl) LINKER="aarch64-linux-musl-gcc" ;;
    *) LINKER="" ;;  # 原生/其他 target：交给 rustc 默认 linker
esac

# ---------------------------------------------------------------------------
# 1. rustup target 安装
# ---------------------------------------------------------------------------
rustup target add "$TARGET"

# ---------------------------------------------------------------------------
# 2. musl linker 检测与安装
# ---------------------------------------------------------------------------
APT_UPDATED=0
apt_install() {
    if command -v apt-get >/dev/null 2>&1; then
        if [[ $APT_UPDATED -eq 0 ]]; then
            sudo_args=(); [[ "$(id -u)" -ne 0 ]] && sudo_args=(sudo)
            "${sudo_args[@]}" apt-get update -y && APT_UPDATED=1
        fi
        "${sudo_args[@]}" apt-get install -y "$@"
    fi
}

if [[ -n "$LINKER" ]] && ! command -v "$LINKER" >/dev/null 2>&1; then
    echo "==> 未找到 $LINKER，尝试安装 musl 交叉工具链..."
    case "$TARGET" in
        x86_64-unknown-linux-musl)
            # 同架构 musl：musl-tools 提供 musl-gcc wrapper（Ubuntu/Debian）
            if apt_install musl-tools && command -v musl-gcc >/dev/null 2>&1; then
                LINKER="musl-gcc"
            fi
            ;;
        aarch64-unknown-linux-musl)
            # 发行版一般无 aarch64 musl 交叉包，尝试 musl-cross-make 预编译产物
            if apt_install musl-tools gcc-aarch64-linux-gnu 2>/dev/null \
               && command -v aarch64-linux-musl-gcc >/dev/null 2>&1; then
                : # 有些发行版提供 aarch64-linux-musl-gcc
            elif ! command -v aarch64-linux-musl-gcc >/dev/null 2>&1; then
                MUSLCC_DIR="$HOME/.local/musl-cross"
                GCC_BIN="$MUSLCC_DIR/aarch64-linux-musl-cross/bin/aarch64-linux-musl-gcc"
                if [[ ! -x "$GCC_BIN" ]]; then
                    echo "==> 下载 musl.cc aarch64-linux-musl-cross..."
                    mkdir -p "$MUSLCC_DIR"
                    if curl -fL --retry 2 \
                         -o "$MUSLCC_DIR/aarch64.tgz" \
                         https://musl.cc/aarch64-linux-musl-cross.tgz \
                       && tar -xzf "$MUSLCC_DIR/aarch64.tgz" -C "$MUSLCC_DIR"; then
                        echo "==> musl.cc 工具链已解压到 $MUSLCC_DIR"
                    else
                        echo "!! 无法自动获取 aarch64 musl 工具链。" >&2
                        echo "!! 手动步骤（二选一）：" >&2
                        echo "!!   a) curl -LO https://musl.cc/aarch64-linux-musl-cross.tgz && tar xf 到 PATH 内" >&2
                        echo "!!   b) 自行编译 musl-cross-make: make MUSL_TARGET=aarch64-linux-musl install" >&2
                        echo "!!   完成后确保 aarch64-linux-musl-gcc 在 PATH 后重跑本脚本。" >&2
                        exit 2
                    fi
                fi
                export PATH="$MUSLCC_DIR/aarch64-linux-musl-cross/bin:$PATH"
                command -v aarch64-linux-musl-gcc >/dev/null 2>&1 || exit 2
            fi
            ;;
    esac
    command -v "$LINKER" >/dev/null 2>&1 || {
        echo "!! linker $LINKER 仍不可用，请按上述指引安装" >&2; exit 2; }
fi

# ---------------------------------------------------------------------------
# 3. linker / 体积优化 配置
# ---------------------------------------------------------------------------
if [[ -n "$LINKER" ]]; then
    export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER="$LINKER"
    export CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER="$LINKER"
    export CC_AARCH64_UNKNOWN_LINUX_MUSL="${LINKER%-gcc}-cc"
    export CC_X86_64_UNKNOWN_LINUX_MUSL="${LINKER%-gcc}-cc"
fi
# 纯 std 零依赖 workspace 无 cc 调用；RUSTFLAGS 仅做静态链接加固
export RUSTFLAGS="-C target-feature=+crt-static"

# ---------------------------------------------------------------------------
# 4. 构建 release
# ---------------------------------------------------------------------------
echo "==> cargo build --release --target $TARGET"
( cd "$RUST_DIR" && cargo build --release --target "$TARGET" -p qmodem-atd )

# ---------------------------------------------------------------------------
# 5. 收集产物
# ---------------------------------------------------------------------------
SRC_BIN="$RUST_DIR/target/$TARGET/release/qmodem-atd"
OUT_DIR="$DIST_DIR/$TARGET"
mkdir -p "$OUT_DIR"
cp "$SRC_BIN" "$OUT_DIR/qmodem-atd"

echo "==> 产物:"
ls -l "$OUT_DIR/qmodem-atd"
sha256sum "$OUT_DIR/qmodem-atd"

# 静态链接自检（musl target 时应为 statically linked）
if command -v file >/dev/null 2>&1; then
    file "$OUT_DIR/qmodem-atd" || true
fi

#!/bin/sh
# build-rust.sh - 在 OpenWrt 构建环境内用 cargo 交叉编译 qmodem 的 Rust 组件。
#
# 由 application/qmodem/Makefile 的 Build/Compile 调用。所需环境变量：
#   RUST_DIR            rust workspace 绝对路径（仓库根的 rust/）
#   RUST_TARGET_TRIPLE  rust 目标三元组（由 Makefile 按 $(ARCH) 推导）
#   OUT_DIR             产物输出目录（Makefile 内为 $(PKG_BUILD_DIR)/rust-bin）
#   TARGET_CC           OpenWrt 交叉 gcc 绝对路径（作 linker）
#   STAGING_DIR         OpenWrt 目标 sysroot（OpenWrt gcc wrapper 需要）
#
# 工具链策略（Rust 平台支持矩阵，2026-10 实测口径）：
#   aarch64-unknown-linux-musl / x86_64-unknown-linux-musl 为 tier 2，
#     std 预编译可得，稳定版工具链即可；
#   mipsel-unknown-linux-musl（及 mips-）为 tier 3 但 std 可用
#     （tier:3 / host_tools:false / std:true），rustup 不提供预编译 std，
#     需要 nightly + rust-src + `-Z build-std=std,panic_abort`。
#   linker 一律用 OpenWrt 自带交叉工具链而非 musl.cc 等第三方工具链，
#   以保证与目标 sysroot 的 musl 配置（mipsel 为 soft-float）及后续
#   libubus / libjson-c / libsqlite3 的 ABI 完全一致。
set -eu

RUST_DIR="${RUST_DIR:?RUST_DIR is required}"
RUST_TARGET_TRIPLE="${RUST_TARGET_TRIPLE:?RUST_TARGET_TRIPLE is required}"
OUT_DIR="${OUT_DIR:?OUT_DIR is required}"
TARGET_CC="${TARGET_CC:?TARGET_CC is required}"
STAGING_DIR="${STAGING_DIR:-}"

[ -f "$RUST_DIR/Cargo.toml" ] || {
	echo "!! rust workspace not found at $RUST_DIR" >&2
	exit 1
}

# tier 3 的 mips/mipsel 需要 nightly 与 rust-src；其余用稳定版。
# 注意：tier 3 目标在 rustup 里没有可下载的 std，`rustup target add` 会直接
# 报错，因此只在 tier 2 分支执行 target add；tier 3 的 std 由 build-std 现编。
# build-std 里列出 panic_abort 时，必须同时给 -C panic=abort，否则
# panic 运行时与 profile 的 unwind 策略不一致，编译 std 时即报错。
case "$RUST_TARGET_TRIPLE" in
	mips-unknown-linux-musl | mipsel-unknown-linux-musl)
		TOOLCHAIN=nightly
		STD_FLAGS="-Zbuild-std=std,panic_abort"
		EXTRA_RUSTFLAGS="-C panic=abort"
		rustup component add rust-src --toolchain nightly >&2
		;;
	*)
		TOOLCHAIN=stable
		STD_FLAGS=""
		EXTRA_RUSTFLAGS=""
		rustup target add "$RUST_TARGET_TRIPLE" --toolchain stable >&2
		;;
esac

# CARGO_TARGET_<TRIPLE> 环境变量名：小写连字符 -> 大写下划线。
UP=$(printf '%s' "$RUST_TARGET_TRIPLE" | tr 'a-z-' 'A-Z_')

LINKER="$TARGET_CC"
AR="${LINKER%gcc}ar"
[ -x "$AR" ] || AR=""

if [ "$TOOLCHAIN" = nightly ]; then
	# --- tier 3 链接环境修复（CI 实测：crt1.o/-lunwind 全部找不到） ---
	# build-std 模式下 rustc 传给链接驱动（gcc）的 sysroot 不是交叉工具链
	# 的 sysroot，gcc 的启动文件（crt1.o/crti.o/...）搜索因此落空；tier 2
	# 不受影响是因为预编译 std 自带 self-contained 组件兜底。修复：
	#   A) 把交叉 gcc 的真实 sysroot 作为最后一个 --sysroot 追加到链接参数
	#      （gcc 取最后一个生效），恢复 crt 启动文件解析；
	#   B) -lunwind 在交叉 sysroot 与 build-std 产物目录里都解析不到
	#      （build-std 产物是带 hash 的 libunwind-<hash>.a），而
	#      panic_abort 模式运行期不需要真正的 unwinder —— 提供一个空
	#      libunwind.a 垫片满足链接即可。
	# 探针输出留在日志里（V=s 重跑可见），便于后续架构调整时核对。
	echo "==> probe: $LINKER -print-sysroot = $("$LINKER" -print-sysroot 2>/dev/null || true)" >&2
	echo "==> probe: crt1.o = $("$LINKER" -print-file-name=crt1.o 2>/dev/null || true)" >&2

	XSYSROOT=$("$LINKER" -print-sysroot 2>/dev/null | tail -1)
	case "$XSYSROOT" in
		/*) if [ -d "$XSYSROOT" ]; then
			EXTRA_RUSTFLAGS="$EXTRA_RUSTFLAGS -C link-arg=--sysroot=$XSYSROOT"
			echo "==> fix A: link-arg --sysroot=$XSYSROOT" >&2
		fi ;;
	esac

	UNWIND_DIR="$OUT_DIR/fake-unwind"
	mkdir -p "$UNWIND_DIR"
	if "${AR:-ar}" crs "$UNWIND_DIR/libunwind.a" 2>/dev/null; then
		EXTRA_RUSTFLAGS="$EXTRA_RUSTFLAGS -C link-arg=-L$UNWIND_DIR"
		echo "==> fix B: empty libunwind.a at $UNWIND_DIR" >&2
	else
		echo "!! fix B skipped: cannot create empty libunwind.a" >&2
	fi
fi

# 纯 std 零依赖组件（qmodem-atd 的 service/tty feature）走全静态 musl，
# 不引入任何 .so 运行时依赖；启用 ubus/uci/sqlite feature 后改由
# -L $(STAGING_DIR)/usr/lib 动态链接（届时另设 RUSTFLAGS，见 Makefile 注释）。
eval "export CARGO_TARGET_${UP}_LINKER=\"\$LINKER\""
[ -n "$AR" ] && eval "export CARGO_TARGET_${UP}_AR=\"\$AR\""
eval "export CARGO_TARGET_${UP}_RUSTFLAGS=\"-C target-feature=+crt-static $EXTRA_RUSTFLAGS\""

echo "==> rust toolchain: $TOOLCHAIN / target: $RUST_TARGET_TRIPLE"
echo "==> linker: $LINKER"

mkdir -p "$OUT_DIR"
LOG="$OUT_DIR/cargo-build.log"
# OpenWrt 默认详细度不会透出 Build/Compile 的输出，cargo 的报错会被
# 「failed to build」一行盖掉；因此把输出落盘，失败时原样倒给 stderr。
# shellcheck disable=SC2086
if ( cd "$RUST_DIR" && \
	cargo +"$TOOLCHAIN" build --release --target "$RUST_TARGET_TRIPLE" \
		$STD_FLAGS -p qmodem-atd ) >"$LOG" 2>&1; then
	echo "==> cargo build OK"
else
	echo "!! cargo build failed, log follows" >&2
	cat "$LOG" >&2
	exit 1
fi

cp "$RUST_DIR/target/$RUST_TARGET_TRIPLE/release/qmodem-atd" "$OUT_DIR/qmodem-atd"

echo "==> built:"
ls -l "$OUT_DIR/qmodem-atd"
if command -v file >/dev/null 2>&1; then
	file "$OUT_DIR/qmodem-atd" || true
fi

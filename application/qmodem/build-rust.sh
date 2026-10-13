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
case "$RUST_TARGET_TRIPLE" in
	mips-unknown-linux-musl | mipsel-unknown-linux-musl)
		TOOLCHAIN=nightly
		STD_FLAGS="-Zbuild-std=std,panic_abort"
		rustup component add rust-src --toolchain nightly >&2
		;;
	*)
		TOOLCHAIN=stable
		STD_FLAGS=""
		rustup target add "$RUST_TARGET_TRIPLE" --toolchain stable >&2
		;;
esac

# CARGO_TARGET_<TRIPLE> 环境变量名：小写连字符 -> 大写下划线。
UP=$(printf '%s' "$RUST_TARGET_TRIPLE" | tr 'a-z-' 'A-Z_')

LINKER="$TARGET_CC"
AR="${LINKER%gcc}ar"
[ -x "$AR" ] || AR=""

# 纯 std 零依赖组件（qmodem-atd 的 service/tty feature）走全静态 musl，
# 不引入任何 .so 运行时依赖；启用 ubus/uci/sqlite feature 后改由
# -L $(STAGING_DIR)/usr/lib 动态链接（届时另设 RUSTFLAGS，见 Makefile 注释）。
eval "export CARGO_TARGET_${UP}_LINKER=\"\$LINKER\""
[ -n "$AR" ] && eval "export CARGO_TARGET_${UP}_AR=\"\$AR\""
eval "export CARGO_TARGET_${UP}_RUSTFLAGS=\"-C target-feature=+crt-static\""

echo "==> rust toolchain: $TOOLCHAIN / target: $RUST_TARGET_TRIPLE"
echo "==> linker: $LINKER"

# shellcheck disable=SC2086
( cd "$RUST_DIR" && \
	cargo +"$TOOLCHAIN" build --release --target "$RUST_TARGET_TRIPLE" \
		$STD_FLAGS -p qmodem-atd )

mkdir -p "$OUT_DIR"
cp "$RUST_DIR/target/$RUST_TARGET_TRIPLE/release/qmodem-atd" "$OUT_DIR/qmodem-atd"

echo "==> built:"
ls -l "$OUT_DIR/qmodem-atd"
if command -v file >/dev/null 2>&1; then
	file "$OUT_DIR/qmodem-atd" || true
fi

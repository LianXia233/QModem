#!/usr/bin/env bash
# check_all.sh - QModem 一站式本地质量门禁
# 用法:
#   bash scripts/check_all.sh                 # 全量跑所有 stage
#   bash scripts/check_all.sh --only shell    # 只跑某个 stage (shell|node|js-test|rust|c|crlf|rust-cross|lua-smoke)
#   bash scripts/check_all.sh --changed       # 仅检查 git diff 涉及的文件
#   FAST=1 bash scripts/check_all.sh          # 或 --fast：额外跑 rust-cross native 构建冒烟（默认 SKIP）
# 汇总各 stage 结果，任一 FAIL 则退出码非 0。
set -u

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

ONLY=""
CHANGED=0
FAST="${FAST:-0}"
while [ $# -gt 0 ]; do
    case "$1" in
        --only) ONLY="${2:-}"; shift 2 ;;
        --changed) CHANGED=1; shift ;;
        --fast) FAST=1; shift ;;
        -h|--help)
            grep '^#' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) echo "未知参数: $1" >&2; exit 2 ;;
    esac
done

RESULTS=()

record() { # record <status> <stage> <detail>
    RESULTS+=("$1|$2|$3")
    printf '[%s] %-8s %s\n' "$1" "$2" "$3"
}

have() { command -v "$1" >/dev/null 2>&1; }

# --changed: 取工作区改动 + 最近一次提交改动涉及的文件
changed_files() {
    { git diff --name-only HEAD 2>/dev/null; git diff --name-only ORIG_HEAD 2>/dev/null; } \
        | sort -u | grep -v '^$' || true
}

in_scope() { # in_scope <file>  — --changed 模式下过滤文件
    [ "$CHANGED" -eq 0 ] && return 0
    changed_files | grep -qxF "$1"
}

should_run() { # should_run <stage>
    [ -z "$ONLY" ] || [ "$ONLY" = "$1" ]
}

# ---------- stage: shell (sh -n 全量扫描) ----------
stage_shell() {
    have sh || { record SKIP shell "sh 不可用"; return 0; }
    local files tmp
    tmp="$(mktemp)"
    find application -type f -name '*.sh' -path '*/files/*' > "$tmp" 2>/dev/null
    # rpcd 脚本无 .sh 后缀，单独按目录发现
    find . -path ./.git -prune -o -path ./rust/target -prune -o \
        -type f -path '*/usr/libexec/rpcd/*' -print >> "$tmp" 2>/dev/null
    sort -u "$tmp" -o "$tmp"
    local fail=0 total=0 skipped=0
    while IFS= read -r f; do
        [ -f "$f" ] || continue
        in_scope "$f" || continue
        total=$((total + 1))
        if head -n 1 "$f" | grep -q '^#!.*bash'; then
            bash -n "$f" 2>/tmp/check_all_err || { record FAIL shell "$f: $(cat /tmp/check_all_err)"; fail=$((fail + 1)); }
        else
            sh -n "$f" 2>/tmp/check_all_err || { record FAIL shell "$f: $(cat /tmp/check_all_err)"; fail=$((fail + 1)); }
        fi
    done < "$tmp"
    rm -f "$tmp"
    if [ "$fail" -gt 0 ]; then
        record FAIL shell "$fail 个文件语法错误"
    elif [ "$total" -eq 0 ]; then
        record SKIP shell "无匹配文件 (--changed 模式下无改动)"
    else
        record PASS shell "sh -n 通过 $total 个文件"
    fi
    return 0
}

# ---------- stage: node (node --check 全量 htdocs JS) ----------
stage_node() {
    if ! have node; then
        record SKIP node "node 未安装"
        return 0
    fi
    local fail=0 total=0
    while IFS= read -r f; do
        in_scope "$f" || continue
        total=$((total + 1))
        node --check "$f" 2>/tmp/check_all_err || { record FAIL node "$f: $(cat /tmp/check_all_err)"; fail=$((fail + 1)); }
    done < <(find luci -type f -name '*.js' -path '*/htdocs/*' 2>/dev/null)
    if [ "$fail" -gt 0 ]; then
        record FAIL node "$fail 个 JS 语法错误"
    elif [ "$total" -eq 0 ]; then
        record SKIP node "无匹配文件 (--changed 模式下无改动)"
    else
        record PASS node "node --check 通过 $total 个文件"
    fi
    return 0
}

# ---------- stage: js-test (luci-app-qmodem-next 单测) ----------
stage_js_test() {
    local t="luci/luci-app-qmodem-next/tests/test_sms_service.js"
    [ -f "$t" ] || { record SKIP js-test "测试文件不存在"; return 0; }
    have node || { record SKIP js-test "node 未安装"; return 0; }
    if [ "$CHANGED" -eq 1 ] && ! changed_files | grep -q '^luci/'; then
        record SKIP js-test "luci/ 无改动"
        return 0
    fi
    if node "$t" >/tmp/check_all_js.log 2>&1; then
        record PASS js-test "test_sms_service.js 通过"
    else
        record FAIL js-test "test_sms_service.js 失败 (见 /tmp/check_all_js.log)"
    fi
    return 0
}

# ---------- stage: rust (cargo test + clippy) ----------
stage_rust() {
    [ -d rust ] || { record SKIP rust "rust/ 目录不存在"; return 0; }
    have cargo || { record SKIP rust "cargo 未安装"; return 0; }
    if [ "$CHANGED" -eq 1 ] && ! changed_files | grep -q '^rust/'; then
        record SKIP rust "rust/ 无改动"
        return 0
    fi
    ( cd rust && cargo test --workspace >/tmp/check_all_cargo.log 2>&1 ) \
        || { record FAIL rust "cargo test 失败 (见 /tmp/check_all_cargo.log)"; return 0; }
    ( cd rust && cargo clippy --workspace -- -D warnings >>/tmp/check_all_cargo.log 2>&1 ) \
        || { record FAIL rust "cargo clippy -D warnings 失败 (见 /tmp/check_all_cargo.log)";
             echo "---- clippy log tail ----" >&2
             tail -n 40 /tmp/check_all_cargo.log >&2
             echo "--------------------------" >&2
             return 0; }
    record PASS rust "cargo test + clippy 通过"
    return 0
}

# ---------- stage: c (gcc -fsyntax-only 改动/全量尽力) ----------
stage_c() {
    have gcc || { record SKIP c "gcc 未安装"; return 0; }
    # 无 OpenWrt 交叉头，本机仅做语法级检查；缺系统头文件时视为 SKIP
    local files fail=0 total=0
    files="$(find application driver -type f -name '*.c' 2>/dev/null)"
    [ -n "$files" ] || { record SKIP c "无 C 源文件"; return 0; }
    if [ "$CHANGED" -eq 1 ]; then
        files="$(changed_files | grep '\.c$')" || true
        [ -n "$files" ] || { record SKIP c "C 文件无改动"; return 0; }
    fi
    local missing_headers=0
    while IFS= read -r f; do
        total=$((total + 1))
        if ! gcc -fsyntax-only "$f" 2>/tmp/check_all_c_err; then
            if grep -qi 'No such file or directory' /tmp/check_all_c_err; then
                missing_headers=$((missing_headers + 1))
            else
                record FAIL c "$f: $(head -3 /tmp/check_all_c_err)"
                fail=$((fail + 1))
            fi
        fi
    done <<< "$files"
    if [ "$fail" -gt 0 ]; then
        record FAIL c "$fail 个 C 文件语法错误"
    elif [ "$missing_headers" -eq "$total" ]; then
        record SKIP c "本机缺少 OpenWrt/系统头文件，$total 个文件无法编译检查"
    elif [ "$missing_headers" -gt 0 ]; then
        record PASS c "gcc -fsyntax-only 通过（$missing_headers 个文件因缺头文件跳过）"
    else
        record PASS c "gcc -fsyntax-only 通过 $total 个文件"
    fi
    return 0
}

# ---------- stage: crlf (CRLF 检查) ----------
# 排除第三方 vendor 目录（quectel 上游源码，预存 CRLF，不做门禁）
stage_crlf() {
    have git || { record SKIP crlf "git 不可用"; return 0; }
    local files
    files="$(git ls-files \
        | grep -vE '^(driver/|application/qfirehose/|application/quectel_CM_5G_M/)' \
        | while IFS= read -r f; do
            [ -f "$f" ] && grep -I -q -l $'\r' "$f" 2>/dev/null && echo "$f"
        done)"
    if [ -n "$files" ]; then
        record FAIL crlf "以下文件含 CRLF:
$files"
    else
        record PASS crlf "无 CRLF 文件"
    fi
    return 0
}

# ---------- stage: rust-cross (交叉构建冒烟，按需) ----------
# cross-build-rust.sh 接口约定（该脚本可能由并行任务创建中，此处仅做调用假设）：
#   bash scripts/cross-build-rust.sh --target <rust-triple>
#   - 支持 --target <triple>；失败时非零退出；产物位于 scripts/dist/<triple>/
# 默认 SKIP：交叉编译耗时长，仅当脚本存在且 FAST=1（或传 --fast）时跑本机 host triple 冒烟。
stage_rust_cross() {
    local script="scripts/cross-build-rust.sh"
    [ -f "$script" ] || { record SKIP rust-cross "scripts/cross-build-rust.sh 不存在"; return 0; }
    [ "$FAST" = "1" ] || { record SKIP rust-cross "未设置 FAST=1/--fast（交叉构建冒烟按需运行，避免拖慢本地门禁）"; return 0; }
    have rustc || { record SKIP rust-cross "rustc 未安装"; return 0; }
    local host
    host="$(rustc -vV 2>/dev/null | sed -n 's/^host: //p')"
    [ -n "$host" ] || { record SKIP rust-cross "无法确定 host triple"; return 0; }
    if bash "$script" --target "$host" >/tmp/check_all_rust_cross.log 2>&1; then
        record PASS rust-cross "native ($host) 构建冒烟通过"
    else
        record FAIL rust-cross "native ($host) 构建冒烟失败 (见 /tmp/check_all_rust_cross.log)"
    fi
    return 0
}

# ---------- stage: lua-smoke (旧栈 lua 语法冒烟) ----------
# 委托 tests/lua/smoke.sh：退出码 0 = 通过或脚本内部 SKIP，1 = 有语法错误。
stage_lua_smoke() {
    local t="tests/lua/smoke.sh"
    [ -f "$t" ] || { record SKIP lua-smoke "tests/lua/smoke.sh 不存在"; return 0; }
    if sh "$t" >/tmp/check_all_lua_smoke.log 2>&1; then
        record PASS lua-smoke "tests/lua/smoke.sh 通过"
    else
        record FAIL lua-smoke "tests/lua/smoke.sh 失败 (见 /tmp/check_all_lua_smoke.log)"
    fi
    return 0
}

STAGES=(shell node js-test rust c crlf rust-cross lua-smoke)
for stage in "${STAGES[@]}"; do
    should_run "$stage" || continue
    "stage_${stage//-/_}"
done

# ---------- 汇总 ----------
echo "==================== 汇总 ===================="
exit_code=0
for line in "${RESULTS[@]}"; do
    status="${line%%|*}"
    rest="${line#*|}"
    printf '%-5s %s\n' "$status" "$rest"
    [ "$status" = "FAIL" ] && exit_code=1
done
[ ${#RESULTS[@]} -eq 0 ] && { echo "未运行任何 stage（检查 --only 参数）"; exit_code=2; }
exit $exit_code

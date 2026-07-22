#!/usr/bin/env bash
# ──────────────────────────────────────────────────────────────
# smart_ex — 全格式回归测试
# 测试: zip 7z rar tar tar.gz tar.xz tar.zst tar.bz2 tar.lz4
#       gz xz zst bz2 lz4 (单文件流)
#       加密 ZIP / 加密 7z / .enc 包装
#       extract-here / extract-as
# ──────────────────────────────────────────────────────────────

# 不使用 set -e, 手动跟踪 pass/fail
BIN="./target/release/smart_ex"
TEST_DIR="/tmp/smart_ex_test_$$"
PASS=0
FAIL=0

GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

pass() { echo -e "${GREEN}  ✅ PASS${NC} $1"; PASS=$((PASS + 1)); }
fail() { echo -e "${RED}  ❌ FAIL${NC} $1: $2"; FAIL=$((FAIL + 1)); }

echo "═══════════════════════════════════════════════════════════"
echo "  smart_ex v0.3.0 全格式回归测试"
echo "═══════════════════════════════════════════════════════════"
echo ""

# 准备测试数据
rm -rf "$TEST_DIR"
mkdir -p "$TEST_DIR"
SRC="$TEST_DIR/src"
mkdir -p "$SRC/subdir"
echo "Hello, World!" > "$SRC/hello.txt"
echo "测试中文内容" > "$SRC/中文文件.txt"
echo "nested data" > "$SRC/subdir/nested.txt"
for i in $(seq 1 100); do echo "Line $i: Lorem ipsum dolor sit amet"; done > "$SRC/large.txt"

# ─── 8 种容器格式 ───
echo "[1] 容器格式压缩/解压"
for fmt in zip 7z tar tar.gz tar.xz tar.zst tar.bz2 tar.lz4; do
    archive="$TEST_DIR/test.$fmt"
    extract_dir="$TEST_DIR/extract_$fmt"
    $BIN compress -i "$SRC" -o "$archive" -f "$fmt" -l 3 >/dev/null 2>&1
    $BIN decompress -i "$archive" -o "$extract_dir" >/dev/null 2>&1
    if [ -f "$extract_dir/hello.txt" ] && [ -f "$extract_dir/中文文件.txt" ]; then
        pass "$fmt (含中文文件名)"
    else
        fail "$fmt" "解压内容不完整"
    fi
done

# ─── 5 种单文件流 ───
echo ""
echo "[2] 单文件流压缩/解压"
for fmt in gz xz zst bz2 lz4; do
    archive="$TEST_DIR/single.$fmt"
    extract_dir="$TEST_DIR/extract_single_$fmt"
    mkdir -p "$extract_dir"
    $BIN compress -i "$SRC/hello.txt" -o "$archive" -f "$fmt" -l 3 >/dev/null 2>&1
    $BIN decompress -i "$archive" -o "$extract_dir" >/dev/null 2>&1
    if [ -n "$(find "$extract_dir" -type f 2>/dev/null)" ]; then
        pass "$fmt (单文件流)"
    else
        fail "$fmt" "解压内容为空"
    fi
done

# ─── 加密测试 ───
echo ""
echo "[3] 加密归档"

# 加密 ZIP
enc_zip="$TEST_DIR/enc.zip"; enc_dir="$TEST_DIR/extract_enc_zip"
$BIN compress -i "$SRC" -o "$enc_zip" -f zip -l 3 --password "test123" >/dev/null 2>&1
$BIN decompress -i "$enc_zip" -o "$enc_dir" --password "test123" >/dev/null 2>&1
if [ -f "$enc_dir/hello.txt" ]; then pass "加密 ZIP (AES-256)"; else fail "加密 ZIP" "内容不完整"; fi

# 加密 7z
enc_7z="$TEST_DIR/enc.7z"; enc7_dir="$TEST_DIR/extract_enc_7z"
$BIN compress -i "$SRC" -o "$enc_7z" -f 7z -l 3 --password "test123" >/dev/null 2>&1
$BIN decompress -i "$enc_7z" -o "$enc7_dir" --password "test123" >/dev/null 2>&1
if [ -f "$enc7_dir/hello.txt" ]; then pass "加密 7z (AES-256)"; else fail "加密 7z" "内容不完整"; fi

# .enc 原生加密
enc_file="$TEST_DIR/secret.enc"
$BIN encrypt -i "$SRC/hello.txt" -o "$enc_file" --password "secret456" >/dev/null 2>&1
$BIN decrypt -i "$enc_file" -o "$TEST_DIR/decrypted.txt" --password "secret456" >/dev/null 2>&1
if diff "$SRC/hello.txt" "$TEST_DIR/decrypted.txt" >/dev/null 2>&1; then
    pass ".enc (AES-256-GCM)"
else
    fail ".enc" "解密内容不匹配"
fi

# tar.zst + .enc 包装
tz_enc="$TEST_DIR/archive.tar.zst.enc"; tz_dir="$TEST_DIR/tarzst_enc_out"
$BIN compress -i "$SRC" -o "$TEST_DIR/archive.tar.zst" -f tar.zst -l 3 --password "wrap789" >/dev/null 2>&1
if [ -f "$tz_enc" ]; then
    $BIN smart -i "$tz_enc" -o "$tz_dir" --password "wrap789" >/dev/null 2>&1
    if [ -f "$tz_dir/hello.txt" ]; then pass "tar.zst + .enc 包装"; else fail "tar.zst+.enc" "内容不完整"; fi
else
    fail "tar.zst+.enc" ".enc 文件未生成"
fi

# ─── extract-here / extract-as ───
echo ""
echo "[4] 右键菜单: extract-here / extract-as"

# extract-here
here_dir="$TEST_DIR/here"; mkdir -p "$here_dir"
$BIN compress -i "$SRC" -o "$here_dir/test.zip" -f zip -l 3 >/dev/null 2>&1
$BIN extract-here -i "$here_dir/test.zip" >/dev/null 2>&1
if [ -f "$here_dir/hello.txt" ]; then pass "extract-here"; else fail "extract-here" "内容不在当前文件夹"; fi

# extract-as
as_dir="$TEST_DIR/as_output"; as_zip="$TEST_DIR/as_test.zip"
$BIN compress -i "$SRC" -o "$as_zip" -f zip -l 3 >/dev/null 2>&1
$BIN extract-as -i "$as_zip" -o "$as_dir" >/dev/null 2>&1
if [ -f "$as_dir/hello.txt" ]; then pass "extract-as"; else fail "extract-as" "内容不在目标目录"; fi

# ─── 汇总 ───
echo ""
echo "═══════════════════════════════════════════════════════════"
echo -e "  ${GREEN}通过: $PASS${NC}  ${RED}失败: $FAIL${NC}"
echo "═══════════════════════════════════════════════════════════"

rm -rf "$TEST_DIR"
[ "$FAIL" -eq 0 ]

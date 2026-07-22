#!/usr/bin/env bash
# ──────────────────────────────────────────────────────────────
# smart_ex — macOS .pkg 安装包构建脚本
#
# 生成标准 macOS .pkg 安装包, 包含:
#   - smart_ex.app bundle (含 Info.plist + 隐私权限声明)
#   - 卸载脚本
#   - 安装后自动注册 Launch Services
#
# 用法: ./installer/build_pkg.sh
# ──────────────────────────────────────────────────────────────

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
VERSION="0.3.0"
BUILD_DIR="$PROJECT_DIR/dist/pkg-build"
PAYLOAD_DIR="$BUILD_DIR/payload"

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'
info()  { echo -e "${GREEN}[INFO]${NC}  $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }

# 检查二进制
BIN="$PROJECT_DIR/target/release/smart_ex"
if [[ ! -f "$BIN" ]]; then
    echo "❌ 未找到 release 二进制: $BIN"
    echo "   请先运行: cargo build --release"
    exit 1
fi

# 检查 pkgbuild
if ! command -v pkgbuild &> /dev/null; then
    echo "❌ 未找到 pkgbuild, 请在 macOS 上运行"
    exit 1
fi

info "构建 macOS .pkg 安装包: smart_ex-${VERSION}.pkg"

# 清理旧构建
rm -rf "$BUILD_DIR"
mkdir -p "$PAYLOAD_DIR"

# ═══════════════════════ 1. 创建 .app bundle ═══════════════════════
APP_DIR="$PAYLOAD_DIR/smart_ex.app"
mkdir -p "$APP_DIR/Contents/MacOS"
mkdir -p "$APP_DIR/Contents/Resources"

# 复制二进制
cp "$BIN" "$APP_DIR/Contents/MacOS/smart_ex"
chmod +x "$APP_DIR/Contents/MacOS/smart_ex"

# 复制 Info.plist
cp "$SCRIPT_DIR/Info.plist" "$APP_DIR/Contents/Info.plist"

# 创建 PkgInfo
printf 'APPL????' > "$APP_DIR/Contents/PkgInfo"

# 复制 LICENSE
if [[ -f "$PROJECT_DIR/LICENSE" ]]; then
    cp "$PROJECT_DIR/LICENSE" "$APP_DIR/Contents/Resources/LICENSE.txt"
fi

# 复制卸载脚本到 Resources
cp "$SCRIPT_DIR/uninstall.sh" "$APP_DIR/Contents/Resources/uninstall.sh"
chmod +x "$APP_DIR/Contents/Resources/uninstall.sh"

info ".app bundle 已创建"

# ═══════════════════════ 2. 创建安装后脚本 ═══════════════════════
POSTINSTALL="$BUILD_DIR/scripts/postinstall"
mkdir -p "$BUILD_DIR/scripts"

cat > "$POSTINSTALL" << 'POST'
#!/bin/bash
set -e

APP_PATH="/Applications/smart_ex.app"
LSREGISTER="/System/Library/Frameworks/CoreServices.framework/Versions/A/Frameworks/LaunchServices.framework/Versions/A/Support/lsregister"

# 注册 Launch Services
if [ -x "$LSREGISTER" ]; then
    "$LSREGISTER" -f "$APP_PATH"
fi

# 创建卸载脚本快捷方式
if [ -f "$APP_PATH/Contents/Resources/uninstall.sh" ]; then
    cp "$APP_PATH/Contents/Resources/uninstall.sh" /usr/local/bin/smart_ex-uninstall
    chmod +x /usr/local/bin/smart_ex-uninstall
fi

echo ""
echo "✅ smart_ex 已安装到 /Applications/smart_ex.app"
echo "   启动: open /Applications/smart_ex.app"
echo "   卸载: sudo smart_ex-uninstall"
echo ""
echo "⚠️  首次访问 下载/桌面/文稿 目录时, macOS 会弹出权限授权对话框"
echo "   请点击「允许」以授予 smart_ex 访问权限"
POST
chmod +x "$POSTINSTALL"

# 卸载前脚本
PREUNINSTALL="$BUILD_DIR/scripts/preuninstall"
cat > "$PREUNINSTALL" << 'PRE'
#!/bin/bash
set -e

APP_PATH="/Applications/smart_ex.app"
LSREGISTER="/System/Library/Frameworks/CoreServices.framework/Versions/A/Frameworks/LaunchServices.framework/Versions/A/Support/lsregister"

# 注销 Launch Services
if [ -x "$LSREGISTER" ]; then
    "$LSREGISTER" -u "$APP_PATH" 2>/dev/null || true
fi

# 删除卸载脚本
rm -f /usr/local/bin/smart_ex-uninstall

# 删除 .app
rm -rf "$APP_PATH"

echo "smart_ex 已卸载"
PRE
chmod +x "$PREUNINSTALL"

# ═══════════════════════ 3. 构建 .pkg ═══════════════════════
info "打包中..."
PKG_FILE="$PROJECT_DIR/dist/smart_ex-${VERSION}.pkg"

pkgbuild \
    --root "$PAYLOAD_DIR" \
    --identifier "com.smartex.app" \
    --version "$VERSION" \
    --scripts "$BUILD_DIR/scripts" \
    --install-location "/Applications" \
    "$PKG_FILE"

info "✅ .pkg 包已生成: $PKG_FILE"
info "   大小: $(du -h "$PKG_FILE" | cut -f1)"
echo ""
echo "  安装: sudo installer -pkg $PKG_FILE -target /"
echo "  或双击 .pkg 文件通过 GUI 安装"
echo ""
warn "注意: 安装后可能需要在 系统设置 → 隐私与安全性 中手动授权"

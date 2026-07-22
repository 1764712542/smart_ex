#!/usr/bin/env bash
# ──────────────────────────────────────────────────────────────
# smart_ex — macOS .app bundle 打包脚本
#
# 将 release 二进制打包为标准 .app bundle, 包含 Info.plist (含隐私权限说明).
# 打包后 macOS 会正确识别应用身份, 首次访问 Downloads/Desktop/Documents 时
# 会弹出权限授权对话框 (而非直接返回 Operation not permitted).
#
# 用法: ./installer/build_macos_app.sh [安装路径]
#   默认安装到 ~/Applications/smart_ex.app
# ──────────────────────────────────────────────────────────────

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
BIN="$PROJECT_DIR/target/release/smart_ex"
APP_DIR="${1:-$PROJECT_DIR/dist/smart_ex.app}"

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info()  { echo -e "${GREEN}[INFO]${NC}  $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }

# 检查二进制
if [[ ! -f "$BIN" ]]; then
    echo "❌ 未找到 release 二进制: $BIN"
    echo "   请先运行: cargo build --release"
    exit 1
fi

info "打包 smart_ex.app → $APP_DIR"

# 清理旧的 bundle
rm -rf "$APP_DIR"

# 创建 .app bundle 结构
mkdir -p "$APP_DIR/Contents/MacOS"
mkdir -p "$APP_DIR/Contents/Resources"

# 复制二进制
cp "$BIN" "$APP_DIR/Contents/MacOS/smart_ex"
chmod +x "$APP_DIR/Contents/MacOS/smart_ex"

# 复制 Info.plist
cp "$SCRIPT_DIR/Info.plist" "$APP_DIR/Contents/Info.plist"

# 创建 PkgInfo (8 字节, APPL + 4 字节签名)
printf 'APPL????' > "$APP_DIR/Contents/PkgInfo"

# 复制 LICENSE (如果有)
if [[ -f "$PROJECT_DIR/LICENSE" ]]; then
    cp "$PROJECT_DIR/LICENSE" "$APP_DIR/Contents/Resources/LICENSE.txt"
fi

info ".app bundle 已创建: $APP_DIR"
echo ""
echo "  二进制: $APP_DIR/Contents/MacOS/smart_ex"
echo "  配置:   $APP_DIR/Contents/Info.plist"
echo ""
echo "  启动方式:"
echo "    open $APP_DIR"
echo "    # 或双击 Finder 中的 smart_ex.app"
echo ""
warn "首次访问 下载/桌面/文稿 目录时, macOS 会弹出权限授权对话框"
warn "请点击「允许」以授予 smart_ex 访问权限"
echo ""
warn "如果之前已拒绝, 请在 系统设置 → 隐私与安全性 中手动添加:"
warn "  - 完全磁盘访问权限 → 添加 smart_ex"
warn "  - 或: 文件与文件夹 → smart_ex → 勾选下载/桌面/文稿文件夹"

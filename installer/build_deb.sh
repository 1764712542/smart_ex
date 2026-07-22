#!/usr/bin/env bash
# ──────────────────────────────────────────────────────────────
# smart_ex — Linux .deb 包构建脚本
#
# 生成可直接安装的 .deb 包, 包含:
#   - 二进制 /usr/bin/smart_ex
#   - .desktop 文件 /usr/share/applications/smart_ex.desktop
#   - MIME 类型 /usr/share/mime/packages/smart_ex-mime.xml
#   - 图标 /usr/share/icons/hicolor/512x512/apps/smart_ex.png
#   - 卸载脚本 /usr/bin/smart_ex-uninstall
#
# 用法: ./installer/build_deb.sh [架构]
#   架构: amd64 (默认) | arm64
# ──────────────────────────────────────────────────────────────

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
VERSION="0.3.0"
ARCH="${1:-amd64}"

GREEN='\033[0;32m'
NC='\033[0m'
info() { echo -e "${GREEN}[INFO]${NC}  $*"; }

# 检查二进制
BIN="$PROJECT_DIR/target/release/smart_ex"
if [[ ! -f "$BIN" ]]; then
    echo "❌ 未找到 release 二进制: $BIN"
    echo "   请先运行: cargo build --release"
    exit 1
fi

# 检查 dpkg-deb
if ! command -v dpkg-deb &> /dev/null; then
    echo "❌ 未找到 dpkg-deb, 请在 Debian/Ubuntu 系统上运行"
    exit 1
fi

BUILD_DIR="$PROJECT_DIR/dist/deb-build"
PKG_DIR="$BUILD_DIR/smart_ex_${VERSION}_${ARCH}"

info "构建 .deb 包: smart_ex_${VERSION}_${ARCH}.deb"

# 清理旧构建
rm -rf "$BUILD_DIR"
mkdir -p "$PKG_DIR/DEBIAN"
mkdir -p "$PKG_DIR/usr/bin"
mkdir -p "$PKG_DIR/usr/share/applications"
mkdir -p "$PKG_DIR/usr/share/mime/packages"
mkdir -p "$PKG_DIR/usr/share/icons/hicolor/512x512/apps"
mkdir -p "$PKG_DIR/usr/share/doc/smart_ex"

# 1. 复制二进制
cp "$BIN" "$PKG_DIR/usr/bin/smart_ex"
chmod 755 "$PKG_DIR/usr/bin/smart_ex"

# 2. 复制 .desktop 文件 (修正 Exec 路径)
cp "$SCRIPT_DIR/smart_ex.desktop" "$PKG_DIR/usr/share/applications/smart_ex.desktop"
sed -i 's|Exec=smart_ex|Exec=/usr/bin/smart_ex|g' "$PKG_DIR/usr/share/applications/smart_ex.desktop"
sed -i 's|Icon=smart_ex|Icon=smart_ex|g' "$PKG_DIR/usr/share/applications/smart_ex.desktop"

# 3. 生成 MIME 类型 XML
cat > "$PKG_DIR/usr/share/mime/packages/smart_ex-mime.xml" << 'XML'
<?xml version="1.0" encoding="UTF-8"?>
<mime-info xmlns="http://www.freedesktop.org/standards/shared-mime-info">
  <mime-type type="application/x-smartex-encrypted">
    <comment>SmartEx Encrypted Archive</comment>
    <glob pattern="*.enc"/>
  </mime-type>
  <mime-type type="application/x-lz4">
    <comment>LZ4 Compressed File</comment>
    <glob pattern="*.lz4"/>
  </mime-type>
  <mime-type type="application/x-zstd">
    <comment>Zstandard Compressed File</comment>
    <glob pattern="*.zst"/>
  </mime-type>
  <mime-type type="application/x-zstd-compressed-tar">
    <comment>Zstandard Compressed TAR Archive</comment>
    <glob pattern="*.tar.zst"/>
    <glob pattern="*.tzst"/>
  </mime-type>
  <mime-type type="application/x-lz4-compressed-tar">
    <comment>LZ4 Compressed TAR Archive</comment>
    <glob pattern="*.tar.lz4"/>
  </mime-type>
</mime-info>
XML

# 4. 生成简单图标 (SVG → 临时 PNG 占位)
# 如果没有图标工具, 创建一个简单的 SVG 图标
cat > "$PKG_DIR/usr/share/icons/hicolor/512x512/apps/smart_ex.svg" << 'SVG'
<svg xmlns="http://www.w3.org/2000/svg" width="512" height="512" viewBox="0 0 512 512">
  <rect width="512" height="512" rx="80" fill="#121620"/>
  <text x="256" y="340" font-family="sans-serif" font-size="280" font-weight="bold"
        text-anchor="middle" fill="#78b4ff">S</text>
</svg>
SVG

# 5. 复制卸载脚本
cp "$SCRIPT_DIR/uninstall.sh" "$PKG_DIR/usr/bin/smart_ex-uninstall"
chmod 755 "$PKG_DIR/usr/bin/smart_ex-uninstall"

# 6. 复制 LICENSE
cp "$PROJECT_DIR/LICENSE" "$PKG_DIR/usr/share/doc/smart_ex/copyright"

# 7. 生成 DEBIAN/control
INSTALLED_SIZE=$(du -sk "$PKG_DIR" | cut -f1)
cat > "$PKG_DIR/DEBIAN/control" << CTRL
Package: smart_ex
Version: $VERSION
Section: utils
Priority: optional
Architecture: $ARCH
Depends: libc6
Installed-Size: $INSTALLED_SIZE
Maintainer: smart_ex <smartex@example.com>
Description: Smart compression/decompression with encryption and GUI
 smart_ex is a high-speed compression tool supporting 14+ archive
 formats including ZIP, 7z, RAR, TAR, and more. Features include:
 .
 - AES-256 encryption (compatible with 7-Zip/WinRAR/Bandizip)
 - Beautiful dark glassmorphism GUI (eframe/egui)
 - Right-click context menu integration
 - Chinese/English bilingual interface
 - Cross-platform: Linux, macOS, Windows
 - Free and open source (MIT)
Homepage: https://github.com/smartex/smart_ex
CTRL

# 8. 生成 DEBIAN/postinst (安装后触发)
cat > "$PKG_DIR/DEBIAN/postinst" << 'POST'
#!/bin/sh
set -e
# 更新 MIME 和桌面数据库
update-mime-database /usr/share/mime 2>/dev/null || true
update-desktop-database /usr/share/applications 2>/dev/null || true
gtk-update-icon-cache /usr/share/icons/hicolor 2>/dev/null || true
echo "smart_ex 已安装成功!"
echo "  启动 GUI:  smart_ex"
echo "  命令行:    smart_ex --help"
echo "  卸载:      sudo apt remove smart_ex  或  smart_ex-uninstall"
POST
chmod 755 "$PKG_DIR/DEBIAN/postinst"

# 9. 生成 DEBIAN/prerm (卸载前触发)
cat > "$PKG_DIR/DEBIAN/prerm" << 'PRE'
#!/bin/sh
set -e
# 清理文件关联
update-mime-database /usr/share/mime 2>/dev/null || true
update-desktop-database /usr/share/applications 2>/dev/null || true
PRE
chmod 755 "$PKG_DIR/DEBIAN/prerm"

# 10. 构建 .deb
info "打包中..."
dpkg-deb --build --root-owner-group "$PKG_DIR" "$BUILD_DIR/smart_ex_${VERSION}_${ARCH}.deb"

DEB_FILE="$BUILD_DIR/smart_ex_${VERSION}_${ARCH}.deb"
info "✅ .deb 包已生成: $DEB_FILE"
info "   大小: $(du -h "$DEB_FILE" | cut -f1)"
echo ""
echo "  安装: sudo dpkg -i $DEB_FILE"
echo "  卸载: sudo apt remove smart_ex"

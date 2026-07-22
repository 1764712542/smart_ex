#!/usr/bin/env bash
# ──────────────────────────────────────────────────────────────
# smart_ex — 跨平台卸载脚本 (macOS / Linux)
#
# 清理内容:
#   - 二进制文件 / .app bundle
#   - 文件关联 (.desktop / Info.plist / 注册表)
#   - MIME 类型注册
#   - 右键菜单
#   - 快捷方式
#
# 用法:
#   ./installer/uninstall.sh           # 交互式卸载
#   ./installer/uninstall.sh --yes     # 自动确认卸载
# ──────────────────────────────────────────────────────────────

set -uo pipefail

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

info()  { echo -e "${GREEN}[INFO]${NC}  $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
error() { echo -e "${RED}[ERROR]${NC} $*"; }
step()  { echo -e "${CYAN}[STEP]${NC}  $*"; }

AUTO_YES=false
for arg in "$@"; do
    case $arg in
        --yes|-y) AUTO_YES=true ;;
        --help|-h)
            echo "用法: uninstall.sh [--yes]"
            echo "  --yes, -y  自动确认, 不交互"
            exit 0
            ;;
    esac
done

echo "═══════════════════════════════════════════════════════════"
echo "  smart_ex 卸载程序"
echo "═══════════════════════════════════════════════════════════"
echo ""

# 确认
if [[ "$AUTO_YES" != "true" ]]; then
    read -rp "确定要卸载 smart_ex 吗? [y/N] " confirm
    if [[ "$confirm" != "y" && "$confirm" != "Y" ]]; then
        echo "已取消卸载"
        exit 0
    fi
fi

OS_TYPE="$(uname -s)"
REMOVED=0

if [[ "$OS_TYPE" == "Darwin" ]]; then
    # ═══════════════════════ macOS ═══════════════════════
    step "卸载 macOS 组件..."

    # 1. 删除 .app bundle
    for app_path in \
        "/Applications/smart_ex.app" \
        "$HOME/Applications/smart_ex.app" \
        "$HOME/project/smart_ex/dist/smart_ex.app"; do
        if [[ -d "$app_path" ]]; then
            info "删除: $app_path"
            rm -rf "$app_path"
            ((REMOVED++))
        fi
    done

    # 2. 删除二进制 (可能通过 install.sh 安装到 /usr/local/bin)
    for bin_path in \
        "/usr/local/bin/smart_ex" \
        "$HOME/.local/bin/smart_ex" \
        "/opt/smart_ex/smart_ex"; do
        if [[ -f "$bin_path" ]]; then
            info "删除: $bin_path"
            rm -f "$bin_path" 2>/dev/null || sudo rm -f "$bin_path"
            ((REMOVED++))
        fi
    done

    # 3. 注销 Launch Services 文件关联
    LSREGISTER="/System/Library/Frameworks/CoreServices.framework/Versions/A/Frameworks/LaunchServices.framework/Versions/A/Support/lsregister"
    if [[ -x "$LSREGISTER" ]]; then
        info "注销 Launch Services 注册..."
        "$LSREGISTER" -u "$HOME/Applications/smart_ex.app" 2>/dev/null || true
        "$LSREGISTER" -u "/Applications/smart_ex.app" 2>/dev/null || true
        "$LSREGISTER" -kill -r 2>/dev/null || true
    fi

    # 4. 清除 duti 设置的默认应用
    if command -v duti &> /dev/null; then
        info "清除 duti 默认应用设置..."
        for ext in zip 7z rar tar tgz txz tbz2 gz xz zst bz2 lz4 enc; do
            duti -x "$ext" 2>/dev/null | grep -q "smart_ex" && duti -d "$ext" 0 2>/dev/null || true
        done
    fi

elif [[ "$OS_TYPE" == "Linux" ]]; then
    # ═══════════════════════ Linux ═══════════════════════
    step "卸载 Linux 组件..."

    # 1. 删除二进制
    for bin_path in \
        "/usr/local/bin/smart_ex" \
        "$HOME/.local/bin/smart_ex" \
        "/usr/bin/smart_ex" \
        "/opt/smart_ex/smart_ex"; do
        if [[ -f "$bin_path" ]]; then
            info "删除: $bin_path"
            rm -f "$bin_path" 2>/dev/null || sudo rm -f "$bin_path"
            ((REMOVED++))
        fi
    done

    # 2. 删除 .desktop 文件
    for desktop_path in \
        "/usr/share/applications/smart_ex.desktop" \
        "$HOME/.local/share/applications/smart_ex.desktop"; do
        if [[ -f "$desktop_path" ]]; then
            info "删除: $desktop_path"
            rm -f "$desktop_path" 2>/dev/null || sudo rm -f "$desktop_path"
            ((REMOVED++))
        fi
    done

    # 3. 删除 MIME 类型注册
    for mime_path in \
        "/usr/share/mime/packages/smart_ex-mime.xml" \
        "$HOME/.local/share/mime/packages/smart_ex-mime.xml"; do
        if [[ -f "$mime_path" ]]; then
            info "删除: $mime_path"
            rm -f "$mime_path" 2>/dev/null || sudo rm -f "$mime_path"
            ((REMOVED++))
        fi
    done

    # 4. 更新 MIME 和桌面数据库
    info "更新系统数据库..."
    if command -v update-mime-database &> /dev/null; then
        sudo update-mime-database /usr/share/mime 2>/dev/null || true
        update-mime-database "$HOME/.local/share/mime" 2>/dev/null || true
    fi
    if command -v update-desktop-database &> /dev/null; then
        sudo update-desktop-database /usr/share/applications 2>/dev/null || true
        update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true
    fi

    # 5. 清理 mimeapps.list 中的 smart_ex 条目
    for mimeapps in \
        "$HOME/.config/mimeapps.list" \
        "/etc/xdg/mimeapps.list"; do
        if [[ -f "$mimeapps" ]] && grep -q "smart_ex" "$mimeapps" 2>/dev/null; then
            info "清理: $mimeapps"
            sed -i '/smart_ex/d' "$mimeapps" 2>/dev/null || sudo sed -i '/smart_ex/d' "$mimeapps"
        fi
    done

    # 6. 删除图标 (如果有)
    for icon_path in \
        "/usr/share/icons/hicolor/512x512/apps/smart_ex.png" \
        "/usr/share/icons/hicolor/512x512/apps/smart_ex.svg" \
        "$HOME/.local/share/icons/hicolor/512x512/apps/smart_ex.png" \
        "$HOME/.local/share/icons/hicolor/512x512/apps/smart_ex.svg"; do
        if [[ -f "$icon_path" ]]; then
            info "删除: $icon_path"
            rm -f "$icon_path" 2>/dev/null || sudo rm -f "$icon_path"
        fi
    done

    # 7. 删除 /opt/smart_ex 目录 (如果存在)
    if [[ -d "/opt/smart_ex" ]]; then
        info "删除: /opt/smart_ex"
        rm -rf "/opt/smart_ex" 2>/dev/null || sudo rm -rf "/opt/smart_ex"
    fi
fi

# ═══════════════════════ 通用清理 ═══════════════════════

# 删除配置文件和缓存 (可选)
for config_path in \
    "$HOME/.config/smart_ex" \
    "$HOME/.cache/smart_ex" \
    "$HOME/.smart_ex"; do
    if [[ -d "$config_path" ]]; then
        info "删除: $config_path"
        rm -rf "$config_path"
    fi
done

echo ""
echo "═══════════════════════════════════════════════════════════"
if [[ $REMOVED -gt 0 ]]; then
    echo -e "  ${GREEN}smart_ex 已成功卸载${NC} (清理 $REMOVED 个组件)"
else
    echo -e "  ${YELLOW}未找到 smart_ex 安装组件${NC}"
    echo "  可能未安装, 或通过其他方式安装 (如 cargo install)"
fi
echo "═══════════════════════════════════════════════════════════"

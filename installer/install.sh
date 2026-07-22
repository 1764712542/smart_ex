#!/usr/bin/env bash
# ──────────────────────────────────────────────────────────────
# smart_ex — Linux/macOS 安装脚本
#
# 功能:
#   - 选择安装路径 (默认 /usr/local/bin 或 ~/.local/bin)
#   - 选择绑定的压缩格式 (全选或单选)
#   - 自动注册文件关联和右键菜单
#   - Linux: .desktop 文件 + MIME 类型注册
#   - macOS: .app bundle + Info.plist + Launch Services 注册
#
# 用法:
#   ./installer/install.sh              # 交互式安装
#   ./installer/install.sh --all        # 安装到默认路径并绑定所有格式
#   ./installer/install.sh --path /opt  # 指定安装路径
# ──────────────────────────────────────────────────────────────

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
VERSION="0.5.0"

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

info()  { echo -e "${GREEN}[INFO]${NC}  $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
error() { echo -e "${RED}[ERROR]${NC} $*"; }
step()  { echo -e "${CYAN}[STEP]${NC}  $*"; }
ask()   { echo -e "${BLUE}[?]${NC} $*"; }

# ───────────────────────── 默认配置 ─────────────────────────

INSTALL_PATH=""
BIND_ALL=false
BIND_FORMATS=()

# 所有可绑定的格式
ALL_FORMATS=(
    "zip:.zip:ZIP Archive"
    "7z:.7z:7-Zip Archive"
    "rar:.rar:RAR Archive"
    "tar:.tar:TAR Archive"
    "targz:.tar.gz:Gzip TAR"
    "tarxz:.tar.xz:LZMA TAR"
    "tarzst:.tar.zst:Zstandard TAR"
    "tarbz2:.tar.bz2:BZip2 TAR"
    "tarlz4:.tar.lz4:LZ4 TAR"
    "gz:.gz:Gzip File"
    "xz:.xz:LZMA File"
    "zst:.zst:Zstandard File"
    "bz2:.bz2:BZip2 File"
    "lz4:.lz4:LZ4 File"
    "enc:.enc:SmartEx Encrypted"
)

# ───────────────────────── 参数解析 ─────────────────────────

while [[ $# -gt 0 ]]; do
    case $1 in
        --all)
            BIND_ALL=true
            shift
            ;;
        --path)
            INSTALL_PATH="$2"
            shift 2
            ;;
        --help|-h)
            echo "用法: install.sh [--all] [--path <目录>]"
            echo "  --all          绑定所有格式 (无需交互)"
            echo "  --path <目录>  指定安装路径"
            exit 0
            ;;
        *)
            error "未知参数: $1"
            exit 1
            ;;
    esac
done

# ───────────────────────── 检测二进制 ─────────────────────────

BIN_NAME="smart_ex"
BIN_PATH="$PROJECT_DIR/target/release/$BIN_NAME"

if [[ ! -f "$BIN_PATH" ]]; then
    warn "未找到 release 二进制, 尝试 debug 构建..."
    BIN_PATH="$PROJECT_DIR/target/debug/$BIN_NAME"
    if [[ ! -f "$BIN_PATH" ]]; then
        error "未找到 smart_ex 二进制. 请先运行: cargo build --release"
        exit 1
    fi
fi

info "找到二进制: $BIN_PATH"

# ───────────────────────── 选择安装路径 ─────────────────────────

if [[ -z "$INSTALL_PATH" ]]; then
    if [[ "$BIND_ALL" == "true" ]]; then
        # 非交互模式: 优先 /usr/local/bin, 回退 ~/.local/bin
        if [[ -w "/usr/local/bin" ]] || [[ "$(id -u)" -eq 0 ]]; then
            INSTALL_PATH="/usr/local/bin"
        else
            INSTALL_PATH="$HOME/.local/bin"
        fi
    else
        ask "请选择安装路径:"
        echo "  1) /usr/local/bin     (系统级, 需要 sudo)"
        echo "  2) ~/.local/bin       (用户级, 推荐)"
        echo "  3) /opt/smart_ex      (自定义目录)"
        read -rp "选择 [1-3] (默认 2): " choice
        case "${choice:-2}" in
            1) INSTALL_PATH="/usr/local/bin" ;;
            2) INSTALL_PATH="$HOME/.local/bin" ;;
            3) INSTALL_PATH="/opt/smart_ex" ;;
            *) INSTALL_PATH="$HOME/.local/bin" ;;
        esac
    fi
fi

# 确保目录存在
NEED_SUDO=""
if [[ ! -w "$INSTALL_PATH" ]] && [[ "$(id -u)" -ne 0 ]]; then
    NEED_SUDO="sudo"
fi

$NEED_SUDO mkdir -p "$INSTALL_PATH"
info "安装路径: $INSTALL_PATH"

# ───────────────────────── 复制二进制 ─────────────────────────

step "复制 $BIN_NAME 到 $INSTALL_PATH/"
$NEED_SUDO cp "$BIN_PATH" "$INSTALL_PATH/$BIN_NAME"
$NEED_SUDO chmod +x "$INSTALL_PATH/$BIN_NAME"
info "二进制已安装: $INSTALL_PATH/$BIN_NAME"

# 确保 PATH 包含安装路径
if [[ "$INSTALL_PATH" == "$HOME/.local/bin" ]]; then
    if ! echo "$PATH" | grep -q "$HOME/.local/bin"; then
        warn "PATH 中未包含 $HOME/.local/bin"
        warn "请将以下内容添加到 ~/.bashrc 或 ~/.zshrc:"
        echo '  export PATH="$HOME/.local/bin:$PATH"'
    fi
fi

# ───────────────────────── 选择绑定格式 ─────────────────────────

if [[ "$BIND_ALL" != "true" ]]; then
    echo ""
    ask "要绑定哪些压缩格式? (输入编号, 空格分隔, a=全选, n=跳过)"
    local_i=1
    for fmt in "${ALL_FORMATS[@]}"; do
        ext="${fmt#*.}"
        ext=".${ext}"
        name="${fmt##*:}"
        printf "  %2d) %-12s %s\n" "$local_i" "$ext" "$name"
        ((local_i++))
    done
    echo "   a) 全选"
    echo "   n) 跳过"
    read -rp "选择: " choices

    if [[ "$choices" == "a" || "$choices" == "A" ]]; then
        BIND_ALL=true
    elif [[ "$choices" == "n" || "$choices" == "N" || -z "$choices" ]]; then
        info "跳过文件关联"
    else
        for c in $choices; do
            if [[ "$c" -ge 1 ]] 2>/dev/null && [[ "$c" -le "${#ALL_FORMATS[@]}" ]]; then
                BIND_FORMATS+=("${ALL_FORMATS[$((c-1))]}")
            fi
        done
    fi
fi

if [[ "$BIND_ALL" == "true" ]]; then
    BIND_FORMATS=("${ALL_FORMATS[@]}")
fi

if [[ ${#BIND_FORMATS[@]} -eq 0 ]]; then
    info "未选择任何格式, 安装完成"
    exit 0
fi

info "绑定 ${#BIND_FORMATS[@]} 种格式"

# ───────────────────────── 注册文件关联 ─────────────────────────

OS_TYPE="$(uname -s)"

if [[ "$OS_TYPE" == "Darwin" ]]; then
    # ═══════════════════════ macOS ═══════════════════════
    step "配置 macOS 文件关联..."

    APP_DIR="$HOME/Applications/smart_ex.app"
    if [[ "$(id -u)" -eq 0 ]]; then
        APP_DIR="/Applications/smart_ex.app"
    fi

    # 创建 .app bundle
    $NEED_SUDO mkdir -p "$APP_DIR/Contents/MacOS"
    $NEED_SUDO mkdir -p "$APP_DIR/Contents/Resources"

    # 复制二进制
    $NEED_SUDO cp "$BIN_PATH" "$APP_DIR/Contents/MacOS/$BIN_NAME"
    $NEED_SUDO chmod +x "$APP_DIR/Contents/MacOS/$BIN_NAME"

    # 复制 Info.plist
    $NEED_SUDO cp "$SCRIPT_DIR/Info.plist" "$APP_DIR/Contents/Info.plist"

    # 创建 PkgInfo
    echo -n "APPL????" | $NEED_SUDO tee "$APP_DIR/Contents/PkgInfo" > /dev/null

    info "macOS .app bundle 已创建: $APP_DIR"

    # 注册文件关联
    step "注册文件关联到 Launch Services..."

    # 使用 duti 或 lsregister 设置默认应用
    LSREGISTER="/System/Library/Frameworks/CoreServices.framework/Versions/A/Frameworks/LaunchServices.framework/Versions/A/Support/lsregister"

    # 注册应用
    $NEED_SUDO "$LSREGISTER" -f "$APP_DIR"

    info "macOS 文件关联已注册"

    # 如果有 duti, 设置每种格式的默认应用
    if command -v duti &> /dev/null; then
        for fmt in "${BIND_FORMATS[@]}"; do
            ext="${fmt#*.}"
            ext=".${ext%%:*}"
            duti -s com.smartex.app "$ext" all 2>/dev/null || true
        done
        info "已通过 duti 设置默认应用"
    else
        warn "建议安装 duti 以自动设置默认应用: brew install duti"
        warn "否则请在 Finder 中右键文件 → 显示简介 → 打开方式 → smart_ex"
    fi

elif [[ "$OS_TYPE" == "Linux" ]]; then
    # ═══════════════════════ Linux ═══════════════════════
    step "配置 Linux 文件关联..."

    # 确定应用目录
    if [[ -w "/usr/share/applications" ]] || [[ "$(id -u)" -eq 0 ]]; then
        APPS_DIR="/usr/share/applications"
        MIME_DIR="/usr/share/mime/packages"
        ICON_DIR="/usr/share/icons/hicolor/512x512/apps"
    else
        APPS_DIR="$HOME/.local/share/applications"
        MIME_DIR="$HOME/.local/share/mime/packages"
        ICON_DIR="$HOME/.local/share/icons/hicolor/512x512/apps"
    fi

    $NEED_SUDO mkdir -p "$APPS_DIR" "$MIME_DIR" "$ICON_DIR"

    # 复制 .desktop 文件, 修正 Exec 路径
    DESKTOP_FILE="$APPS_DIR/smart_ex.desktop"
    $NEED_SUDO cp "$SCRIPT_DIR/smart_ex.desktop" "$DESKTOP_FILE"
    $NEED_SUDO sed -i "s|Exec=smart_ex|Exec=$INSTALL_PATH/$BIN_NAME|g" "$DESKTOP_FILE"
    $NEED_SUDO chmod +x "$DESKTOP_FILE"

    info ".desktop 文件已安装: $DESKTOP_FILE"

    # 为绑定格式生成 MIME 类型覆盖
    MIME_XML="$MIME_DIR/smart_ex-mime.xml"
    cat > /tmp/smart_ex-mime.xml << 'XMLHEADER'
<?xml version="1.0" encoding="UTF-8"?>
<mime-info xmlns="http://www.freedesktop.org/standards/shared-mime-info">
XMLHEADER

    declare -A MIME_MAP=(
        ["zip"]="application/zip"
        ["7z"]="application/x-7z-compressed"
        ["rar"]="application/x-rar"
        ["tar"]="application/x-tar"
        ["targz"]="application/x-compressed-tar"
        ["tarxz"]="application/x-xz-compressed-tar"
        ["tarzst"]="application/x-zstd-compressed-tar"
        ["tarbz2"]="application/x-bzip-compressed-tar"
        ["tarlz4"]="application/x-lz4-compressed-tar"
        ["gz"]="application/gzip"
        ["xz"]="application/x-xz"
        ["zst"]="application/x-zstd"
        ["bz2"]="application/x-bzip2"
        ["lz4"]="application/x-lz4"
        ["enc"]="application/x-smartex-encrypted"
    )

    for fmt in "${BIND_FORMATS[@]}"; do
        key="${fmt%%:*}"
        ext="${fmt#*.}"
        ext="${ext%%:*}"  # .zip
        ext="${ext#.}"     # zip
        mime="${MIME_MAP[$key]:-application/octet-stream}"

        cat >> /tmp/smart_ex-mime.xml << XML
  <mime-type type="$mime">
    <comment>SmartEx $ext archive</comment>
    <glob pattern="*.$ext"/>
  </mime-type>
XML
    done

    echo '</mime-info>' >> /tmp/smart_ex-mime.xml

    $NEED_SUDO cp /tmp/smart_ex-mime.xml "$MIME_XML"
    rm -f /tmp/smart_ex-mime.xml

    info "MIME 类型已注册: $MIME_XML"

    # 更新 MIME 数据库
    if command -v update-mime-database &> /dev/null; then
        $NEED_SUDO update-mime-database "${MIME_DIR%/*}" 2>/dev/null || true
    elif command -v update-desktop-database &> /dev/null; then
        $NEED_SUDO update-desktop-database "$APPS_DIR" 2>/dev/null || true
    fi

    # 设置默认应用 (mimeapps.list)
    MIMEAPPS="$HOME/.config/mimeapps.list"
    mkdir -p "$(dirname "$MIMEAPPS")"

    if [[ ! -f "$MIMEAPPS" ]]; then
        echo "[Default Applications]" > "$MIMEAPPS"
    fi

    for fmt in "${BIND_FORMATS[@]}"; do
        key="${fmt%%:*}"
        mime="${MIME_MAP[$key]:-application/octet-stream}"
        # 只在尚未设置时添加 (避免覆盖用户已有偏好)
        if ! grep -q "$mime=" "$MIMEAPPS" 2>/dev/null; then
            echo "$mime=smart_ex.desktop" >> "$MIMEAPPS"
        fi
    done

    info "默认应用已设置: $MIMEAPPS"

    # 更新桌面数据库
    if command -v update-desktop-database &> /dev/null; then
        update-desktop-database "$APPS_DIR" 2>/dev/null || true
    fi

    echo ""
    info "Linux 文件关联已配置完成"
    info "右键压缩文件即可看到 '解压到当前文件夹' 和 '解压另存为...' 选项"
fi

# ───────────────────────── 完成 ─────────────────────────

echo ""
echo -e "${GREEN}═══════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}  smart_ex v$VERSION 安装完成!${NC}"
echo -e "${GREEN}═══════════════════════════════════════════════════════════${NC}"
echo ""
echo "  二进制位置:  $INSTALL_PATH/$BIN_NAME"
echo "  启动 GUI:    $BIN_NAME"
echo "  命令行:      $BIN_NAME --help"
echo ""
echo "  支持格式:    zip 7z rar tar tar.gz tar.xz tar.zst tar.bz2 tar.lz4 gz xz zst bz2 lz4 enc"
echo "  加密兼容:    AES-256 (ZIP/7z) / ZipCrypto / RAR / smart_ex .enc"
echo "  右键菜单:    解压到当前文件夹 / 解压另存为..."
echo ""
if [[ "$OS_TYPE" == "Darwin" ]]; then
    echo "  注意: 如需设置默认应用, 建议安装 duti:"
    echo "    brew install duti"
fi

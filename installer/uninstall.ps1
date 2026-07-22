# ──────────────────────────────────────────────────────────────
# smart_ex — Windows 卸载脚本 (PowerShell)
#
# 清理内容:
#   - 二进制文件
#   - 注册表文件关联
#   - 右键菜单
#   - 快捷方式
#
# 用法: 以管理员身份运行 PowerShell
#   .\uninstall.ps1              # 交互式卸载
#   .\uninstall.ps1 -Auto        # 自动确认
# ──────────────────────────────────────────────────────────────

param([switch]$Auto)

$ErrorActionPreference = "SilentlyContinue"

Write-Host "═══════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "  smart_ex Windows 卸载程序" -ForegroundColor Cyan
Write-Host "═══════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""

# 确认
if (-not $Auto) {
    $confirm = Read-Host "确定要卸载 smart_ex 吗? [y/N]"
    if ($confirm -notmatch "^[yY]$") {
        Write-Host "已取消卸载" -ForegroundColor Yellow
        exit 0
    }
}

# 检查管理员权限
$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    Write-Host "[WARN] 未以管理员身份运行, 部分注册表项可能无法删除" -ForegroundColor Yellow
}

$removed = 0

# 1. 删除安装目录
$installPaths = @(
    "${env:ProgramFiles}\smart_ex",
    "${env:ProgramFiles(x86)}\smart_ex",
    "${env:LOCALAPPDATA}\smart_ex",
    "$env:USERPROFILE\.local\bin\smart_ex"
)

foreach ($path in $installPaths) {
    if (Test-Path $path) {
        Write-Host "[INFO]  删除: $path" -ForegroundColor Green
        Remove-Item -Path $path -Recurse -Force
        $removed++
    }
}

# 2. 清理注册表文件关联
$extensions = @(
    ".zip", ".7z", ".rar", ".r00", ".tar",
    ".tar.gz", ".tgz", ".tar.xz", ".txz",
    ".tar.zst", ".tzst", ".tar.bz2", ".tbz2",
    ".tar.lz4", ".gz", ".xz", ".zst", ".bz2", ".lz4", ".enc"
)

$progIds = @(
    "smart_ex.archive", "smart_ex.7z", "smart_ex.rar",
    "smart_ex.tar", "smart_ex.targz", "smart_ex.tarxz",
    "smart_ex.tarzst", "smart_ex.tarbz2", "smart_ex.tarlz4",
    "smart_ex.gz", "smart_ex.xz", "smart_ex.zst",
    "smart_ex.bz2", "smart_ex.lz4", "smart_ex.enc"
)

Write-Host "[STEP] 清理注册表文件关联..." -ForegroundColor Cyan

foreach ($ext in $extensions) {
    $key = "HKCR:\$ext"
    if (Test-Path $key) {
        $val = (Get-ItemProperty -Path $key -ErrorAction SilentlyContinue)."(default)"
        if ($val -match "smart_ex") {
            Remove-Item -Path $key -Recurse -Force
            Write-Host "  [INFO]  清理: $ext" -ForegroundColor Green
        }
    }
}

foreach ($progId in $progIds) {
    $key = "HKCR:\$progId"
    if (Test-Path $key) {
        Remove-Item -Path $key -Recurse -Force
        Write-Host "  [INFO]  清理: $progId" -ForegroundColor Green
        $removed++
    }
}

# 3. 清理右键菜单
$shellKeys = @(
    "HKCR:\*\shell\smart_ex_compress",
    "HKCR:\Directory\shell\smart_ex_compress"
)

foreach ($key in $shellKeys) {
    if (Test-Path $key) {
        Remove-Item -Path $key -Recurse -Force
        Write-Host "  [INFO]  清理右键菜单: $key" -ForegroundColor Green
        $removed++
    }
}

# 4. 删除开始菜单快捷方式
$startMenuPaths = @(
    "${env:ProgramData}\Microsoft\Windows\Start Menu\Programs\smart_ex",
    "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\smart_ex"
)

foreach ($path in $startMenuPaths) {
    if (Test-Path $path) {
        Remove-Item -Path $path -Recurse -Force
        Write-Host "[INFO]  删除快捷方式: $path" -ForegroundColor Green
        $removed++
    }
}

# 5. 删除桌面快捷方式
$desktopLinks = @(
    "$env:USERPROFILE\Desktop\smart_ex.lnk",
    "${env:PUBLIC}\Desktop\smart_ex.lnk"
)

foreach ($link in $desktopLinks) {
    if (Test-Path $link) {
        Remove-Item -Path $link -Force
        Write-Host "[INFO]  删除: $link" -ForegroundColor Green
        $removed++
    }
}

# 6. 删除注册表中的卸载信息 (Inno Setup 创建的)
$uninstallKey = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\{B7F3E2A1-9C4D-4E8B-AF12-6D5A3C7E9F01}_is1"
if (Test-Path $uninstallKey) {
    Remove-Item -Path $uninstallKey -Recurse -Force
    Write-Host "[INFO]  清理卸载注册信息" -ForegroundColor Green
    $removed++
}

# 7. 刷新资源管理器
Write-Host "[STEP] 刷新系统..." -ForegroundColor Cyan
taskkill /f /im explorer.exe 2>$null
Start-Process explorer.exe

Write-Host ""
Write-Host "═══════════════════════════════════════════════════════════" -ForegroundColor Cyan
if ($removed -gt 0) {
    Write-Host "  smart_ex 已成功卸载 (清理 $removed 个组件)" -ForegroundColor Green
} else {
    Write-Host "  未找到 smart_ex 安装组件" -ForegroundColor Yellow
}
Write-Host "═══════════════════════════════════════════════════════════" -ForegroundColor Cyan

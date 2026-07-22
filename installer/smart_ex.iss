; ──────────────────────────────────────────────────────────────
; smart_ex — Windows Inno Setup 安装脚本
;
; 功能: 可选安装路径 / 可选文件格式绑定 (全选或单选) /
;       右键菜单 "解压到当前文件夹" + "解压另存为..." /
;       桌面 & 开始菜单快捷方式
;
; 编译: iscc installer\smart_ex.iss
; ──────────────────────────────────────────────────────────────

#define MyAppName "smart_ex"
#define MyAppVersion "0.5.0"
#define MyAppPublisher "smart_ex"
#define MyAppExeName "smart_ex.exe"
#define MyAppDescription "Smart Compression & Decompression"

[Setup]
AppId={{B7F3E2A1-9C4D-4E8B-AF12-6D5A3C7E9F01}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppDescription={#MyAppDescription}
AppCopyright=Copyright (C) 2026 {#MyAppPublisher}
License=LICENSE
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
OutputDir=installer\output
OutputBaseFilename=smart_ex-{#MyAppVersion}-windows-x64-setup
Compression=lzma2/ultra
SolidCompression=yes
ArchitecturesAllowed=x64
ArchitecturesInstallIn64BitMode=x64
PrivilegesRequired=admin
UninstallDisplayIcon={app}\{#MyAppExeName}
WizardStyle=modern
ShowLanguageDialog=yes

[Languages]
Name: "chinesesimp"; MessagesFile: "compiler:Languages\ChineseSimplified.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

; ───────────────────────── 组件 (文件关联选择) ─────────────────────────
; 父组件 "assoc" = 全选; 子组件 = 单独格式
; 勾选父组件自动勾选所有子组件; 取消父组件自动取消所有子组件
[Components]
Name: "main";        Description: "smart_ex 主程序 (必需)"; Types: full compact custom; Flags: fixed
Name: "assoc";       Description: "绑定压缩文件格式 (右键解压)"; Types: full
Name: "assoc\zip";   Description: ".zip  — ZIP 归档"; Types: full
Name: "assoc\7z";    Description: ".7z   — 7-Zip 归档"; Types: full
Name: "assoc\rar";   Description: ".rar  — RAR 归档 (仅解压)"; Types: full
Name: "assoc\tar";   Description: ".tar  — TAR 归档"; Types: full
Name: "assoc\targz"; Description: ".tar.gz / .tgz — Gzip 压缩 TAR"; Types: full
Name: "assoc\tarxz"; Description: ".tar.xz / .txz — LZMA 压缩 TAR"; Types: full
Name: "assoc\tarzst";Description: ".tar.zst / .tzst — Zstandard 压缩 TAR"; Types: full
Name: "assoc\tarbz2";Description: ".tar.bz2 / .tbz2 — BZip2 压缩 TAR"; Types: full
Name: "assoc\tarlz4";Description: ".tar.lz4 — LZ4 压缩 TAR"; Types: full
Name: "assoc\gz";    Description: ".gz   — Gzip 单文件"; Types: full
Name: "assoc\xz";    Description: ".xz   — LZMA 单文件"; Types: full
Name: "assoc\zst";   Description: ".zst  — Zstandard 单文件"; Types: full
Name: "assoc\bz2";   Description: ".bz2  — BZip2 单文件"; Types: full
Name: "assoc\lz4";   Description: ".lz4  — LZ4 单文件"; Types: full
Name: "assoc\enc";   Description: ".enc  — smart_ex 加密归档"; Types: full

; ───────────────────────── 安装文件 ─────────────────────────
[Files]
Source: "target\release\{#MyAppExeName}"; DestDir: "{app}"; Components: main; Flags: ignoreversion
Source: "LICENSE"; DestDir: "{app}"; Components: main; Flags: ignoreversion

; ───────────────────────── 快捷方式 ─────────────────────────
[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Components: main
Name: "{group}\卸载 {#MyAppName}"; Filename: "{uninstallexe}"; Components: main
Name: "{commondesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon; Components: main

[Tasks]
Name: "desktopicon"; Description: "创建桌面快捷方式"; GroupDescription: "附加选项:"

; ───────────────────────── 注册表: 文件关联 + 右键菜单 ─────────────────────────
; 每种格式注册:
;   .ext → smart_ex.archive
;   smart_ex.archive\shell\open          → 双击解压
;   smart_ex.archive\shell\extractHere   → 右键 "解压到当前文件夹"
;   smart_ex.archive\shell\extractAs     → 右键 "解压另存为..."
[Registry]

; === .zip ===
Root: HKCR; Subkey: ".zip"; ValueType: string; ValueName: ""; ValueData: "smart_ex.archive"; Flags: uninsdeletevalue; Components: assoc\zip
Root: HKCR; Subkey: "smart_ex.archive"; ValueType: string; ValueName: ""; ValueData: "smart_ex Archive"; Flags: uninsdeletekey; Components: assoc\zip
Root: HKCR; Subkey: "smart_ex.archive\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#MyAppExeName},0"; Components: assoc\zip
Root: HKCR; Subkey: "smart_ex.archive\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" decompress -i ""%1"""; Components: assoc\zip
Root: HKCR; Subkey: "smart_ex.archive\shell\extractHere"; ValueType: string; ValueName: ""; ValueData: "解压到当前文件夹"; Components: assoc\zip
Root: HKCR; Subkey: "smart_ex.archive\shell\extractHere\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" extract-here -i ""%1"""; Components: assoc\zip
Root: HKCR; Subkey: "smart_ex.archive\shell\extractAs"; ValueType: string; ValueName: ""; ValueData: "解压另存为..."; Components: assoc\zip
Root: HKCR; Subkey: "smart_ex.archive\shell\extractAs\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" extract-as -i ""%1"""; Components: assoc\zip

; === .7z ===
Root: HKCR; Subkey: ".7z"; ValueType: string; ValueName: ""; ValueData: "smart_ex.7z"; Flags: uninsdeletevalue; Components: assoc\7z
Root: HKCR; Subkey: "smart_ex.7z"; ValueType: string; ValueName: ""; ValueData: "smart_ex 7-Zip Archive"; Flags: uninsdeletekey; Components: assoc\7z
Root: HKCR; Subkey: "smart_ex.7z\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#MyAppExeName},0"; Components: assoc\7z
Root: HKCR; Subkey: "smart_ex.7z\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" decompress -i ""%1"""; Components: assoc\7z
Root: HKCR; Subkey: "smart_ex.7z\shell\extractHere"; ValueType: string; ValueName: ""; ValueData: "解压到当前文件夹"; Components: assoc\7z
Root: HKCR; Subkey: "smart_ex.7z\shell\extractHere\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" extract-here -i ""%1"""; Components: assoc\7z
Root: HKCR; Subkey: "smart_ex.7z\shell\extractAs"; ValueType: string; ValueName: ""; ValueData: "解压另存为..."; Components: assoc\7z
Root: HKCR; Subkey: "smart_ex.7z\shell\extractAs\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" extract-as -i ""%1"""; Components: assoc\7z

; === .rar / .r00 ===
Root: HKCR; Subkey: ".rar"; ValueType: string; ValueName: ""; ValueData: "smart_ex.rar"; Flags: uninsdeletevalue; Components: assoc\rar
Root: HKCR; Subkey: ".r00"; ValueType: string; ValueName: ""; ValueData: "smart_ex.rar"; Flags: uninsdeletevalue; Components: assoc\rar
Root: HKCR; Subkey: "smart_ex.rar"; ValueType: string; ValueName: ""; ValueData: "smart_ex RAR Archive"; Flags: uninsdeletekey; Components: assoc\rar
Root: HKCR; Subkey: "smart_ex.rar\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#MyAppExeName},0"; Components: assoc\rar
Root: HKCR; Subkey: "smart_ex.rar\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" decompress -i ""%1"""; Components: assoc\rar
Root: HKCR; Subkey: "smart_ex.rar\shell\extractHere"; ValueType: string; ValueName: ""; ValueData: "解压到当前文件夹"; Components: assoc\rar
Root: HKCR; Subkey: "smart_ex.rar\shell\extractHere\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" extract-here -i ""%1"""; Components: assoc\rar
Root: HKCR; Subkey: "smart_ex.rar\shell\extractAs"; ValueType: string; ValueName: ""; ValueData: "解压另存为..."; Components: assoc\rar
Root: HKCR; Subkey: "smart_ex.rar\shell\extractAs\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" extract-as -i ""%1"""; Components: assoc\rar

; === .tar ===
Root: HKCR; Subkey: ".tar"; ValueType: string; ValueName: ""; ValueData: "smart_ex.tar"; Flags: uninsdeletevalue; Components: assoc\tar
Root: HKCR; Subkey: "smart_ex.tar"; ValueType: string; ValueName: ""; ValueData: "smart_ex TAR Archive"; Flags: uninsdeletekey; Components: assoc\tar
Root: HKCR; Subkey: "smart_ex.tar\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#MyAppExeName},0"; Components: assoc\tar
Root: HKCR; Subkey: "smart_ex.tar\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" decompress -i ""%1"""; Components: assoc\tar
Root: HKCR; Subkey: "smart_ex.tar\shell\extractHere"; ValueType: string; ValueName: ""; ValueData: "解压到当前文件夹"; Components: assoc\tar
Root: HKCR; Subkey: "smart_ex.tar\shell\extractHere\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" extract-here -i ""%1"""; Components: assoc\tar
Root: HKCR; Subkey: "smart_ex.tar\shell\extractAs"; ValueType: string; ValueName: ""; ValueData: "解压另存为..."; Components: assoc\tar
Root: HKCR; Subkey: "smart_ex.tar\shell\extractAs\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" extract-as -i ""%1"""; Components: assoc\tar

; === .tar.gz / .tgz ===
Root: HKCR; Subkey: ".tar.gz"; ValueType: string; ValueName: ""; ValueData: "smart_ex.targz"; Flags: uninsdeletevalue; Components: assoc\targz
Root: HKCR; Subkey: ".tgz"; ValueType: string; ValueName: ""; ValueData: "smart_ex.targz"; Flags: uninsdeletevalue; Components: assoc\targz
Root: HKCR; Subkey: "smart_ex.targz"; ValueType: string; ValueName: ""; ValueData: "smart_ex Gzip TAR"; Flags: uninsdeletekey; Components: assoc\targz
Root: HKCR; Subkey: "smart_ex.targz\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#MyAppExeName},0"; Components: assoc\targz
Root: HKCR; Subkey: "smart_ex.targz\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" decompress -i ""%1"""; Components: assoc\targz
Root: HKCR; Subkey: "smart_ex.targz\shell\extractHere"; ValueType: string; ValueName: ""; ValueData: "解压到当前文件夹"; Components: assoc\targz
Root: HKCR; Subkey: "smart_ex.targz\shell\extractHere\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" extract-here -i ""%1"""; Components: assoc\targz
Root: HKCR; Subkey: "smart_ex.targz\shell\extractAs"; ValueType: string; ValueName: ""; ValueData: "解压另存为..."; Components: assoc\targz
Root: HKCR; Subkey: "smart_ex.targz\shell\extractAs\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" extract-as -i ""%1"""; Components: assoc\targz

; === .tar.xz / .txz ===
Root: HKCR; Subkey: ".tar.xz"; ValueType: string; ValueName: ""; ValueData: "smart_ex.tarxz"; Flags: uninsdeletevalue; Components: assoc\tarxz
Root: HKCR; Subkey: ".txz"; ValueType: string; ValueName: ""; ValueData: "smart_ex.tarxz"; Flags: uninsdeletevalue; Components: assoc\tarxz
Root: HKCR; Subkey: "smart_ex.tarxz"; ValueType: string; ValueName: ""; ValueData: "smart_ex LZMA TAR"; Flags: uninsdeletekey; Components: assoc\tarxz
Root: HKCR; Subkey: "smart_ex.tarxz\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#MyAppExeName},0"; Components: assoc\tarxz
Root: HKCR; Subkey: "smart_ex.tarxz\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" decompress -i ""%1"""; Components: assoc\tarxz
Root: HKCR; Subkey: "smart_ex.tarxz\shell\extractHere"; ValueType: string; ValueName: ""; ValueData: "解压到当前文件夹"; Components: assoc\tarxz
Root: HKCR; Subkey: "smart_ex.tarxz\shell\extractHere\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" extract-here -i ""%1"""; Components: assoc\tarxz
Root: HKCR; Subkey: "smart_ex.tarxz\shell\extractAs"; ValueType: string; ValueName: ""; ValueData: "解压另存为..."; Components: assoc\tarxz
Root: HKCR; Subkey: "smart_ex.tarxz\shell\extractAs\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" extract-as -i ""%1"""; Components: assoc\tarxz

; === .tar.zst / .tzst ===
Root: HKCR; Subkey: ".tar.zst"; ValueType: string; ValueName: ""; ValueData: "smart_ex.tarzst"; Flags: uninsdeletevalue; Components: assoc\tarzst
Root: HKCR; Subkey: ".tzst"; ValueType: string; ValueName: ""; ValueData: "smart_ex.tarzst"; Flags: uninsdeletevalue; Components: assoc\tarzst
Root: HKCR; Subkey: "smart_ex.tarzst"; ValueType: string; ValueName: ""; ValueData: "smart_ex Zstandard TAR"; Flags: uninsdeletekey; Components: assoc\tarzst
Root: HKCR; Subkey: "smart_ex.tarzst\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#MyAppExeName},0"; Components: assoc\tarzst
Root: HKCR; Subkey: "smart_ex.tarzst\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" decompress -i ""%1"""; Components: assoc\tarzst
Root: HKCR; Subkey: "smart_ex.tarzst\shell\extractHere"; ValueType: string; ValueName: ""; ValueData: "解压到当前文件夹"; Components: assoc\tarzst
Root: HKCR; Subkey: "smart_ex.tarzst\shell\extractHere\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" extract-here -i ""%1"""; Components: assoc\tarzst
Root: HKCR; Subkey: "smart_ex.tarzst\shell\extractAs"; ValueType: string; ValueName: ""; ValueData: "解压另存为..."; Components: assoc\tarzst
Root: HKCR; Subkey: "smart_ex.tarzst\shell\extractAs\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" extract-as -i ""%1"""; Components: assoc\tarzst

; === .tar.bz2 / .tbz2 ===
Root: HKCR; Subkey: ".tar.bz2"; ValueType: string; ValueName: ""; ValueData: "smart_ex.tarbz2"; Flags: uninsdeletevalue; Components: assoc\tarbz2
Root: HKCR; Subkey: ".tbz2"; ValueType: string; ValueName: ""; ValueData: "smart_ex.tarbz2"; Flags: uninsdeletevalue; Components: assoc\tarbz2
Root: HKCR; Subkey: "smart_ex.tarbz2"; ValueType: string; ValueName: ""; ValueData: "smart_ex BZip2 TAR"; Flags: uninsdeletekey; Components: assoc\tarbz2
Root: HKCR; Subkey: "smart_ex.tarbz2\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#MyAppExeName},0"; Components: assoc\tarbz2
Root: HKCR; Subkey: "smart_ex.tarbz2\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" decompress -i ""%1"""; Components: assoc\tarbz2
Root: HKCR; Subkey: "smart_ex.tarbz2\shell\extractHere"; ValueType: string; ValueName: ""; ValueData: "解压到当前文件夹"; Components: assoc\tarbz2
Root: HKCR; Subkey: "smart_ex.tarbz2\shell\extractHere\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" extract-here -i ""%1"""; Components: assoc\tarbz2
Root: HKCR; Subkey: "smart_ex.tarbz2\shell\extractAs"; ValueType: string; ValueName: ""; ValueData: "解压另存为..."; Components: assoc\tarbz2
Root: HKCR; Subkey: "smart_ex.tarbz2\shell\extractAs\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" extract-as -i ""%1"""; Components: assoc\tarbz2

; === .tar.lz4 ===
Root: HKCR; Subkey: ".tar.lz4"; ValueType: string; ValueName: ""; ValueData: "smart_ex.tarlz4"; Flags: uninsdeletevalue; Components: assoc\tarlz4
Root: HKCR; Subkey: "smart_ex.tarlz4"; ValueType: string; ValueName: ""; ValueData: "smart_ex LZ4 TAR"; Flags: uninsdeletekey; Components: assoc\tarlz4
Root: HKCR; Subkey: "smart_ex.tarlz4\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#MyAppExeName},0"; Components: assoc\tarlz4
Root: HKCR; Subkey: "smart_ex.tarlz4\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" decompress -i ""%1"""; Components: assoc\tarlz4
Root: HKCR; Subkey: "smart_ex.tarlz4\shell\extractHere"; ValueType: string; ValueName: ""; ValueData: "解压到当前文件夹"; Components: assoc\tarlz4
Root: HKCR; Subkey: "smart_ex.tarlz4\shell\extractHere\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" extract-here -i ""%1"""; Components: assoc\tarlz4
Root: HKCR; Subkey: "smart_ex.tarlz4\shell\extractAs"; ValueType: string; ValueName: ""; ValueData: "解压另存为..."; Components: assoc\tarlz4
Root: HKCR; Subkey: "smart_ex.tarlz4\shell\extractAs\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" extract-as -i ""%1"""; Components: assoc\tarlz4

; === .gz ===
Root: HKCR; Subkey: ".gz"; ValueType: string; ValueName: ""; ValueData: "smart_ex.gz"; Flags: uninsdeletevalue; Components: assoc\gz
Root: HKCR; Subkey: "smart_ex.gz"; ValueType: string; ValueName: ""; ValueData: "smart_ex Gzip File"; Flags: uninsdeletekey; Components: assoc\gz
Root: HKCR; Subkey: "smart_ex.gz\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#MyAppExeName},0"; Components: assoc\gz
Root: HKCR; Subkey: "smart_ex.gz\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" decompress -i ""%1"""; Components: assoc\gz
Root: HKCR; Subkey: "smart_ex.gz\shell\extractHere"; ValueType: string; ValueName: ""; ValueData: "解压到当前文件夹"; Components: assoc\gz
Root: HKCR; Subkey: "smart_ex.gz\shell\extractHere\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" extract-here -i ""%1"""; Components: assoc\gz
Root: HKCR; Subkey: "smart_ex.gz\shell\extractAs"; ValueType: string; ValueName: ""; ValueData: "解压另存为..."; Components: assoc\gz
Root: HKCR; Subkey: "smart_ex.gz\shell\extractAs\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" extract-as -i ""%1"""; Components: assoc\gz

; === .xz ===
Root: HKCR; Subkey: ".xz"; ValueType: string; ValueName: ""; ValueData: "smart_ex.xz"; Flags: uninsdeletevalue; Components: assoc\xz
Root: HKCR; Subkey: "smart_ex.xz"; ValueType: string; ValueName: ""; ValueData: "smart_ex LZMA File"; Flags: uninsdeletekey; Components: assoc\xz
Root: HKCR; Subkey: "smart_ex.xz\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#MyAppExeName},0"; Components: assoc\xz
Root: HKCR; Subkey: "smart_ex.xz\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" decompress -i ""%1"""; Components: assoc\xz
Root: HKCR; Subkey: "smart_ex.xz\shell\extractHere"; ValueType: string; ValueName: ""; ValueData: "解压到当前文件夹"; Components: assoc\xz
Root: HKCR; Subkey: "smart_ex.xz\shell\extractHere\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" extract-here -i ""%1"""; Components: assoc\xz
Root: HKCR; Subkey: "smart_ex.xz\shell\extractAs"; ValueType: string; ValueName: ""; ValueData: "解压另存为..."; Components: assoc\xz
Root: HKCR; Subkey: "smart_ex.xz\shell\extractAs\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" extract-as -i ""%1"""; Components: assoc\xz

; === .zst ===
Root: HKCR; Subkey: ".zst"; ValueType: string; ValueName: ""; ValueData: "smart_ex.zst"; Flags: uninsdeletevalue; Components: assoc\zst
Root: HKCR; Subkey: "smart_ex.zst"; ValueType: string; ValueName: ""; ValueData: "smart_ex Zstandard File"; Flags: uninsdeletekey; Components: assoc\zst
Root: HKCR; Subkey: "smart_ex.zst\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#MyAppExeName},0"; Components: assoc\zst
Root: HKCR; Subkey: "smart_ex.zst\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" decompress -i ""%1"""; Components: assoc\zst
Root: HKCR; Subkey: "smart_ex.zst\shell\extractHere"; ValueType: string; ValueName: ""; ValueData: "解压到当前文件夹"; Components: assoc\zst
Root: HKCR; Subkey: "smart_ex.zst\shell\extractHere\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" extract-here -i ""%1"""; Components: assoc\zst
Root: HKCR; Subkey: "smart_ex.zst\shell\extractAs"; ValueType: string; ValueName: ""; ValueData: "解压另存为..."; Components: assoc\zst
Root: HKCR; Subkey: "smart_ex.zst\shell\extractAs\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" extract-as -i ""%1"""; Components: assoc\zst

; === .bz2 ===
Root: HKCR; Subkey: ".bz2"; ValueType: string; ValueName: ""; ValueData: "smart_ex.bz2"; Flags: uninsdeletevalue; Components: assoc\bz2
Root: HKCR; Subkey: "smart_ex.bz2"; ValueType: string; ValueName: ""; ValueData: "smart_ex BZip2 File"; Flags: uninsdeletekey; Components: assoc\bz2
Root: HKCR; Subkey: "smart_ex.bz2\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#MyAppExeName},0"; Components: assoc\bz2
Root: HKCR; Subkey: "smart_ex.bz2\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" decompress -i ""%1"""; Components: assoc\bz2
Root: HKCR; Subkey: "smart_ex.bz2\shell\extractHere"; ValueType: string; ValueName: ""; ValueData: "解压到当前文件夹"; Components: assoc\bz2
Root: HKCR; Subkey: "smart_ex.bz2\shell\extractHere\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" extract-here -i ""%1"""; Components: assoc\bz2
Root: HKCR; Subkey: "smart_ex.bz2\shell\extractAs"; ValueType: string; ValueName: ""; ValueData: "解压另存为..."; Components: assoc\bz2
Root: HKCR; Subkey: "smart_ex.bz2\shell\extractAs\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" extract-as -i ""%1"""; Components: assoc\bz2

; === .lz4 ===
Root: HKCR; Subkey: ".lz4"; ValueType: string; ValueName: ""; ValueData: "smart_ex.lz4"; Flags: uninsdeletevalue; Components: assoc\lz4
Root: HKCR; Subkey: "smart_ex.lz4"; ValueType: string; ValueName: ""; ValueData: "smart_ex LZ4 File"; Flags: uninsdeletekey; Components: assoc\lz4
Root: HKCR; Subkey: "smart_ex.lz4\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#MyAppExeName},0"; Components: assoc\lz4
Root: HKCR; Subkey: "smart_ex.lz4\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" decompress -i ""%1"""; Components: assoc\lz4
Root: HKCR; Subkey: "smart_ex.lz4\shell\extractHere"; ValueType: string; ValueName: ""; ValueData: "解压到当前文件夹"; Components: assoc\lz4
Root: HKCR; Subkey: "smart_ex.lz4\shell\extractHere\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" extract-here -i ""%1"""; Components: assoc\lz4
Root: HKCR; Subkey: "smart_ex.lz4\shell\extractAs"; ValueType: string; ValueName: ""; ValueData: "解压另存为..."; Components: assoc\lz4
Root: HKCR; Subkey: "smart_ex.lz4\shell\extractAs\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" extract-as -i ""%1"""; Components: assoc\lz4

; === .enc (smart_ex 加密归档) ===
Root: HKCR; Subkey: ".enc"; ValueType: string; ValueName: ""; ValueData: "smart_ex.enc"; Flags: uninsdeletevalue; Components: assoc\enc
Root: HKCR; Subkey: "smart_ex.enc"; ValueType: string; ValueName: ""; ValueData: "smart_ex Encrypted Archive"; Flags: uninsdeletekey; Components: assoc\enc
Root: HKCR; Subkey: "smart_ex.enc\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#MyAppExeName},0"; Components: assoc\enc
Root: HKCR; Subkey: "smart_ex.enc\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" gui"; Components: assoc\enc
Root: HKCR; Subkey: "smart_ex.enc\shell\extractHere"; ValueType: string; ValueName: ""; ValueData: "解密到当前文件夹"; Components: assoc\enc
Root: HKCR; Subkey: "smart_ex.enc\shell\extractHere\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" extract-here -i ""%1"""; Components: assoc\enc
Root: HKCR; Subkey: "smart_ex.enc\shell\extractAs"; ValueType: string; ValueName: ""; ValueData: "解密另存为..."; Components: assoc\enc
Root: HKCR; Subkey: "smart_ex.enc\shell\extractAs\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" extract-as -i ""%1"""; Components: assoc\enc

; === 右键菜单: 文件/文件夹压缩 ===
Root: HKCR; Subkey: "*\shell\smart_ex_compress"; ValueType: string; ValueName: ""; ValueData: "用 smart_ex 压缩..."; Components: main
Root: HKCR; Subkey: "*\shell\smart_ex_compress\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" gui"; Components: main
Root: HKCR; Subkey: "Directory\shell\smart_ex_compress"; ValueType: string; ValueName: ""; ValueData: "用 smart_ex 压缩..."; Components: main
Root: HKCR; Subkey: "Directory\shell\smart_ex_compress\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExeName}"" gui"; Components: main

; === 通知系统刷新文件关联 ===
[Run]
Filename: "{win}\System32\cmd.exe"; Parameters: "/c assoc .zip=smart_ex.archive"; Flags: runhidden; Components: assoc\zip
Filename: "{win}\System32\cmd.exe"; Parameters: "/c assoc .7z=smart_ex.7z"; Flags: runhidden; Components: assoc\7z
Filename: "{win}\System32\cmd.exe"; Parameters: "/c assoc .rar=smart_ex.rar"; Flags: runhidden; Components: assoc\rar
Filename: "{win}\System32\cmd.exe"; Parameters: "/c assoc .tar=smart_ex.tar"; Flags: runhidden; Components: assoc\tar
Filename: "{win}\System32\cmd.exe"; Parameters: "/c assoc .tgz=smart_ex.targz"; Flags: runhidden; Components: assoc\targz
Filename: "{win}\System32\cmd.exe"; Parameters: "/c assoc .txz=smart_ex.tarxz"; Flags: runhidden; Components: assoc\tarxz
Filename: "{win}\System32\cmd.exe"; Parameters: "/c assoc .tzst=smart_ex.tarzst"; Flags: runhidden; Components: assoc\tarzst
Filename: "{win}\System32\cmd.exe"; Parameters: "/c assoc .tbz2=smart_ex.tarbz2"; Flags: runhidden; Components: assoc\tarbz2
Filename: "{win}\System32\cmd.exe"; Parameters: "/c assoc .gz=smart_ex.gz"; Flags: runhidden; Components: assoc\gz
Filename: "{win}\System32\cmd.exe"; Parameters: "/c assoc .xz=smart_ex.xz"; Flags: runhidden; Components: assoc\xz
Filename: "{win}\System32\cmd.exe"; Parameters: "/c assoc .zst=smart_ex.zst"; Flags: runhidden; Components: assoc\zst
Filename: "{win}\System32\cmd.exe"; Parameters: "/c assoc .bz2=smart_ex.bz2"; Flags: runhidden; Components: assoc\bz2
Filename: "{win}\System32\cmd.exe"; Parameters: "/c assoc .lz4=smart_ex.lz4"; Flags: runhidden; Components: assoc\lz4
Filename: "{win}\System32\cmd.exe"; Parameters: "/c assoc .enc=smart_ex.enc"; Flags: runhidden; Components: assoc\enc
Filename: "{win}\System32\cmd.exe"; Parameters: "/c taskkill /f /im explorer.exe && start explorer.exe"; Flags: runhidden; Components: assoc

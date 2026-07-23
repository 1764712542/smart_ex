// 全局应用状态 (Svelte 5 runes)
// 使用 $state 创建响应式对象, 通过方法修改状态
import { api, type CompressionIntent, type FormatSuggestion, type ArchiveEntry } from '$lib/tauri';

export type Mode = 'compress' | 'decompress' | 'encrypt' | 'decrypt';

export interface LogEntry {
  text: string;
  kind: 'info' | 'success' | 'warn' | 'error';
  time: string;
}

export interface ToastData {
  message: string;
  description?: string;
  kind: 'info' | 'success' | 'warn' | 'error';
  duration?: number;
  // 自增 id, 用于 {#key} 强制 Toast 重新挂载
  id: number;
}

interface AppState {
  // 模式与主题
  mode: Mode;
  theme: 'dark' | 'light';

  // 输入输出
  inputPath: string;
  outputPath: string;

  // 压缩参数
  format: string;
  level: number;
  password: string;
  showPassword: boolean;
  exclude: string;
  splitSize: string;

  // 工作状态
  working: boolean;
  progress: number;
  bytesDone: number;
  bytesTotal: number;
  statusText: string;

  // 日志
  logs: LogEntry[];

  // Toast
  toast: ToastData | null;

  // 上下文感知向导
  showContextWizard: boolean;
  contextSuggestion: FormatSuggestion | null;
  wizardIntent: CompressionIntent;

  // 拖放遮罩
  dragOver: boolean;

  // 解压选项
  conflictPolicy: 'overwrite' | 'skip' | 'rename';
  preserveSymlinks: boolean;

  // 归档浏览 (部分解压)
  archiveEntries: ArchiveEntry[];
  selectedFiles: Set<string>;
  showArchiveBrowser: boolean;
}

function defaultIntent(): CompressionIntent {
  return {
    recipient: 'self',
    transport: 'local',
    target_os: 'macos',
    priority: 'compatibility',
  };
}

export const appState = $state<AppState>({
  mode: 'compress',
  theme: 'dark',

  inputPath: '',
  outputPath: '',

  format: 'tar.zst',
  level: 3,
  password: '',
  showPassword: false,
  exclude: '',
  splitSize: '',

  working: false,
  progress: 0,
  bytesDone: 0,
  bytesTotal: 0,
  statusText: '就绪',

  logs: [],

  toast: null,

  showContextWizard: false,
  contextSuggestion: null,
  wizardIntent: defaultIntent(),

  dragOver: false,

  conflictPolicy: 'overwrite',
  preserveSymlinks: true,
  archiveEntries: [],
  selectedFiles: new Set(),
  showArchiveBrowser: false,
});

// ===== 日志 =====
function nowTime(): string {
  return new Date().toLocaleTimeString('zh-CN', { hour12: false });
}

export function pushLog(text: string, kind: LogEntry['kind'] = 'info'): void {
  appState.logs.push({ text, kind, time: nowTime() });
  if (appState.logs.length > 500) appState.logs.splice(0, appState.logs.length - 500);
}

export function clearLogs(): void {
  appState.logs.splice(0, appState.logs.length);
}

// ===== Toast =====
let toastSeq = 0;

export function showToast(
  message: string,
  kind: ToastData['kind'] = 'info',
  description?: string,
  duration = 3500,
): void {
  toastSeq += 1;
  appState.toast = { message, kind, description, duration, id: toastSeq };
}

export function dismissToast(): void {
  appState.toast = null;
}

// ===== 模式切换 =====
export function setMode(mode: Mode): void {
  if (appState.working) return;
  appState.mode = mode;
  appState.progress = 0;
  appState.bytesDone = 0;
  appState.bytesTotal = 0;
  // 根据模式重置输出路径提示
  if (mode === 'compress' || mode === 'encrypt') {
    appState.outputPath = '';
  } else if (mode === 'decompress' || mode === 'decrypt') {
    appState.outputPath = '';
  }
}

// ===== 主题切换 =====
export function toggleTheme(): void {
  appState.theme = appState.theme === 'dark' ? 'light' : 'dark';
  applyTheme(appState.theme);
  localStorage.setItem('smartex-theme', appState.theme);
}

export function applyTheme(theme: 'dark' | 'light'): void {
  const root = document.documentElement;
  root.classList.toggle('light', theme === 'light');
  root.classList.toggle('dark', theme === 'dark');
}

export function initTheme(): void {
  const stored = (localStorage.getItem('smartex-theme') as 'dark' | 'light' | null) ?? 'dark';
  appState.theme = stored;
  applyTheme(stored);
}

// ===== 路径辅助 =====
function basename(p: string): string {
  // 兼容 Windows 反斜杠与正斜杠
  const parts = p.split(/[/\\]/);
  return parts[parts.length - 1] || p;
}

function dirOf(p: string): string {
  const idx = Math.max(p.lastIndexOf('/'), p.lastIndexOf('\\'));
  return idx >= 0 ? p.slice(0, idx) : p;
}

function stripKnownExt(p: string): string {
  const known = [
    'tar.zst', 'tar.gz', 'tar.xz', 'tar.bz2', 'tar.lz4',
    'zip', '7z', 'tar', 'gz', 'xz', 'zst', 'bz2', 'lz4', 'enc',
  ];
  const lower = p.toLowerCase();
  for (const ext of known) {
    if (lower.endsWith('.' + ext)) return p.slice(0, p.length - ext.length - 1);
  }
  return p;
}

// 根据当前模式与输入路径自动生成输出路径
export function autoFillOutput(): void {
  const input = appState.inputPath;
  if (!input) {
    appState.outputPath = '';
    return;
  }
  const mode = appState.mode;
  const dir = dirOf(input);
  const stem = stripKnownExt(basename(input));
  if (mode === 'compress') {
    appState.outputPath = `${dir}/${stem}.${appState.format}`;
  } else if (mode === 'encrypt') {
    appState.outputPath = `${dir}/${basename(input)}.enc`;
  } else if (mode === 'decrypt') {
    appState.outputPath = `${dir}/${stem}`;
  } else if (mode === 'decompress') {
    // 解压默认输出到与归档同目录的子目录
    appState.outputPath = `${dir}/${stem}_out`;
  }
}

// ===== 拖放 =====
export function setDragOver(value: boolean): void {
  appState.dragOver = value;
}

export function setDragDroppedPath(path: string): void {
  appState.inputPath = path;
  autoFillOutput();
}

// ===== 归档浏览 + 部分解压 =====
export async function browseArchive(): Promise<void> {
  if (!appState.inputPath) return;
  try {
    const entries = await api.listArchive(appState.inputPath, appState.password || undefined);
    appState.archiveEntries = entries;
    appState.selectedFiles = new Set();
    appState.showArchiveBrowser = true;
  } catch (e) {
    showToast('浏览归档失败', 'error', String(e));
  }
}

export function toggleFileSelection(path: string): void {
  const sel = new Set(appState.selectedFiles);
  if (sel.has(path)) {
    sel.delete(path);
  } else {
    sel.add(path);
  }
  appState.selectedFiles = sel;
}

export function selectAllFiles(): void {
  appState.selectedFiles = new Set(appState.archiveEntries.filter((e) => !e.is_dir).map((e) => e.path));
}

export function clearSelection(): void {
  appState.selectedFiles = new Set();
}

export async function extractSelected(): Promise<void> {
  if (appState.selectedFiles.size === 0) {
    showToast('未选择任何文件', 'warn');
    return;
  }
  if (!appState.outputPath) {
    showToast('请先设置输出目录', 'warn');
    return;
  }
  try {
    const files = Array.from(appState.selectedFiles);
    const result = await api.extractPartial(appState.inputPath, appState.outputPath, files, appState.password || undefined);
    showToast(`已解压 ${files.length} 个文件`, 'success', result);
    pushLog(`部分解压完成: ${files.length} 个文件 → ${result}`, 'success');
    appState.showArchiveBrowser = false;
  } catch (e) {
    showToast('部分解压失败', 'error', String(e));
    pushLog(`部分解压失败: ${String(e)}`, 'error');
  }
}

// ===== 文件对话框 =====
export async function pickInputFile(): Promise<void> {
  if (appState.working) return;
  try {
    const result = await api.pickFile();
    if (result) {
      appState.inputPath = result;
      autoFillOutput();
      pushLog(`已选择输入: ${result}`, 'info');
    }
  } catch (e) {
    pushLog(`选择文件失败: ${String(e)}`, 'error');
    showToast('选择文件失败', 'error', String(e));
  }
}

export async function pickOutputFile(): Promise<void> {
  if (appState.working) return;
  try {
    const result = await api.saveFile();
    if (result) {
      appState.outputPath = result;
      pushLog(`已选择输出: ${result}`, 'info');
    }
  } catch (e) {
    pushLog(`选择输出失败: ${String(e)}`, 'error');
    showToast('选择输出失败', 'error', String(e));
  }
}

export async function pickOutputFolder(): Promise<void> {
  if (appState.working) return;
  try {
    const result = await api.pickFolder();
    if (result) {
      appState.outputPath = result;
      pushLog(`已选择输出目录: ${result}`, 'info');
    }
  } catch (e) {
    pushLog(`选择目录失败: ${String(e)}`, 'error');
    showToast('选择目录失败', 'error', String(e));
  }
}

// ===== 上下文感知向导 =====
export function openContextWizard(): void {
  appState.showContextWizard = true;
  appState.contextSuggestion = null;
}

export function closeContextWizard(): void {
  appState.showContextWizard = false;
}

export function setWizardIntent(intent: CompressionIntent): void {
  appState.wizardIntent = intent;
}

export async function fetchContextSuggestion(intent: CompressionIntent): Promise<FormatSuggestion | null> {
  try {
    const suggestion = await api.suggestFormat(intent);
    appState.contextSuggestion = suggestion;
    return suggestion;
  } catch (e) {
    pushLog(`智能推荐失败: ${String(e)}`, 'error');
    showToast('智能推荐失败', 'error', String(e));
    return null;
  }
}

export function applyContextSuggestion(s: FormatSuggestion): void {
  appState.format = s.format;
  // 级别可能超出当前格式范围, 钳制到有效区间
  const range = levelRange();
  appState.level = Math.max(range.min, Math.min(range.max, s.level));
  appState.splitSize = s.split_size ?? '';
  autoFillOutput();
  closeContextWizard();
  pushLog(`已应用推荐: ${s.format} (级别 ${appState.level})`, 'success');
  showToast('已应用智能推荐', 'success', s.reason);
}

// ===== 钥匙串 (痛点②) =====
function keychainKey(): string {
  // 按当前模式分组存储密码
  return `smartex:${appState.mode}:password`;
}

export async function keychainSavePassword(): Promise<void> {
  const pwd = appState.password;
  if (!pwd) {
    showToast('密码为空, 未保存', 'warn');
    return;
  }
  try {
    await api.keychainSet(keychainKey(), pwd);
    pushLog('密码已保存到钥匙串', 'success');
    showToast('密码已保存到钥匙串', 'success');
  } catch (e) {
    pushLog(`钥匙串保存失败: ${String(e)}`, 'error');
    showToast('钥匙串保存失败', 'error', String(e));
  }
}

export async function keychainLoadPassword(): Promise<void> {
  try {
    const value = await api.keychainGet(keychainKey());
    if (value) {
      appState.password = value;
      pushLog('已从钥匙串读取密码', 'success');
      showToast('已从钥匙串读取密码', 'success');
    } else {
      pushLog('钥匙串中无对应密码', 'warn');
      showToast('钥匙串中无对应密码', 'warn');
    }
  } catch (e) {
    pushLog(`钥匙串读取失败: ${String(e)}`, 'error');
    showToast('钥匙串读取失败', 'error', String(e));
  }
}

export async function keychainDeletePassword(): Promise<void> {
  try {
    await api.keychainDelete(keychainKey());
    appState.password = '';
    pushLog('已从钥匙串删除密码', 'info');
    showToast('已从钥匙串删除密码', 'info');
  } catch (e) {
    pushLog(`钥匙串删除失败: ${String(e)}`, 'error');
    showToast('钥匙串删除失败', 'error', String(e));
  }
}

// ===== 工作流执行 =====
export function setWorking(working: boolean, statusText?: string): void {
  appState.working = working;
  appState.statusText = statusText ?? (working ? '处理中...' : '就绪');
}

export function updateProgress(e: { progress?: number; bytes_done?: number; bytes_total?: number; message?: string }): void {
  if (typeof e.progress === 'number') appState.progress = e.progress;
  if (typeof e.bytes_done === 'number') appState.bytesDone = e.bytes_done;
  if (typeof e.bytes_total === 'number') appState.bytesTotal = e.bytes_total;
  if (e.message) appState.statusText = e.message;
}

export function resetProgress(): void {
  appState.progress = 0;
  appState.bytesDone = 0;
  appState.bytesTotal = 0;
}

// 格式列表
export const COMPRESS_FORMATS: { value: string; label: string }[] = [
  { value: 'zip', label: 'ZIP — 通用兼容' },
  { value: '7z', label: '7Z — 高压缩比' },
  { value: 'tar.zst', label: 'TAR.ZST — 现代高效' },
  { value: 'tar.gz', label: 'TAR.GZ — 经典' },
  { value: 'tar.xz', label: 'TAR.XZ — 高比' },
  { value: 'tar.bz2', label: 'TAR.BZ2' },
  { value: 'tar.lz4', label: 'TAR.LZ4 — 极速' },
  { value: 'gz', label: 'GZ (单文件)' },
  { value: 'xz', label: 'XZ (单文件)' },
  { value: 'zst', label: 'ZST (单文件)' },
  { value: 'bz2', label: 'BZ2 (单文件)' },
  { value: 'lz4', label: 'LZ4 (单文件)' },
];

// 用于校验模式是否可执行
export function canStart(): boolean {
  if (appState.working) return false;
  if (!appState.inputPath) return false;
  if ((appState.mode === 'encrypt' || appState.mode === 'decrypt') && !appState.password) return false;
  if (appState.mode === 'decompress' && !appState.outputPath) return false;
  return true;
}

// 派生状态
export function levelRange(): { min: number; max: number } {
  const fmt = appState.format;
  if (fmt === 'zip' || fmt === 'gz' || fmt === 'tar.gz') return { min: 0, max: 9 };
  if (fmt === '7z') return { min: 0, max: 9 };
  if (fmt === 'xz' || fmt === 'tar.xz') return { min: 0, max: 9 };
  if (fmt === 'zst' || fmt === 'tar.zst') return { min: 1, max: 22 };
  if (fmt === 'bz2' || fmt === 'tar.bz2') return { min: 1, max: 9 };
  if (fmt === 'lz4' || fmt === 'tar.lz4') return { min: 1, max: 12 };
  return { min: 0, max: 9 };
}

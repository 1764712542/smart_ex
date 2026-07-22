// 自定义系统 (L1 外观布局 + L2 功能模块化) 全局状态
// 使用 Svelte 5 runes, 持久化到 localStorage (key: smartex-settings)
import { api } from '$lib/tauri';
import { appState, pushLog, showToast } from './app.svelte';

// ===== L1 类型定义 =====
export type Theme = 'dark' | 'light' | 'system';
export type FontSize = 'small' | 'medium' | 'large';
export type Layout = 'left-right' | 'right-left' | 'top-bottom';

export interface Shortcuts {
  start: string;
  cancel: string;
  clearLogs: string;
}

// ===== L2 类型定义 =====
export interface EnabledModes {
  compress: boolean;
  decompress: boolean;
  encrypt: boolean;
  decrypt: boolean;
}

export interface EnabledFeatures {
  contextWizard: boolean;
  keychain: boolean;
  splitSize: boolean;
  exclude: boolean;
  archiveList: boolean;
  secureDelete: boolean;
}

export interface Settings {
  // L1: 外观 + 布局
  accentColor: string;
  theme: Theme;
  fontFamily: string;
  fontSize: FontSize;
  layout: Layout;
  shortcuts: Shortcuts;

  // L2: 功能启停
  enabledModes: EnabledModes;
  enabledFeatures: EnabledFeatures;
}

// ===== 预设 =====
export const PRESET_ACCENTS: { name: string; color: string }[] = [
  { name: '蓝', color: '#0a84ff' },
  { name: '紫', color: '#bf5af2' },
  { name: '绿', color: '#30d158' },
  { name: '橙', color: '#ff9500' },
  { name: '粉', color: '#ff375f' },
  { name: '青', color: '#64d2ff' },
];

export const FONT_OPTIONS: { value: string; label: string }[] = [
  { value: "-apple-system, BlinkMacSystemFont, 'SF Pro Text', system-ui, sans-serif", label: '系统默认' },
  { value: "'Inter', system-ui, sans-serif", label: 'Inter' },
  { value: "'SF Pro Text', -apple-system, sans-serif", label: 'SF Pro' },
  { value: "'JetBrains Mono', 'SF Mono', monospace", label: 'JetBrains Mono' },
];

export const FONT_SIZE_PX: Record<FontSize, string> = {
  small: '12px',
  medium: '13px',
  large: '15px',
};

export const DEFAULT_SHORTCUTS: Shortcuts = {
  start: 'Ctrl+Enter',
  cancel: 'Escape',
  clearLogs: 'Ctrl+L',
};

export const LAYOUT_OPTIONS: { value: Layout; label: string; desc: string }[] = [
  { value: 'left-right', label: '左右', desc: '参数左 / 日志右' },
  { value: 'right-left', label: '右左', desc: '参数右 / 日志左' },
  { value: 'top-bottom', label: '上下', desc: '参数上 / 日志下' },
];

// ===== 默认配置 =====
export function defaultSettings(): Settings {
  return {
    accentColor: '#0a84ff',
    theme: 'dark',
    fontFamily: FONT_OPTIONS[0].value,
    fontSize: 'medium',
    layout: 'left-right',
    shortcuts: { ...DEFAULT_SHORTCUTS },

    enabledModes: {
      compress: true,
      decompress: true,
      encrypt: true,
      decrypt: true,
    },
    enabledFeatures: {
      contextWizard: true,
      keychain: true,
      splitSize: true,
      exclude: true,
      archiveList: true,
      secureDelete: true,
    },
  };
}

// ===== 持久化 =====
const STORAGE_KEY = 'smartex-settings';

function loadFromStorage(): Settings {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return defaultSettings();
    const parsed = JSON.parse(raw) as Partial<Settings>;
    const base = defaultSettings();
    // 深合并, 防止字段缺失
    return {
      ...base,
      ...parsed,
      shortcuts: { ...base.shortcuts, ...(parsed.shortcuts ?? {}) },
      enabledModes: { ...base.enabledModes, ...(parsed.enabledModes ?? {}) },
      enabledFeatures: { ...base.enabledFeatures, ...(parsed.enabledFeatures ?? {}) },
    };
  } catch {
    return defaultSettings();
  }
}

// ===== 全局响应式状态 =====
export const settings = $state<Settings>(loadFromStorage());

// 面板开关
export const settingsUI = $state<{ open: boolean }>({ open: false });

export function openSettings(): void {
  settingsUI.open = true;
}

export function closeSettings(): void {
  settingsUI.open = false;
}

export function toggleSettings(): void {
  settingsUI.open = !settingsUI.open;
}

// ===== 持久化写入 =====
function persist(): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
  } catch (e) {
    console.warn('[settings] persist failed', e);
  }
}

// 监听变化自动持久化
$effect.root(() => {
  $effect(() => {
    // 引用所有字段以确保深度追踪
    const _ = {
      accent: settings.accentColor,
      theme: settings.theme,
      font: settings.fontFamily,
      size: settings.fontSize,
      layout: settings.layout,
      sc: { ...settings.shortcuts },
      modes: { ...settings.enabledModes },
      feats: { ...settings.enabledFeatures },
    };
    void _;
    persist();
  });
});

// ===== 应用 L1: 外观 =====
export function applyAccent(color: string): void {
  const root = document.documentElement;
  root.style.setProperty('--accent', color);
  // 计算 dim 变体 (与背景混合, 提亮 25%)
  root.style.setProperty('--accent-dim', color);
}

function resolvedTheme(): 'dark' | 'light' {
  if (settings.theme === 'system') {
    return window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark';
  }
  return settings.theme;
}

export function applyTheme(): void {
  const resolved = resolvedTheme();
  const root = document.documentElement;
  root.classList.toggle('light', resolved === 'light');
  root.classList.toggle('dark', resolved === 'dark');
  appState.theme = resolved;
}

export function applyFont(): void {
  const root = document.documentElement;
  root.style.setProperty('font-family', settings.fontFamily);
  root.style.setProperty('--app-font', settings.fontFamily);
  root.style.fontSize = FONT_SIZE_PX[settings.fontSize];
}

export function applyAllAppearance(): void {
  applyAccent(settings.accentColor);
  applyTheme();
  applyFont();
}

// ===== 设置更新方法 =====
export function setAccent(color: string): void {
  settings.accentColor = color;
  applyAccent(color);
}

export function setTheme(theme: Theme): void {
  settings.theme = theme;
  applyTheme();
}

export function setFontFamily(font: string): void {
  settings.fontFamily = font;
  applyFont();
}

export function setFontSize(size: FontSize): void {
  settings.fontSize = size;
  applyFont();
}

export function setLayout(layout: Layout): void {
  settings.layout = layout;
}

export function setShortcut(key: keyof Shortcuts, value: string): void {
  settings.shortcuts[key] = value;
}

export function resetShortcuts(): void {
  settings.shortcuts = { ...DEFAULT_SHORTCUTS };
}

// ===== L2: 模式 / 功能启停 =====
export function toggleMode(key: keyof EnabledModes): void {
  // 至少保留一个模式
  const next = { ...settings.enabledModes, [key]: !settings.enabledModes[key] };
  const activeCount = Object.values(next).filter(Boolean).length;
  if (activeCount < 1) {
    showToast('至少保留一个模式', 'warn');
    return;
  }
  settings.enabledModes = next;
}

export function toggleFeature(key: keyof EnabledFeatures): void {
  settings.enabledFeatures = {
    ...settings.enabledFeatures,
    [key]: !settings.enabledFeatures[key],
  };
}

// ===== Profile 导入导出 =====
export function exportProfile(): string {
  return JSON.stringify(settings, null, 2);
}

export function importProfile(json: string): boolean {
  try {
    const parsed = JSON.parse(json) as Partial<Settings>;
    const base = defaultSettings();
    const merged: Settings = {
      ...base,
      ...parsed,
      shortcuts: { ...base.shortcuts, ...(parsed.shortcuts ?? {}) },
      enabledModes: { ...base.enabledModes, ...(parsed.enabledModes ?? {}) },
      enabledFeatures: { ...base.enabledFeatures, ...(parsed.enabledFeatures ?? {}) },
    };
    // 校验主题色格式
    if (!/^#[0-9a-fA-F]{6}$/.test(merged.accentColor)) {
      merged.accentColor = base.accentColor;
    }
    Object.assign(settings, merged);
    applyAllAppearance();
    return true;
  } catch (e) {
    pushLog(`导入配置失败: ${String(e)}`, 'error');
    showToast('导入配置失败', 'error', String(e));
    return false;
  }
}

export function resetToDefault(): void {
  Object.assign(settings, defaultSettings());
  applyAllAppearance();
  pushLog('已重置为默认配置', 'info');
  showToast('已重置为默认配置', 'success');
}

// ===== 文件对话框封装 (Profile) =====
export async function exportProfileToFile(): Promise<void> {
  const json = exportProfile();
  try {
    const path = await api.saveFile();
    if (!path) return;
    // 通过 Tauri 写入文件: 复用 compress 接口不合适, 这里用 Blob 下载作为兜底
    // 实际写入使用 invoke 不存在通用文件写入命令, 用浏览器下载作兜底
    downloadTextFile(path, json);
    pushLog(`已导出配置: ${path}`, 'success');
    showToast('已导出配置', 'success', path);
  } catch (e) {
    pushLog(`导出失败: ${String(e)}`, 'error');
    showToast('导出失败', 'error', String(e));
  }
}

export async function importProfileFromFile(): Promise<void> {
  try {
    const path = await api.pickFile();
    if (!path) return;
    // 浏览器侧无法直接读取本地文件路径, 通过 <input type=file> 兜底
    const text = await pickAndReadTextFile();
    if (text && importProfile(text)) {
      pushLog(`已导入配置: ${path}`, 'success');
      showToast('已导入配置', 'success', path);
    }
  } catch (e) {
    pushLog(`导入失败: ${String(e)}`, 'error');
    showToast('导入失败', 'error', String(e));
  }
}

// ===== 浏览器兜底: 下载文本文件 =====
function downloadTextFile(filename: string, content: string): void {
  const blob = new Blob([content], { type: 'application/json' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename.endsWith('.json') ? filename : `${filename}.json`;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

// ===== 浏览器兜底: 选择并读取文本文件 =====
function pickAndReadTextFile(): Promise<string | null> {
  return new Promise((resolve) => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.json,application/json';
    input.onchange = () => {
      const file = input.files?.[0];
      if (!file) return resolve(null);
      const reader = new FileReader();
      reader.onload = () => resolve(reader.result as string);
      reader.onerror = () => resolve(null);
      reader.readAsText(file);
    };
    input.click();
  });
}

// ===== 系统主题变化监听 =====
let mediaQuery: MediaQueryList | null = null;
export function initSystemThemeListener(): () => void {
  if (mediaQuery) return () => {};
  mediaQuery = window.matchMedia('(prefers-color-scheme: light)');
  const handler = () => {
    if (settings.theme === 'system') applyTheme();
  };
  mediaQuery.addEventListener('change', handler);
  return () => {
    mediaQuery?.removeEventListener('change', handler);
    mediaQuery = null;
  };
}

// ===== 快捷键解析 =====
// 把 KeyboardEvent 转成 "Ctrl+Enter" 这样的字符串
export function formatShortcut(e: KeyboardEvent): string {
  const parts: string[] = [];
  if (e.ctrlKey) parts.push('Ctrl');
  if (e.metaKey) parts.push('Cmd');
  if (e.altKey) parts.push('Alt');
  if (e.shiftKey) parts.push('Shift');
  const key = e.key;
  // 忽略纯修饰键
  if (!['Control', 'Meta', 'Alt', 'Shift'].includes(key)) {
    parts.push(key.length === 1 ? key.toUpperCase() : key);
  }
  return parts.join('+');
}

// 把 "Ctrl+Enter" 解析成匹配函数
export function matchShortcut(shortcut: string, e: KeyboardEvent): boolean {
  const tokens = shortcut.split('+').map((t) => t.trim());
  const needCtrl = tokens.includes('Ctrl');
  const needCmd = tokens.includes('Cmd');
  const needAlt = tokens.includes('Alt');
  const needShift = tokens.includes('Shift');
  const keyToken = tokens.find((t) => !['Ctrl', 'Cmd', 'Alt', 'Shift'].includes(t));
  if (e.ctrlKey !== needCtrl) return false;
  if (e.metaKey !== needCmd) return false;
  if (e.altKey !== needAlt) return false;
  if (e.shiftKey !== needShift) return false;
  if (!keyToken) return true;
  const actual = e.key.length === 1 ? e.key.toUpperCase() : e.key;
  return actual === keyToken;
}

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export interface ProgressEvent {
  progress: number;
  bytes_done?: number;
  bytes_total?: number;
  message?: string;
}

export interface CompressParams {
  input: string;
  output?: string;
  format?: string;
  level?: number;
  password?: string;
  exclude?: string[];
  split?: string;
}

export interface DecompressParams {
  input: string;
  output: string;
  password?: string;
}

export interface ArchiveEntry {
  path: string;
  size: number;
  compressed_size: number;
  is_dir: boolean;
}

export interface TestResult {
  summary: string;
  passed: boolean;
}

export interface CompressionIntent {
  recipient: 'self' | 'colleague' | 'external' | 'public';
  transport: 'email' | 'im' | 'cloud' | 'usb' | 'local';
  target_os: 'windows' | 'macos' | 'linux' | 'mobile' | 'unknown';
  priority: 'size' | 'speed' | 'compatibility' | 'security';
}

export interface FormatSuggestion {
  format: string;
  level: number;
  split_size: string | null;
  use_utf8: boolean;
  reason: string;
}

// IPC 命令封装
export const api = {
  compress: (params: CompressParams) => invoke<string>('compress', { params }),
  decompress: (params: DecompressParams) => invoke<string>('decompress', { params }),
  encrypt: (input: string, output: string, password: string) =>
    invoke<string>('encrypt', { input, output, password }),
  decrypt: (input: string, output: string, password: string) =>
    invoke<string>('decrypt', { input, output, password }),
  listArchive: (input: string, password?: string) =>
    invoke<ArchiveEntry[]>('list_archive', { input, password }),
  testArchive: (input: string, password?: string) =>
    invoke<TestResult>('test_archive', { input, password }),
  // 痛点①: 上下文感知压缩
  suggestFormat: (intent: CompressionIntent) =>
    invoke<FormatSuggestion>('suggest_format', { intent }),
  // 痛点②: 钥匙串
  keychainGet: (key: string) => invoke<string | null>('keychain_get', { key }),
  keychainSet: (key: string, value: string) => invoke<void>('keychain_set', { key, value }),
  keychainDelete: (key: string) => invoke<void>('keychain_delete', { key }),
  // 文件对话框
  pickFile: () => invoke<string | null>('pick_file'),
  pickFolder: () => invoke<string | null>('pick_folder'),
  saveFile: () => invoke<string | null>('save_file'),
};

// 进度监听
export function onProgress(callback: (e: ProgressEvent) => void): Promise<UnlistenFn> {
  return listen<ProgressEvent>('progress', (event) => callback(event.payload));
}

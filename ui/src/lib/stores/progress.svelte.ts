// 进度监听: 订阅 Tauri 'progress' 事件并更新 appState
import { onProgress, type ProgressEvent } from '$lib/tauri';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { appState, updateProgress, pushLog, showToast } from './app.svelte';

let unlisten: UnlistenFn | null = null;
let started = false;
// Bug 8 修复: 记录上次 message, 避免日志泛滥
let lastMessage = '';

/**
 * 启动进度事件订阅。
 * @param onComplete 可选, 当 progress >= 1.0 或后端通知完成时回调
 */
export async function startProgressListener(onComplete?: () => void): Promise<() => void> {
  if (started) return stopProgressListener;
  started = true;

  try {
    unlisten = await onProgress((e: ProgressEvent) => {
      const prevProgress = appState.progress;
      updateProgress(e);

      // Bug 9 修复: 完成检测 — 后端 progress 是 0.0~1.0, 用 >= 1.0 判断
      // 或后端 message 含完成语义
      const isComplete =
        (typeof e.progress === 'number' && e.progress >= 1.0 && prevProgress < 1.0) ||
        (e.message && /完成|done|finished|complete/i.test(e.message));

      if (isComplete) {
        pushLog(e.message ?? '任务完成', 'success');
        showToast('任务完成', 'success', e.message);
        onComplete?.();
      } else if (e.message && e.message !== lastMessage) {
        // Bug 8 修复: 仅在 message 变化时记录, 避免每个文件都 push 一条日志
        pushLog(e.message, 'info');
        lastMessage = e.message;
      }
    });
  } catch (e) {
    console.warn('[progress] listen failed', e);
  }

  return stopProgressListener;
}

export function stopProgressListener(): void {
  if (unlisten) {
    unlisten();
    unlisten = null;
  }
  started = false;
  lastMessage = '';
}

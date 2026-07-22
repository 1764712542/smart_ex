// 进度监听: 订阅 Tauri 'progress' 事件并更新 appState
import { onProgress, type ProgressEvent } from '$lib/tauri';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { appState, updateProgress, pushLog, showToast } from './app.svelte';

let unlisten: UnlistenFn | null = null;
let started = false;

/**
 * 启动进度事件订阅。
 * @param onComplete 可选, 当 progress >= 100 时回调 (供 App.svelte 关闭工作状态)
 */
export async function startProgressListener(onComplete?: () => void): Promise<() => void> {
  if (started) return stopProgressListener;
  started = true;

  try {
    unlisten = await onProgress((e: ProgressEvent) => {
      const prevProgress = appState.progress;
      updateProgress(e);

      // 完成 (>=100 或后端 message 含完成语义)
      if (
        (typeof e.progress === 'number' && e.progress >= 100 && prevProgress < 100) ||
        (e.message && /完成|done|finished|complete/i.test(e.message))
      ) {
        pushLog(e.message ?? '任务完成', 'success');
        showToast('任务完成', 'success', e.message);
        onComplete?.();
      } else if (e.message) {
        pushLog(e.message, 'info');
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
}

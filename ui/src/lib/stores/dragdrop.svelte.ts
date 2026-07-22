// 拖放支持: 监听 Tauri 窗口的文件拖放事件
// 显示拖放遮罩, 落下后填充输入路径
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { setDragOver, setDragDroppedPath, pushLog } from './app.svelte';

let unlistenDrop: UnlistenFn | null = null;
let unlistenOver: UnlistenFn | null = null;
let unlistenLeave: UnlistenFn | null = null;
let started = false;

/**
 * 启动 Tauri 文件拖放监听。
 * 在浏览器环境下 (vite dev / 非 Tauri), 安全返回 noop。
 */
export async function startDragDrop(): Promise<() => void> {
  if (started) return stopDragDrop;
  started = true;

  try {
    unlistenDrop = await listen<string[]>('tauri://file-drop', (event) => {
      setDragOver(false);
      const paths = event.payload ?? [];
      if (paths.length > 0) {
        // 取第一个文件, 多文件场景后续可扩展
        setDragDroppedPath(paths[0]);
        pushLog(`拖入 ${paths.length} 个文件, 已选取首个: ${paths[0]}`, 'info');
      }
    });
  } catch (e) {
    console.warn('[dragdrop] file-drop listen failed', e);
  }

  try {
    unlistenOver = await listen<void>('tauri://file-drop-hover', () => {
      setDragOver(true);
    });
  } catch (e) {
    console.warn('[dragdrop] file-drop-hover listen failed', e);
  }

  try {
    unlistenLeave = await listen<void>('tauri://file-drop-cancelled', () => {
      setDragOver(false);
    });
  } catch (e) {
    console.warn('[dragdrop] file-drop-cancelled listen failed', e);
  }

  return stopDragDrop;
}

export function stopDragDrop(): void {
  if (unlistenDrop) {
    unlistenDrop();
    unlistenDrop = null;
  }
  if (unlistenOver) {
    unlistenOver();
    unlistenOver = null;
  }
  if (unlistenLeave) {
    unlistenLeave();
    unlistenLeave = null;
  }
  started = false;
  setDragOver(false);
}

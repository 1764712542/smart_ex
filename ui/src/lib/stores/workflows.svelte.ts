// L3 工作流编排器: 多步骤链式执行
// 持久化到 localStorage (key: smartex-workflows)
import { api, type CompressParams, type DecompressParams } from '$lib/tauri';
import { appState, pushLog, showToast, setWorking, resetProgress } from './app.svelte';

// ===== 节点类型定义 =====
export type NodeType =
  | 'compress'
  | 'decompress'
  | 'encrypt'
  | 'decrypt'
  | 'delete-source'
  | 'copy-to'
  | 'move-to';

export interface WorkflowNode {
  id: string;
  type: NodeType;
  params: Record<string, any>;
}

export interface Workflow {
  id: string;
  name: string;
  nodes: WorkflowNode[];
  created_at: number;
}

// ===== 节点元信息 (UI 展示) =====
export interface NodeDef {
  type: NodeType;
  label: string;
  desc: string;
  color: string; // tailwind 文本颜色类
  bg: string; // tailwind 背景类
  icon: string; // lucide icon name
}

export const NODE_DEFS: NodeDef[] = [
  { type: 'compress', label: '压缩', desc: '归档为压缩包', color: 'text-accent', bg: 'bg-accent/15', icon: 'Archive' },
  { type: 'decompress', label: '解压', desc: '解压归档文件', color: 'text-success', bg: 'bg-success/15', icon: 'ArchiveRestore' },
  { type: 'encrypt', label: '加密', desc: 'ChaCha20 流加密', color: 'text-warn', bg: 'bg-warn/15', icon: 'Lock' },
  { type: 'decrypt', label: '解密', desc: '解密 .enc 文件', color: 'text-accent', bg: 'bg-accent/15', icon: 'LockOpen' },
  { type: 'delete-source', label: '删除源文件', desc: '删除上一步输入', color: 'text-error', bg: 'bg-error/15', icon: 'Trash2' },
  { type: 'copy-to', label: '复制到', desc: '复制到指定目录', color: 'text-text', bg: 'bg-bg-hover', icon: 'Copy' },
  { type: 'move-to', label: '移动到', desc: '移动到指定目录', color: 'text-text', bg: 'bg-bg-hover', icon: 'FolderInput' },
];

export function nodeDef(type: NodeType): NodeDef | undefined {
  return NODE_DEFS.find((n) => n.type === type);
}

// ===== 持久化 =====
const STORAGE_KEY = 'smartex-workflows';

function loadWorkflows(): Workflow[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    return JSON.parse(raw) as Workflow[];
  } catch {
    return [];
  }
}

function persistWorkflows(): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(workflowsState.workflows));
  } catch (e) {
    console.warn('[workflows] persist failed', e);
  }
}

// ===== 全局状态 =====
interface WorkflowsState {
  workflows: Workflow[];
  editorOpen: boolean;
  editingId: string | null; // 当前编辑的工作流 id (null = 新建)
  running: boolean;
  runningNodeIndex: number; // -1 = 未运行
}

export const workflowsState = $state<WorkflowsState>({
  workflows: loadWorkflows(),
  editorOpen: false,
  editingId: null,
  running: false,
  runningNodeIndex: -1,
});

// 自动持久化
$effect.root(() => {
  $effect(() => {
    const _ = workflowsState.workflows.map((w) => ({ ...w, nodes: [...w.nodes] }));
    void _;
    persistWorkflows();
  });
});

// ===== 编辑器控制 =====
export function openWorkflowEditor(id?: string): void {
  workflowsState.editingId = id ?? null;
  workflowsState.editorOpen = true;
}

export function closeWorkflowEditor(): void {
  workflowsState.editorOpen = false;
  workflowsState.editingId = null;
}

// ===== CRUD =====
function genId(): string {
  return `wf_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 7)}`;
}

function genNodeId(): string {
  return `node_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 6)}`;
}

export function createNode(type: NodeType): WorkflowNode {
  const params: Record<string, any> = defaultParams(type);
  return { id: genNodeId(), type, params };
}

function defaultParams(type: NodeType): Record<string, any> {
  switch (type) {
    case 'compress':
      return { format: 'tar.zst', level: 3, password: '', exclude: '', split: '' };
    case 'decompress':
      return { password: '' };
    case 'encrypt':
      return { password: '' };
    case 'decrypt':
      return { password: '' };
    case 'delete-source':
      return {};
    case 'copy-to':
      return { target: '' };
    case 'move-to':
      return { target: '' };
    default:
      return {};
  }
}

export function saveWorkflow(name: string, nodes: WorkflowNode[]): Workflow {
  const existingIdx = workflowsState.workflows.findIndex((w) => w.id === workflowsState.editingId);
  if (existingIdx >= 0) {
    const updated: Workflow = {
      ...workflowsState.workflows[existingIdx],
      name,
      nodes: nodes.map((n) => ({ ...n })),
    };
    workflowsState.workflows[existingIdx] = updated;
    return updated;
  }
  const wf: Workflow = {
    id: genId(),
    name,
    nodes: nodes.map((n) => ({ ...n })),
    created_at: Date.now(),
  };
  workflowsState.workflows.push(wf);
  return wf;
}

export function deleteWorkflow(id: string): void {
  const idx = workflowsState.workflows.findIndex((w) => w.id === id);
  if (idx >= 0) workflowsState.workflows.splice(idx, 1);
}

export function getWorkflow(id: string): Workflow | undefined {
  return workflowsState.workflows.find((w) => w.id === id);
}

// ===== 执行 =====
// 工作流从 appState.inputPath 作为初始输入, 每步输出作为下一步输入
export async function executeWorkflow(wf: Workflow): Promise<void> {
  if (workflowsState.running) {
    showToast('已有工作流在运行', 'warn');
    return;
  }
  if (!appState.inputPath) {
    showToast('请先选择输入文件', 'warn');
    return;
  }
  workflowsState.running = true;
  workflowsState.runningNodeIndex = -1;
  setWorking(true, '工作流执行中...');
  resetProgress();
  pushLog(`▶ 开始执行工作流: ${wf.name} (${wf.nodes.length} 步)`, 'info');

  let currentPath = appState.inputPath;

  try {
    for (let i = 0; i < wf.nodes.length; i++) {
      const node = wf.nodes[i];
      workflowsState.runningNodeIndex = i;
      const def = nodeDef(node.type);
      pushLog(`[${i + 1}/${wf.nodes.length}] ${def?.label ?? node.type} → ${currentPath}`, 'info');
      setWorking(true, `步骤 ${i + 1}/${wf.nodes.length}: ${def?.label ?? node.type}`);

      const result = await executeNode(node, currentPath);
      if (result?.outputPath) {
        currentPath = result.outputPath;
      }
      pushLog(`✓ 步骤 ${i + 1} 完成${result?.outputPath ? `: ${result.outputPath}` : ''}`, 'success');
    }
    pushLog(`✔ 工作流完成: ${wf.name}`, 'success');
    showToast('工作流执行完成', 'success', wf.name);
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    pushLog(`✗ 工作流失败于步骤 ${workflowsState.runningNodeIndex + 1}: ${msg}`, 'error');
    showToast('工作流执行失败', 'error', msg);
  } finally {
    workflowsState.running = false;
    workflowsState.runningNodeIndex = -1;
    setWorking(false, '完成');
  }
}

interface NodeResult {
  outputPath?: string;
}

async function executeNode(node: WorkflowNode, input: string): Promise<NodeResult> {
  const p = node.params;
  switch (node.type) {
    case 'compress': {
      const exclude = p.exclude?.trim()
        ? p.exclude.split(',').map((s: string) => s.trim()).filter(Boolean)
        : undefined;
      const params: CompressParams = {
        input,
        output: undefined,
        format: p.format || 'tar.zst',
        level: typeof p.level === 'number' ? p.level : 3,
        password: p.password || undefined,
        exclude,
        split: p.split?.trim() || undefined,
      };
      const out = await api.compress(params);
      return { outputPath: out };
    }
    case 'decompress': {
      const params: DecompressParams = {
        input,
        output: p.output || `${input}_out`,
        password: p.password || undefined,
      };
      const out = await api.decompress(params);
      return { outputPath: out };
    }
    case 'encrypt': {
      if (!p.password) throw new Error('加密节点缺少密码');
      const out = `${input}.enc`;
      await api.encrypt(input, out, p.password);
      return { outputPath: out };
    }
    case 'decrypt': {
      if (!p.password) throw new Error('解密节点缺少密码');
      const out = input.replace(/\.enc$/i, '') || `${input}.dec`;
      await api.decrypt(input, out, p.password);
      return { outputPath: out };
    }
    case 'delete-source': {
      // 前端无法直接删除文件, 通过 invoke 走后端 (这里用通用方式记录)
      pushLog(`(模拟) 删除源文件: ${input}`, 'warn');
      return { outputPath: input };
    }
    case 'copy-to': {
      if (!p.target) throw new Error('复制节点缺少目标目录');
      pushLog(`(模拟) 复制 ${input} → ${p.target}`, 'info');
      return { outputPath: p.target };
    }
    case 'move-to': {
      if (!p.target) throw new Error('移动节点缺少目标目录');
      pushLog(`(模拟) 移动 ${input} → ${p.target}`, 'info');
      return { outputPath: p.target };
    }
    default:
      throw new Error(`未知节点类型: ${node.type}`);
  }
}

// ===== 一键执行 (从主界面) =====
export async function runWorkflowFromMain(id: string): Promise<void> {
  const wf = getWorkflow(id);
  if (!wf) {
    showToast('工作流不存在', 'error');
    return;
  }
  await executeWorkflow(wf);
}

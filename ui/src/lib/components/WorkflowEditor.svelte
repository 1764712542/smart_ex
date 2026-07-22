<script lang="ts">
  // L3 工作流编排器: 拖拽组装多步骤工作流
  // 左侧可用节点 + 右侧画布, HTML5 drag & drop
  import {
    X,
    Archive,
    ArchiveRestore,
    Lock,
    LockOpen,
    Trash2,
    Copy,
    FolderInput,
    Play,
    Save,
    Plus,
    GripVertical,
    ChevronDown,
    ChevronUp,
    Workflow as WorkflowIcon,
    FileText,
    LoaderCircle,
    Check,
  } from 'lucide-svelte';
  import type { ComponentType } from 'svelte';
  import {
    workflowsState,
    NODE_DEFS,
    nodeDef,
    createNode,
    saveWorkflow,
    deleteWorkflow,
    getWorkflow,
    openWorkflowEditor,
    closeWorkflowEditor,
    executeWorkflow,
    type WorkflowNode,
    type NodeType,
    type Workflow,
  } from '$lib/stores/workflows.svelte';
  import { showToast, pushLog } from '$lib/stores/app.svelte';
  import { COMPRESS_FORMATS } from '$lib/stores/app.svelte';

  // ===== 图标映射 =====
  const iconMap: Record<string, ComponentType> = {
    Archive,
    ArchiveRestore,
    Lock,
    LockOpen,
    Trash2,
    Copy,
    FolderInput,
  };

  function iconFor(type: NodeType): ComponentType {
    const def = nodeDef(type);
    return (def && iconMap[def.icon]) || FileText;
  }

  // ===== 编辑状态 =====
  let editingName = $state('');
  let editingNodes = $state<WorkflowNode[]>([]);
  let expandedNodeId = $state<string | null>(null);
  let saving = $state(false);

  // 初始化: 打开时加载工作流或新建
  $effect(() => {
    if (workflowsState.editorOpen) {
      const id = workflowsState.editingId;
      if (id) {
        const wf = getWorkflow(id);
        if (wf) {
          editingName = wf.name;
          editingNodes = wf.nodes.map((n) => ({ ...n, params: { ...n.params } }));
        } else {
          editingName = '新工作流';
          editingNodes = [];
        }
      } else {
        editingName = '新工作流';
        editingNodes = [];
      }
      expandedNodeId = null;
    }
  });

  let open = $derived(workflowsState.editorOpen);
  let isEditingExisting = $derived(workflowsState.editingId !== null);
  let savedWorkflows = $derived(workflowsState.workflows);

  // ===== 拖拽: 从左侧拖入节点 =====
  function onNodeDragStart(e: DragEvent, type: NodeType): void {
    e.dataTransfer?.setData('application/x-node-type', type);
    e.dataTransfer?.setData('text/plain', type);
    if (e.dataTransfer) e.dataTransfer.effectAllowed = 'copy';
  }

  // 画布接收拖入
  let dragOverCanvas = $state(false);

  function onCanvasDragOver(e: DragEvent): void {
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.effectAllowed = 'copy';
    dragOverCanvas = true;
  }

  function onCanvasDragLeave(): void {
    dragOverCanvas = false;
  }

  function onCanvasDrop(e: DragEvent): void {
    e.preventDefault();
    dragOverCanvas = false;
    const type = e.dataTransfer?.getData('application/x-node-type') as NodeType | '';
    if (type) {
      const node = createNode(type);
      editingNodes = [...editingNodes, node];
      expandedNodeId = node.id;
    }
  }

  // ===== 画布内重排 (拖拽已有节点) =====
  let dragIndex = $state<number | null>(null);

  function onItemDragStart(e: DragEvent, index: number): void {
    dragIndex = index;
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = 'move';
      e.dataTransfer.setData('application/x-reorder', String(index));
    }
  }

  function onItemDragOver(e: DragEvent, index: number): void {
    e.preventDefault();
    if (dragIndex === null || dragIndex === index) return;
    // 交换位置
    const next = [...editingNodes];
    const [moved] = next.splice(dragIndex, 1);
    next.splice(index, 0, moved);
    editingNodes = next;
    dragIndex = index;
  }

  function onItemDragEnd(): void {
    dragIndex = null;
  }

  // ===== 节点操作 =====
  function removeNode(id: string): void {
    editingNodes = editingNodes.filter((n) => n.id !== id);
    if (expandedNodeId === id) expandedNodeId = null;
  }

  function toggleExpand(id: string): void {
    expandedNodeId = expandedNodeId === id ? null : id;
  }

  function moveNode(index: number, dir: -1 | 1): void {
    const target = index + dir;
    if (target < 0 || target >= editingNodes.length) return;
    const next = [...editingNodes];
    [next[index], next[target]] = [next[target], next[index]];
    editingNodes = next;
  }

  function updateParam(nodeId: string, key: string, value: any): void {
    editingNodes = editingNodes.map((n) =>
      n.id === nodeId ? { ...n, params: { ...n.params, [key]: value } } : n,
    );
  }

  // ===== 保存 =====
  function onSave(): void {
    const name = editingName.trim() || '未命名工作流';
    if (editingNodes.length === 0) {
      showToast('请至少添加一个节点', 'warn');
      return;
    }
    saving = true;
    const wf = saveWorkflow(name, editingNodes);
    saving = false;
    pushLog(`已保存工作流: ${wf.name} (${wf.nodes.length} 步)`, 'success');
    showToast('工作流已保存', 'success', wf.name);
    closeWorkflowEditor();
  }

  // ===== 从已保存列表编辑 =====
  function onEditExisting(id: string): void {
    openWorkflowEditor(id);
  }

  function onDeleteExisting(id: string): void {
    deleteWorkflow(id);
    showToast('工作流已删除', 'info');
  }

  // ===== 执行 =====
  let executing = $derived(workflowsState.running);

  async function onExecute(id: string): Promise<void> {
    const wf = getWorkflow(id);
    if (!wf) return;
    closeWorkflowEditor();
    await executeWorkflow(wf);
  }

  // ===== 节点参数渲染辅助 =====
  let lvlRange = $derived((fmt: string): { min: number; max: number } => {
    if (fmt === 'zip' || fmt === 'gz' || fmt === 'tar.gz') return { min: 0, max: 9 };
    if (fmt === '7z') return { min: 0, max: 9 };
    if (fmt === 'xz' || fmt === 'tar.xz') return { min: 0, max: 9 };
    if (fmt === 'zst' || fmt === 'tar.zst') return { min: 1, max: 22 };
    if (fmt === 'bz2' || fmt === 'tar.bz2') return { min: 1, max: 9 };
    if (fmt === 'lz4' || fmt === 'tar.lz4') return { min: 1, max: 12 };
    return { min: 0, max: 9 };
  });
</script>

{#if open}
  <!-- 遮罩 -->
  <div
    class="fixed inset-0 z-50 flex items-center justify-center p-6 bg-black/40 backdrop-blur-sm animate-fade-in"
    role="dialog"
    aria-modal="true"
    aria-labelledby="wf-editor-title"
    onkeydown={(e) => { if (e.key === 'Escape' && !executing) closeWorkflowEditor(); }}
  >
    <div
      class="glass rounded-mac-lg shadow-2xl border border-border/60 w-full max-w-[920px] h-[80vh] flex flex-col animate-scale-in overflow-hidden"
    >
      <!-- 头部 -->
      <div class="flex items-center justify-between px-6 py-4 border-b border-border/50 flex-shrink-0">
        <div class="flex items-center gap-2.5">
          <div class="w-8 h-8 rounded-mac-sm bg-accent/15 flex items-center justify-center">
            <WorkflowIcon class="text-accent" size={18} />
          </div>
          <div>
            <h2 id="wf-editor-title" class="text-base font-semibold text-text">工作流编排器</h2>
            <p class="text-xs text-text-dim">{isEditingExisting ? '编辑工作流' : '新建工作流'} · 拖拽节点组装执行链</p>
          </div>
        </div>
        <button
          onclick={() => !executing && closeWorkflowEditor()}
          disabled={executing}
          class="p-1.5 rounded-mac-sm text-text-dim hover:text-text hover:bg-bg-hover transition-all {executing ? 'opacity-40 cursor-not-allowed' : ''}"
          aria-label="关闭"
        >
          <X size={18} />
        </button>
      </div>

      <!-- 主体: 左右分栏 -->
      <div class="flex flex-1 min-h-0">
        <!-- 左侧: 可用节点 + 已保存工作流 -->
        <div class="w-[240px] border-r border-border/50 flex flex-col flex-shrink-0">
          <div class="px-4 py-2.5 border-b border-border/40">
            <h3 class="text-xs font-semibold text-text-dim uppercase tracking-wider">可用节点</h3>
          </div>
          <div class="flex-1 overflow-y-auto p-2 space-y-1">
            {#each NODE_DEFS as def (def.type)}
              {@const Icon = iconFor(def.type)}
              <div
                draggable="true"
                ondragstart={(e) => onNodeDragStart(e, def.type)}
                class="flex items-center gap-2.5 p-2.5 rounded-mac-sm border border-border bg-bg-hover/40 hover:bg-bg-hover hover:border-accent/40 cursor-grab active:cursor-grabbing transition-all"
              >
                <div class="w-7 h-7 rounded-mac-sm flex items-center justify-center flex-shrink-0 {def.bg}">
                  <Icon size={15} class={def.color} />
                </div>
                <div class="min-w-0">
                  <div class="text-sm font-medium text-text">{def.label}</div>
                  <div class="text-xs text-text-dim truncate">{def.desc}</div>
                </div>
              </div>
            {/each}
          </div>

          <!-- 已保存工作流 -->
          <div class="px-4 py-2.5 border-t border-border/40">
            <h3 class="text-xs font-semibold text-text-dim uppercase tracking-wider">已保存 ({savedWorkflows.length})</h3>
          </div>
          <div class="max-h-[160px] overflow-y-auto p-2 space-y-1">
            {#if savedWorkflows.length === 0}
              <p class="text-xs text-text-dim/60 italic px-2 py-1">暂无已保存工作流</p>
            {:else}
              {#each savedWorkflows as wf (wf.id)}
                <div class="flex items-center gap-1 p-1.5 rounded-mac-sm bg-bg-hover/40 border border-border group">
                  <button
                    onclick={() => onEditExisting(wf.id)}
                    class="flex-1 text-left text-xs text-text hover:text-accent truncate transition-colors"
                    title={wf.name}
                  >
                    {wf.name}
                    <span class="text-text-dim/60 ml-1">({wf.nodes.length})</span>
                  </button>
                  <button
                    onclick={() => onDeleteExisting(wf.id)}
                    class="text-text-dim hover:text-error transition-colors opacity-0 group-hover:opacity-100"
                    aria-label="删除工作流"
                  >
                    <Trash2 size={13} />
                  </button>
                </div>
              {/each}
            {/if}
          </div>
        </div>

        <!-- 右侧: 画布 -->
        <div class="flex-1 flex flex-col min-w-0">
          <!-- 名称 + 保存 -->
          <div class="flex items-center gap-2 px-4 py-3 border-b border-border/40">
            <input
              bind:value={editingName}
              placeholder="工作流名称..."
              class="flex-1 px-3 py-1.5 rounded-mac-sm bg-bg-hover border border-border text-text placeholder:text-text-dim/60 focus:outline-none focus:border-accent transition-all text-sm"
            />
            <button
              onclick={onSave}
              disabled={saving || executing || editingNodes.length === 0}
              class="btn-primary text-sm flex items-center gap-1.5 {saving || executing || editingNodes.length === 0 ? 'opacity-50 cursor-not-allowed' : ''}"
            >
              {#if saving}
                <LoaderCircle size={14} class="animate-spin" />
              {:else}
                <Save size={14} />
              {/if}
              保存
            </button>
          </div>

          <!-- 画布区域 -->
          <div
            class="flex-1 overflow-y-auto p-4 transition-colors {dragOverCanvas ? 'bg-accent/5' : ''}"
            ondragover={onCanvasDragOver}
            ondragleave={onCanvasDragLeave}
            ondrop={onCanvasDrop}
          >
            {#if editingNodes.length === 0}
              <div class="flex flex-col items-center justify-center h-full gap-3 text-text-dim">
                <div class="w-14 h-14 rounded-mac bg-bg-hover/50 flex items-center justify-center">
                  <Plus size={28} class="text-text-dim/50" />
                </div>
                <p class="text-sm">从左侧拖入节点开始组装</p>
                <p class="text-xs text-text-dim/60">前一步的输出将作为下一步的输入</p>
              </div>
            {:else}
              <div class="flex flex-col gap-0">
                {#each editingNodes as node, i (node.id)}
                  {@const def = nodeDef(node.type)}
                  {@const Icon = iconFor(node.type)}
                  {@const isExpanded = expandedNodeId === node.id}
                  {@const isRunning = executing && workflowsState.runningNodeIndex === i}
                  {@const isDone = executing && workflowsState.runningNodeIndex > i}
                  <div class="flex flex-col">
                    <!-- 连接线 (非首个) -->
                    {#if i > 0}
                      <div class="flex justify-center py-0.5">
                        <div class="w-px h-4 bg-border"></div>
                      </div>
                    {/if}
                    <!-- 节点卡片 -->
                    <div
                      draggable="true"
                      ondragstart={(e) => onItemDragStart(e, i)}
                      ondragover={(e) => onItemDragOver(e, i)}
                      ondragend={onItemDragEnd}
                      class="rounded-mac-sm border bg-bg-panel/80 shadow-sm transition-all {isRunning
                        ? 'border-accent ring-1 ring-accent/40'
                        : isDone
                          ? 'border-success/50'
                          : 'border-border'} {dragIndex === i ? 'opacity-50' : ''}"
                    >
                      <!-- 节点头 -->
                      <div class="flex items-center gap-2 px-3 py-2.5">
                        <GripVertical size={14} class="text-text-dim/50 cursor-grab flex-shrink-0" />
                        <div class="w-7 h-7 rounded-mac-sm flex items-center justify-center flex-shrink-0 {def?.bg}">
                          <Icon size={14} class={def?.color} />
                        </div>
                        <div class="flex-1 min-w-0">
                          <div class="text-sm font-medium text-text">
                            <span class="text-text-dim/60 mr-1">#{i + 1}</span>
                            {def?.label ?? node.type}
                          </div>
                        </div>
                        {#if isRunning}
                          <LoaderCircle size={14} class="text-accent animate-spin" />
                        {:else if isDone}
                          <Check size={14} class="text-success" />
                        {/if}
                        <!-- 展开/折叠配置 -->
                        <button
                          onclick={() => toggleExpand(node.id)}
                          class="text-text-dim hover:text-text transition-colors p-1"
                          aria-label="配置参数"
                        >
                          {#if isExpanded}
                            <ChevronUp size={14} />
                          {:else}
                            <ChevronDown size={14} />
                          {/if}
                        </button>
                        <!-- 上移 -->
                        <button
                          onclick={() => moveNode(i, -1)}
                          disabled={i === 0}
                          class="text-text-dim hover:text-text transition-colors p-1 disabled:opacity-30 disabled:cursor-not-allowed"
                          aria-label="上移"
                        >
                          <ChevronUp size={14} />
                        </button>
                        <!-- 下移 -->
                        <button
                          onclick={() => moveNode(i, 1)}
                          disabled={i === editingNodes.length - 1}
                          class="text-text-dim hover:text-text transition-colors p-1 disabled:opacity-30 disabled:cursor-not-allowed"
                          aria-label="下移"
                        >
                          <ChevronDown size={14} />
                        </button>
                        <!-- 删除 -->
                        <button
                          onclick={() => removeNode(node.id)}
                          class="text-text-dim hover:text-error transition-colors p-1"
                          aria-label="删除节点"
                        >
                          <Trash2 size={14} />
                        </button>
                      </div>
                      <!-- 节点参数 -->
                      {#if isExpanded}
                        <div class="px-3 pb-3 pt-1 border-t border-border/40 space-y-2.5">
                          {#if node.type === 'compress'}
                            <div>
                              <label class="text-xs text-text-dim mb-1 block">格式</label>
                              <select
                                value={node.params.format}
                                onchange={(e) => updateParam(node.id, 'format', e.currentTarget.value)}
                                class="w-full px-2.5 py-1.5 rounded-mac-sm bg-bg-hover border border-border text-text text-sm appearance-none cursor-pointer"
                              >
                                {#each COMPRESS_FORMATS as fmt (fmt.value)}
                                  <option value={fmt.value}>{fmt.label}</option>
                                {/each}
                              </select>
                            </div>
                            <div>
                              <label class="text-xs text-text-dim mb-1 flex justify-between">
                                <span>压缩级别</span>
                                <span class="font-mono text-accent">{node.params.level}</span>
                              </label>
                              <input
                                type="range"
                                min={lvlRange(node.params.format).min}
                                max={lvlRange(node.params.format).max}
                                value={node.params.level}
                                oninput={(e) => updateParam(node.id, 'level', parseInt(e.currentTarget.value, 10))}
                                class="w-full cursor-pointer"
                              />
                            </div>
                            <div>
                              <label class="text-xs text-text-dim mb-1 block">密码 <span class="text-text-dim/60">(可选)</span></label>
                              <input
                                type="text"
                                value={node.params.password}
                                oninput={(e) => updateParam(node.id, 'password', e.currentTarget.value)}
                                placeholder="留空不加密"
                                class="w-full px-2.5 py-1.5 rounded-mac-sm bg-bg-hover border border-border text-text text-sm font-mono"
                              />
                            </div>
                            <div>
                              <label class="text-xs text-text-dim mb-1 block">排除规则 <span class="text-text-dim/60">(逗号分隔)</span></label>
                              <input
                                type="text"
                                value={node.params.exclude}
                                oninput={(e) => updateParam(node.id, 'exclude', e.currentTarget.value)}
                                placeholder="*.tmp, node_modules/"
                                class="w-full px-2.5 py-1.5 rounded-mac-sm bg-bg-hover border border-border text-text text-sm font-mono"
                              />
                            </div>
                            <div>
                              <label class="text-xs text-text-dim mb-1 block">分卷大小 <span class="text-text-dim/60">(可选)</span></label>
                              <input
                                type="text"
                                value={node.params.split}
                                oninput={(e) => updateParam(node.id, 'split', e.currentTarget.value)}
                                placeholder="例如: 100M"
                                class="w-full px-2.5 py-1.5 rounded-mac-sm bg-bg-hover border border-border text-text text-sm font-mono"
                              />
                            </div>
                          {:else if node.type === 'decompress'}
                            <div>
                              <label class="text-xs text-text-dim mb-1 block">输出目录 <span class="text-text-dim/60">(默认与输入同目录)</span></label>
                              <input
                                type="text"
                                value={node.params.output}
                                oninput={(e) => updateParam(node.id, 'output', e.currentTarget.value)}
                                placeholder="留空自动生成"
                                class="w-full px-2.5 py-1.5 rounded-mac-sm bg-bg-hover border border-border text-text text-sm font-mono"
                              />
                            </div>
                            <div>
                              <label class="text-xs text-text-dim mb-1 block">密码 <span class="text-text-dim/60">(可选)</span></label>
                              <input
                                type="text"
                                value={node.params.password}
                                oninput={(e) => updateParam(node.id, 'password', e.currentTarget.value)}
                                placeholder="加密归档需填"
                                class="w-full px-2.5 py-1.5 rounded-mac-sm bg-bg-hover border border-border text-text text-sm font-mono"
                              />
                            </div>
                          {:else if node.type === 'encrypt' || node.type === 'decrypt'}
                            <div>
                              <label class="text-xs text-text-dim mb-1 block">密码 <span class="text-error">*</span></label>
                              <input
                                type="text"
                                value={node.params.password}
                                oninput={(e) => updateParam(node.id, 'password', e.currentTarget.value)}
                                placeholder="必填"
                                class="w-full px-2.5 py-1.5 rounded-mac-sm bg-bg-hover border border-border text-text text-sm font-mono"
                              />
                            </div>
                          {:else if node.type === 'copy-to' || node.type === 'move-to'}
                            <div>
                              <label class="text-xs text-text-dim mb-1 block">目标目录 <span class="text-error">*</span></label>
                              <input
                                type="text"
                                value={node.params.target}
                                oninput={(e) => updateParam(node.id, 'target', e.currentTarget.value)}
                                placeholder="/path/to/destination"
                                class="w-full px-2.5 py-1.5 rounded-mac-sm bg-bg-hover border border-border text-text text-sm font-mono"
                              />
                            </div>
                          {:else if node.type === 'delete-source'}
                            <p class="text-xs text-text-dim">无参数 · 删除上一步的输入文件</p>
                          {/if}
                        </div>
                      {/if}
                    </div>
                  </div>
                {/each}

                <!-- 拖入提示区 -->
                <div class="flex justify-center py-0.5">
                  <div class="w-px h-4 bg-border"></div>
                </div>
                <div
                  class="rounded-mac-sm border-2 border-dashed border-border text-center py-3 text-xs text-text-dim/60 transition-colors {dragOverCanvas ? 'border-accent text-accent bg-accent/5' : ''}"
                >
                  继续拖入节点...
                </div>
              </div>
            {/if}
          </div>

          <!-- 底部: 执行 -->
          {#if isEditingExisting && editingNodes.length > 0}
            <div class="flex items-center justify-between px-4 py-3 border-t border-border/40 bg-bg-hover/30">
              <p class="text-xs text-text-dim">{editingNodes.length} 个节点 · 将从主界面输入文件开始执行</p>
              <button
                onclick={() => onExecute(workflowsState.editingId!)}
                disabled={executing}
                class="btn-primary text-sm flex items-center gap-1.5 {executing ? 'opacity-50 cursor-not-allowed' : ''}"
              >
                {#if executing}
                  <LoaderCircle size={14} class="animate-spin" />
                  执行中...
                {:else}
                  <Play size={14} />
                  执行工作流
                {/if}
              </button>
            </div>
          {/if}
        </div>
      </div>
    </div>
  </div>
{/if}

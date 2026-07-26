// 本文件拥有出入库草稿工作台的共享契约类型；它不实现任何领域行为或 HTTP 请求。
import type { ComputedRef, Ref } from "vue";
import type { ItemOptionResponse } from "../../api/items";

/** 工作台可承载的最小行形状；具体字段由各领域行模型扩展。 */
export interface StockDraftLineBase {
  lineId: string;
  item: ItemOptionResponse;
}

/** 工作台壳与领域装配之间的桥；由页面在创建装配前构造，模板挂载后回填。 */
export interface StockDraftWorkspaceHandle {
  /** 打开指定行的明细编辑 Dialog；未挂载时为空操作。 */
  openLineEditor: (lineId: string) => void;
  /** 打开物品选择 Dialog；未挂载时为空操作。 */
  openItemPicker: () => void;
}

/** 创建默认的空操作桥，供装配在 workspace 挂载前安全调用。 */
export function createWorkspaceHandle(): StockDraftWorkspaceHandle {
  return { openLineEditor: () => {}, openItemPicker: () => {} };
}

/** 领域装配提供给通用工作台壳的状态与行为面。 */
export interface StockDraftFlow<L extends StockDraftLineBase> {
  /** 单据头：来源/去向。 */
  source: Ref<string>;
  /** 单据备注。 */
  notes: Ref<string>;
  /** 备注输入是否展开。 */
  notesOpen: Ref<boolean>;
  /** 当前草稿明细；工作台只读渲染，增删由 flow 方法完成。 */
  lines: Ref<L[]>;
  /** 是否已尝试过校验，用于错误态样式。 */
  validationAttempted: Ref<boolean>;
  /** 是否正在提交。 */
  submitting: Ref<boolean>;
  /** 本机是否存在可保存/需确认的草稿内容。 */
  hasDraft: ComputedRef<boolean>;
  /** 当前用户能否直接完成单据（直接入库/直接出库）。 */
  canDirect: ComputedRef<boolean>;
  /** 单据头输入引用，由工作台挂载、领域聚焦逻辑使用。 */
  sourceInput: Ref<HTMLInputElement | null>;

  /** 行阻塞原因；null 表示该行已完整。 */
  lineError(line: L): string | null;
  /** 行编辑按钮的无障碍标签。 */
  lineEditLabel(line: L): string;
  /** 去重加入物品并返回目标行；已存在时返回既有行。 */
  addItem(item: ItemOptionResponse, options?: { silent?: boolean }): L;
  /** 移除一行并完成领域侧清理。 */
  removeLine(lineId: string): void;
  /** 行编辑 Dialog 打开前的领域初始化（如出库分配草稿与批次加载）。 */
  onEditorOpen(line: L): void;
  /** “暂存并关闭”时的领域写回。 */
  onEditorStash(line: L): void;
  /** “完成并继续添加”：校验并提交行内容；失败时自行提示与聚焦并返回 false。 */
  commitEditor(line: L): boolean;
  /** 顶部提交前的整单校验；失败时自行提示与聚焦并返回 false。 */
  reviewGate(): boolean;
  /** 执行提交；返回值决定确认 Dialog 是否关闭（失败仍保留时返回 "keep"）。 */
  performSubmit(): Promise<"close" | "keep">;
  /** 确认清空草稿后的领域重置。 */
  clearDraft(): void;
  /** 物品选择器请求新建物品（仅入库域提供）。 */
  onCreateItemRequest?: () => void;
}

/** 工作台壳按领域渲染所需的稳定文案与列配置。 */
export interface StockDraftTexts {
  /** 页面根节点附加 class，便于领域级样式微调。 */
  rootClass: string;
  /** 摘要条 aria-label。 */
  summaryAriaLabel: string;
  /** 工作区标题（入库单信息与明细 / 出库单信息与明细）。 */
  workspaceTitle: string;
  /** 单据头区 aria-label。 */
  metaAriaLabel: string;
  /** 来源字段标题。 */
  sourceLabel: string;
  /** 来源字段表单 name。 */
  sourceName: string;
  /** 来源字段占位说明。 */
  sourcePlaceholder: string;
  /** 备注字段表单 name。 */
  notesName: string;
  /** 备注字段占位说明。 */
  notesPlaceholder: string;
  /** 明细区 aria-label。 */
  linesAriaLabel: string;
  /** 空态标题与提示。 */
  emptyTitle: string;
  emptyHint: string;
  /** 物品列与操作列之间的中间列表头。 */
  columns: string[];
  /** 行编辑 Dialog 标题与说明。 */
  editorTitle: string;
  editorDescription: string;
  /** 行编辑 Dialog 是否用宽布局（出库分配需要）。 */
  editorWide: boolean;
  /** 物品选择 Dialog 标题与搜索控件 name。 */
  pickerTitle: string;
  pickerSearchName: string;
  /** 清空确认文案。 */
  clearTitle: string;
  clearDescription: string;
  /** 离开确认正文。 */
  leaveBody: string;
  /** 提交确认标题与说明（按直接/待审批分列）。 */
  submitTitleDirect: string;
  submitTitlePending: string;
  submitDescriptionDirect: string;
  submitDescriptionPending: string;
  /** 顶部提交按钮文案。 */
  submitButtonDirect: string;
  submitButtonPending: string;
  /** 提交确认按钮文案。 */
  submitConfirmDirect: string;
  submitConfirmPending: string;
}

// 本文件定义库位管理组件共享的纯前端呈现类型；它不复制后端 DTO 或业务校验。

/** 分组选择控件使用的层级化选项。 */
export interface LocationGroupOption {
  /** 分组 ID。 */
  id: number;
  /** 包含层级缩进的分组名称。 */
  label: string;
  /** 分组所在层级，根分组为 1。 */
  depth: number;
}

/** 库位管理删除确认目标。 */
export interface LocationDeleteTarget {
  /** 删除对象类型。 */
  kind: "group" | "location";
  /** 删除对象 ID。 */
  id: number;
  /** 删除确认中显示的对象名称。 */
  label: string;
  /** 分组删除后用于恢复选择的上级分组 ID。 */
  parentId?: number | null;
}

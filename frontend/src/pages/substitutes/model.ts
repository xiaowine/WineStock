// 本文件拥有全局替代关系的分组和本地搜索纯函数；它不请求 API 或修改编辑草稿。
import type { SubstituteRelationResponse } from '../../api/substitutes'

/** 全局列表中按主物品聚合后的替代关系组。 */
export interface SubstituteRelationGroupModel {
  itemId: number
  itemName: string
  itemSku: string
  relations: SubstituteRelationResponse[]
  firstSubstitute: SubstituteRelationResponse
  hasNotes: boolean
}

/** 替代关系编辑 Dialog 固定展示的主物品上下文。 */
export interface SubstituteEditorTarget {
  id: number
  name: string
  sku: string
}

/** 按主物品聚合关系，并稳定保持优先级顺序。 */
export function groupSubstituteRelations(relations: readonly SubstituteRelationResponse[]): SubstituteRelationGroupModel[] {
  const groups = new Map<number, SubstituteRelationResponse[]>()
  for (const relation of relations) {
    const current = groups.get(relation.item_id)
    if (current) current.push(relation)
    else groups.set(relation.item_id, [relation])
  }

  return Array.from(groups.values())
    .map((items) => {
      const sorted = items.slice().sort((left, right) => left.priority - right.priority || left.substitute_item_id - right.substitute_item_id)
      const firstSubstitute = sorted[0]
      return {
        itemId: firstSubstitute.item_id,
        itemName: firstSubstitute.item_name,
        itemSku: firstSubstitute.item_sku,
        relations: sorted,
        firstSubstitute,
        hasNotes: sorted.some((relation) => Boolean(relation.notes?.trim())),
      }
    })
    .sort((left, right) => left.itemName.localeCompare(right.itemName, 'zh-CN') || left.itemId - right.itemId)
}

/** 在当前已加载的全量关系中匹配主物品、替代物品和备注。 */
export function filterSubstituteRelationGroups(
  groups: readonly SubstituteRelationGroupModel[],
  search: string,
): SubstituteRelationGroupModel[] {
  const keyword = search.trim().toLocaleLowerCase('zh-CN')
  if (!keyword) return groups.slice()

  return groups.filter((group) => [
    group.itemName,
    group.itemSku,
    ...group.relations.flatMap((relation) => [relation.substitute_item_name, relation.substitute_item_sku, relation.notes ?? '']),
  ].some((value) => value.toLocaleLowerCase('zh-CN').includes(keyword)))
}

/** 统计当前关系组包含的真实关系条数。 */
export function countGroupedRelations(groups: readonly SubstituteRelationGroupModel[]): number {
  return groups.reduce((total, group) => total + group.relations.length, 0)
}

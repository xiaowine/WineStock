// 本文件拥有前端应用壳演示数据，属于 frontend 原型层；后续接入 API 后应移除业务假数据。
export const navItems = [
  { key: 'stock', label: '库存' },
  { key: 'inbound', label: '入库' },
  { key: 'outbound', label: '出库' },
  { key: 'locations', label: '库位' },
  { key: 'users', label: '用户' },
]

export const metrics = [
  { label: '库存项', value: '1,284', caption: '较昨日 +18' },
  { label: '低库存', value: '27', caption: '需要补货' },
  { label: '待审批', value: '9', caption: '入库 5 / 出库 4' },
]

export const stockItems = [
  {
    sku: 'CAP-0603-104',
    name: '贴片电容 0.1uF',
    location: '默认库区 / A-01',
    quantity: '12,800',
    status: '正常',
    statusKind: 'ok',
  },
  {
    sku: 'RES-0805-103',
    name: '贴片电阻 10K',
    location: '默认库区 / B-04',
    quantity: '640',
    status: '偏低',
    statusKind: 'warn',
  },
  {
    sku: 'PLA-BLK-175',
    name: 'PLA 黑色耗材',
    location: '打印耗材 / R-02',
    quantity: '3',
    status: '补货',
    statusKind: 'danger',
  },
]

export const filters = ['全部物料', '低库存', '待审批', '默认库区', '打印耗材']

# WineStock 跨平台品牌图标实施记录

## 状态

已选定并实施 **Indexed Cube** 作为跨平台品牌图标。权威母版位于仓库根 `brand/`；原评审板和独立概念母版保留为设计决策记录。

评审板：[`assets/winestock-brand-concept-indexed-cube.svg`](assets/winestock-brand-concept-indexed-cube.svg)

独立矢量母版：[`assets/winestock-mark-concept-indexed-cube.svg`](assets/winestock-mark-concept-indexed-cube.svg)

## 设计方向

**Indexed Cube** 的主体是分离三面的等距库存箱体，侧面的镂空标签表达结构化编号和归档，更强调单个库存对象及其身份。它不使用酒杯或酒瓶图形，因此不会把 WineStock 限定为酒类库存，也适合未来扩展到桌面和其它 Shell。

当前项目中的文字 `W` 只视为占位符，不作为正式品牌元素，也不参与这套图标的形态来源。评审板里的 `WineStock` 仅表示图标与产品名称在界面中并排时的组合关系，产品名称不属于图标本体。

标志只由一个可填充轮廓组成，不依赖字体、描边或渐变。相同轮廓可以直接派生以下资源：

- 前端品牌标志、紧凑导航标志和 favicon；
- Android adaptive icon 前景、monochrome themed icon 和 SplashScreen 图标；
- 未来桌面 Shell 的窗口、任务栏、ICO 和 ICNS 图标；
- 单色打印、深色主题和系统动态着色场景。

## 颜色

| 角色 | 色值 | 用途 |
| --- | --- | --- |
| Wine red | `#6F2A36` | 品牌主色、浅色背景上的标志 |
| Cold white | `#F4F5F7` | 深色或酒红背景上的标志 |
| Blush accent | `#D48B98` | 少量辅助强调，不进入单色母版 |

品牌图标不使用渐变。Android 主题图标和小尺寸图标只使用母版轮廓与单一前景色。

## 跨平台约束

- 母版采用 `108 x 108` 视口，主体保留在中央区域，圆形和圆角方形遮罩下不裁切关键结构。
- `16px` 下优先保留箱体三面轮廓；侧面索引标签允许弱化，不得为维持标签而增加更细的内部装饰。
- 前端组合标志中的 `WineStock` 使用产品现有系统字体，不把文字转成另一个不可维护的图形资产。
- `BETA` 是组合方式示例，不属于标志路径。
- Android、前端和未来桌面资源应从同一母版派生，任何平台不得自行重画一个近似版本。

## 实施边界

仓库级 `brand/` 保存平台无关母版。前端通过可复用 `BrandMark` 替换应用壳、认证页、初始化向导和运行设置中的文字占位，并使用同一图形生成 favicon。Android 从母版派生 adaptive foreground、background、monochrome、SplashScreen 和原生兼容页 VectorDrawable。

正式 Desktop Shell 尚未实现，因此只保留母版与导出规则，不提前创建无归属脚手架或未消费的 ICO/ICNS/PNG。

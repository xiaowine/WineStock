// en-US 语言包聚合：键必须与 zh-CN 基准完全一致（satisfies 在构建期强制双语言键对齐）。
import { approvals } from "./approvals";
import { auth } from "./auth";
import { bridge } from "./bridge";
import { common } from "./common";
import { components } from "./components";
import { dashboard } from "./dashboard";
import { error } from "./error";
import { events } from "./events";
import { items } from "./items";
import { locations } from "./locations";
import { misc } from "./misc";
import { nav } from "./nav";
import { orders } from "./orders";
import { runtime } from "./runtime";
import { startup } from "./startup";
import { stockDraft } from "./stockDraft";
import { substitutes } from "./substitutes";
import { templates } from "./templates";
import { update } from "./update";
import { users } from "./users";
import { validation } from "./validation";
import type { zhCN } from "../zh-CN";

/** en-US 完整消息树；缺键或多余键都会在类型检查期报错。 */
export const enUS = {
  common,
  nav,
  error,
  validation,
  bridge,
  update,
  auth,
  startup,
  runtime,
  dashboard,
  items,
  stockDraft,
  orders,
  approvals,
  templates,
  locations,
  substitutes,
  users,
  events,
  components,
  misc,
} satisfies typeof zhCN;

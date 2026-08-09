// zh-CN 语言包聚合：各域文件在此合并为单一消息树；键按域组织，禁止跨域重复。
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

/** zh-CN 完整消息树；作为语言包键结构的基准类型。 */
export const zhCN = {
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
};

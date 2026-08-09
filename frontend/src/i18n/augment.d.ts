// vue-i18n 与 Vue 全局属性类型增强：t/$t 使用消息键结构，$title 用于翻译路由标题。
import type { MessageSchema } from "./schema";

declare module "vue-i18n" {
  export interface DefineLocaleMessage extends MessageSchema {}
}

declare module "vue" {
  interface ComponentCustomProperties {
    /** 翻译路由标题消息键；传入非键（如品牌名）时原样返回。 */
    $title(key: unknown): string;
  }
}

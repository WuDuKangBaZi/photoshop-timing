import { createApp } from "vue";
import Log from "./log.vue"; // 确保路径正确
import ElementPlus from "element-plus";
import zhCn from 'element-plus/es/locale/lang/zh-cn'
import "element-plus/dist/index.css";

createApp(Log).use(ElementPlus,{
    locale: zhCn // 设置 Element Plus 的语言为中文
}).mount("#app");
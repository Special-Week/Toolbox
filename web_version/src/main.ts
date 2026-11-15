// 创建并挂载网页应用，并引入统一样式。
import { createApp } from "vue";
import App from "./App.vue";
import "katex/dist/katex.min.css";
import "./style.css";

createApp(App).mount("#app");

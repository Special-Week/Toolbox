// 管理界面偏好（深浅主题、收藏工具、最近使用）与 7 大分类视觉标识。
export type ThemeMode = "light" | "dark";

export interface UiPrefs {
  theme: ThemeMode;
  favorites: string[];
  recent: string[];
}

const UI_PREFS_KEY = "toolbox:ui-prefs:v1";

export function loadUiPrefs(): UiPrefs {
  try {
    const raw = localStorage.getItem(UI_PREFS_KEY);
    if (!raw) return defaultPrefs();
    const parsed = JSON.parse(raw) as Partial<UiPrefs>;
    const theme: ThemeMode = parsed.theme === "dark" ? "dark" : "light";
    const favorites = Array.isArray(parsed.favorites)
      ? parsed.favorites.filter((id): id is string => typeof id === "string")
      : [];
    const recent = Array.isArray(parsed.recent)
      ? parsed.recent
          .filter((id): id is string => typeof id === "string")
          .slice(0, 8)
      : [];
    return { theme, favorites, recent };
  } catch {
    return defaultPrefs();
  }
}

export function saveUiPrefs(prefs: UiPrefs): void {
  try {
    localStorage.setItem(UI_PREFS_KEY, JSON.stringify(prefs));
  } catch {
    /* 本地存储不可用时忽略持久化错误 */
  }
}

function defaultPrefs(): UiPrefs {
  return { theme: "light", favorites: [], recent: [] };
}

export function applyTheme(theme: ThemeMode): void {
  if (typeof document !== "undefined") {
    document.documentElement.dataset.theme = theme;
  }
}

export function groupKey(group: string): string {
  switch (group) {
    case "单位换算":
      return "units";
    case "ITS90 温标换算":
      return "temp";
    case "压力工具":
      return "pressure";
    case "温湿度工具":
      return "humidity";
    case "误差工具":
      return "errors";
    case "量规计算":
      return "gauges";
    default:
      return "other";
  }
}

export function groupShortName(group: string): string {
  switch (group) {
    case "单位换算":
      return "单位";
    case "ITS90 温标换算":
      return "温标";
    case "压力工具":
      return "压力";
    case "温湿度工具":
      return "温湿度";
    case "误差工具":
      return "误差";
    case "量规计算":
      return "量规";
    case "其他工具":
      return "其他";
    default:
      return group;
  }
}

// 简洁线性 SVG 路径（24x24 viewBox），对齐桌面端 Phosphor 风格
export function groupIconPath(group: string): string {
  switch (group) {
    case "单位换算":
      return "M7 7h10l-3-3m3 13H7l3 3M7 7l3-3M17 17l-3 3";
    case "ITS90 温标换算":
      return "M14 14.76V5a2 2 0 1 0-4 0v9.76a4 4 0 1 0 4 0Z";
    case "压力工具":
      return "M12 13l4-4m-9.2 7.2A8 8 0 1 1 17.2 16.2";
    case "温湿度工具":
      return "M12 3.5c-3.8 4.8-6 8.1-6 11a6 6 0 0 0 12 0c0-2.9-2.2-6.2-6-11Z";
    case "误差工具":
      return "M5 19h14M7 15v-4m5 4V7m5 8v-6";
    case "量规计算":
      return "M4 15.5 15.5 4 20 8.5 8.5 20 4 15.5Zm6.5-6.5 2 2m-4 2 2 2m4-8 2 2";
    default:
      return "M14.7 6.3a4 4 0 0 0-5.4 5.4l-5 5a1.5 1.5 0 1 0 2.1 2.1l5-5a4 4 0 0 0 5.4-5.4l-2.3 2.3-2.1-2.1 2.3-2.3Z";
  }
}

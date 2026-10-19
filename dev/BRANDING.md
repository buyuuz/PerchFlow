# PerchFlow 品牌图标素材

2026-10-19：采用用户确认的蓝色流线形透明 PerchFlow 徽标，并仅在本次 **前端开发分支**更新品牌展示和浏览器图标。

## 源素材

- `frontend/assets/perchflow-emblem.webp`：从确认的透明 PNG 中截取的蓝色图形，192 × 192，带真实 Alpha 通道，来源于用户确认的透明版 PerchFlow Logo
- 导出时未使用白色底色；登录页的“PerchFlow”名称使用前端排版，与图形分离以适配深色背景

## 自动生成资源

运行以下脚本可以从批准素材重新生成图标（仅素材重新处理需要 Pillow）：

```powershell
python -m pip install Pillow
python dev/build_logo_assets.py
python dev/apply_logo_branding.py
```

生成／替换 `frontend/assets/logo.png`、`favicon.ico`、`favicon.svg`、`favicon-96x96.png`、`logo.ico`、`apple-touch-icon.png`、两个 `web-app-manifest-*.png`。

浏览器缓存版本标记：`?v=brand2`。

**注意：** 仓库根目录 `logo.ico` 是 Windows EXE 图标，本阶段未修改。正式版本仍须在 UI 定稿后由 GitHub Actions 构建，不应因为修改 Logo 就立刻替换现有 EXE。

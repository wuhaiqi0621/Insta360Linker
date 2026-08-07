# Mica UI 重构日志

项目：F:\Insta360onWin
目标：为 Luna Studio 添加/完善 Windows 11 原生 Mica 背景效果，并全面适配前端 UI 以配合 Mica。
当前入口：run_release.bat -> LunaStudio.exe（即 html_app 二进制）。

## 计划

1. 记录项目上下文与计划到本文件。
2. 完善后端 src/bin/html_app.rs 的 Mica 实现（主题检测、回退、主题变更时重应用）。
3. 优化前端 web/index.html 的 CSS 变量，使其更契合 Mica（透明基底、半透明表面、明暗主题变量）。
4. 重构目前对 Mica 不友好的硬编码深色区域（相机控制台、预览弹窗、水印演示区）。
5. 添加前端系统主题同步与细节打磨。
6. 构建并验证。

## 操作记录

### 2026-07-23 00: 创建日志并梳理现状

- 当前 src/bin/html_app.rs 已包含 apply_mica 函数，使用 DWMWA_SYSTEMBACKDROP_TYPE + DWMSBT_MAINWINDOW 实现 Mica，并已在 WindowEvent::ThemeChanged 时重应用。
- 当前 web/index.html 已具备透明背景、backdrop-filter 毛玻璃、明暗主题变量。
- 存在问题：部分区域（camera-console、预览弹窗、水印演示区）仍使用硬编码不透明深色背景，破坏 Mica 效果；整体透明度与对比度仍可优化。
- 决定：以 html_app 为主入口进行改造，不改动 eframe 版 src/main.rs（README 说明当前日用版本为 html_app）。

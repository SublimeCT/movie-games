# 更新日志 (Changelog)

## [Unreleased] - 2026-01-04

### 新增 (Added)
- **主页优化**: 在主页右上角增加了 GitHub 仓库链接按钮，方便用户访问源码。
- **文档完善**: 
  - 更新了 `README.md`，使其结构更加清晰，包含特性列表、技术栈、快速开始和贡献指南，对齐开源项目标准。
  - 创建了 `CHANGES.md` 用于记录项目变更。

### 修复 (Fixed)
- **Home.vue**: 修复了 GitHub 图标不显示的问题。通过将 `Github` 组件重命名为 `GithubIcon`，解决了与 HTML 标签解析冲突的问题。

### 优化 (Changed)
- **文档**: 恢复并完善了 `README.md` 中的部署指南，补充了前端 Nginx 配置示例和后端 `sensitive-rs` 字典配置的详细说明。
- **Home.vue**: 更新了 Header 布局以容纳 GitHub 按钮。

# 改动记录 (Requirement Changes)

本文档用于记录 `movie-games` 项目所有的需求变更与代码改动历史。

## 功能模块 (Features & Modules)

### 1. 数据安全 (Data Security)
- **Shared Record ID 隐藏**:
  - **后端改动**: 移除 `get_shared_record_id_by_request_id` 接口，API 响应 (`/records/meta`, `/records`) 不再返回 `shared_records.id`，仅返回 `requestId` (`glm_requests.id`)。
  - **前端改动**: `Records.vue`、`Designer.vue`、`Ending.vue` 等组件全面移除对 `sharedRecordId` 的依赖，统一使用 `requestId` 作为记录唯一标识。
  - **目的**: 严格防止内部数据库 ID 泄露，消除潜在的安全风险。

### 2. 剧情设计器 (Designer)
- **剧情树可视化组件化**:
  - 将剧情树 (`VueFlow`) 提取为独立的 `PlotTree.vue` 组件，确保设计页 (`Designer.vue`) 和结局页 (`Ending.vue`) 的展示逻辑与样式严格一致。
  - 组件支持 `zoom-on-scroll`, `zoom-on-pinch`, `zoom-on-double-click`, `pan-on-drag` 等全套缩放平移交互。
  - 组件内部封装 BFS 分层布局算法，统一节点排布逻辑。
  - 支持 `highlightedIds` 属性，用于结局页高亮通关路径。
- **编辑功能**:
  - 允许在设计页修改“剧情简介”和“剧情类型”。
  - 默认锁定剧情树节点位置 (`nodes-draggable="false"`)，防止误操作。
  - 修复节点数据保存问题：保存时强制使用本地最新草稿同步 `gameData`，解决后端返回旧数据导致覆盖本地修改的 Bug。
  - 修复点击节点报错问题 (`TypeError: Cannot read properties of undefined`)。
  - 修复剧情树滚轮缩放失效问题（启用 `preventScrolling` 以正确捕获滚轮事件）。
- **访问控制**:
  - `shared` 模式（分享链接进入）下，自动转换为 `import` 模式，允许用户在本地查看和编辑副本，不再显示“禁止访问”。

### 3. 游戏引擎与核心逻辑 (Game Engine)
- **状态管理 (State Management)**:
  - 重构 `useGameState` hook，使用**全局单例模式**管理 `gameData` 和 `endingData`，彻底修复跨组件（Designer -> Game）数据不同步的问题。
- **Prompt 优化**:
  - 优化“AI 智能扩写”提示词，强制要求生成 300-400 字、有起伏转折的剧情简介。
  - 优化生成提示词，确保节点好感度逻辑合理。
- **数据回填**:
  - 首页 (`Home.vue`) 挂载时，若存在活跃的 `gameData`，自动回填剧情简介、类型和主题，方便用户基于上次游戏进行二次创作或修改。

### 4. 结局页 (Ending Page)
- **用户体验**:
  - 角色好感度列表增加用户头像 (`CharacterAvatar`) 展示。
  - 剧情树视图完全接入 `PlotTree.vue` 组件，保持与设计页一致的交互体验。
  - 高亮显示本次游玩的路径。

### 5. 后端与 API (Backend)
- **Bug 修复**:
  - 修复“分享剧情”按钮点击报错“Service Busy”的问题，大幅提升 `daily_ip` 分享频率限制（从 3 提升至 100）。
- **数据安全**:
  - 分享接口 (`/share`) 增加权限校验，防止非创建者操作。

### 6. 文档 (Documentation)
- **结构调整**:
  - 将 `REQUIREMENT_CHANGES.md` 从按日期记录调整为按**功能模块**分类记录，提高可读性。

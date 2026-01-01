# Movie Games 产品需求文档 (PRD)

> **基于源代码的 100% 准确功能清单**
> 
> 本文档严格基于当前代码库 (`front/`, `server/`) 编写，列出所有实际存在的页面、接口及业务逻辑。
> 最后更新时间: 2026-01-01（更新）

## 0. 工程约束

- 前端必须通过 `pnpm -C front build`（包含 `vue-tsc -b` 类型构建检查）
- 后端启动会自动执行 SQLx migrations
- 已经应用到数据库的迁移文件视为不可变；任何结构变更必须新增迁移文件，禁止修改已应用迁移
- 若出现 `VersionMismatch`，必须通过“恢复旧迁移文件原内容 + 新增迁移承载变更”修复；`MOVIE_GAMES_ALLOW_MIGRATE_VERSION_MISMATCH=1` 仅用于应急排障，不作为长期方案

## 1. 产品页面清单 (Page Inventory)

### 1.1 首页 (Home Page)
**路径**: `/`
**组件**: `front/src/components/Home.vue`

**核心功能区**:
1.  **向导模式表单 (Wizard Mode)**:
    *   *注: 这是当前唯一可用的界面模式。*
    *   **游戏主题 (Theme)**: 文本输入框。支持 "随机主题" (🎲 骰子动画，点击后随机填充主题、类型、简介、角色)。
        *   **去重机制**: 连续 3 次点击内不会出现重复主题 (通过 `recentThemeIndices` 记录最近选择的历史)。
    *   **剧情类型 (Genres)**: 多选标签 (预设：科幻, 剧情, 爱情, 悬疑, 喜剧, 青春, 历史, 冒险, 武侠, 伦理, 悲剧, 职场, 爽文)。支持手动添加自定义类型。
    *   **剧情简介 (Synopsis)**: 多行文本框。
        *   **AI 智能扩写**: 点击按钮调用 `/expand/worldview` 接口，根据主题自动生成简介。
    *   **角色阵容 (Characters)**: 角色列表。
        *   每位角色包含: 名字, 性别 (默认为"其他"), 身份/性格描述, 是否主角 (复选框)。
        *   角色本地存储 (`mg_characters`) 允许附带 `avatarPath`（由设计器上传头像后写入）；调用后端生成/扩写/导入保存接口时会自动剔除该字段，仅提交 `name/description/gender/isMain`。
        *   **AI 生成角色**: 点击按钮调用 `/expand/character` 接口，根据主题和简介自动生成角色列表。
        *   支持手动添加/删除角色。
2.  **操作按钮**:
    *   **开始生成**: 校验必填项 (主题) 后，跳转至 `/generating`。
    *   **仅生成提示词**: 调用 `/generate/prompt`，在弹窗中显示生成的 Prompt，支持一键复制。
3.  **辅助功能模态框 (Modals)**:
    *   **连接设置 (Settings)**: 配置 GLM API Key, Base URL, Model。支持校验 URL 格式。
        *   **安全锁判定**: 当 Base URL 或 Model 不等于默认值时，触发“数据安全锁”（API Key 的变化不触发）。
        *   **影响**: 触发后将禁用“分享/设计”相关入口（例如导入弹窗的“导入并设计”按钮）。
    *   **导入存档 (Import)**: 支持粘贴 JSON 文本或上传 JSON 文件导入剧情数据。
        *   支持的 JSON 格式：`MovieTemplate`（含或不含 `requestId`）、数据库字段 `processed_response`（即不含 `requestId` 的 `MovieTemplate`）、以及后端生成接口返回的 `{ id, template }` 包装结构。
        *   导入后可选择：直接进入游玩(`/game`) 或进入剧情设计器(`/design`)（若触发数据安全锁则禁用“导入并设计”）。
        *   **本地输入回填**: 选择“导入并游玩/导入并设计”后，会把导入 JSON 的 `meta/characters` 回填到本地向导存储（`mg_theme/mg_synopsis/mg_genres/mg_characters`），避免后续页面读取到旧的首页输入。
        *   支持“导入并保存”：会调用后端 `POST /import` 新增一条数据库记录并返回 `requestId`；请求体会同时包含“首页填写信息 + 导入 JSON 全量数据”（合并后提交）。主题允许直接来自导入 JSON（若两者都缺失则报错）。
    *   **帮助 (Help)**: 显示设计理念和操作技巧。

**代码级差异说明**:
*   **自由模式 (Free Mode)**: 代码中定义了 `mode` 变量 (默认 'wizard') 和 `freeInput` 变量，但在 UI 模板中 **完全没有渲染** 自由模式的输入框或切换按钮。向导模式表单是无条件显示的。因此，自由模式在当前版本中 **不可用**。
*   **节点数量控制**: UI 上 **不存在** 节点数量选择滑块 (Slider)。用户无法自定义生成的节点数量。

### 1.2 生成页 (Generating Page)
**路径**: `/generating`
**组件**: `front/src/components/Generating.vue`

**功能**:
1.  **加载状态**: 显示 `CinematicLoader` 动画。
    *   **鼠标移动特效**: 加载中叠加 `FluidCursor` 流体光标效果。
2.  **生成逻辑**: 页面加载时自动从 `localStorage` (`mg_generate_params`) 读取生成参数并调用 `/generate` 接口。
    *   **禁止使用 query 参数**: 生成参数通过 `localStorage` 传递，读取后立即清除。
3.  **错误处理**:
    *   若发生 API Key 缺失 (`API_KEY_REQUIRED`) 或限流 (`TOO_MANY_REQUESTS`) 错误，显示错误提示。
    *   支持 5 秒倒计时自动返回首页。
    *   错误信息会通过 `sessionStorage` (`mg_last_error`) 传递回首页并在首页自动弹出设置框。

### 1.3 游戏页 (Game Page)
**路径**: `/game`
**组件**: `front/src/components/Game.vue`

**功能**:
1.  **浏览器标题**: 页面标题会自动追加当前剧情标题前缀（格式：`{剧情标题} - {原页面标题}`），离开页面会恢复。
2.  **剧情卡片**: 3D 悬停效果卡片 (`ThreeDCard`)，显示当前剧情内容。
    *   支持 "聚光灯" (Spotlight) 鼠标跟随效果。
2.  **角色显示**: 顶部显示当前场景涉及的角色头像 (`CharacterAvatar`)。
3.  **分支选择**: 底部列出当前剧情的选项，点击后跳转至下一节点。
4.  **状态管理**:
    *   自动保存当前节点 ID (`mg_current_node`) 和玩家状态 (`mg_player_state`)。
    *   自动保存角色好感度状态 (`mg_affinity_state`)，用于驱动角色表情与结局页展示（主角除外）。
    *   **剧情模板标准化**: 读取 `mg_active_game_data` 时会对数据做标准化，自动补齐 `meta/characters/provenance` 等必需字段，并在发现旧存档缺字段时写回本地存储，保证游戏/结局/设计器拿到完整 `MovieTemplate`。
    *   **起始节点兼容**: 若 `start/root` 节点没有任何选项且存在节点 `1`，则自动跳转到节点 `1`；否则按“数字 ID 优先 + 数值升序 + 字典序”选择第一个有选项的节点。
    *   **节点内容兼容**: 节点内容同时支持 `string` 与 legacy `{ text }` 格式，用于剧情展示与角色表情推断。
    *   **返回上一步**: 点击左上角返回按钮，回退至上一节点 (利用状态栈)。
    *   **返回首页**: 点击时弹窗提示将清空当前游戏进度与剧情；确认后清除游戏状态并返回首页，取消则继续游玩。

### 1.4 结局页 (Ending Page)
**路径**: `/ending`
**组件**: `front/src/components/Ending.vue`

**功能**:
1.  **结局展示**: 显示结局类型 (Happy/Bad/Neutral) 和描述。
2.  **数据统计**: 显示经历的节点数、解锁结局数、登场角色数。
3.  **角色好感度**: 显示各角色好感度百分比（排除主角），数据来自 `localStorage.mg_affinity_state`。
4.  **剧情分析弹窗**: 展示剧情元信息、分享状态与关键标识；当剧情处于已分享状态时提供“取消分享”入口。
5.  **剧情树**:
    *   以 SVG `treeGraph` 展示节点/结局关系（可拖拽平移、滚轮缩放、自适应居中）。
    *   **起始节点兼容**: 若存在 `start` 节点但其没有任何选项，且存在节点 `1`，则以节点 `1` 作为起点。
6.  **进入设计**:
    *   创建者入口：结局页提供“进入设计”按钮，跳转到 `/design`（若有 `requestId` 则携带 `?id=:requestId` 以从后端加载可编辑数据）。
    *   导入入口：导入模式(`mg_play_entry=import`)下也允许进入设计器（仅本地编辑）。
    *   若触发“数据安全锁”，创建者入口的“进入设计”会被禁用并阻止跳转。
7.  **分享功能 (仅创建者可见)**:
    *   通过 `GET /records/meta/:requestId` 判断当前是否为创建者 (owner) 以及分享状态。
    *   创建者可调用 `POST /share` 切换分享状态，分享成功时会返回 `sharedRecordId` 并写入本地历史记录。
    *   若触发“数据安全锁”，分享按钮会被禁用，并阻止发起请求。
    *   非创建者进入结局页时，分享按钮不会出现 (避免泄露 sharedRecordId)。
    *   导入模式(`mg_play_entry=import`)下禁止在线分享，并且不显示分享入口与分享链接。
8.  **导出**:
    *   仅支持导出为 JSON（复制 JSON / 下载 JSON）。
    *   导出内容为数据库 `processed_response` 同结构：导出的顶层为 `MovieTemplate`（不包含 `requestId`），可直接用于首页/设计器导入。
9.  **操作**:
    *   **重新开始**: 回到首页重置游戏。
    *   **再次挑战**: 清空本次游玩进度并回到开始节点；若入口为分享模式则返回 `/play/:id`，否则返回 `/game`。

### 1.5 游玩页 (Play Page)
**路径**: `/play/:id`
**组件**: `front/src/components/Play.vue`

**功能**:
1.  **浏览器标题**: 加载完成并进入游戏后，页面标题会自动追加当前剧情标题前缀（格式：`{剧情标题} - {原页面标题}`）。
2.  **加载共享游戏**: 根据 URL 中的 ID 调用 `/api/play/:id` 接口获取游戏数据。
3.  **入口标识与重开定位**:
    *   会写入 `sessionStorage.mg_shared_play_id = :id`，用于结局页“再次挑战”回到正确的 `/play/:id`。
    *   默认将 `sessionStorage.mg_play_entry` 视为 `shared`；但当 `mg_play_entry=owner` 且 `sessionStorage.mg_owner_play_id === :id` 时，保持创建者入口（用于历史记录页从创建者视角进入 `/play/:id`）。
4.  **直接显示游戏**: 获取数据成功后，直接在当前页面渲染 `Game` 组件，**不进行路由跳转**。
    *   通过设置 `localStorage` (`mg_active_game_data` 等) 让 Game 组件读取数据。
    *   与 `/game` 页面使用相同的 Game 组件和状态管理逻辑。
5.  **刷新恢复**: 在游玩完成并从结局页“再次挑战”返回后，若用户在 `/play/:id` 刷新页面，即使本地 `mg_current_node` 恰好为 `start` 且数据需要异步恢复，也必须能自动跳转到可游玩节点并正常渲染剧情与选项（优先节点 `1`，否则选择第一个有选项的节点）。

### 1.6 历史记录页 (Records Page)
**路径**: `/records`
**组件**: `front/src/components/Records.vue`

**功能**:
1.  **本地历史记录索引**: 从 `localStorage` (`mg_record_ids`) 读取 `shared_records.id` 列表并去重。
2.  **拉取列表数据**: 调用 `POST /api/records` 批量获取记录的轻量信息 (标题/简介/分享时间/游玩次数等)。
3.  **一键游玩**: 点击“游玩”跳转到 `/play/:requestId`，并标记入口为创建者视角。
4.  **进入设计**: 点击“进入设计”跳转到 `/design?id=:requestId`（设计页会再次通过 `/records/meta/:requestId` 校验是否为创建者；若触发数据安全锁则禁用）。
5.  **复制链接**: 复制 `.../play/:requestId` 到剪贴板。
6.  **删除剧情 (服务端删除)**: 点击“删除剧情”弹出确认弹窗；确认后调用 `POST /api/template/delete` 删除服务端保存的剧情数据（会同时删除分享记录与游玩记录），并从本地列表移除。
7.  **仅移除列表 (本地移除)**: 点击“仅移除列表”会弹出确认弹窗；确认后从 `mg_record_ids` 中移除该记录 (不删除后端数据)。
8.  **取消分享 / 重新分享**: 通过 `POST /api/share` 切换分享状态（若触发数据安全锁则禁用）。取消分享会弹出站内确认弹窗。

### 1.7 剧情设计器页 (Designer Page)
**路径**: `/design`
**组件**: `front/src/components/Designer.vue`

**功能**:
1.  **编辑首页输入 (本地持久化)**:
    *   主题、角色阵容等字段与首页保持一致，并同步保存到本地。
    *   **显示一致性修复**: 进入设计器后会从当前剧情模板 (`MovieTemplate.meta/characters`) 回填这些字段到本地存储，避免显示到其他剧本或旧的首页向导输入。
    *   **设计页只读限制**: “剧情简介”“剧情类型”在设计页显示但禁止修改。
    *   **角色性别选择**: 角色性别必须通过下拉选择（男 / 女 / 其他），禁止自由输入。
    *   **角色头像上传**: 角色阵容支持上传图片并在前端转为 base64 字符串保存（并尝试按角色名同步到模板角色的 `avatarPath`）。
    *   **角色删除限制**: 若角色名称在任意节点的出场角色列表中被引用，则在设计页禁止删除。
2.  **编辑剧情模板 (草稿机制)**:
    *   以 `mg_active_game_data` 为基底生成草稿并可编辑。
    *   **保存**: 创建者模式且存在 `requestId` 时，会调用 `POST /template/update` 将草稿写回数据库，同时强制刷新本地 `mg_active_game_data`，确保刷新/再次进入设计不会回到旧数据；导入模式(`mg_play_entry=import`)下点击“保存/保存并游玩”会调用 `POST /import` 创建一条新的数据库记录并返回 `requestId`，成功后自动切换为创建者模式（随后保存走 `POST /template/update`），失败则回退为仅本地保存。
    *   **保存并游玩**: 执行与“保存”一致的持久化逻辑后，清理本次游玩状态并跳转 `/game`。
    *   **分享 (仅创建者可见)**: 当且仅当创建者模式且存在 `requestId` 时，工具栏显示“分享/取消分享”按钮；通过 `GET /records/meta/:requestId` 判断当前是否为创建者 (owner) 以及分享状态，通过 `POST /share` 切换分享状态；分享成功会弹出链接弹窗并将 `sharedRecordId` 写入 `mg_record_ids`（供历史记录页使用）；触发“数据安全锁”时禁用分享。
    *   **导入并覆盖**: 设计页提供“导入”按钮，支持粘贴/上传 JSON 覆盖当前草稿；创建者模式下支持“覆盖并保存”将内容写回数据库，并标记该记录 `template_source=import`。
    *   **导出**: 设计页提供“导出”按钮，弹窗展示完整 JSON，并支持复制 / 下载（交互与结局页一致）。导出 JSON 与数据库 `processed_response` 数据结构一致（顶层为 `MovieTemplate`，不包含 `requestId`），并可直接在首页/设计器导入。
    *   **工具栏按钮可读性**: “新增节点 / 保存 / 分享 / 导出”按钮文本不换行；禁用态具备明显的透明度与鼠标样式反馈。
3.  **节点树编辑**:
    *   采用与结局页一致的 SVG `treeGraph` 分层布局展示（可拖拽平移、滚轮缩放、自适应居中）。
    *   **起始节点兼容**: 若存在 `start` 节点但其没有任何选项，且存在节点 `1`，则剧情树以节点 `1` 作为起点；同时将 `start` 视为孤立节点并在“孤立节点”列表中展示原因。
    *   点击节点会在右上角展示节点信息卡片（节点内容摘要 / 选项跳转 / 出场角色），并可一键进入编辑。
    *   支持新增/删除/改名节点，修改节点内容、出场角色、选项及跳转目标（删除节点会弹出站内确认弹窗）。
    *   **选项好感度影响**: 每个选项可配置对某个角色的好感度增减（-20~20），仅允许选择当前节点出场角色，且主角不可被影响。
    *   **节点 ID 展示兼容**: 编辑节点弹窗必须展示节点 ID；若节点对象缺少 `id` 字段，则以节点 Key 自动补全并写回草稿。
    *   **选项操作可用性**: 编辑节点弹窗内新增/删除选项必须可用，且按钮文案必须明确为“新增选项 / 删除选项”；编辑节点弹窗不提供“新增子节点/删除节点”按钮。
    *   **角色展示兼容**: 若节点历史数据以角色 `id` 存储，也会在 UI 中自动解析并显示为角色 `name`。
    *   支持“孤立节点”列表：无入边或从起始节点不可达的节点会被列出，并展示孤立原因与删除入口。
4.  **结局编辑**:
    *   支持新增/删除/改名结局 Key，修改结局类型、描述及可选绑定节点（删除结局会弹出站内确认弹窗）。
    *   结局 Key 变更会同步更新所有引用该 Key 的选项跳转。
5.  **访问控制与入口**:
    *   `mg_play_entry=shared`（分享访问）下禁止进入设计与编辑。
    *   `mg_play_entry=owner`（创建者视角）下，仅当后端元信息 `isOwner=true` 才允许编辑。
    *   `mg_play_entry=import`（导入模式）下允许编辑，但不显示分享入口；同时导入数据会清除 `requestId`，避免被误判为创建者在线数据。
    *   入口：首页“导入并设计”、结局页“进入设计”（仅创建者）、历史记录页“进入设计”（创建者入口）。

---

## 2. 后端接口清单 (Interface Inventory)
**文件**: `server/src/app.rs`, `server/src/handlers.rs`

### 2.1 健康检查 (Health Check)
*   **URL**: `GET /`
*   **功能**: 返回简单的问候信息 ("Hello, World!")，用于验证服务是否存活。

### 2.2 游戏生成 (Generate)
*   **URL**: `POST /generate`
*   **功能**: 根据用户输入生成完整的游戏 JSON 数据。
*   **参数**:
    *   `theme` (String): 主题
    *   `synopsis` (String): 简介
    *   `characters` (List): 角色列表
    *   `mode` (String): 模式 (前端固定发送 `wizard`)
    *   `apiKey`, `baseUrl`, `model`: GLM 配置 (可选)
*   **返回值类型** (TypeScript):
    ```typescript
    interface GenerateResponse {
      code: string;      // "0" 表示成功
      msg: string;       // "success" 或错误信息
      data: {
        id: string;      // 游戏记录 ID (UUID)
        template: MovieTemplate;
      };
    }

    interface MovieTemplate {
      projectId: string;
      title: string;
      version: string;
      owner: string;
      meta: {
        logline: string;
        synopsis: string;
        targetRuntimeMinutes: number;
        genre: string;
        language: string;
      };
      nodes: Record<string, StoryNode>;
      endings: Record<string, Ending>;
      characters: Record<string, Character>;
      backgroundImageBase64?: string;
      requestId?: string;
    }

    interface StoryNode {
      id: string;
      content: string;
      endingKey: string | null;
      level: number | null;
      characters: string[] | null;
      choices: Choice[];
    }

    interface Choice {
      text: string;
      nextNodeId: string;
    }

    interface Ending {
      type: 'good' | 'bad' | 'neutral';
      description: string;
    }

    interface Character {
      id: string;
      name: string;
      gender: string;
      age: number;
      role: string;
      background: string;
      avatarPath?: string;
    }
    ```

    *   **characters 的 key**: 使用角色名 (`name`) 作为 key，而不是 `id`。
    *   **role 和 background**: 不再相同，`role` 保留 AI 生成的值，`background` 仅在为空时使用前端传入的 `description`。

### 2.2.1 剧情导入并保存 (Import)
*   **URL**: `POST /import`
*   **功能**: 接收前端导入的 `MovieTemplate`，进行节点/结局/图结构与好感度等数据清理后保存到数据库（写入 `glm_requests.processed_response`，并标记 `template_source=import`），返回可编辑的 `requestId`。
*   **参数**:
    *   `template` (MovieTemplate): 需要导入的完整剧情模板（必须包含 `nodes`；`endings` 可缺省但最终会标准化为对象）。
    *   `theme` (String, 可选): 首页主题字段（合并进模板：写入 `template.title` 与 `template.meta.logline`，同时随请求记录到 `request_payload`）。
    *   `synopsis` (String, 可选): 首页剧情简介（合并进模板：写入 `template.meta.synopsis`）。
    *   `genre` (String[], 可选): 首页剧情类型多选（合并进模板：写入 `template.meta.genre`，以 ` / ` 拼接）。
    *   `characters` (CharacterInput[], 可选): 首页角色阵容（随请求记录到 `request_payload`，并用于头像兜底处理；若模板缺少角色集合则会用该列表补全）。
    *   `language` (String, 可选): 前端语言（合并进模板：写入 `template.meta.language`）。
*   **返回**:
    *   `id` (UUID): 新生成的记录 ID（前端会写入为 `requestId`）。
    *   `template` (MovieTemplate): 清理后的剧情模板。

### 2.3 生成提示词 (Generate Prompt)
*   **URL**: `POST /generate/prompt`
*   **功能**: 仅生成发送给 LLM 的提示词，不进行实际游戏生成。用于调试或复制提示词。
*   **参数**: 同 `/generate`。

### 2.4 扩写世界观 (Expand Worldview)
*   **URL**: `POST /expand/worldview`
*   **功能**: AI 扩写剧情简介。
*   **参数**: `theme`, `synopsis` (可选基础内容)。

### 2.5 生成角色 (Expand Character)
*   **URL**: `POST /expand/character`
*   **功能**: AI 生成角色列表。
*   **参数**: `theme`, `synopsis`, `current_characters` (现有角色)。

### 2.6 分享状态 (Share)
*   **URL**: `POST /share`
*   **功能**: 切换某个生成记录 (`glm_requests`) 的分享状态，并在分享开启时写入/更新 `shared_records`。
*   **权限**:
    *   仅允许创建该生成记录的请求方 (按 IP 判定) 操作。
    *   仅当生成状态为 `success` 时允许分享。
*   **参数**:
    *   `id` (UUID): 生成记录 ID (`glm_requests.id`)
    *   `shared` (Boolean): 是否分享
*   **返回**:
    *   `sharedRecordId` (UUID | null): 当 `shared=true` 时返回对应 `shared_records.id`；当 `shared=false` 时返回该请求对应的历史记录 ID（若存在），否则为 null。

### 2.7 更新剧情模板 (Update Template)
*   **URL**: `POST /template/update`
*   **功能**: 将某个生成记录 (`glm_requests`) 的 `processed_response` 更新为前端提交的剧情模板（用于设计器“保存/保存并游玩”）。
*   **权限**:
    *   仅允许创建该生成记录的请求方 (按 IP 判定) 操作。
    *   仅当生成状态为 `success` 时允许更新。
*   **参数**:
    *   `id` (UUID): 生成记录 ID (`glm_requests.id`)
    *   `template` (MovieTemplate): 完整剧情模板 JSON
*   **返回**: 更新后的剧情模板 JSON。

### 2.8 删除剧情模板 (Delete Template)
*   **URL**: `POST /template/delete`
*   **功能**: 删除某个生成记录 (`glm_requests`) 及其关联数据。
*   **删除范围**:
    *   `glm_requests` 该 ID 行
    *   `shared_records` 中对应 request_id 的分享记录
    *   `records` 中对应 request_id 的游玩记录
*   **权限**: 仅允许创建该生成记录的请求方 (按 IP 判定) 删除。
*   **参数**:
    *   `id` (UUID): 生成记录 ID (`glm_requests.id`)
*   **返回**: `{ deleted: true }`

### 2.9 获取共享游戏 (Get Shared Game)
*   **URL**: `GET /play/:id`
*   **功能**: 获取指定 ID 的游戏数据。
*   **访问控制**: 若该游戏未分享 (`shared=false`)，则仅创建者可访问；其他用户会返回 NOT_FOUND。
*   **副作用**: 记录访问日志 (IP, User-Agent, Referer)。
*   **返回**: 游戏数据 JSON。

### 2.10 批量获取历史记录列表 (List Records)
*   **URL**: `POST /records`
*   **功能**: 根据 `shared_records.id` 批量返回列表展示所需的轻量字段。
*   **参数**: `ids` (UUID[]): `shared_records.id` 列表（上限 200）。
*   **返回**: `SharedRecordListItem[]`
    *   `id` (UUID): shared_records.id
    *   `requestId` (UUID): glm_requests.id
    *   `title` (String)
    *   `sharedAt` (String)
    *   `shared` (Boolean)
    *   `synopsis` (String)
    *   `genre` (String)
    *   `language` (String)
    *   `playCount` (Number)

### 2.11 获取分享元信息 (Get Shared Record Meta)
*   **URL**: `GET /records/meta/:requestId`
*   **功能**: 查询某个 `glm_requests.id` 对应的分享元信息，用于前端判断是否为创建者以及当前分享状态。
*   **行为**:
    *   若该 `requestId` 存在但从未创建过 `shared_records`（即未分享过），仍会返回元信息：`shared=false`，`sharedRecordId=null`，`sharedAt=null`。
    *   若该 `requestId` 不存在，则返回 `NOT_FOUND`。
*   **返回**:
    *   `sharedRecordId` (UUID | null): 仅当请求方为创建者且存在分享记录时返回真实 ID，否则为 null。
    *   `requestId` (UUID)
    *   `shared` (Boolean)
    *   `sharedAt` (String | null)
    *   `isOwner` (Boolean)

---

## 3. 业务逻辑与差异说明 (Business Logic & Discrepancies)

### 3.1 节点数量控制 (Node Count)
*   **现状**: 前端 Home.vue **没有** 节点数量选择器。
*   **逻辑**: 后端 `server/src/prompt.rs` 中硬编码了节点数量限制：
    ```rust
    // prompt.rs
    const MIN_NODES: usize = 35;
    const MAX_NODES: usize = 45;
    ```
    无论用户在文本中如何要求，Prompt 都会强制要求 LLM 生成 35-45 个节点。

### 3.2 自由模式 (Free Mode)
*   **现状**: 代码逻辑中包含自由模式 (`mode = 'free'`)，允许用户输入 `freeInput`。
*   **UI**: 前端模板中 **未渲染** 自由模式的任何入口，且向导模式表单无条件显示。
*   **结论**: 自由模式代码是死代码 (Dead Code)，用户无法使用。

### 3.3 接口限流与配额
*   **后端配额 (数据库事务 + advisory lock 防并发穿透)**:
    *   `/generate` 全站每日最多写入 60 条 `glm_requests`（按 `created_at > current_date` 统计），超出返回 `SERVICE_BUSY`。
    *   免费额度（仅当未使用用户自带 API Key 时生效）:
        *   同一 IP 同一路由每日最多 30 次，超出返回 `API_KEY_REQUIRED_DAILY_LIMIT`。
        *   同一 IP 同一路由 5 分钟内最多 2 次，超出返回 `API_KEY_REQUIRED`。
    *   `/share`（创建/更新 `shared_records`）:
        *   全站每日最多 20 条分享记录，超出返回 `SERVICE_BUSY`。
        *   同一 IP 每日最多 3 条分享记录，超出返回 `SERVICE_BUSY`。
*   **前端体验**:
    *   对 `API_KEY_REQUIRED` / `API_KEY_REQUIRED_DAILY_LIMIT` / `TOO_MANY_REQUESTS` 等错误会提示用户并引导配置自己的 API Key。
    *   对 `SERVICE_BUSY` 会提示用户“服务繁忙”。

### 3.3.1 敏感词过滤 (Sensitive Content)
*   **覆盖范围**: 后端对所有前端请求 payload 统一执行敏感词过滤（`/generate`、`/import`、`/template/update`、`/share` 等）。
*   **处理方式**:
    *   命中敏感词会被替换为 `*`（按字符逐位替换）。
    *   单次请求中命中敏感内容数量 `> 3` 时，直接拒绝请求并返回错误码 `SENSITIVE_CONTENT`，错误信息为：`该剧情存在不当内容, 已拒绝服务`。
    *   出于安全考虑，会跳过对 `apiKey` / `baseUrl` / `model` / `size` 等字段的过滤。
*   **词库来源**:
    *   环境变量 `SENSITIVE_WORDS`（支持逗号/换行分隔）。
    *   文件 `SENSITIVE_WORDS_PATH`（默认 `./sensitive_words.txt`，支持注释行 `#`）。

### 3.4 节点 ID 归一化 (Node ID Normalization)
*   **目的**: 兼容旧数据/旧 Prompt 输出的 `node_`/`n_` 前缀，同时尽量收敛为“纯数字 key + start”的规范。
*   **逻辑**: 后端在生成后会对 `nodes` 的 key 进行归一化：
    *   `start` / `n_start` → `start`
    *   `node_123` → `123`
    *   `n_123` → `123`
    *   同步重写 `StoryNode.id` 及 `choices.nextNodeId`

### 3.5 分享数据安全 (Share Security)
*   **目标**: 防止非创建者获取 `shared_records.id` 并在历史记录页反向枚举/伪造。
*   **实现**:
    *   `GET /records/meta/:requestId` 对非创建者返回 `sharedRecordId = null`。
    *   `POST /share` 强制校验创建者身份（按 IP 判定），否则返回 FORBIDDEN。

### 3.6 角色好感度系统 (Affinity)
*   **数据结构**: `Choice.affinityEffect` 包含 `characterId` 与 `delta`；当无影响时不输出该字段（不为 null）。
*   **生成约束**: LLM 生成时要求至少 30% 的节点包含 `affinityEffect`（至少一个选项带该字段）。
*   **规则**:
    *   主角不允许被影响（由角色信息推断“默认主角”）。
    *   单次变动幅度限制在 -20 ~ 20。
    *   仅允许影响当前节点出场的角色。
    *   好感度范围强制限制在 0% ~ 100%，默认值为 50%。
*   **前端运行时**:
    *   选择选项时应用好感度变动并写入 `localStorage.mg_affinity_state`。
    *   角色头像表情会参考好感度值做二次推断（主角保持原逻辑）。
    *   重新开始/重新生成/加载新剧本会清空 `mg_affinity_state`。
*   **后端数据校验**:
    *   后端在生成与模板更新流程中会对 `affinityEffect` 做裁剪与清理，保证无效配置不会进入可执行数据。
    *   内置兜底剧情（当缺少 start 节点时自动补齐）的选项默认不携带 `affinityEffect`。

### 3.7 游玩状态持久化 (Play State Persistence)
*   **本地持久化载体**: `localStorage`（核心）、`sessionStorage`（入口标记）。
*   **关键键名**:
    *   `mg_active_game_data`: 当前游玩/设计使用的完整剧情模板。
    *   `mg_current_node`: 当前所在节点 ID。
    *   `mg_player_state`: 玩家状态（用于回退/恢复）。
    *   `mg_history_stack`: 历史栈（用于“返回上一步”）。
    *   `mg_ending`: 当前结局信息（进入结局页展示）。
    *   `mg_affinity_state`: 角色好感度状态（用于表情与结局页展示）。
    *   `mg_play_entry`(sessionStorage): 进入方式标记（owner/shared/import）。

---

## 4. UI 视觉效果 (Visual Effects)

### 4.1 波浪背景 (WavyBackground)
**组件**: `front/src/components/ui/wavy-background/WavyBackground.vue`

**效果**:
*   基于 Simplex Noise 生成动态波浪效果。
*   多层波浪叠加，支持自定义颜色、宽度、模糊度、速度。
*   **运动速度**: 快速模式速度为 `0.002`，慢速模式为 `0.001`。

### 4.2 Vortex 粒子效果
**组件**: `front/src/components/ui/vortex/Vortex.vue`

**效果**:
*   螺旋旋转粒子效果，从外向中心移动。
*   画布尺寸使用对角线 * 2，确保粒子旋转时所有区域都有覆盖。
*   分离 canvas 坐标和 view 坐标，粒子淡出效果基于可视区域计算。
*   粒子时间状态保存到 `localStorage` (`mg_vortex_time`)。

### 4.3 流体光标 (FluidCursor)
**组件**: `front/src/components/ui/fluid-cursor/FluidCursor.vue`

**效果**:
*   基于 WebGL 着色器实现流体模拟。
*   支持鼠标/触摸交互，实时流体动力学模拟。

### 4.4 3D 卡片 (ThreeDCard)
**组件**: `front/src/components/ui/ThreeDCard.vue`

**效果**:
*   CSS 3D 变换实现卡片悬停效果。
*   支持聚光灯 (Spotlight) 鼠标跟随效果。

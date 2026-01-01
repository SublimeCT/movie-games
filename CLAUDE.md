# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

AI 驱动的互动电影游戏生成器。用户输入主题、世界观和角色，系统通过调用智谱 AI (GLM) 大模型生成完整的互动电影游戏。

## 技术架构

### 前端 (front/)
- **Vue 3** + **TypeScript** + **Vite** (使用 rolldown-vite)
- **Vue Router** - 路由：首页(/)、游戏页(/game)、结局页(/ending)
- **Tailwind CSS** - 样式系统
- **@vueuse/core** - 响应式工具（用于 localStorage 状态持久化）
- **@vueuse/motion** - 动画效果
- 组件位于 `front/src/components/`，UI 组件位于 `front/src/components/ui/`

### 后端 (server/)
- **Rust** + **Axum** - 异步 Web 框架
- **SQLx** - 数据库 ORM（PostgreSQL）
- **Reqwest** - HTTP 客户端（调用智谱 API）
- **Tokio** - 异步运行时
- 默认端口：35275

### API 路由
| 路由 | 方法 | 描述 |
|------|------|------|
| `/` | GET | 健康检查 |
| `/generate` | POST | 生成完整游戏（返回 MovieTemplate） |
| `/generate/prompt` | POST | 获取生成的 prompt（不调用 AI） |
| `/expand/worldview` | POST | 扩展世界观/简介 |
| `/expand/character` | POST | 生成角色设定 |

### 数据模型 (共享)
- 前端：[front/src/types/movie.ts](front/src/types/movie.ts)
- 后端：[server/src/types.rs](server/src/types.rs)
- 核心结构：`MovieTemplate` → 包含 `nodes`（故事节点）、`characters`（角色）、`endings`（结局）

**类型兼容性处理**（Rust 端）:
- `String | Vec<String>` → 自动合并为单个字符串
- `Map<String, Character> | Vec<Character>` → 自动转换为 HashMap
- `Option<String> | Option<Vec<String>>` → 统一处理为 `Option<Vec<String>>`

## 常用命令

```bash
# 前端开发
cd front && pnpm dev          # 启动开发服务器 (localhost:18939)
pnpm run dev:frontend         # 从根目录启动前端

# 后端开发
cd server && cargo run        # 启动后端服务器 (localhost:35275)
pnpm run dev:backend          # 从根目录启动后端

# 同时启动前后端
pnpm run dev

# 构建
pnpm run build:frontend       # 构建 front/dist/
pnpm run build:backend        # 构建 server/target/release/
pnpm run build                # 构建全部

# 前端检查
cd front && pnpm check        # Biome lint + TypeScript check
cd front && pnpm test         # 运行 Vitest 测试

# 单个测试文件
cd front && pnpm test <filename>  # 运行特定测试文件

# 部署
pnpm run build:frontend:deploy  # 构建并上传到服务器
pnpm run upload:backend         # 上传后端代码

# 安装依赖
pnpm run install:frontend      # 安装前端依赖
```

## 环境变量

### 后端 (server/.env)
```env
MOVIE_GAMES_DATABASE_URL=postgres://USER:PASSWORD@localhost/DB_NAME
GLM_API_KEY=your-glm-api-key
PORT=35275          # 默认端口
```

### API 调用配置
前端可以通过请求参数覆盖：
- `apiKey` - 自定义智谱 API Key
- `baseUrl` - 自定义 API 端点
- `model` - 使用的模型（默认 glm-4.6v-flash）

### 端口配置
- **前端开发服务器**: 18939（Vite 默认）
- **后端服务器**: 35275（Rust Axum）
- **API 代理**: 开发环境 Vite 将 `/api` → `http://localhost:35275`（去掉 `/api` 前缀）
- **生产环境**: Nginx 反向代理 `/api/` → 后端服务器

## 数据库迁移

迁移文件位于 `server/migrations/`：
- `20240523000000_init.sql` - 初始化表（glm_requests, games）
- `20241222000000_add_user_agent.sql` - 添加 user_agent 字段

应用启动时自动执行迁移（通过 `sqlx::migrate!()`）。

## 前端状态管理

游戏数据通过 `@vueuse/core` 的 `useStorage` 持久化到 localStorage：
- `mg_active_game_data` - 当前游戏数据（MovieTemplate）
- `mg_ending` - 结局数据
- `mg_current_node` - 当前节点 ID
- `mg_player_state` - 玩家状态
- `mg_history_stack` - 历史记录栈（用于返回）

## 开发注意事项

1. **必须使用 pnpm** - 禁止使用 npm/npx
2. **API 代理** - 开发环境 Vite 代理 `/api` → `http://localhost:35275`
3. **生产环境** - Nginx 反向代理 `/api/` → 后端（去掉 /api 前缀）
4. **图像生成** - 使用智谱 CogView API 生成背景和角色头像
5. **图像 fallback** - 生成失败时使用 SVG data URI 作为备选

## 后端代码结构

```
server/src/
├── main.rs       # 入口，启动服务器
├── app.rs        # 路由配置
├── handlers.rs   # API 处理函数（generate, expand_*）
├── glm.rs        # 智谱 API 调用封装
├── images.rs     # 图像生成处理
├── prompt.rs     # Prompt 构造和 JSON 清理
├── template.rs   # 模板转换和规范化
├── db.rs         # 数据库连接和操作
├── api_types.rs  # API 请求/响应类型
└── types.rs      # 核心数据类型（MovieTemplate 等）
```

## 前端代码结构

```
front/src/
├── main.ts           # 入口
├── App.vue           # 根组件，状态管理
├── api.ts            # API 客户端
├── router/index.ts   # 路由配置
├── types/movie.ts    # 类型定义
├── lib/utils.ts      # 工具函数
├── components/
│   ├── Home.vue      # 首页（游戏生成表单）
│   ├── Game.vue      # 游戏页面
│   ├── Ending.vue    # 结局页面
│   └── ui/           # UI 组件（3D 卡片、头像、加载器等）
```

## 图像尺寸规范

支持的尺寸（通过 normalize_cogview_size 转换）：
- `1024x1024` - 默认
- `864x1152` - 竖版
- `1152x864` - 横版

## 核心业务逻辑

### 游戏生成流程
1. **前端收集参数**: Home.vue 收集用户输入（主题、简介、角色），通过 localStorage 传递到 Generating 页
2. **后端调用 AI**: `/generate` 接口调用智谱 GLM API，生成完整的互动电影游戏
3. **数据结构**: 返回 `MovieTemplate`，包含：
   - `nodes`: 故事节点地图（节点 ID → StoryNode）
   - `characters`: 角色地图（角色名 → Character）
   - `endings`: 结局地图（结局 ID → Ending）
   - `meta`: 元信息（类型、时长、语言等）

### 节点数量控制
**硬编码限制**（`server/src/prompt.rs`）:
- `MIN_NODES: 35`
- `MAX_NODES: 45`

无论用户输入如何，Prompt 都会强制要求 LLM 生成 35-45 个节点。

### 角色数据特殊处理
- **Character 的 key**: 使用角色名（`name`）而不是 `id` 作为 HashMap 的 key
- **兼容性**: 支持从旧版数组格式自动转换
- **字段差异**: `role` 和 `background` 不再相同，保留 AI 生成的原始值

### 图像生成与 Fallback
- 使用智谱 CogView API 生成背景和角色头像
- 生成失败时自动使用 SVG data URI 作为备选方案
- 支持尺寸：`1024x1024`（默认）、`864x1152`（竖版）、`1152x864`（横版）

### 状态持久化策略
游戏状态通过 `@vueuse/core` 的 `useStorage` 自动持久化到 localStorage：
- `mg_active_game_data` - MovieTemplate 完整数据
- `mg_current_node` - 当前节点 ID
- `mg_player_state` - 玩家状态对象
- `mg_history_stack` - 历史记录栈（用于"返回上一步"功能）
- `mg_generate_params` - 生成参数（Generating 页使用后立即清除）

### 分享功能机制
1. Ending 页点击分享 → 调用 `/share` 接口 → 获取 UUID
2. 访问链接 `/play/:id` → 调用 `/api/play/:id` → 获取游戏数据
3. Play 页不跳转，直接在当前页面渲染 Game 组件

## 数据库设计

### 表结构
- **glm_requests**: GLM API 调用日志（请求/响应/耗时/错误）
- **games**: 游戏存档（UUID、模板 JSON、发布状态、游玩次数）
- **shared_games**: 分享的游戏数据（UUID 外键）
- **play_records**: 游玩记录（IP、User-Agent、时间戳）

### 迁移机制
- 迁移文件位于 `server/migrations/`
- 应用启动时通过 `sqlx::migrate!()` 自动执行所有未运行的迁移
- 无需手动执行迁移命令

## 重要的代码约定

### Rust 后端
- **类型安全**: 使用 `serde` 进行严格的 JSON 反序列化，支持多种格式兼容（`String | Vec<String>` → String）
- **错误处理**: 统一返回 `GenerateResponse { code, msg, data }` 格式
- **CORS**: 开发环境全开放（`AllowOrigin: Any`），生产环境需配置 Nginx

### Vue 前端
- **路由守卫**: 无需，数据通过 localStorage 传递
- **错误传递**: 通过 `sessionStorage.mg_last_error` 在页面间传递错误信息
- **组件通信**: Game 和 Play 组件使用相同的 localStorage 状态逻辑

### 不可用的功能（死代码）
- **自由模式 (Free Mode)**: 代码存在但 UI 未渲染，用户无法使用
- **节点数量选择器**: 前端无此控件，硬编码为 35-45

## 强制要求
- **每次改动之前必须阅读 `REQUIREMENTS.md`** 了解整体需求, 并且明确告诉我已经阅读完毕
- **每次改动之后必须更新 `REQUIREMENTS.md`**，必须保证 **所有的需求** 都写入此文件，必须使用中文
- **必须使用 pnpm**，禁止使用 npm/npx

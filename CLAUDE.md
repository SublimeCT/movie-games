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

## 常用命令

```bash
# 前端开发
cd front && pnpm dev          # 启动开发服务器 (localhost:5173)
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
PORT=35275
```

### API 调用配置
前端可以通过请求参数覆盖：
- `apiKey` - 自定义智谱 API Key
- `baseUrl` - 自定义 API 端点
- `model` - 使用的模型（默认 glm-4.6v-flash）

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

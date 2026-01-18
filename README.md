# Movie Games

<div align="center">

![Movie Games Banner](https://via.placeholder.com/1200x400?text=Movie+Games)

**AI 驱动的互动剧情游戏生成器**

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Vue](https://img.shields.io/badge/vue-3.x-green.svg)](https://vuejs.org/)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![PostgreSQL](https://img.shields.io/badge/postgresql-14%2B-blue.svg)](https://www.postgresql.org/)

[在线演示](https://movie-games.xiaban.run) | [报告问题](https://github.com/SublimeCT/movie-games/issues) | [贡献指南](#贡献指南)

</div>

## 📖 简介

**Movie Games** 是一个基于 AI 的互动电影游戏生成平台。用户只需输入简单的想法（主题、类型、简介），AI 将自动扩写世界观、生成角色，并构建一个包含分支剧情和多重结局的复杂叙事树。

通过结合 **Vue 3** 的现代化前端交互与 **Rust** 的高性能后端处理，以及大语言模型（GLM-4）的创意生成能力，Movie Games 让每个人都能成为互动电影的导演。

## ✨ 特性

- **🧙‍♂️ 向导模式**: 零门槛创作，通过简单的表单引导，一步步生成完整的游戏剧本。
- **🎲 随机灵感**: 内置随机主题生成器，一键获取创意灵感。
- **🤖 AI 智能扩写**: 
  - 自动扩写剧情大纲，丰富故事细节。
  - 根据剧情自动生成性格鲜明的角色阵容。
- **🎭 沉浸式体验**:
  - 3D 悬停卡片与流体光标效果。
  - 自适应布局，支持 PC 与移动端。
- **🌳 剧情设计器**:
  - 可视化节点树编辑。
  - 自由调整剧情分支与选项。
  - 角色好感度系统。
- **💾 存档与分享**:
  - 支持导入/导出 JSON 存档。
  - 一键分享游戏链接，邀请朋友体验你的作品。
- **🔒 数据安全**: 
  - 敏感词过滤。
  - 本地 API Key 配置支持。

## 🛠️ 技术栈

### 前端 (Front)
- **Framework**: [Vue 3](https://vuejs.org/) (Composition API)
- **Build Tool**: [Vite](https://vitejs.dev/)
- **Language**: [TypeScript](https://www.typescriptlang.org/)
- **Styling**: [Tailwind CSS](https://tailwindcss.com/)
- **Icons**: [Lucide Vue](https://lucide.dev/)

### 后端 (Server)
- **Language**: [Rust](https://www.rust-lang.org/)
- **Web Framework**: [Axum](https://github.com/tokio-rs/axum)
- **Runtime**: [Tokio](https://tokio.rs/)
- **Database**: [PostgreSQL](https://www.postgresql.org/) + [SQLx](https://github.com/launchbadge/sqlx)
- **LLM SDK**: Custom GLM client

## 🚀 快速开始

### 前置要求
- Node.js (>= 18)
- pnpm
- Rust (>= 1.70)
- PostgreSQL (>= 14)

### 1. 数据库设置

本项目使用 `sqlx` 进行数据库迁移。

1. 创建 PostgreSQL 数据库（例如 `movie_games`）。
2. 将 `server/migrations` 目录下的 SQL 文件应用到数据库。
   ```bash
   # 如果安装了 sqlx-cli
   cd server
   export DATABASE_URL=postgres://USER:PASSWORD@localhost/movie_games
   sqlx migrate run
   ```

### 2. 后端启动

1. 进入 `server` 目录：
   ```bash
   cd server
   ```

2. 创建 `.env` 文件：
   ```env
   MOVIE_GAMES_DATABASE_URL=postgres://USER:PASSWORD@localhost/movie_games
   GLM_API_KEY=your-glm-api-key
   PORT=35275
   ```

3. 运行服务器：
   ```bash
   cargo run
   ```

### 3. 前端启动

1. 进入 `front` 目录：
   ```bash
   cd front
   ```

2. 安装依赖：
   ```bash
   pnpm install
   ```

3. 启动开发服务器：
   ```bash
   pnpm dev
   ```

访问 http://localhost:5173 即可开始体验。

## 📦 部署指南

### 前端部署 (Frontend)

1. **构建**:
   ```bash
   cd front
   pnpm build
   ```
   构建完成后，静态文件位于 `front/dist` 目录。

2. **Nginx 配置**:
   将 `front/dist` 目录下的所有文件上传至服务器 Web 根目录（例如 `/var/www/movie-games`）。
   配置 Nginx 反向代理 API 请求到后端端口（默认 35275）。

   示例配置：
   ```nginx
   server {
       listen 80;
       server_name movie-games.example.com;
       root /var/www/movie-games;
       index index.html;

       location / {
           try_files $uri $uri/ /index.html;
       }

       location /api/ {
           proxy_pass http://localhost:35275/;
           proxy_set_header Host $host;
           proxy_set_header X-Real-IP $remote_addr;
           proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
       }
   }
   ```

### 后端部署 (Backend)

1. **构建**:
   ```bash
   cd server
   cargo build --release
   ```
   构建产物为 `server/target/release/server` 二进制文件。

2. **运行环境准备**:
   将编译好的二进制文件上传至服务器（例如 `/srv/movie-games-server/server`）。

   **⚠️ 关键配置**: 敏感词字典
   后端服务启动时需要加载敏感词字典。由于生产环境通常没有 Cargo 缓存，**必须在运行目录下手动创建 `dict` 目录并放入 `dict.txt`**。
   
   您可以将本地开发环境中的 `sensitive-rs` 默认字典上传到服务器：
   - **本地源路径**: `/Users/kuidi/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/sensitive-rs-0.5.0/dict/dict.txt` (版本号可能不同，请根据实际情况调整)
   - **服务器目标路径**: `/srv/movie-games-server/dict/dict.txt`
   
   > 提示：您可以直接编辑 `dict.txt` 文件，根据业务需求增加或删除敏感词。

3. **环境变量**:
   在运行目录下创建 `.env` 文件：
   ```env
   MOVIE_GAMES_DATABASE_URL=postgres://user:password@localhost/movie_games
   GLM_API_KEY=your_key_here
   PORT=35275
   ```

4. **启动服务**:
   ```bash
   ./server
   ```
   推荐使用 `systemd` 或 `supervisor` 进行进程守护。

## 🤝 贡献指南

欢迎提交 Pull Request 或 Issue！

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交改动 (`git commit -m 'Add some AmazingFeature'`)
4. 推送分支 (`git push origin feature/AmazingFeature`)
5. 提交 Pull Request

## 📄 开源协议

本项目采用 [MIT](LICENSE) 协议开源。

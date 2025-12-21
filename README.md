# 职场逃离：17:30的选择

一个基于 Vue 3 + Rust 的 AI 驱动互动电影游戏：在 17:30 的关键时刻做出选择，推动剧情走向不同结局。

## 项目结构

```
movie-games.xiaban.run/
├── front/                 # 前端项目 (Vue 3 + Vite + TypeScript)
│   ├── src/               # 页面与组件
│   ├── index.html         # SEO meta / favicon
│   └── package.json
├── server/                # 后端项目 (Rust + Axum，负责调用大模型)
│   ├── src/
│   │   └── main.rs       # 服务器主文件
│   └── Cargo.toml
└── README.md
```

## 技术栈

### 前端
- **Vue 3** - 响应式框架
- **Vite** - 构建工具
- **TypeScript** - 类型安全
- **Vue Router** - 路由管理
- **Tailwind CSS** - 样式与动效

### 后端
- **Rust** - 系统编程语言
- **Axum** - Web 框架
- **Tokio** - 异步运行时
- **Reqwest** - HTTP 客户端
- **Serde** - 序列化/反序列化

## 安装和运行

### 前置要求
- Node.js (>= 18)
- pnpm
- Rust (>= 1.70)
- PostgreSQL (>= 14)

### 数据库设置

**生产环境 (Axum 最佳实践)**：
本项目使用 `sqlx` 进行数据库迁移。
1. 在数据库中创建用户和数据库（例如 `movie_games`）。
2. 将 `server/migrations` 目录下的 SQL 文件上传到服务器。
3. 手动执行 SQL 文件，或者依赖应用启动时的自动迁移（应用内置了 `sqlx::migrate!()`）。
4. 确保环境变量 `MOVIE_GAMES_DATABASE_URL` 配置正确。

**迁移文件位置**：
`server/migrations/20240523000000_init.sql`

### 后端设置

1. 进入 server 目录：
```bash
cd server
```

2. 创建 `.env` 文件并配置环境变量（必须）：
```bash
# 复制示例文件 (如果有) 或者直接创建
touch .env
```

在 `.env` 中填入以下内容：
```env
# 数据库连接 (请替换 USER, PASSWORD, DB_NAME 为实际值)
# 必须使用 postgres 协议
MOVIE_GAMES_DATABASE_URL=postgres://USER:PASSWORD@localhost/DB_NAME

# 智谱 AI API Key
GLM_API_KEY=your-actual-api-key

# (可选) 自定义端口，默认 35275
PORT=35275
```

3. 运行服务器：
```bash
cargo run
```

服务器默认在 http://0.0.0.0:35275 启动

### 前端设置

1. 进入 front 目录：
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
前端将在 http://localhost:5173 启动。
**注意**：开发环境下，Vite 已配置代理，将 `/api` 开头的请求转发到后端 `http://localhost:35275`。

## 部署指南 (Production)

### 1. 服务器环境准备
- 操作系统：Ubuntu 20.04/22.04 / Debian 11/12
- 安装依赖：
  ```bash
  apt update && apt install -y postgresql nginx supervisor build-essential
  ```
- 安装 Rust (推荐使用 rustup):
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```

### 2. 数据库部署
参考上文“数据库设置”章节。
手动上传并执行 `server/migrations` 目录下的 SQL 文件，或者依赖后端启动时的自动迁移。

### 3. 后端部署

1. **构建 Release 版本** (在本地或 CI/CD 服务器):
   ```bash
   cd server
   cargo build --release
   ```
   产物位于 `target/release/server`。

2. **上传二进制文件** 到服务器 (例如 `/srv/movie-games-server/`)。

3. **配置 systemd 服务**:
   创建 `/etc/systemd/system/movie-games.service`:
   ```ini
   [Unit]
   Description=Movie Games Backend
   After=network.target postgresql.service

   [Service]
   Type=simple
   User=root
   WorkingDirectory=/srv/movie-games-server
   ExecStart=/srv/movie-games-server/server
   Restart=always
   
   # 环境变量配置
   Environment="MOVIE_GAMES_DATABASE_URL=postgres://YOUR_DB_USER:YOUR_SECURE_PASSWORD@localhost/YOUR_DB_NAME"
    Environment="GLM_API_KEY=your_glm_api_key"
    Environment="PORT=35275"

   [Install]
   WantedBy=multi-user.target
   ```
   **注意**：生产环境务必将 `DATABASE_URL` 中的密码替换为实际密码，切勿提交到代码库！

4. **启动服务**:
   ```bash
   systemctl daemon-reload
   systemctl enable movie-games
   systemctl start movie-games
   ```

### 4. 前端部署

1. **构建前端**:
   ```bash
   cd front
   pnpm build
   ```
   产物位于 `dist` 目录。

2. **上传静态文件** 到服务器 (例如 `/srv/movie-games-front/`)。

### 5. Nginx 配置 (反向代理)

配置 Nginx 以通过域名访问，并处理 `/api` 转发。

编辑 `/etc/nginx/sites-available/movie-games` (示例):

```nginx
server {
    listen 80;
    server_name your-domain.com;  # 替换为你的域名

    root /srv/movie-games-front;
    index index.html;

    # 前端静态文件
    location / {
        try_files $uri $uri/ /index.html;
    }

    # 后端 API 转发
    # 前端请求 /api/generate -> Nginx -> 后端 /generate
    location /api/ {
        # 去掉 /api 前缀
        rewrite ^/api/(.*) /$1 break;
        
        proxy_pass http://127.0.0.1:35275;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_cache_bypass $http_upgrade;
        
        # 传递真实 IP
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    }
}
```

启用配置并重启 Nginx:
```bash
ln -s /etc/nginx/sites-available/movie-games /etc/nginx/sites-enabled/
nginx -t
systemctl restart nginx
```

## License

MIT

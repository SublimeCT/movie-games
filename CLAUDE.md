# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

### 核心概念
- **多智能体制作系统**：使用 Claude AI 代理作为专业团队成员协作
- **可复现性**：所有产物都包含 provenance（来源追踪）信息
- **结构化工作流**：基于 [prompts/WORKFLOW.md](prompts/WORKFLOW.md) 的机器可读流程
- **工件驱动开发**：专注于生成和管理结构化 artifact

## 项目结构

```
xiaban.run/
├── Storyline.md              # 游戏完整故事线和角色设定
├── prompts.md                # 项目详细需求和角色背景
├── .claude/agents/           # AI 代理角色定义
│   ├── director.md          # 导演角色
│   ├── cinematographer.md    # 摄影指导角色
│   ├── producer.md          # 制片人角色
│   ├── screenwriter.md      # 编剧角色
│   └── ...                  # 其他角色
├── prompts/                  # 制作工作流产物
│   ├── WORKFLOW.md          # 多代理协作工作流
│   ├── movie.json           # 电影结构化数据
│   ├── movie.template.json  # 数据模板
│   ├── owner_script.md      # Owner提供的剧本
│   └── types/               # TypeScript类型定义
│       └── movie.ts         # 数据模型接口
└── output/                   # 生成输出
    └── movie.json           # 最终生成的电影数据
```

## 开发工作流

### 1. 多代理协作流程
基于 [WORKFLOW.md](prompts/WORKFLOW.md) 的标准流程：

1. **STEP 0** - Owner 提供剧本 (`prompts/owner_script.md`)
2. **STEP 1** - 构建故事树 (NarrativeAgent + DirectorAgent)
3. **STEP 2** - 生成镜头计划 (StoryBoardAgent + CinematographyAgent + ProductionDesignAgent)
4. **STEP 3** - 生成低分辨率 Proof (VideoGenAgent)
5. **STEP 4** - 迭代修正 (QCAgent + 各负责Agent)
6. **STEP 5** - 输出最终 movie.json

### 2. 工件（Artifact）管理
每个工件必须包含：
```json
{
  "artifactId": "proj1_S01_SH01_v001",
  "type": "video|audio|image|prompt|shotplan|edl|qc",
  "originAgent": "VideoGenAgent",
  "createdAt": "ISO8601",
  "payload": {
    "prompt": "...",
    "model": "model-name",
    "seed": 12345
  },
  "provenance": {
    "directorShotRef": "S01_SH01",
    "scriptRef": "script_v3_page_12"
  },
  "status": "draft|testing|approved|needs_fix"
}
```

### 3. 核心原则
- **Owner-first**：owner_script.md 是唯一创作源，禁止修改
- **不可瞎编**：不得新增剧情、角色、结局或节点
- **可溯源**：所有生成物必须关联 script_ref 和 source_paragraph_ref
- **可校验**：所有中间产物必须结构化（JSON）
- **可审计**：iterationLog 记录完整的修改历史

## 数据模型

使用 [prompts/types/movie.ts](prompts/types/movie.ts) 定义的 TypeScript 接口：

- **MovieTemplate**：项目顶层结构
- **MetaInfo**：元信息（logline、synopsis等）
- **GlobalSettings**：全局渲染与音频设置
- **PlayerState**：玩家状态（flags + variables）
- **StoryNode**：故事节点定义
- **Choice**：玩家选择及其效果
- **Character**：角色定义
- **ShotEntry**：镜头定义
- **ArtifactRecord**：工件记录
- **IterationLogEntry**：迭代日志

## 重要角色设定

- **玩家**：28岁外包程序员
- **张总**：45岁老板，信奉"酒桌即生产力"
- **王哥**：50岁甲方，炸鸡连锁店老板
- **小美**：24岁UI设计师，提供情绪价值
- **小强**：26岁卷王同事
- **小雅**：27岁女友，等待陪伴

## 开发规范

### 1. 文件命名
- 镜头ID格式：`S{场景号}_SH{镜头号}`
- 工件ID格式：`{projectId}_{shotId}_v{版本号}`
- 节点ID格式：在 movie.json 中保持唯一性

### 2. 状态管理
- `draft`：草稿
- `testing`：测试中
- `approved`：已批准
- `needs_fix`：需要修复

### 3. 质量标准
- 视频时长与预估误差 ≤ 0.5秒
- 必须包含 prompt、model、seed
- 音频采样率必须匹配全局设置
- 最大迭代次数：每个镜头 4 轮

## 常用命令

当前项目处于**前期策划阶段**，主要操作：

```bash
# 查看项目故事线
cat Storyline.md

# 查看工作流定义
cat prompts/WORKFLOW.md

# 查看数据模型
cat prompts/types/movie.ts

# 查看 Owner 剧本
cat prompts/owner_script.md

# 查看生成的电影数据
cat output/movie.json

# 列出所有 AI 代理角色
ls .claude/agents/

# 查看特定代理定义
cat .claude/agents/director.md
```

## Agent 职责说明

| Agent | 职责 | 限制 |
|-------|------|------|
| DirectorAgent | 解释剧本，规划整体结构 | 不可修改 owner_script.md |
| NarrativeAgent | 解析为 nodes/choices/effects | 不可新增主线结局 |
| StoryBoardAgent | 生成 Shot 计划 | 必须引用 source_paragraph_ref |
| CinematographyAgent | 镜头设计 | 仅操作 shotplan 数据 |
| ProductionDesignAgent | 场景/道具/服装设计 | 不得修改 narrative 文本 |
| SoundAgent | 对白、环境音、配乐 | 仅操作 audioSpec 字段 |
| VideoGenAgent | 整合 prompt，生成视频 | 不可修改 narrative 或 shots |
| EditorAgent | 组装 timeline/EDL | 仅操作 artifact 排序 |
| VFXAgent | 修复/合成镜头 | 仅操作 artifact payload |
| QCAgent | 技术QC、模型许可校验 | 检查 artifact payload |
| DataAgent | artifact/provenance 管理 | 确保追溯信息完整 |

## 特色架构

1. **Agent-Based Architecture**：每个代理职责明确，接口清晰
2. **Artifact-Driven Development**：专注工件生成而非传统编码
3. **Provenance Tracking**：所有工件都可追溯到来源
4. **QC-First Approach**：每个步骤都内建质量控制
5. **Machine-Readable Workflow**：工作流程完全机器可读

## 注意事项

- 这是一个创新的电影游戏开发方式，使用 AI 代理协作
- 项目目前处于概念和规划阶段，尚未进入实现阶段
- 所有工作都遵循可复现性和可审计性原则
- Owner（人类投资者）保留最终决策权
- 禁止生成任何中间文件，所有数据仅保存在 movie.json 内

## 重要提示
- 必须使用中文
- 必须使用 `pnpm`, 禁止使用 `npm` / `npx`

```log
27.214.33.217 - - [15/Dec/2025:21:12:26 +0800] "GET /static-videos/jining-change-face.mp4 HTTP/1.1" 206 8257536 "https://blog.xiaban.run/posts/2025/2025-jining/" "Mozilla/5.0 (iPhone; CPU iPhone OS 18_7 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.0.1 Mobile/15E148 Safari/604.1"
112.7.103.119 - - [15/Dec/2025:21:27:12 +0800] "GET /_astro/jining-xuzhou-mesuem4.Ce7RF0ga_Z2i59sq.webp HTTP/1.1" 200 90406 "https://blog.xiaban.run/posts/2025/2025-jining/" "Mozilla/5.0 (Linux; Android 16; PKC130 Build/BP2A.250605.015; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/142.0.7444.172 Mobile Safari/537.36 XWEB/1420045 MMWEBSDK/20251006 MMWEBID/6945 MicroMessenger/8.0.66.2980(0x28004236) WeChat/arm64 Weixin NetType/WIFI Language/zh_CN ABI/arm64"
```
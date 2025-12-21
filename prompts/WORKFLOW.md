# WORKFLOW.md
单人出品 · 多 Agent · 互动电影类游戏工作流（Machine-Readable / Owner-First）

## 一、核心原则

- **Owner-first**：`prompts/input/owner_script.md` 是唯一创作源，LLM 只能读取引用，禁止修改。
- **不可瞎编**：任何 Agent 都不得新增剧情、角色、结局或节点。
- **可溯源**：所有生成物必须关联 `script_ref` 和 `source_paragraph_ref`。
- **可校验**：所有中间产物必须结构化（JSON），可通过 Orchestrator 验证。
- **可审计**：iterationLog 必须记录 actor、timestamp、action、reason、diff。

## 二、输入与输出文件路径

- **输入**：`prompts/input/owner_script.md`（Owner 编写，可任意内容格式）
- **输出**：`prompts/output/movie.json`（唯一生成文件, 类型定义参考 `prompts/types/movie.ts`）
- **禁止生成任何中间文件**，所有数据和 artifact 仅保存在 movie.json 内

## 三、Agent 角色与职责

| Agent | 职责 | 限制 |
|-------|------|------|
| DirectorAgent | 解释 owner_script.md，规划整体分支结构与节点顺序 | 不可修改 owner_script.md |
| NarrativeAgent | 将 owner_script.md 解析为 nodes/choices/effects 数据 | 不可新增主线结局，严格对应 owner_script.md |
| StoryBoardAgent | 为每个 Node 生成 Shot 计划（shots） | Shot 必须引用 source_paragraph_ref |
| CinematographyAgent | 镜头设计（构图/运动/景别/光线） | 仅操作 shotplan 数据 |
| ProductionDesignAgent | 场景/道具/服装 moodboard 与 assets | 不得修改 narrative 文本 |
| SoundAgent | 对白、环境音、配乐提示 | 仅操作 audioSpec 字段 |
| VideoGenAgent | 整合视觉/音频 prompt，生成低分辨率 proof artifact | 不可修改 narrative 或 shots |
| EditorAgent | 组装 timeline/EDL 数据 | 仅操作 artifact 排序与时间线 |
| VFXAgent | 修复/合成镜头 | 仅操作 artifact payload，不修改 narrative |
| QCAgent | 技术 QC、模型许可校验 | 检查 artifact payload 与全局 settings |
| DataAgent | artifact/provenance 管理 | 确保每个 artifact 记录 sourceNode/shotRef/scriptRef |

## 四、工作流步骤

### STEP 0 - Owner 提供剧本
- 输入：`prompts/input/owner_script.md`
- 任务：
  1. Owner 编写完整剧情文本，可包含分支说明、角色、事件。
  2. Agent 仅读取文本，不修改。
- 输出：原始剧本文本供 LLM 解析引用。
- 注意：
  - 不限制文本格式。
  - 所有 narrative 树必须来自该文件。

### STEP 1 - 构建故事树
- OwnerAgent: NarrativeAgent 协同 DirectorAgent
- 输入：owner_script.md
- 输出：`nodes` JSON 数据（含 Node/choices/effects）
- 规范：
  - NodeId 唯一，与 JSON key 一致。
  - Choice 中 effects 严格对应 initialState 字段。
  - 所有 branching 节点必须可映射到玩家选择。
- QC：
  - DirectorAgent 审核 nodes 顺序和逻辑一致性。
  - Owner 拥有最终修改权。

### STEP 2 - 生成镜头计划
- OwnerAgent: StoryBoardAgent 协同 CinematographyAgent + ProductionDesignAgent
- 输入：`nodes` JSON
- 输出：shots 数组，每个 shot 包含 shotId、description、visualIntent、cameraSpec、audioSpec
- QC：
  - 每个 Node 至少一组 shots。
  - 记录 requiredAgents。
  - 所有视觉/音频参考必须关联 source_paragraph_ref。

### STEP 3 - Prompt 与低分辨率 Proof
- OwnerAgent: VideoGenAgent
- 输入：shots + assets + model 配置
- 输出：artifact（低分辨率视频 proof + payload metadata）
- QC：QCAgent
  - 检查 duration、frame rate、audio sample rate。
  - 检查 prompt payload 含 model/seed/license。

### STEP 4 - Iteration / 修正
- OwnerAgent: QCAgent 触发
- 规则：
  - max_rounds_per_node 默认 4
  - QCAgent fail → 分派回原 Agent 创建 FixTask
  - Agent 冲突 → weighted_vote，Owner 可 override
  - partial-merge 支持多个 Agent 输出补充字段合并

### STEP 5 - 输出 movie.json
- OwnerAgent: NarrativeAgent + DirectorAgent 协同 VideoGenAgent
- 输出：`prompts/output/movie.json`
  - 包含 nodes、shots、choices、effects、ending、artifacts、iterationLog、assets、provenance
- 注意：
  - 严格对应 TS 类型 `prompts/types/movie.ts`
  - 禁止生成任何中间文件
  - 所有 artifact 必须记录 provenance

## 五、分支与玩家选择规范

- 每个 Node 中 choice 必须至少包含：
  - id, text, nextNodeId, effects, conditions（可选）
- 所有 effects 必须记录被修改 key 与类型（set/increment/decrement）
- GameState 用于驱动可见条件、后续分支逻辑
- 结局 Node 的 ending 字段说明 type 与 description

## 六、Artifact & Provenance

- 所有 artifact payload 必须包含：
  - prompt, model, seed, params
- provenance 必须包含 originAgent、createdAt、sourceNode/shotRef、scriptRef
- DataAgent 保证可索引、回溯与调试

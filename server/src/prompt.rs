
use crate::api_types::GenerateRequest;

pub(crate) fn clean_json(s: &str) -> String {
    let s = s.trim();
    if s.starts_with("```json") {
        s.trim_start_matches("```json")
            .trim_end_matches("```")
            .trim()
            .to_string()
    } else if s.starts_with("```") {
        s.trim_start_matches("```")
            .trim_end_matches("```")
            .trim()
            .to_string()
    } else {
        s.to_string()
    }
}

pub(crate) fn construct_prompt(req: &GenerateRequest) -> String {
    let topic = req
        .theme
        .as_deref()
        .or(req.free_input.as_deref())
        .unwrap_or("Unknown Theme");
    let language_tag = req.language.as_deref().unwrap_or("zh-CN");
    let language_label = if language_tag.to_lowercase().starts_with("zh") {
        "简体中文".to_string()
    } else if language_tag.to_lowercase().starts_with("en") {
        "English".to_string()
    } else {
        language_tag.to_string()
    };

    let types_def = r#"
export interface MovieTemplate {
  projectId: string
  title: string
  version: string
  owner: string
  meta: MetaInfo
  backgroundImageBase64?: string
  nodes: Record<string, StoryNode>
  endings?: Record<string, Ending>
  characters: Record<string, Character>
  provenance: Provenance
}

export interface MetaInfo {
  logline: string
  synopsis: string
  targetRuntimeMinutes: number
  genre: string
  language: string
}

export interface Character {
  id: string
  name: string
  gender?: string
  age: number
  role: string
  background: string
  avatarPath?: string
}

export interface StoryNode {
  id: string
  endingKey?: string
  content: NodeContent
  characters?: string[]
  choices: Choice[]
}

export interface NodeContent {
  text: string
  notes?: string
}

export interface Choice {
  text: string
  nextNodeId: string
}

export interface Ending {
  type: 'good' | 'neutral' | 'bad'
  description: string
  endingKey?: string
  nodeId?: string
  reachedAt?: string
}

export interface Provenance {
  createdBy: string
  createdAt: string
}
"#;

    let characters_json = req
        .characters
        .as_ref()
        .and_then(|cs| serde_json::to_string_pretty(cs).ok())
        .unwrap_or_else(|| "[]".to_string());

    let protagonist_name = req
        .characters
        .as_ref()
        .and_then(|cs| cs.iter().find(|c| c.is_main).or_else(|| cs.first()))
        .map(|c| c.name.clone())
        .unwrap_or_else(|| "主角".to_string());

    format!(
        r#"
# 角色定义
你是一位享誉全球的互动电影游戏编剧和总导演。你擅长创作引人入胜、逻辑严密且充满情感冲击力的多分支剧情。
你的任务是根据用户提供的主题，创作一个完整的互动电影剧本，并将其直接输出为符合 TypeScript 接口定义的 JSON 格式。

# 用户输入主题
\"{}\"

# 核心要求 (必须严格遵守)
1. **第一人称叙事**：所有的 `node.content.text` 必须使用**第一人称 (\"我\")** 进行叙述。玩家就是主角，代入感必须极强。
2. **禁止循环引用**：剧情节点之间严禁出现死循环。必须确保所有分支最终都能导向结局。
3. **字数限制**：每个节点的 `text` (AI 智能扩写) 字数必须控制在 **200 到 250 字**之间。不能太短，也不能太长。
4. **剧情深度**：这是一款高质量的互动电影游戏。剧情必须跌宕起伏，人物性格必须鲜明。
   - **拒绝流水账**。
   - **拒绝平铺直叙**。
   - **拒绝假大空**。
   - **必须像真正的电影剧本一样真实、细腻**。
   - **禁止任何无意义的故事情节**。
   - **禁止重复啰嗦的废话**。
5. **JSON 结构**：
    - 严格遵循下方的 `TypeScript` 类型定义。
    - **结局分离**：所有的结局节点必须定义在顶层的 `endings` 字段中，而不是混在 `nodes` 里。
    - `StoryNode` 中的 `choices` 里的 `nextNodeId` 如果指向结局，必须指向 `endings` 中的 key。
    - `nodes` 的 Key 必须严格遵循 `n_` 开头的格式 (例如: `n_start`, `n_1`, `n_conflict`)，严禁使用 `node_` 开头。
6. **语言**：所有剧情内容必须使用 **{}**。
7. **数量硬约束（必须满足，否则输出视为错误）**：
    - `nodes` 的数量必须 **严格在 40 到 60 之间（含 40/60）**。
    - `endings` 的数量必须 **3 到 5**。
8. **节点选项比例硬约束（必须满足，否则输出视为错误）**：
    - 只有一个选项的节点应该少于 20%。
    - 两个选项的节点应该少于 50%。
    - 三个及以上选项的节点至少占比 60%。
    - 只有少于 20% 的节点存在指向相同节点的选项。
9. **角色一致性 (极其重要)**：
    - 下面是用户提供的角色清单。你必须把这些角色写入顶层 `characters`。
    - **严禁创造新角色**：你只能使用清单里提供的角色。
    - **严禁修改角色**：name / gender / isMain 必须与清单严格一致。
    - **严禁改名**：不允许将角色改名为“玩家”、“主角”、“我”或其他名字。必须使用清单中的原名。
    - 主角姓名必须严格等于：\"{}\"。
    - 每个 `StoryNode.characters` 必须列出该场景出现的 1~3 个角色名字（来自 `characters`），并且必须在整部剧里出现大量双人/三人同场景，禁止每个节点都只有一个人。
10. **无环硬约束 (必须满足，否则输出视为错误)**：
    - 你必须保证剧情图是 **有向无环图 (DAG)**。
    - 任何 `choices.nextNodeId` 都 **不允许** 指向自身节点。
    - 任何 `choices.nextNodeId` 都 **不允许** 指向“之前的节点”。为此你必须将所有节点 key 设计为严格递增的编号：`n_01`...`n_60`（起始节点固定为 `n_start`，它等价于 `n_01`）。
    - 规则：每个节点的 `choices.nextNodeId` 只能指向更大的编号节点（例如 `n_07` 只能跳到 `n_08` 及之后）或 `endings` 的 key。
11. **禁止重复节点 (必须满足，否则输出视为错误)**：
    - 任意两个节点 **绝对禁止** 出现 **完全相同的** `content.text`。
    - 任意两个节点 **绝对禁止** 出现 **完全相同的** 选项集合（`choices.text` + `choices.nextNodeId` 逐项一致）。
12. **结局节点标识 (必须提供)** ：
    - 除了通过 `choices.nextNodeId` 指向 `endings` 外，你必须支持“走到某个节点即结束”的情况：此类节点必须设置 `endingKey`（值必须是 `endings` 中的 key）。
    - 当节点设置了 `endingKey` 时，该节点的 `choices` 必须为空数组。
    - 任何 `choices` 为空数组的节点，必须设置 `endingKey`，否则输出视为错误。
    - 你必须确保至少存在 3 个带有 `endingKey` 的节点（每个结局至少一个入口节点）。
13. **禁止无效引用 (必须满足，否则输出视为错误)**：
    - 所有 `choices.nextNodeId` 必须引用一个真实存在的目标：要么是 `nodes` 中存在的节点 key，要么是 `endings` 中存在的 key。
    - 所有 `endingKey` 必须是 `endings` 中存在的 key。
    - 严禁出现引用不存在的节点/结局（例如写了 `n_12` 但 `nodes` 里没有 `n_12`）。
    - 输出前必须自检：遍历所有节点与选项，确保每一个引用都能在 JSON 内部找到。

# 用户提供的角色清单 (JSON)
{}

# TypeScript 类型定义 (Schema)
```typescript
{}
```

# 输出规则
- 输出必须是 **纯 JSON** 文本。
- **不要** 包含 markdown 代码块标记 (如 ```json ... ```)。
- **不要** 包含任何解释性文字。
- 确保所有字段类型正确，不要遗漏必填项。
- `projectId` 使用 UUID 或随机字符串。
- `nodes` 必须在 40~60 个之间。
- `endings` 必须在 3~5 个之间。
- `endings` 的 key 必须使用：`ending_good` / `ending_neutral` / `ending_bad`（如果有额外结局，必须保持同样的 `ending_*` 风格）。
- 确保 `n_start` (作为起始节点) 存在于 `nodes` 中。

开始创作！
"#,
        topic, language_label, protagonist_name, characters_json, types_def
    )
}

use crate::api_types::GenerateRequest;

pub(crate) fn clean_json(s: &str) -> String {
    let s = s.trim();
    let raw = if s.starts_with("```json") {
        s.trim_start_matches("```json")
            .trim_end_matches("```")
            .trim()
    } else if s.starts_with("```") {
        s.trim_start_matches("```").trim_end_matches("```").trim()
    } else {
        s
    };

    let mut output = String::with_capacity(raw.len());
    let mut in_string = false;
    let mut chars = raw.chars();

    while let Some(c) = chars.next() {
        if in_string {
            match c {
                '\\' => {
                    output.push('\\');
                    if let Some(next_c) = chars.next() {
                        output.push(next_c);
                    }
                }
                '"' => {
                    output.push('"');
                    in_string = false;
                }
                '\n' => output.push_str("\\n"),
                '\r' => output.push_str("\\r"),
                '\t' => output.push_str("\\t"),
                c if c.is_control() => {
                    // Skip other control characters to avoid parse errors
                }
                _ => output.push(c),
            }
        } else {
            if c == '"' {
                in_string = true;
            }
            output.push(c);
        }
    }
    output
}

pub(crate) fn construct_prompt(req: &GenerateRequest) -> String {
    let topic = req
        .theme
        .as_deref()
        .or(req.free_input.as_deref())
        .unwrap_or("Unknown Theme");

    let synopsis = req.synopsis.as_deref().unwrap_or("");
    let full_topic = if !synopsis.is_empty() {
        format!("Theme/Genre: {}\nSynopsis: {}", topic, synopsis)
    } else {
        format!("Theme/Genre: {}", topic)
    };

    let language_tag = req.language.as_deref().unwrap_or("zh-CN");
    let language_label = if language_tag.to_lowercase().starts_with("zh") {
        "简体中文".to_string()
    } else if language_tag.to_lowercase().starts_with("en") {
        "English".to_string()
    } else {
        language_tag.to_string()
    };

    let types_def = r#"interface MovieTemplate {
  title: string
  backgroundImageBase64?: string
  nodes: Record<string, StoryNode>
  characters: Record<string, Character>
}
interface Character {
  id: string
  name: string
  gender?: string
  age: number
  role: string
  background: string
  avatarPath?: string
}
interface StoryNode {
  content: string
  level?: number
  characters?: string[]
  choices: Choice[]
}
interface Choice {
  text: string
  nextNodeId: string // 指向 nodes 的 key 或 endings 的 key
}
interface Ending {
  type: 'good' | 'neutral' | 'bad'
  description: string
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
        r#"# 角色定义
你是一位享誉全球的互动电影游戏编剧和总导演。你擅长创作引人入胜、逻辑严密且充满情感冲击力的多分支剧情。
你的任务是根据用户提供的主题，创作一个完整的互动电影剧本，并将其直接输出为符合 TypeScript 接口定义的 JSON 格式。

# 用户输入主题
"{}"

# 一、核心叙事与风格要求
- 第一人称沉浸式叙事：所有的 `node.content` 必须使用 **第一人称 ("我")** 进行叙述。玩家就是主角，代入感必须极强。
- 剧情深度与质量：
    - 拒绝流水账、拒绝平铺直叙、拒绝假大空。
    - 必须具备电影剧本般的 **真实感、细腻度与情感张力**。
    - 严禁任何无意义的故事情节或重复啰嗦的废话。
- 语言指定：所有剧情内容必须使用 **{}** 撰写。

# 二、基础结构与格式约束
- JSON 结构规范：
    - 严格遵循下方的 `TypeScript` 类型定义。
    - 禁止返回 `meta` / `projectId` / `nodes[].id` / `version` / `owner` / `provenance` 等字段。
    - 结局分离：所有的结局节点必须定义在顶层的 `endings` 字段中。
    - ID 格式：
        - `nodes` 的 Key 必须是 **纯数字字符串** (例如 "1", "2", "3"...)。
        - **绝对禁止** 使用 `n_` 前缀 (如 `n_1`, `n_start` 等都是错误的)。
        - 唯一例外：起始节点的 Key 必须固定为 **"start"**。
    - 结局引用：`StoryNode` 中的 `choices` 若指向结局，必须引用 `endings` 中的 key。

# 三、数值硬性约束 (校验失败将视为错误)
- 节点总数：`nodes` 的数量必须在 **35 到 45** 之间 (含 35/45)。
- 结局数量：`endings` 的数量必须在 **4 到 6** 之间。
- 单节点字数：每个节点的 `content` (AI 智能扩写) 字数必须严格控制在 **45 到 85 字** 之间。
- 路径深度：必须保证所有的故事线都经过 **至少 12 个节点**。

# 四、Nodes 结构与逻辑约束 (重点)

## 1. 图结构与流程
- DAG 无环结构：剧情必须构成 **有向无环图 (DAG)**。严禁任何形式的死循环。
- ID 递增原则：所有节点 key (除了 "start") 必须是严格递增的数字。
    - `choices.nextNodeId` 只能指向 **数字更大** 的节点或 `endings`。
    - "start" 节点被视为 0，可以指向任何数字节点。
    - **严禁回退，严禁指向自身**。

## 2. Level (层级) 控制
- 起始层级：`start` 节点的 `level` 必须为 **1**。
- 层级递进：后续节点的 `level` 必须大于当前节点的 level (通常 +1)。
- 层级宽度：每个 level 下最多只能存在 **5 个节点**。
- 层级分布：
    - 原则上每个 level 至少 2 个节点。
    - 允许收束：必须允许 **至少 15%** 的 level 只有 1 个节点 (剧情收束点)。
- 结局一致性：所有结局 (`endings`) 视为处于同一个最终 Level。

## 3. 节点复用与收束 (关键)
- 多对一结构：并不是每个节点的选项都必须指向全新的节点！
- 必须复用：多个节点可以指向同一个下一级节点。务必设计 **“多对一”** 的路径以减少节点浪费。

## 4. 选项与分支
- 去重：任意两个节点 **绝对禁止** 出现完全相同的 `content` 或 选项集合。
- 引用有效性：所有 `nextNodeId` 必须引用真实存在的 Key (在 `nodes` 或 `endings` 中)。
- 选项分布：
    - 1 个选项的节点：**< 20%**
    - 2 个选项的节点：**< 50%**
    - 3+ 个选项的节点：**>= 60%**

# 五、角色与互动约束
- 非空约束：每个节点必须至少包含 **1 个角色** (严禁 0 角色)。
- 多人互动：绝大多数节点必须包含 **至少 2 个角色**。单人独白节点 < 10%。
- 角色一致性：
    - 必须使用列表中的角色，严禁改名、创造新角色。
    - 主角姓名必须为：**"{}"**。
    - 必须在 `characters` 字段中正确引用。

# 六、结局触发机制
- 灵活结局：`endings` 的 Key 不再固定，可以根据剧情自由命名 (如 `ending_hero`, `ending_regret` 等)。
- 结局描述：每个结局的 `description` 长度不能超过 **40 个字**。
- 快速通道：**必须包含一个可以快速到达的结局路径**。
    - 例如：从 Start -> 节点 3 -> 节点 5 -> (选择某选项) -> 直接到达结局。
    - 也就是说，在较早的层级 (如 Level 3-5) 就允许通过特定选项直接进入结局。
- 互斥规则：
    - `nodes` 中的节点 **不允许** 包含 `endingKey` 属性。
    - 结局只能通过 `choices.nextNodeId` 指向 `endings` 的 Key 来触发。

# 用户提供的角色清单 (JSON)
{}
# TypeScript 类型定义 (Schema)
```typescript
{}
```
# 输出规则
- 输出必须是 **纯 JSON** 文本。
- **不要** 包含 markdown 代码块标记。
- `nodes` 数量：**35~45**。
- `endings` 数量：**4~6**。
- 必须包含 `start` 节点。
开始创作！
"#,
        full_topic, language_label, protagonist_name, characters_json, types_def
    )
}

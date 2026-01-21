use crate::api_types::{ExpandCharacterRequest, ExpandWorldviewRequest, GenerateRequest};
use crate::types::{BluePrintAct, LDAGNode};

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

// Old prompt function, kept for reference or legacy
pub(crate) fn construct_prompt(req: &GenerateRequest) -> String {
    // Use default values for level_count and act_count for legacy prompt generation
    construct_blueprint_prompt(req, 40, 3)
}



// ... existing expand prompts ...
pub(crate) fn construct_expand_worldview_prompt(req: &ExpandWorldviewRequest) -> String {
    // ... (Keep existing implementation)
    let language = req.language.as_deref().unwrap_or("zh-CN");
    if let Some(synopsis) = req.synopsis.as_ref().filter(|s| !s.trim().is_empty()) {
        format!(
            "你是一名资深电影编剧。
请根据以下故事大纲，将其扩写为一个更加丰富、细节饱满、逻辑严密的电影故事梗概（Synopsis）。

原大纲：
{}

要求：
1. 保持原有的核心冲突和人物关系。
2. 增加环境描写、情感细节和情节转折，剧情必须像真正的电影一样，有起伏，有转折。
3. 明确故事的起承转合（开端、发展、高潮、结局）。
4. 篇幅在 600-800 字之间，尽可能详细地描述剧情，字数不能少于 600 字。
5. 语言风格要符合【{}】题材的调性。

# 语言要求
输出语言：{}。

请直接输出扩写后的文本，不要包含任何前言后语。",
            synopsis, req.theme, language
        )
    } else {
        format!(
            "你是一名资深电影编剧。
请为一个【{}】题材的电影，创作一个精彩、引人入胜的电影故事梗概（Synopsis）。

要求：
1. 包含核心冲突、主要人物和关键情节。
2. 明确故事的起承转合（开端、发展、高潮、结局），剧情必须像真正的电影一样，有起伏，有转折。
3. 篇幅在 600-800 字之间，尽可能详细的描述剧情，字数不能少于 600 字。

# 语言要求
输出语言：{}。

请直接输出创作的文本，不要包含任何前言后语。",
            req.theme, language
        )
    }
}

pub(crate) fn construct_expand_character_prompt(req: &ExpandCharacterRequest) -> String {
    // ... (Keep existing implementation)
    let language = req.language.as_deref().unwrap_or("zh-CN");
    // Use worldview as the synopsis source since frontend sends it in 'worldview' field
    let synopsis_content = if !req.worldview.is_empty() {
        Some(req.worldview.as_str())
    } else {
        req.synopsis.as_deref()
    };

    if let Some(synopsis) = synopsis_content {
        format!(
            "你是一名资深电影编剧。

请为一部【{}】电影，基于以下故事大纲，生成一个完整、立体、真实可信的角色设定。

故事大纲：
{}

要求：
1. 数量要求：至少生成 3 个主要角色（根据剧情复杂度可适当增加）。
2. 角色基本信息（姓名、年龄、性别、职业、社会阶层）
   - 性别字段是必填项，禁止为空！必须明确为 '男'、'女' 或 '其他'。
3. 外貌特征（用于电影镜头表现）
4. 性格特质（优点、缺点、矛盾点）
5. 角色的“表层目标”（他/她现在想要什么）
6. 角色的“深层需求”（内心真正缺失的东西）
7. 角色的创伤或过去经历（推动性格形成）
8. 角色在故事中的功能（主角 / 反派 / 配角 / 镜像角色）
9. 角色可能经历的转变弧线（开场 → 结尾）
10. 一句能概括该角色的核心主题句

请避免模板化、脸谱化角色，强调现实逻辑与情感动机。

# 语言要求
输出语言：{}。

# 输出格式
请输出为 JSON 数组，格式如下：
[
  {{
    \"name\": \"角色姓名\",
    \"gender\": \"男\", // 严禁为空！必须是 \"男\" 或 \"女\" 或 \"其他\"
    \"isMain\": true/false,
    \"description\": \"这里包含上述所有详细设定（外貌、性格、目标、创伤等），请组织成一段通顺的文字或使用换行符分隔。注意：字数绝对不能超过 100 字。\"
  }}
]
注意：必须严格遵守 JSON 格式，不要包含 Markdown 代码块标记。description 字段字数绝对不能超过 100 字。",
            req.theme, synopsis, language
        )
    } else {
        format!(
            "你是一名资深电影编剧。

请为一部【{}】电影，生成一个完整、立体、真实可信的角色设定。

要求：
1. 数量要求：至少生成 3 个主要角色（根据剧情复杂度可适当增加）。
2. 角色基本信息（姓名、年龄、性别、职业、社会阶层）
   - 性别字段是必填项，禁止为空！必须明确为 '男'、'女' 或 '其他'。
3. 外貌特征（用于电影镜头表现）
4. 性格特质（优点、缺点、矛盾点）
5. 角色的“表层目标”（他/她现在想要什么）
6. 角色的“深层需求”（内心真正缺失的东西）
7. 角色的创伤或过去经历（推动性格形成）
8. 角色在故事中的功能（主角 / 反派 / 配角 / 镜像角色）
9. 角色可能经历的转变弧线（开场 → 结尾）
10. 一句能概括该角色的核心主题句

请避免模板化、脸谱化角色，强调现实逻辑与情感动机。

# 语言要求
输出语言：{}。

# 输出格式
请输出为 JSON 数组，格式如下：
[
  {{
    \"name\": \"角色姓名\",
    \"gender\": \"男\", // 严禁为空！必须是 \"男\" 或 \"女\" 或 \"其他\"
    \"isMain\": true/false,
    \"description\": \"这里包含上述所有详细设定（外貌、性格、目标、创伤等），请组织成一段通顺的文字或使用换行符分隔。注意：字数绝对不能超过 100 字。\"
  }}
]
注意：必须严格遵守 JSON 格式，不要包含 Markdown 代码块标记。description 字段字数绝对不能超过 100 字。",
            req.theme, language
        )
    }
}


// --- New Prompts ---

const BLUEPRINT_TYPES_DEF: &str = r#"
/** 剧本 */
export interface BluePrint {
  /** 层数, 值为 35-45 */
  levelCount: number
  /** 幕数, 值为 3-4 */
  actCount: number
  /** 起始节点 */
  startNode: StartNode
  /**
   * 关键标记, 数量控制在 1-4 个
   * - key 为标记的名称, 例如 越狱成功, 获得道具等, 必须是确切具体的内容, 不超过 10 个字;
   * - value 为标记的数据
   * @description 这是会影响剧情走向或结局的重要剧情状态, 也是可能造成重要转折, 必须精心设计
   */
  flags: {
    [flagName: string]: {
      /** 标记的详细描述, 必须是确切具体的内容, 不超过 25 字 */
      content: string
      /** 触发/获得 该标记的 level 索引, 值为 2-{@link levelCount}, 必须根据实际剧情({@link acts}) 生成 */
      triggerLevel: number
      /** 此 flag 产生副作用的 level 索引, 值为 {@link triggerLevel}-{@link levelCount}, 必须根据实际剧情({@link acts}) 生成 */
      effectLevel: number
    }
  }
  /** 结局节点, 数量控制在 3-5 个, key 为结局的名称, 例如 成功, 失败等; value 为结局信息 */
  endings: {
    [endingName: string]: {
      /** 结局的详细描述, 不超过 35 字 */
      content: string
      /** 触发 该结局的 level 索引, 值为 2-{@link levelCount}, 必须根据实际剧情({@link acts}) 生成 */
      triggerLevel: number
    }
  }
  /**
   * 幕(阶段), 表示相对独立的剧情阶段, 例如 1-5 层为第一阶段, 6-13 层为第二阶段等
   * @description 阶段数量为 {@link actCount} 个
   */
  acts: Array<BluePrintAct>
}

/**
 * 节点 ID, 格式为 `$level-$index`, 例如 `L1N1` 表示第一层中的第一个节点
 * @description level 表示第几层, index 表示该层中的第几个节点(从 1 开始索引)
 * @example L1N1
 */
type NodeId = `L${number}N${number}`

/** 起始节点 */
interface StartNode {
  /** 节点 ID, 值为 L1N1 */
  id: 'L1N1'
  /** 节点内容, 必须以第一个主角的第一人称视角编写, 不超过 60 字 */
  content: string
  /** 该节点的角色 name, 数量控制在 1-3 个 */
  characters: Array<string>
  /** 选项列表, 数量为 2 */
  choices: [StartNodeChoice, StartNodeChoice]
}

/** 起始节点的选项 */
interface StartNodeChoice {
  /** 该选项的内容, 必须以第一个主角的第一人称视角编写, 不超过 35 字 */
  content: string
  /** 该选项指向的下一个节点 ID */
  nextNodeId: NodeId
}

/** 剧本中的幕(阶段) */
interface BluePrintAct {
  /**
   * 该阶段的 level 范围(总层数为 {@link levelCount})
   * 例如 [1, 5] 表示此幕为 第 1 到第 5 层, 也就是说包含 1-5 层的所有节点; [6, 12] 表示此幕为 第 6 到第 12 层
   */
  levelRange: [number, number]
  /** 该阶段的名称, 不能超过 10 个字 */
  name: string
  /**
   * 该阶段剧情的完整具体的描述, ⚠️ **必须是确切具体的内容, 禁止任何 可能/模糊/模糊描述/猜测/假设/推测 等内容**, 200-240 字
   * @description 必须 **概括可能出现的所有剧情分支**, 确保不遗漏任何重要信息
   */
  description: string
}
"#;

pub(crate) fn construct_blueprint_prompt(
    req: &GenerateRequest,
    level_count: u32,
    act_count: u32,
) -> String {
    let title = req.theme.as_deref().unwrap_or("Unknown Title");
    let summary = req.synopsis.as_deref().unwrap_or("");
    let characters = req
        .characters
        .as_ref()
        .map(|cs| serde_json::to_string_pretty(cs).unwrap_or_default())
        .unwrap_or_default();

    format!(
        r#"# 角色定义
你是一位互动电影游戏编剧和总导演, 你擅长创作 引人入胜 / 逻辑严密 / 充满情感冲击力 的多分支剧情

## 主题
{title}

## 剧情简稿
{summary}

## 角色
{characters}

## 约束条件
- 剧本的总层数: {level_count}
- 剧本的总幕数: {act_count}

# 输出
以下是类型定义, 你需要返回的是一个JSON 数据, 具体要求:
- JSON 数据的类型为 `BluePrint`
- **不允许出现任何 `BluePrint` 的类型定义中没有的字段, 绝对不允许出现 nodes**
- **必须认真阅读注释内容, 并严格遵守注释中的要求, 特别是字数或者数量限制**
- 禁止包含 `\n`
- 至少有一个结局节点可以从非最后一个 act 到达

```typescript
{types}
```

---

根据用户提供的 主题 / 剧情简稿 / 角色, 发挥你的才华 **反复打磨剧情** 并创作一个完整的互动电影剧本, 并根据 [输出](#输出) 中的要求返回 `BluePrint` 类型的 JSON 数据, **禁止在任何内容的首尾出现中文的双引号**"#,
        title = title,
        summary = summary,
        characters = characters,
        level_count = level_count,
        act_count = act_count,
        types = BLUEPRINT_TYPES_DEF
    )
}

const LDAG_TYPES_DEF: &str = r#"
/** 至少包含一个元素的数组（非空数组） */
type NonEmptyArray<T> = readonly [T, ...T[]]

/**
 * 第一行只能有一个元素，后续每一行至少一个元素的二维数组
 */
type RestrictedTwoDimensionalArray<T> = readonly [
  readonly [T],
  ...NonEmptyArray<T>[]
]

/**
 * 每一幕的剧情节点图
 */
export type LDAGNodes = RestrictedTwoDimensionalArray<LDAGNode>

/**
 * 节点 ID, 格式为 `$level-$index`, 例如 `L1N1` 表示第一层中的第一个节点
 * @description level 表示第几层, index 表示该层中的第几个节点(从 1 开始索引)
 * @example L1N1
 */
type NodeId = `L${number}N${number}`

/** 结局节点 ID */
type EndingNodeId = `ENDING_${string}`

/**
 * LDAG 分层有向无环图节点
 */
interface LDAGNode {
  /**
   * 节点 ID, 格式为 `$level-$index`, 例如 `L1N1` 表示第一层中的第一个节点
   * @example L1N1 起始节点
   * @example L2N2 位于第二层的第二个节点(索引为 2 的节点)
   */
  id: NodeId
  /** 节点内容, 不超过 50 字 */
  content: string
  /** 该节点包含或关联的角色 name, 数量控制在 1-3 个 */
  characters: Array<string>
  /**
   * 选项列表, 数量为 1-3
   * ## 允许节点收束
   * 允许选项指向相同的节点
   */
  choices: [LDAGNodeChoice] | [LDAGNodeChoice, LDAGNodeChoice] | [LDAGNodeChoice, LDAGNodeChoice, LDAGNodeChoice]
}

/** 节点的选项 */
interface LDAGNodeChoice {
  /** 该选项的内容, 不超过 25 字 */
  content: string
  /** 
   * 触发(设置)的 flag 名称 
   * @description 对应 BluePrint 中的 flags[flagName].triggerLevel
   */
  triggerFlag?: string
  /** 
   * 该选项指向的下一个节点 ID 
   * @description 如果是 string, 表示无条件跳转; 如果是对象, 表示根据 checkFlag 的值跳转(对应 BluePrint 中的 flags[flagName].effectLevel)
   */
  nextNodeId: NodeId | EndingNodeId | ConditionalNextNodeId
}

/** 条件跳转节点 ID */
interface ConditionalNextNodeId {
  /** 需要检查的 flag 名称 */
  checkFlag: string
  /** flag 为 true 时的下一个节点 ID */
  trueId: NodeId | EndingNodeId
  /** flag 为 false 时的下一个节点 ID */
  falseId: NodeId | EndingNodeId
}
"#;

pub(crate) fn construct_fill_node_content_prompt(
    req: &GenerateRequest,
    act_info: &BluePrintAct,
    ldag_nodes: &Vec<Vec<LDAGNode>>,
) -> String {
    let title = req.theme.as_deref().unwrap_or("Unknown Title");
    let summary = req.synopsis.as_deref().unwrap_or("");
    let characters = req
        .characters
        .as_ref()
        .map(|cs| serde_json::to_string_pretty(cs).unwrap_or_default())
        .unwrap_or_default();
    
    let act_info_json = serde_json::to_string_pretty(act_info).unwrap_or_default();
    let ldag_nodes_json = serde_json::to_string_pretty(ldag_nodes).unwrap_or_default();

    format!(
        r#"# 角色定义
你是一位互动电影游戏编剧和总导演, 你擅长创作 引人入胜 / 逻辑严密 / 充满情感冲击力 的多分支剧情

## 主题
{title}

## 剧情简稿
{summary}

## 角色
{characters}

## 当前幕剧情信息
{act_info}

## 任务
你需要根据给定的 **剧情节点图结构** 和 **当前幕剧情信息**, 为每个节点填充具体的 **剧情内容** 和 **选项内容**。

## 剧情节点图结构
{ldag_nodes}

## 输出要求
请输出符合以下 TypeScript 类型定义的 JSON 数据, 也就是一个 `LDAGNodes` 类型的 JSON 数据:

```typescript
{types}
```

## 注意事项
1. 保持剧情的连贯性和逻辑性, 必须符合 **当前幕剧情信息** 的描述。
2. 节点的 ID 和连接关系 **必须** 与输入的 **剧情节点图结构** 完全一致，不能增加、删除或修改节点和连线，只能填充内容。
3. `content` 字段为剧情文本。
4. `choices` 字段中的 `content` 为选项文本。
5. 严格遵守 JSON 格式输出。"#,
        title = title,
        summary = summary,
        characters = characters,
        act_info = act_info_json,
        ldag_nodes = ldag_nodes_json,
        types = LDAG_TYPES_DEF
    )
}

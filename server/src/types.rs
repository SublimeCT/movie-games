use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

fn deserialize_string_or_vec<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrVec {
        String(String),
        Vec(Vec<String>),
    }

    match StringOrVec::deserialize(deserializer)? {
        StringOrVec::String(s) => Ok(s),
        StringOrVec::Vec(v) => Ok(v.join("\n")),
    }
}

fn deserialize_string_or_vec_to_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_string_or_vec(deserializer)
}

fn deserialize_option_vec_or_string<'de, D>(
    deserializer: D,
) -> Result<Option<Vec<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OptionVecOrString {
        Vec(Vec<String>),
        String(String),
    }

    let opt: Option<OptionVecOrString> = Option::deserialize(deserializer)?;
    match opt {
        Some(OptionVecOrString::Vec(v)) => Ok(Some(v)),
        Some(OptionVecOrString::String(s)) => Ok(Some(vec![s])),
        None => Ok(None),
    }
}

fn deserialize_characters<'de, D>(deserializer: D) -> Result<HashMap<String, Character>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum MapOrVec {
        Map(HashMap<String, Character>),
        Vec(Vec<Character>),
    }

    match MapOrVec::deserialize(deserializer)? {
        MapOrVec::Map(m) => Ok(m),
        MapOrVec::Vec(v) => {
            let mut m = HashMap::new();
            for c in v {
                let key = if !c.id.is_empty() {
                    c.id.clone()
                } else if !c.name.is_empty() {
                    c.name.clone()
                } else {
                    format!("char_{}", m.len())
                };
                m.insert(key, c);
            }
            Ok(m)
        }
    }
}

/// 电影剧本模板
/// 包含所有游戏数据：节点、角色、结局、元数据等
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MovieTemplate {
    /// 项目 ID
    pub project_id: String,
    /// 标题
    pub title: String,
    /// 版本号
    pub version: String,
    /// 所有者标识
    pub owner: String,
    /// 元数据（简介、时长、题材等）
    pub meta: MetaInfo,
    /// 背景图片 Base64
    #[serde(default)]
    pub background_image_base64: Option<String>,
    /// 剧情节点集合，Key 为节点 ID
    #[serde(default)]
    pub nodes: HashMap<String, StoryNode>,
    /// 结局集合，Key 为结局 ID
    #[serde(default)]
    pub endings: HashMap<String, Ending>,
    /// 角色集合，Key 为角色 ID
    #[serde(default, deserialize_with = "deserialize_characters")]
    pub characters: HashMap<String, Character>,
    /// 来源信息
    #[serde(default)]
    pub provenance: Provenance,
    /// 游戏中的 Flag 定义 (Generating 2.0)
    #[serde(default)]
    pub flags: HashMap<String, FlagInfo>,
}

/// 元数据信息
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct MetaInfo {
    /// 一句话梗概 (Logline)
    #[serde(default, deserialize_with = "deserialize_string_or_vec")]
    pub logline: String,
    /// 详细简介 (Synopsis)
    #[serde(default, deserialize_with = "deserialize_string_or_vec")]
    pub synopsis: String,
    /// 目标时长（分钟）
    #[serde(default)]
    pub target_runtime_minutes: u32,
    /// 题材/类型
    #[serde(default, deserialize_with = "deserialize_string_or_vec_to_string")]
    pub genre: String,
    /// 语言
    #[serde(default)]
    pub language: String,
}

/// 角色信息
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Character {
    /// 角色 ID
    pub id: String,
    /// 姓名
    pub name: String,
    /// 性别
    pub gender: String,
    /// 年龄
    pub age: u32,
    /// 角色定位/职业
    pub role: String,
    /// 背景故事
    pub background: String,
    /// 头像路径或 Base64
    pub avatar_path: Option<String>,
}

/// 剧情节点
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StoryNode {
    /// 节点 ID
    #[serde(default)]
    pub id: String, // Renamed from node_id
    /// 剧情文本内容
    pub content: String,
    /// 如果是结局节点，此字段存储结局 ID
    #[serde(default)]
    pub ending_key: Option<String>,
    /// 节点所在的层级（用于进度计算）
    #[serde(default)]
    pub level: Option<u32>,
    /// 节点涉及的角色 ID 列表
    #[serde(default, deserialize_with = "deserialize_option_vec_or_string")]
    pub characters: Option<Vec<String>>,
    /// 选项列表
    #[serde(default)]
    pub choices: Vec<Choice>,
}

/// 选项
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Choice {
    /// 选项文本
    pub text: String,
    /// 下一个节点 ID
    pub next_node_id: String,
    /// 触发的 Flag (Generating 2.0)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger_flag: Option<String>,
    /// 条件判断 (Generating 2.0)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<ChoiceCondition>,
}

/// 选项的条件判断
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChoiceCondition {
    /// 检查的 Flag 名称
    pub check_flag: String,
    /// 期望的值 (目前默认为 true)
    pub expected_value: bool,
    /// 如果条件不满足，跳转到的节点 ID (Alternative Path)
    /// 如果条件满足，则跳转到 Choice.next_node_id
    pub fallback_node_id: String,
}

/// 结局信息
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Ending {
    /// 结局类型：good, neutral, bad
    pub r#type: String, // 'good' | 'neutral' | 'bad'
    /// 结局描述
    pub description: String,
}

/// 来源信息
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Provenance {
    /// 创建者
    pub created_by: String,
    /// 创建时间
    pub created_at: String,
}

// --- New Types for Generating 2.0 ---

/// 剧本蓝图，由 LLM 生成的第一阶段产物
/// 对应前端 types/BluePrint.ts 中的 BluePrint
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BluePrint {
    /// 层数, 值为 35-45
    pub level_count: u32,
    /// 幕数, 值为 3-4
    pub act_count: u32,
    /// 起始节点
    pub start_node: StartNode,
    /// 关键标记, 数量控制在 1-4 个
    /// - key 为标记的名称, 例如 越狱成功, 获得道具等, 必须是确切具体的内容, 不超过 10 个字;
    /// - value 为标记的数据
    /// @description 这是会影响剧情走向或结局的重要剧情状态, 也是可能造成重要转折, 必须精心设计
    pub flags: HashMap<String, FlagInfo>,
    /// 结局节点, 数量控制在 3-5 个, key 为结局的名称, 例如 成功, 失败等; value 为结局信息
    pub endings: HashMap<String, EndingInfo>,
    /// 幕(阶段), 表示相对独立的剧情阶段, 例如 1-5 层为第一阶段, 6-13 层为第二阶段等
    /// @description 阶段数量为 {@link actCount} 个
    pub acts: Vec<BluePrintAct>,
}

/// 起始节点定义
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StartNode {
    /// 节点 ID, 值为 L1N1
    pub id: String, // 'L1N1'
    /// 节点内容, 必须以第一个主角的第一人称视角编写, 不超过 60 字
    pub content: String,
    /// 该节点的角色 name, 数量控制在 1-3 个
    pub characters: Vec<String>,
    /// 选项列表, 数量为 2
    pub choices: [StartNodeChoice; 2],
}

/// 起始节点选项
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StartNodeChoice {
    /// 该选项的内容, 必须以第一个主角的第一人称视角编写, 不超过 35 字
    pub content: String,
    /// 该选项指向的下一个节点 ID
    pub next_node_id: String,
}

/// Flag 信息
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FlagInfo {
    /// 标记的详细描述, 必须是确切具体的内容, 不超过 25 字
    pub content: String,
    /// 触发/获得 该标记的 level 索引, 值为 2-{@link levelCount}, 必须根据实际剧情({@link acts}) 生成
    /// @description 指在该 level 中的 **某个节点的某个选项** 会触发该标记, 允许该 level 有多个节点触发, 但 **禁止该 level 的所有节点的所有选项都触发**
    pub trigger_level: u32,
    /// 此 flag 产生副作用的 level 索引, 值为 {@link triggerLevel}-{@link levelCount}, 必须根据实际剧情({@link acts}) 生成
    /// @description 指在该 level 中的 **某个节点的某个选项** 会根据该标记产生分支/跳转, 允许该 level 有多个节点产生影响, 但 **禁止该 level 的所有节点的所有选项都产生影响**
    pub effect_level: u32,
}

/// 结局信息 (BluePrint)
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EndingInfo {
    /// 结局的详细描述, 不超过 35 字
    pub content: String,
    /// 触发 该结局的 level 索引, 值为 2-{@link levelCount}, 必须根据实际剧情({@link acts}) 生成
    /// @description 指在该 level 中的 **某个节点的某个选项** 会触发该结局, 允许该 level 有多个节点触发, 但 **禁止该 level 的所有节点的所有选项都触发**
    pub trigger_level: u32,
}

/// 幕 (Act) 定义
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BluePrintAct {
    /// 该阶段的 level 范围(总层数为 {@link levelCount})
    /// 例如 [1, 5] 表示此幕为 第 1 到第 5 层, 也就是说包含 1-5 层的所有节点; [6, 12] 表示此幕为 第 6 到第 12 层
    pub level_range: (u32, u32),
    /// 该阶段的名称, 不能超过 10 个字
    pub name: String,
    /// 该阶段剧情的完整具体的描述, ⚠️ **必须是确切具体的内容, 禁止任何 可能/模糊/模糊描述/猜测/假设/推测 等内容**, 200-240 字
    /// @description 必须 **概括可能出现的所有剧情分支**, 确保不遗漏任何重要信息
    pub description: String,
}

/// LDAG 节点
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LDAGNode {
    /// 节点 ID, 格式为 `$level-$index`, 例如 `L1N1` 表示第一层中的第一个节点
    /// @example L1N1 起始节点
    /// @example L2N2 位于第二层的第二个节点(索引为 2 的节点)
    pub id: String,
    /// 节点内容, 不超过 50 字
    pub content: String,
    /// 该节点包含或关联的角色 name, 数量控制在 1-3 个
    pub characters: Vec<String>,
    /// 选项列表, 数量为 1-3
    /// ## 允许节点收束
    /// 允许选项指向相同的节点
    pub choices: Vec<LDAGNodeChoice>,
}

/// LDAG 节点选项
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LDAGNodeChoice {
    /// 该选项的内容, 不超过 25 字
    pub content: String,
    /// 触发(设置)的 flag 名称 
    /// @description 对应 BluePrint 中的 flags[flagName].triggerLevel
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger_flag: Option<String>,
    /// 该选项指向的下一个节点 ID 
    /// @description 如果是 string, 表示无条件跳转; 如果是对象, 表示根据 checkFlag 的值跳转(对应 BluePrint 中的 flags[flagName].effectLevel)
    pub next_node_id: NextNodeId,
}

/// 下一个节点 ID 类型
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum NextNodeId {
    /// 简单跳转
    Simple(String),
    /// 条件跳转
    Conditional(ConditionalNextNodeId),
}

/// 条件跳转定义
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConditionalNextNodeId {
    /// 需要检查的 flag 名称
    pub check_flag: String,
    /// flag 为 true 时的下一个节点 ID
    pub true_id: String,
    /// flag 为 false 时的下一个节点 ID
    pub false_id: String,
}

/// 完整故事结构
/// 完整的生成完毕后的剧情信息
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Story {
    /// 数据库请求 ID
    pub request_id: Option<String>,
    /// 标题
    pub title: String,
    /// 元数据（简介、时长、题材等）
    pub meta: MetaInfo,
    /// 角色集合，Key 为角色 ID
    #[serde(default)]
    pub characters: HashMap<String, Character>,
    /// 蓝图
    #[serde(flatten)]
    pub blueprint: BluePrint,
    /// 剧情节点图 JSON
    pub act_list: Vec<Vec<Vec<LDAGNode>>>,
}


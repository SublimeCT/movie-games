/**
 * 与 `prompts/movie.template.json` 完全对应的 TypeScript 类型定义
 * 注意：字段与结构严格对应 JSON，未新增、未删除、未重命名
 */

/**
 * 顶层互动电影模板类型（精简版）
 */
export interface MovieTemplate {
  /** 项目 ID，用于唯一标识该互动电影项目 */
  projectId: string

  /** 电影标题，用于展示与检索 */
  title: string

  /** 模板版本号 */
  version: string

  /** 所有者标识（通常为 ProducerAgent 或 Owner） */
  owner: string

  /** 元信息（梗概、摘要、时长、类型、语言） */
  meta: MetaInfo

  /** 以节点 ID 为 key 的故事节点集合 */
  nodes: Record<string, StoryNode>

  /** 结局集合（与 nodes 分离） */
  endings?: Record<string, Ending>

  /** 角色定义集合 */
  characters: Record<string, Character>

  /** 文档级可追溯信息（谁何时创建或最后修改） */
  provenance: Provenance
}

/**
 * 元信息结构
 */
export interface MetaInfo {
  /** 一句话梗概（Logline） */
  logline: string

  /** 简短剧情摘要 */
  synopsis: string

  /** 目标片长（分钟） */
  targetRuntimeMinutes: number

  /** 影片类型，例如 "Drama / Interactive" */
  genre: string

  /** 影片语言（BCP47，例如 zh-CN） */
  language: string
}

/**
 * 角色定义结构
 */
export interface Character {
  /** 角色唯一 ID */
  id: string

  /** 角色姓名 */
  name: string

  /** 角色性别 */
  gender?: string

  /** 角色年龄 */
  age: number

  /** 角色职业/身份 */
  role: string

  /** 角色背景故事 */
  background: string

  /** 角色头像/形象资源路径 */
  avatarPath?: string
}

/**
 * 单个故事节点（StoryNode）
 */
export interface StoryNode {
  /** 节点唯一 ID（与 nodes 的 key 一致） */
  id: string

  /** 节点内容（文本与注记） */
  content: NodeContent

  /** 本节点出现的角色名字列表 */
  characters?: string[]

  /** 玩家在该节点可做出的所有选择 */
  choices: Choice[]
}

/**
 * 节点文本与注记
 */
export interface NodeContent {
  /** 用于叙事与生成的主文本 */
  text: string

  /** 给制作方/Agent 的额外注记（可选） */
  notes?: string
}

/**
 * 玩家选项条目
 */
export interface Choice {
  /** 界面展示文本 */
  text: string

  /** 选中后跳转的下一个节点 ID */
  nextNodeId: string
}

/**
 * 结局节点描述
 */
export interface Ending {
  /** 结局类型标识（good / neutral / bad） */
  type: 'good' | 'neutral' | 'bad'

  /** 对结局的文字说明 */
  description: string
}

/**
 * 文档级可追溯信息
 */
export interface Provenance {
  /** 创建者标识（Agent id 或 Owner） */
  createdBy: string

  /** 创建时间 ISO 字符串 */
  createdAt: string
}

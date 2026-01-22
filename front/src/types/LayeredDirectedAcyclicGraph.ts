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
 * 包含全部剧情节点的 分层有向无环图 (`Layered Directed Acyclic Graph`)
 * @description 具有严格详细数学约束的 **分层有向无环图** 剧情节点图
 */
export type LDAGActs = NonEmptyArray<LDAGNodes>

/**
 * 每一幕的剧情节点图
 */
export type LDAGNodes = RestrictedTwoDimensionalArray<LDAGNode>

/**
 * 结局节点集合
 * @description endingName 为结局节点名称, EndingNode 为结局节点信息
 */
export type EndingNodes = {
  [endingName: `ENDING_${string}`]: EndingNode
}

/** 结局节点 */
export interface EndingNode {
  /** 结局的详细描述, 不超过 35 字 */
  content: string
  /** 触发 该结局的 level 索引 */
  triggerLevel: number
}

/**
 * 节点 ID, 格式为 `$level-$index`, 例如 `L1N1` 表示第一层中的第一个节点
 * @description level 表示第几层, index 表示该层中的第几个节点(从 1 开始索引)
 * @example L1N1
 */
export type NodeId = `L${number}N${number}`

/** 结局节点 ID */
export type EndingNodeId = `ENDING_${string}`

/**
 * LDAG 分层有向无环图节点
 */
export interface LDAGNode {
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
export interface LDAGNodeChoice {
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
export interface ConditionalNextNodeId {
  /** 需要检查的 flag 名称 */
  checkFlag: string
  /** flag 为 true 时的下一个节点 ID */
  trueId: NodeId | EndingNodeId
  /** flag 为 false 时的下一个节点 ID */
  falseId: NodeId | EndingNodeId
}
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
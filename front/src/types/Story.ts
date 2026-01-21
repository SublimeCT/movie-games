import type { EndingNodes, LDAGActs } from "./LayeredDirectedAcyclicGraph";
import { type BluePrint } from './BluePrint'

/** 完整的生成完毕后的剧情信息 */
export interface Story extends BluePrint {
  /** 剧情节点图 JSON */
  actList: LDAGActs
  /** 结局节点集合 */
  endings: EndingNodes
}
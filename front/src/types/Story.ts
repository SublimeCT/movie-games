import type { LDAGActs } from "./LayeredDirectedAcyclicGraph";
import { type BluePrint } from './BluePrint'
import type { Character, MetaInfo } from './movie'

/** 完整的生成完毕后的剧情信息 */
export interface Story extends BluePrint {
  /** 数据库请求 ID */
  requestId?: string;
  /** 标题 */
  title: string;
  /** 元数据（简介、时长、题材等） */
  meta: MetaInfo;
  /** 角色集合，Key 为角色 ID */
  characters: Record<string, Character>;
  /** 剧情节点图 JSON */
  actList: LDAGActs
}
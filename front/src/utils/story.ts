import type { Story } from '../types/Story';
import type { LDAGNode, LDAGNodeChoice, NodeId, EndingNodeId, ConditionalNextNodeId } from '../types/LayeredDirectedAcyclicGraph';

/**
 * 将 Story (LDAGActs) 扁平化为以 ID 为 Key 的节点映射表
 * @param story - 完整的剧情 Story 对象
 * @returns 节点映射表 Record<string, LDAGNode>
 */
export const buildNodeMap = (story: Story): Record<string, LDAGNode> => {
  const map: Record<string, LDAGNode> = {};
  if (!story.actList || !Array.isArray(story.actList)) return map;
  
  for (const act of story.actList) {
    if (!Array.isArray(act)) continue;
    for (const level of act) {
      if (!Array.isArray(level)) continue;
      for (const node of level) {
        if (node && node.id) {
          map[node.id] = node;
        }
      }
    }
  }
  return map;
};

/**
 * 根据当前 flag 状态获取选项的下一个节点 ID
 * @param choice - 选项对象 (LDAGNodeChoice)
 * @param flags - 当前玩家的 flag 状态表
 * @returns 下一个节点的 ID (string)
 */
export const getNextNodeId = (
  choice: LDAGNodeChoice,
  flags: Record<string, boolean>
): string => {
  const next = choice.nextNodeId;
  if (typeof next === 'string') return next;
  
  // Handle ConditionalNextNodeId
  if (typeof next === 'object' && 'checkFlag' in next) {
    const { checkFlag, trueId, falseId } = next as ConditionalNextNodeId;
    return flags[checkFlag] ? trueId : falseId;
  }
  
  // Should not happen if types are correct
  return '';
};

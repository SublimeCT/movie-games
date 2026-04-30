import type { Story } from '../types/Story';
import type { LDAGNode, LDAGNodeChoice, ConditionalNextNodeId } from '../types/LayeredDirectedAcyclicGraph';
import type { MovieTemplate, StoryNode, Ending } from '../types/movie';

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

/**
 * 将 Story 转换为 MovieTemplate，以便在 Designer 等仅支持 MovieTemplate 的组件中使用
 * @param story - 完整的剧情 Story 对象
 * @returns MovieTemplate 对象
 */
export const convertStoryToTemplate = (story: Story): MovieTemplate => {
  const nodes: Record<string, StoryNode> = {};
  const endings: Record<string, Ending> = {};
  
  // 转换节点
  const ldagMap = buildNodeMap(story);
  for (const [id, ldagNode] of Object.entries(ldagMap)) {
    nodes[id] = {
      id: ldagNode.id,
      content: ldagNode.content,
      characters: ldagNode.characters,
      choices: (ldagNode.choices || []).map((c: LDAGNodeChoice) => {
        let nextNodeId = '';
        if (typeof c.nextNodeId === 'string') {
          nextNodeId = c.nextNodeId;
        } else if (c.nextNodeId && typeof c.nextNodeId === 'object' && 'checkFlag' in c.nextNodeId) {
          // Designer 暂不支持条件跳转，取 trueId 作为回退
          nextNodeId = c.nextNodeId.trueId;
        }
        
        return {
          text: c.content,
          nextNodeId,
          triggerFlag: c.triggerFlag
        };
      })
    };
  }

  // 转换结局
  if (story.endings) {
    for (const [key, ending] of Object.entries(story.endings)) {
      endings[key] = {
        type: 'neutral', // 默认类型，如果能推断可以修改
        description: ending.content,
        endingKey: key,
      };
    }
  }

  return {
    projectId: (story as any).projectId || crypto.randomUUID(),
    requestId: story.requestId,
    title: story.title || '',
    version: (story as any).version || '1.0.0',
    owner: (story as any).owner || 'User',
    meta: {
      logline: story.meta?.logline || story.title || '',
      synopsis: story.meta?.synopsis || '',
      targetRuntimeMinutes: story.meta?.targetRuntimeMinutes || 30,
      genre: story.meta?.genre || '',
      language: story.meta?.language || navigator.language
    },
    backgroundImageBase64: (story as any).backgroundImageBase64,
    nodes,
    endings,
    characters: story.characters || {},
    provenance: (story as any).provenance || {
      createdBy: 'import_story',
      createdAt: new Date().toISOString()
    }
  };
};

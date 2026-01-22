# 生成剧情信息

## 简介
对于 `/generating` 接口, 重写现有的生成剧情信息逻辑

## 步骤
1. 生成剧本的 [静态数据](#生成剧本中的静态数据)
2. 调用 LLM 生成 **剧本** JSON, 参考 [生成剧本](#生成剧本)
3. 直接在后端根据 **剧本** 生成具有严格数学约束的 **分层有向无环图** 剧情节点图 JSON, 参考 [通过程序生成剧情节点图](#通过程序生成剧情节点图)
4. 将每一幕的 剧情节点图 JSON 发送给 LLM(并发), 要求 LLM 并发填充节点内容, 参考 [并发生成每一幕的剧情节点图 JSON 中的剧情内容](#并发生成每一幕的剧情节点图-JSON-中的剧情内容)
5. 后端将完整的剧情数据 JSON 返回给前端, 参考 [组装剧情数据并返回](#组装剧情数据并返回)

为了降低成本, 其中 LLM 为 DeepSeek 的非思考模型

## 生成剧本中的静态数据
在生成剧情信息之前, 后端代码会随机生成以下静态数据(不依赖 LLM):

- 层数, 值为 30 - 40
- 幕数, 值为 3 - 4, 每一幕包含的层数在 6 - 12 之间

**在后续步骤中, 会将这些生成的静态数据作为参数传递给 LLM 和 LDAG 生成模块**

## 生成剧本
具体的 Prompt 参考 [BluePrint.md](./BluePrint.md), 其中有以下占位符, 分别代表不同的内容:

- `__TITLE__`: 剧情的主题, 接口传参
- `__SUMMARY__`: 剧情的简介, 接口传参
- `__CHARACTERS__`: 剧情中出现的角色, 接口传参
- `__LEVEL_COUNT__`: 随机生成的层数
- `__ACT_COUNT__`: 随机生成的幕数
- `__BLUEPRINT_TYPES__`: `BluePrint` 类型定义, **内容为 [BluePrint.ts](../../../front/src/types/BluePrint.ts)**

最终输出 `BluePrint` 类型(数据结构见 [BluePrint.ts](../../../front/src/types/BluePrint.ts))的 JSON 数据, 也就是 **剧本 JSON**

## 通过程序生成剧情节点图
为了完美解决 LLM 输出的剧情节点图的不稳定的问题, 直接在后端通过程序生成剧情节点图(不包含实际剧情内容), 具体要求如下:

- 具体规则参考 [LDAG.md](./LDAG.md)
- 具体的参数为 **剧本 JSON** 以及 **静态数据(层数/幕数)**
- 具体输出的 JSON 的数据类型参考 [LayeredDirectedAcyclicGraph.ts](../../../front/src/types/LayeredDirectedAcyclicGraph.ts)

最终输出 `LDAGActs` 类型(数据结构见 [LayeredDirectedAcyclicGraph.ts](../../../front/src/types/LayeredDirectedAcyclicGraph.ts))的 JSON 数据, 也就是 **剧情节点图 JSON**

:::tip
注意: 剧情节点图 JSON 中只包含剧情节点信息(不包含结局节点, 因为结局节点在 **剧本 JSON** 中已经包含了), 不包含实际剧情内容
:::

具体要求为:

- 在后端创建一个单独的模块来负责根据 **剧本 JSON** 生成 **分层有向无环图** 剧情节点图 JSON
- 必须严格按照 [LDAG.md](./LDAG.md) 以及 [LayeredDirectedAcyclicGraph.ts](../../../front/src/types/LayeredDirectedAcyclicGraph.ts) 中的规则生成 **剧情节点图 JSON**
- 必须为此模块编写覆盖率 100% 的测试用例, 必须通过全部测试用例

## 并发生成每一幕的剧情节点图 JSON 中的剧情内容
在上一步([通过程序生成剧情节点图](#通过程序生成剧情节点图))中, 我们已经生成了 **剧情节点图 JSON**, 现在需要遍历每一幕 (`LDAGNodes`), **并发**调用 LLM 填充每个节点的剧情内容

**注意:**
1. **第一幕的第一层节点 (L1N1) 必须直接使用剧本 JSON 中的 `startNode` 内容, 不需要调用 LLM 生成**
2. **生成 Acts 必须是并发的**, 严禁顺序生成
3. LLM 的 `max_tokens` 必须固定为 **7890**
4. LLM 的 `response_format` 必须设置为 `{'type': 'json_object'}`
5. 提示词必须明确禁止输出 Markdown 代码块, 必须输出原始 JSON
6. 提示词必须要求输出一个包含 `nodes` 字段的 JSON 对象, 以满足 `json_object` 模式的要求
7. **创作质量要求**: 提示词必须明确要求 LLM 以电影剧本或高品质小说的水准进行创作, 使用第一人称("我"), 强调细节描写、氛围感和内心独白, 杜绝流水账。

具体的 Prompt 参考 [fill-node-content.md](./fill-node-content.md), 其中有以下占位符, 分别代表不同的内容:

- `__TITLE__`: 剧情的主题, 接口传参
- `__SUMMARY__`: 剧情的简介, 接口传参
- `__CHARACTERS__`: 剧情中出现的角色, 接口传参
- `__ACT_INFO__`: 当前幕的详细信息, 也就是 `BluePrint` 中的 `acts` 数组中的某一项的 JSON 字符串
- `__LDAG_NODES__`: 当前幕的剧情节点图 JSON (不含内容), 也就是 `LDAGNodes` 类型的 JSON 数据
- `__LDAG_TYPES__`: `LDAGNodes` 类型定义, **内容为 [LayeredDirectedAcyclicGraph.ts](../../../front/src/types/LayeredDirectedAcyclicGraph.ts)**

## 组装剧情数据并返回
将前面所有步骤的数据组装为一个 `Story` 类型的 JSON 数据并返回给前端, 类型定义参考 [Story.ts](../../../front/src/types/Story.ts)
**注意: 需要确保 `BluePrint.endings` 中的 key 格式化为 `ENDING_${string}` 以匹配 `Story` 类型定义**

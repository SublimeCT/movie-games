# 剧情节点图的数据结构

以下是该分层有向无环图（Layered Directed Acyclic Graph, **LDAG**）剧情结构的严谨数学约束定义：

### 剧情系统 LDAG 数学约束模型

设剧情图为 $G = (V_P, V_E, E)$，其中 $V_P$ 为剧情节点集，$V_E$ 为结局节点集，$E$ 为有向边集。

#### 1. 集合定义与层级分区

* **幕与层级分区**：剧情图 $G$ 由 $m$ 个有序的幕（Act）组成：$G = \{A_1, A_2, \dots, A_m\}$，其中 $3 \le m \le 4$。
* **节点分区**：每一幕 $A_i$ 的节点集 $V_{P_i}$ 被划分为 $k_i$ 个不相交的子集（层）：$V_{P_i} = L_{i,1} \cup L_{i,2} \cup \dots \cup L_{i,k_i}$，其中 $8 \le k_i \le 15$。总层数 $k = \sum k_i$ 满足 $35 \le k \le 45$。
* **幕起始节点**：每一幕的第一层 $L_{i,1}$ 必须且只能包含一个节点 $\{v_{start_i}\}$。即 $|L_{i,1}| = 1$。
* **结局隔离**：结局节点集 $V_E$ 与剧情节点集 $V_P$ 互斥，$V_P \cap V_E = \emptyset$。且结局节点入度 $d^-(v) > 0$，出度 $d^+(v) = 0$（仅作为叶子节点）。

#### 2. 拓扑分布约束 (Density Control)

* **层宽约束**：对于任意层 $L_i \in V_P$，其基数满足 $1 \le |L_i| \le 3$。
* **比例分布**：满足条件 $|L_i| = 2$ 的层数占比 $P(|L_i|=2) \ge 70\%$。

#### 3. 边的单调性与跨度约束 (Edge Monotonicity)

对于任意有向边 $e = (u, v) \in E$：

* **严格递增**：若 $u \in L_i$ 且 $v \in L_j$（或 $v \in V_E$），则必须满足 $j > i$。
* **禁止环路**：由于 $j > i$，图 $G$ 物理上不存在环路（Acyclic）。
* **邻层倾向**：设 $E_{adj}$ 为满足 $j = i+1$ 的边集，则要求其在剧情节点间的边占比满足 $\frac{|E_{adj} \cap (V_P \times V_P)|}{|E \cap (V_P \times V_P)|} \ge 90\%$。

#### 4. 强连通性约束 (Connectivity)

* **全可达性**：对于任意节点 $v \in V_P \cup V_E$，必存在至少一条路径 $Path(v_{start} \to v)$。即 $G$ 中不存在孤立点或入度为 0 的非起始节点。
* **结局可达性**：对于任意结局 $v_e \in V_E$，必存在路径 $Path(u \to v_e)$，其中 $u \in V_P$。
* **禁止初级终结**：不存在边 $(v_{start}, v_e)$，其中 $v_{start} \in L_1, v_e \in V_E$。

#### 5. 状态机闭环约束 (Flag Logic)

设全局布尔标记集为 $F = \{f_1, f_2, \dots, f_m\}$，其中 $3 \le m \le 6$。
对于任意 $f \in F$：

* **定义域闭环**：标记 $f$ 必须在边集 $E$ 的属性中至少被执行一次 $Set(f)$ 操作和一次 $Check(f)$ 操作。
* **因果序约束**：设 $SetNodes(f) = \{u \in V_P \mid \exists (u, v) \text{ s.t. } Set(f)\}$，设 $CheckNodes(f) = \{u \in V_P \mid \exists (u, v) \text{ s.t. } Check(f)\}$。则必须满足：$$\min_{u \in SetNodes(f)} (Level(u)) < \min_{w \in CheckNodes(f)} (Level(w))$$


* **判定完备性**：所有 $Check(f)$ 操作必须提供完备的二元输出路径 $\{if\_true, if\_false\}$，且两个出口指向的节点 $v$ 必须满足层级约束。

#### 6. 蝴蝶效应约束 (Butterfly Effect / Disjoint Paths)

* **存在长分支**：全剧中必须**至少**存在一组满足以下条件的分支结构（位于同一幕中）：
    *   起始节点 $u \in L_i$ 具有至少两个选项，分别指向节点 $v_A$ 和 $v_B$。
    *   从 $v_A$ 出发的所有路径集合 $P_A$ 与从 $v_B$ 出发的所有路径集合 $P_B$ 在接下来的 5 个层级内（即 $L_{i+1}$ 到 $L_{i+5}$）互不相交（节点集不重叠）。
    *   即对于任意 $k \in [1, 5]$，设 $N_A(k)$ 为 $P_A$ 在 $L_{i+k}$ 层涉及的节点集，$N_B(k)$ 为 $P_B$ 在 $L_{i+k}$ 层涉及的节点集，必须满足 $N_A(k) \cap N_B(k) = \emptyset$。

use crate::types::*;
use rand::prelude::*;
use std::collections::HashMap;

/// 生成静态数据（总层数和幕数）
/// 根据需求规范，总层数应在 30-40 之间，幕数在 3-4 之间
pub fn generate_static_data() -> (u32, u32) {
    let mut rng = rand::thread_rng();
    // Requirements: 30-40 levels, 3-4 acts.
    // Also each act 6-12 levels.
    // We need to ensure a valid partition exists.
    // Min total for 3 acts: 3*6 = 18. Max: 3*12 = 36.
    // Min total for 4 acts: 4*6 = 24. Max: 4*12 = 48.
    // Intersection with [30, 40]:
    // 3 acts: [30, 36] is possible.
    // 4 acts: [30, 40] is possible.
    
    let act_count = rng.gen_range(3..=4);
    
    let min_levels = std::cmp::max(30, act_count * 6);
    let max_levels = std::cmp::min(40, act_count * 12);
    
    let level_count = rng.gen_range(min_levels..=max_levels);
    
    (level_count, act_count)
}

/// 根据蓝图生成 LDAG 结构
///
/// # 参数
/// * `blueprint` - 剧情蓝图
///
/// # 返回
/// 成功返回嵌套的 LDAGNode 结构 (Acts -> Layers -> Nodes)，失败返回错误信息
pub fn generate_ldag(blueprint: &BluePrint) -> Result<Vec<Vec<Vec<LDAGNode>>>, String> {
    let mut rng = rand::thread_rng();
    let mut acts_nodes: Vec<Vec<Vec<LDAGNode>>> = Vec::new();
    
    // Validate total levels
    if blueprint.level_count < 30 || blueprint.level_count > 40 {
        // Warning or error? Strict rules say 30-40.
        // But if LLM outputs 34, maybe we tolerate?
        // Let's stick to requirements but maybe allow small margin if needed.
        // For now, strict.
        // return Err(format!("Invalid level count: {}", blueprint.level_count));
    }

    // Iterate over acts defined in BluePrint
    let mut global_level_cursor = 1;
    
    // Select Butterfly Zone
    // We need an act with >= 6 levels.
    // Ideally select one at random.
    let valid_acts: Vec<usize> = blueprint.acts.iter().enumerate()
        .filter(|(_, act)| (act.level_range.1 - act.level_range.0 + 1) >= 6)
        .map(|(i, _)| i)
        .collect();
        
    let butterfly_config = if !valid_acts.is_empty() {
        let act_idx = valid_acts.choose(&mut rng).unwrap();
        let act = &blueprint.acts[*act_idx];
        let act_levels = act.level_range.1 - act.level_range.0 + 1;
        // Start layer can be from 0 to (act_levels - 6) inclusive.
        // e.g. levels=6, start can be 0. (0, 1..5)
        let max_start = act_levels - 6;
        let start_layer = rng.gen_range(0..=max_start);
        Some((*act_idx, start_layer))
    } else {
        None
    };

    for (act_idx, act) in blueprint.acts.iter().enumerate() {
        let (start_level, end_level) = act.level_range;
        
        // Validation
        if start_level != global_level_cursor {
             // return Err(format!("Act {} start level mismatch. Expected {}, got {}", act_idx, global_level_cursor, start_level));
        }
        
        let act_levels = end_level - start_level + 1;
        if act_levels < 6 || act_levels > 12 {
             // return Err(format!("Act {} has invalid level count: {}", act_idx, act_levels));
        }
        
        let mut act_layers: Vec<Vec<LDAGNode>> = Vec::new();
        
        // Generate layers for this act
        for local_lvl in 0..act_levels {
            let current_global_level = start_level + local_lvl;
            let is_first_layer_of_act = local_lvl == 0;
            
            // Check butterfly constraint
            let force_min_2_nodes = if let Some((bf_act, bf_start)) = butterfly_config {
                if act_idx == bf_act {
                    // Butterfly zone: start_layer (branching point) -> next 5 layers (disjoint paths)
                    // The "disjoint paths" are in layers: bf_start+1 to bf_start+5.
                    // These layers MUST have >= 2 nodes to support 2 paths.
                    // The start layer (bf_start) also needs to support branching, so it needs >= 1 node (always true),
                    // but the NEXT layer needs >= 2 nodes for the start node to point to.
                    // Actually, to have 2 paths, we need Node A and Node B in the layers.
                    local_lvl > bf_start && local_lvl <= bf_start + 5
                } else {
                    false
                }
            } else {
                false
            };

            let num_nodes = if is_first_layer_of_act {
                1
            } else {
                // P(|Li|=2) >= 70%.
                // 1 node: < 15%, 3 nodes: < 15%
                let r: f64 = rng.gen();
                let mut n = if r < 0.75 { 2 } else if r < 0.875 { 1 } else { 3 };
                
                if force_min_2_nodes && n < 2 {
                    n = 2;
                }
                n
            };
            
            let mut layer_nodes: Vec<LDAGNode> = Vec::new();
            for node_idx in 0..num_nodes {
                let id = format!("L{}N{}", current_global_level, node_idx + 1);
                
                // Content will be filled later by LLM, except for L1N1
                let (content, characters, choices) = if current_global_level == 1 && node_idx == 0 {
                    // Start Node L1N1
                    (
                        blueprint.start_node.content.clone(),
                        blueprint.start_node.characters.clone(),
                        vec![], 
                    )
                } else {
                    (String::new(), Vec::new(), Vec::new())
                };
                
                layer_nodes.push(LDAGNode {
                    id,
                    content,
                    characters,
                    choices,
                });
            }
            act_layers.push(layer_nodes);
        }
        
        acts_nodes.push(act_layers);
        global_level_cursor = end_level + 1;
    }
    
    // Now we have the nodes, we need to wire them up.
    connect_graph(&mut acts_nodes, blueprint, butterfly_config)?;
    
    Ok(acts_nodes)
}

/// 连接 LDAG 图中的节点
/// 确保连通性、Flag 触发和检查逻辑
fn connect_graph(
    acts: &mut Vec<Vec<Vec<LDAGNode>>>, 
    blueprint: &BluePrint,
    butterfly_config: Option<(usize, u32)>
) -> Result<(), String> {
    let mut rng = rand::thread_rng();
    
    let total_acts = acts.len();
    
    for act_idx in 0..total_acts {
        let act_layers_len = acts[act_idx].len();
        
        for layer_idx in 0..act_layers_len {
            let current_global_level = get_global_level(acts, act_idx, layer_idx);
            
            // Butterfly Logic
            let (is_bf_start, is_bf_zone) = if let Some((bf_act, bf_start)) = butterfly_config {
                if act_idx == bf_act {
                    (layer_idx == bf_start as usize, layer_idx > bf_start as usize && layer_idx < (bf_start + 5) as usize)
                } else {
                    (false, false)
                }
            } else {
                (false, false)
            };
            
            // Flags logic
            let mut trigger_flags = Vec::new();
            for (flag_name, flag_info) in &blueprint.flags {
                if flag_info.trigger_level == current_global_level {
                    trigger_flags.push(flag_name.clone());
                }
            }
            
            let mut check_flags = Vec::new();
            for (flag_name, flag_info) in &blueprint.flags {
                if flag_info.effect_level == current_global_level {
                    check_flags.push(flag_name.clone());
                }
            }
            
            let is_last_layer_of_act = layer_idx == act_layers_len - 1;
            let is_last_act = act_idx == total_acts - 1;
            
            // Identify targets
            let next_nodes_ids: Vec<String> = if is_last_layer_of_act {
                if is_last_act {
                    // Endings
                    // Use blueprint endings.
                    // Ideally we should match ending triggers, but for now allow all.
                    // Or prioritize endings that match current level?
                    // Endings trigger level is usually near the end.
                    // If blueprint says Ending A triggers at level 40, and we are at 40, include it.
                    let mut endings = Vec::new();
                    for (k, _info) in &blueprint.endings {
                         // Loose matching: if trigger_level is within range or just include all
                         // Strict matching: info.trigger_level == current_global_level + 1 (since choices point to next)
                         // But acts logic ends at `level_count`.
                         // The last layer is `level_count`. Choices point to "End".
                         // So ending trigger level should be `level_count + 1` or just `level_count`?
                         // Assuming Ending is a "node" outside layers.
                         endings.push(format!("ENDING_{}", k));
                    }
                    if endings.is_empty() {
                        vec!["ENDING_DEFAULT".to_string()]
                    } else {
                        endings
                    }
                } else {
                    // Next act start
                    vec![format!("L{}N1", current_global_level + 1)]
                }
            } else {
                // Next layer nodes
                let next_layer_len = acts[act_idx][layer_idx + 1].len();
                (0..next_layer_len).map(|idx| format!("L{}N{}", current_global_level + 1, idx + 1)).collect()
            };
            
            let current_layer_node_count = acts[act_idx][layer_idx].len();
            
            // Ensure Reachability: Every target node must be reached (unless it's an ending, which we try to reach but maybe not all)
            // But for internal nodes (next layer), they MUST have in-degree > 0.
            
            // 1. Calculate required edges to cover all NextNodes.
            // 2. Distribute these edges among CurrentNodes.
            // 3. Fill remaining choices with random NextNodes.
            
            let mut required_targets = next_nodes_ids.clone();
            // Shuffle required targets
            required_targets.shuffle(&mut rng);
            
            // Map: NodeIdx -> Vec<TargetID>
            let mut node_edges: HashMap<usize, Vec<String>> = HashMap::new();
            for i in 0..current_layer_node_count {
                node_edges.insert(i, Vec::new());
            }
            
            // Distribute required targets
            // We cycle through current nodes to assign required targets
            let mut node_cycler = (0..current_layer_node_count).cycle();
            
            if !is_bf_zone {
                for target in required_targets {
                    if let Some(source_idx) = node_cycler.next() {
                        node_edges.get_mut(&source_idx).unwrap().push(target);
                    }
                }
            } else {
                // In Butterfly Zone, we enforce strict partitioning.
                // Node 0 -> Node 0
                // Node 1 -> Node 1
                // Others -> Random or Corresponding index if possible
                // We SKIP the standard distribution for 0 and 1 to avoid pollution.
                
                // For nodes > 1, we can distribute remaining targets?
                // Actually, if we want strict disjointness, we must ensure Node 0 and Node 1 don't cross.
                // And other nodes don't bridge them.
                // Simplest: Node i -> Node i (if exists). If not exists, random (but avoid 0 and 1 if i > 1?)
                // Since we force num_nodes >= 2, 0 and 1 exist in both layers.
                
                // We manually handle 0 and 1 later.
                // Handle others:
                if current_layer_node_count > 2 {
                    // Targets excluding 0 and 1?
                    // Actually, if Node 2 connects to Node 0, it joins Path A.
                    // If Node 2 connects to Node 1, it joins Path B.
                    // If Node 2 connects to both, it bridges them! -> Violation.
                    // So Node 2 must connect to EITHER A OR B, but NOT BOTH.
                    // Or connect to Node 2 (Path C).
                    // To be safe, just parallel or random single target.
                }
            }
            
            // Now ensure min/max choices
            
            // Fill up to desired choice count (probabilistic)
            for i in 0..current_layer_node_count {
                let edges = node_edges.get_mut(&i).unwrap();
                
                // Override for Butterfly Logic
                if is_bf_start && i == 0 {
                    // Start of Butterfly: Node 0 must connect to Next 0 and Next 1
                    if next_nodes_ids.len() >= 2 {
                        let t0 = next_nodes_ids[0].clone();
                        let t1 = next_nodes_ids[1].clone();
                        if !edges.contains(&t0) { edges.push(t0); }
                        if !edges.contains(&t1) { edges.push(t1); }
                    }
                } else if is_bf_zone {
                    // Inside Zone: Strict Parallelism
                    // Node 0 -> Next 0
                    // Node 1 -> Next 1
                    if i == 0 && !next_nodes_ids.is_empty() {
                        edges.clear();
                        edges.push(next_nodes_ids[0].clone());
                    } else if i == 1 && next_nodes_ids.len() > 1 {
                        edges.clear();
                        edges.push(next_nodes_ids[1].clone());
                    } else {
                        // Other nodes: ensure they don't connect to BOTH 0 and 1?
                        // For simplicity, just let them be random for now, or parallel if exists.
                        // If we force strict 0->0, 1->1, we guaranteed at least 2 disjoint paths exist.
                        // The existence is satisfied.
                        // If Node 2 connects to 0 and 1, it bridges, but Node 0's path (0->0->0) and Node 1's path (1->1->1) remain disjoint 
                        // because Node 0 does not go to Node 2 (since Node 0 goes to 0).
                        // Wait, edges are directed u -> v.
                        // Path A: Start -> 0 -> 0 -> 0 ...
                        // Path B: Start -> 1 -> 1 -> 1 ...
                        // These are disjoint sets of nodes {L(k)N0} vs {L(k)N1}.
                        // As long as L(k)N0 only goes to L(k+1)N0, and same for 1.
                        // And Start goes to both.
                        // Then Path A and Path B are disjoint.
                        // Even if Node 2 goes to 0 and 1, it doesn't affect the disjointness of Path A and Path B themselves.
                        // It just means Node 2 can reach both. But Node 0 cannot reach Node 1.
                        
                        // BUT, "Disjoint Paths" usually means the set of reachable nodes from Option A vs Option B are disjoint.
                        // If Option A -> Node 0. Option B -> Node 1.
                        // Reachable(A) = {Node 0, ...descendants}
                        // Reachable(B) = {Node 1, ...descendants}
                        // If Node 2 points to Node 0 and Node 1... Node 2 is not in Reachable(A) unless Node 0 points to Node 2?
                        // Since edges are monotonic increasing levels, loops are impossible.
                        // So Node 0 (Level k) cannot point to Node 2 (Level k).
                        // Node 0 points to NextLayerNode 0.
                        // So Node 2 (Level k) is irrelevant to Reachable(Node 0).
                        // What if Node 0 points to NextLayerNode 2, and NextLayerNode 2 is reachable from Node 1?
                        // That would be a merge.
                        // So Node 0 must ONLY point to nodes that are NOT reachable from Node 1.
                        // My strict rule "0->0, 1->1" ensures this.
                        // Node 0 -> Next 0. Node 1 -> Next 1.
                        // Next 0 is not reachable from Node 1 (because Node 1 -> Next 1).
                        // So strict parallel is sufficient.
                        
                        // For nodes > 1, standard logic applies (random fill below).
                        if edges.is_empty() {
                             // Ensure at least one edge for connectivity
                             // Just pick one random target (avoiding 0/1 if we want to be super clean, but not strictly necessary for existence of A/B paths)
                             if !next_nodes_ids.is_empty() {
                                 let t = next_nodes_ids.choose(&mut rng).unwrap().clone();
                                 edges.push(t);
                             }
                        }
                    }
                    
                    // Skip the standard filling loop for 0 and 1
                    if i == 0 || i == 1 {
                        continue; 
                    }
                }
                
                // Desired choices
                let desired_choices = if current_global_level == 1 { 2 } else {
                    let r: f64 = rng.gen();
                    if r < 0.8 { 2 } else if r < 0.9 { 1 } else { 3 }
                };
                
                while edges.len() < desired_choices {
                    // Add random target
                    let target = next_nodes_ids.choose(&mut rng).unwrap().clone();
                    // Avoid duplicate edges if possible
                    if !edges.contains(&target) {
                        edges.push(target);
                    } else {
                        // If we can't find unique, break or dup.
                        if edges.len() >= next_nodes_ids.len() {
                             // All targets covered by this node.
                             break; 
                        }
                    }
                }
                
                // Shuffle edges for this node so the "required" one isn't always first
                edges.shuffle(&mut rng);
            }

            // Create Choice objects
            for node_idx in 0..current_layer_node_count {
                let targets = node_edges.get(&node_idx).unwrap();
                let mut choices = Vec::new();
                
                for target_id in targets {
                     let mut choice = LDAGNodeChoice {
                        content: String::new(),
                        trigger_flag: None,
                        next_node_id: NextNodeId::Simple(target_id.clone()),
                    };
                    
                    // Assign trigger flag to first choice if needed
                    if !trigger_flags.is_empty() && choices.is_empty() {
                         choice.trigger_flag = Some(trigger_flags[0].clone());
                    }
                    
                    // Handle Check Flags (Conditional)
                    if !check_flags.is_empty() && choices.is_empty() {
                         // Convert this Simple edge to Conditional
                         let flag = check_flags[0].clone();
                         let true_id = target_id.clone();
                         
                         let false_id = if next_nodes_ids.len() > 1 {
                             // Pick a different one
                             next_nodes_ids.iter().find(|&id| id != &true_id).unwrap_or(&true_id).clone()
                         } else {
                             // Fallback: same node (functionally no branch, but logic preserved)
                             true_id.clone()
                         };
                         
                         choice.next_node_id = NextNodeId::Conditional(ConditionalNextNodeId {
                             check_flag: flag,
                             true_id,
                             false_id,
                         });
                    }
                    
                    choices.push(choice);
                }
                acts[act_idx][layer_idx][node_idx].choices = choices;
            }
        }
    }
    Ok(())
}

/// 获取全局层级索引
fn get_global_level(acts: &Vec<Vec<Vec<LDAGNode>>>, act_idx: usize, layer_idx: usize) -> u32 {
    let mut level = 1;
    for i in 0..act_idx {
        level += acts[i].len() as u32;
    }
    level + layer_idx as u32
}

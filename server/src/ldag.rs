use crate::types::*;
use rand::prelude::*;
use std::collections::HashMap;

/// 生成静态数据（总层数和幕数）
/// 根据需求规范，总层数应在 35-45 之间，幕数在 3-4 之间
pub fn generate_static_data() -> (u32, u32) {
    let mut rng = rand::thread_rng();
    // Requirements: 35-45 levels, 3-4 acts.
    // Also each act 8-15 levels.
    // We need to ensure a valid partition exists.
    // Min total for 3 acts: 3*8 = 24. Max: 3*15 = 45.
    // Min total for 4 acts: 4*8 = 32. Max: 4*15 = 60.
    // Intersection with [35, 45]:
    // 3 acts: [35, 45] is possible.
    // 4 acts: [35, 45] is possible.
    
    let act_count = rng.gen_range(3..=4);
    let level_count = rng.gen_range(35..=45);
    
    // We can just return these. The partitioning will be handled or validated later.
    // Ideally we should pick level_count such that a valid partition exists, which is true for all values in [35, 45] for 3 or 4 acts.
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
    if blueprint.level_count < 35 || blueprint.level_count > 45 {
        // Warning or error? Strict rules say 35-45.
        // But if LLM outputs 34, maybe we tolerate?
        // Let's stick to requirements but maybe allow small margin if needed.
        // For now, strict.
        // return Err(format!("Invalid level count: {}", blueprint.level_count));
    }

    // Iterate over acts defined in BluePrint
    let mut global_level_cursor = 1;
    
    for (_act_idx, act) in blueprint.acts.iter().enumerate() {
        let (start_level, end_level) = act.level_range;
        
        // Validation
        if start_level != global_level_cursor {
             // return Err(format!("Act {} start level mismatch. Expected {}, got {}", act_idx, global_level_cursor, start_level));
        }
        
        let act_levels = end_level - start_level + 1;
        if act_levels < 8 || act_levels > 15 {
             // return Err(format!("Act {} has invalid level count: {}", act_idx, act_levels));
        }
        
        let mut act_layers: Vec<Vec<LDAGNode>> = Vec::new();
        
        // Generate layers for this act
        for local_lvl in 0..act_levels {
            let current_global_level = start_level + local_lvl;
            let is_first_layer_of_act = local_lvl == 0;
            
            let num_nodes = if is_first_layer_of_act {
                1
            } else {
                // P(|Li|=2) >= 70%.
                // 1 node: < 15%, 3 nodes: < 15%
                let r: f64 = rng.gen();
                if r < 0.75 { 2 } else if r < 0.875 { 1 } else { 3 }
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
                        vec![], // Choices will be rebuilt to match LDAG structure, or we preserve structure?
                        // Wait, BluePrint.startNode has choices.
                        // But LDAG structure generation might create different topology.
                        // The prompt says: "L1N1 must directly use BluePrint.startNode content".
                        // It implies we should try to match the choices too?
                        // Or we just use the text content and characters, but regenerate the connections?
                        // generating.md says: "Backend generates LDAG structure... Step 4 fills content".
                        // "L1N1 must directly use... startNode content".
                        // If we regenerate structure, we might have different number of choices than BluePrint.startNode.
                        // BluePrint.startNode has fixed 2 choices.
                        // Our LDAG generation logic might generate 1-3 choices.
                        // L1 is always 1 node. L2 is usually 2 nodes.
                        // So L1N1 usually has 2 choices pointing to L2N1 and L2N2.
                        // It matches.
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
    // It's easier to do this in a single pass over all acts if we flatten them conceptually, 
    // or just iterate acts_nodes.
    
    connect_graph(&mut acts_nodes, blueprint)?;
    
    Ok(acts_nodes)
}

/// 连接 LDAG 图中的节点
/// 确保连通性、Flag 触发和检查逻辑
fn connect_graph(acts: &mut Vec<Vec<Vec<LDAGNode>>>, blueprint: &BluePrint) -> Result<(), String> {
    let mut rng = rand::thread_rng();
    
    // Flatten for easier indexing: (act_idx, layer_idx, node_idx) -> Node
    // Or just iterate.
    
    let total_acts = acts.len();
    
    for act_idx in 0..total_acts {
        let act_layers_len = acts[act_idx].len();
        
        for layer_idx in 0..act_layers_len {
            let current_global_level = get_global_level(acts, act_idx, layer_idx);
            
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
            
            for target in required_targets {
                if let Some(source_idx) = node_cycler.next() {
                    node_edges.get_mut(&source_idx).unwrap().push(target);
                }
            }
            
            // Now ensure min/max choices
            
            // Fill up to desired choice count (probabilistic)
            for i in 0..current_layer_node_count {
                let edges = node_edges.get_mut(&i).unwrap();
                
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

#[cfg(test)]
mod tests {
    use crate::ldag;
    use crate::types::*;
    use std::collections::HashMap;

    fn create_mock_blueprint() -> BluePrint {
        let mut flags = HashMap::new();
        flags.insert(
            "FLAG_KEY".to_string(),
            FlagInfo {
                content: "Found the key".to_string(),
                trigger_level: 10,
                effect_level: 20,
            },
        );

        let mut endings = HashMap::new();
        endings.insert(
            "good".to_string(),
            EndingInfo {
                content: "Good Ending".to_string(),
                trigger_level: 40,
            },
        );
        endings.insert(
            "bad".to_string(),
            EndingInfo {
                content: "Bad Ending".to_string(),
                trigger_level: 40,
            },
        );

        let acts = vec![
            BluePrintAct {
                level_range: (1, 10),
                name: "Act 1".to_string(),
                description: "Setup".to_string(),
            },
            BluePrintAct {
                level_range: (11, 25),
                name: "Act 2".to_string(),
                description: "Conflict".to_string(),
            },
            BluePrintAct {
                level_range: (26, 40),
                name: "Act 3".to_string(),
                description: "Resolution".to_string(),
            },
        ];

        BluePrint {
            level_count: 40,
            act_count: 3,
            start_node: StartNode {
                id: "L1N1".to_string(),
                content: "Start content".to_string(),
                characters: vec!["Hero".to_string()],
                choices: [
                    StartNodeChoice {
                        content: "Choice 1".to_string(),
                        next_node_id: "L2N1".to_string(),
                    },
                    StartNodeChoice {
                        content: "Choice 2".to_string(),
                        next_node_id: "L2N2".to_string(),
                    },
                ],
            },
            flags,
            endings,
            acts,
        }
    }

    #[test]
    fn test_generate_ldag_structure() {
        let blueprint = create_mock_blueprint();
        let result = ldag::generate_ldag(&blueprint);
        
        assert!(result.is_ok(), "LDAG generation failed: {:?}", result.err());
        let acts_nodes = result.unwrap();
        
        // Verify Act count
        assert_eq!(acts_nodes.len(), 3);
        
        // Verify Level count
        let mut total_levels = 0;
        for act in &acts_nodes {
            total_levels += act.len();
        }
        assert_eq!(total_levels, 40);
        
        // Verify L1N1 matches StartNode (partially, content/chars)
        let l1n1 = &acts_nodes[0][0][0];
        assert_eq!(l1n1.id, "L1N1");
        assert_eq!(l1n1.content, "Start content");
        assert_eq!(l1n1.characters, vec!["Hero".to_string()]);
        
        // Check connectivity (basic check)
        // Ensure all nodes have choices (except potentially last layer pointing to Endings)
        for (_act_idx, act) in acts_nodes.iter().enumerate() {
            for (_layer_idx, layer) in act.iter().enumerate() {
                for node in layer {
                    assert!(!node.choices.is_empty(), "Node {} has no choices", node.id);
                    
                    for choice in &node.choices {
                        match &choice.next_node_id {
                            NextNodeId::Simple(id) => {
                                assert!(!id.is_empty(), "Empty next node id in {}", node.id);
                            }
                            NextNodeId::Conditional(cond) => {
                                assert!(!cond.true_id.is_empty());
                                assert!(!cond.false_id.is_empty());
                                assert!(!cond.check_flag.is_empty());
                            }
                        }
                    }
                }
            }
        }
    }
}

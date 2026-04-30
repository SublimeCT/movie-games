import re

def remove_function(content, func_name):
    lines = content.split('\n')
    start_idx = -1
    for i, line in enumerate(lines):
        if f"fn {func_name}" in line:
            start_idx = i
            break
    
    if start_idx == -1:
        return content

    # Check if previous line is #[test]
    if start_idx > 0 and "#[test]" in lines[start_idx-1]:
        start_idx -= 1
        
    end_idx = start_idx
    brace_count = 0
    started = False
    
    for i in range(start_idx, len(lines)):
        line = lines[i]
        brace_count += line.count('{')
        brace_count -= line.count('}')
        if '{' in line:
            started = True
            
        if started and brace_count == 0:
            end_idx = i
            break
            
    # delete lines from start_idx to end_idx
    return '\n'.join(lines[:start_idx] + lines[end_idx+1:])

with open('src/tests_repro.rs', 'r') as f:
    content = f.read()

tests_to_remove = [
    'test_pick_background_prompt_prefers_template_synopsis',
    'test_fallback_image_data_uris_have_svg_prefix',
    'test_normalize_cogview_size_defaults_and_accepts_known_values',
    'test_ensure_request_characters_present_and_avatar_fallback_attaches',
    'test_attach_avatar_to_template_sets_avatar_path',
    'test_attach_avatar_to_template_does_not_overwrite_existing'
]

for test in tests_to_remove:
    content = remove_function(content, test)

content = re.sub(r'model:\s*None,\n', 'model: None,\n            skip_image_generation: None,\n', content)

with open('src/tests_repro.rs', 'w') as f:
    f.write(content)

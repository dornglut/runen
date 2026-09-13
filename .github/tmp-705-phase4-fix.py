from pathlib import Path

p = Path('crates/runen-hir/src/build.rs')
text = p.read_text()
old = '''        accessibility: Accessibility::ModulePrivate,
        type_parameters: Vec::new(),
        parameters: parameters.clone(),
        result,
        safe_reference_result_contract,
        function_type: None,
        body: body_node.clone(),
        location: declaration_location,
'''
new = '''        accessibility: Accessibility::ModulePrivate,
        type_parameters: Vec::new(),
        result,
        safe_reference_result_contract,
        execution: FunctionHeaderExecution::Runen {
            parameters: parameters.clone(),
            function_type: None,
            body: body_node.clone(),
        },
        location: declaration_location,
'''
if text.count(old) != 1:
    raise SystemExit(f'synthetic closure FunctionHeader anchor mismatch: {text.count(old)}')
p.write_text(text.replace(old, new, 1))
print('aligned synthetic closure FunctionHeader with Runen execution origin')

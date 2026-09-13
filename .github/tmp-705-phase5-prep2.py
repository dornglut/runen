from pathlib import Path

p = Path('crates/runen-core-lowering/src/lib.rs')
text = p.read_text()
old = '''    for function in &compilation.functions {
        collect_statement_raw_pointer_types(&function.body.statements, &mut pointees);
    }
'''
new = '''    for function in &compilation.functions {
        for parameter in &function.parameters {
            collect_raw_pointer_type(parameter.ty, &mut pointees);
        }
        if let Some(result) = function.result {
            collect_raw_pointer_type(result, &mut pointees);
        }
        collect_statement_raw_pointer_types(&function.body.statements, &mut pointees);
    }
'''
if text.count(old) != 1:
    raise SystemExit(f'raw-pointer scanner staging anchor mismatch: {text.count(old)}')
p.write_text(text.replace(old, new, 1))
print('normalized temporary raw-pointer scanner anchor for phase5 exact transform')

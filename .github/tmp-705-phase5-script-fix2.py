from pathlib import Path

p = Path('.github/tmp-705-phase5.py')
text = p.read_text()
old = '''old = \'\'\'    for function in &compilation.functions {
        for parameter in &function.parameters {
            collect_raw_pointer_type(parameter.ty, &mut pointees);
        }
        if let Some(result) = function.result {
            collect_raw_pointer_type(result, &mut pointees);
        }
        collect_statement_raw_pointer_types(&function.body.statements, &mut pointees);
    }
\'\'\'
new = \'\'\'    for function in &compilation.functions {
        if let Some(parameters) = function.runen_parameters() {
            for parameter in parameters {
                collect_raw_pointer_type(parameter.ty, &mut pointees);
            }
        }
        if let Some(result) = function.result {
            collect_raw_pointer_type(result, &mut pointees);
        }
        if let Some(body) = function.runen_body() {
            collect_statement_raw_pointer_types(&body.statements, &mut pointees);
        }
    }
\'\'\'
if text.count(old) != 1:
    raise SystemExit(\'raw-pointer type discovery anchor mismatch\')
text = text.replace(old, new, 1)
'''
new = '''old = \'\'\'    for function in &compilation.functions {
        collect_statement_raw_pointer_types(&function.body.statements, &mut pointees);
    }
\'\'\'
new = \'\'\'    for function in &compilation.functions {
        if let Some(body) = function.runen_body() {
            collect_statement_raw_pointer_types(&body.statements, &mut pointees);
        }
    }
\'\'\'
if text.count(old) != 1:
    raise SystemExit(\'raw-pointer type discovery anchor mismatch\')
text = text.replace(old, new, 1)
'''
if text.count(old) != 1:
    raise SystemExit(f'phase5 raw-pointer script anchor mismatch: {text.count(old)}')
p.write_text(text.replace(old, new, 1))
print('aligned phase5 raw-pointer discovery with existing body-only owner')

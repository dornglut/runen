from pathlib import Path

p = Path('crates/runen-core-lowering/src/lib.rs')
text = p.read_text()
old = '''            if let hir::CallTarget::Direct {
                function,
                type_arguments,
            } = target
            {
                specializations.push(specialization_key_for_call(
                    compilation,
                    current,
                    *function,
                    type_arguments,
                )?);
            }
'''
new = '''                if let hir::CallTarget::Direct {
                    function,
                    type_arguments,
                } = target
                {
                    specializations.push(specialization_key_for_call(
                        compilation,
                        current,
                        *function,
                        type_arguments,
                    )?);
                }
'''
if text.count(old) != 1:
    raise SystemExit(f'value-call specialization anchor mismatch: {text.count(old)}')
p.write_text(text.replace(old, new, 1))
print('normalized temporary value-call specialization anchor for phase5 exact transform')

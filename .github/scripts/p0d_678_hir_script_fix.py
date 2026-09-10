from pathlib import Path

path = Path('.github/scripts/p0d_678_hir_patch.py')
text = path.read_text()
old = '''build = replace_once(\n    build,\n    \"\"\"        Type::Parameter(_) | Type::SafeReference { .. } | Type::RawPointer(_) => false,\\n\"\"\",\n    \"\"\"        Type::Parameter(_)\\n        | Type::SafeReference { .. }\\n        | Type::RawPointer(_)\\n        | Type::Function(_) => false,\\n\"\"\",\n    \"raw pointee function exclusion\",\n)\n'''
new = '''build = replace_once(\n    build,\n    \"\"\"fn raw_pointer_pointee_type_is_valid(ty: Type, records: &[Record]) -> bool {\\n    match ty {\\n        Type::Intrinsic(_) => true,\\n        Type::Record(record) => records[record.0]\\n            .fields\\n            .iter()\\n            .all(|field| raw_pointer_pointee_type_is_valid(field.ty, records)),\\n        Type::Parameter(_) | Type::SafeReference { .. } | Type::RawPointer(_) => false,\\n    }\\n}\\n\"\"\",\n    \"\"\"fn raw_pointer_pointee_type_is_valid(ty: Type, records: &[Record]) -> bool {\\n    match ty {\\n        Type::Intrinsic(_) => true,\\n        Type::Record(record) => records[record.0]\\n            .fields\\n            .iter()\\n            .all(|field| raw_pointer_pointee_type_is_valid(field.ty, records)),\\n        Type::Parameter(_)\\n        | Type::SafeReference { .. }\\n        | Type::RawPointer(_)\\n        | Type::Function(_) => false,\\n    }\\n}\\n\"\"\",\n    \"raw pointee function exclusion\",\n)\n'''
assert text.count(old) == 1
path.write_text(text.replace(old, new, 1))
print('narrowed raw-pointer exhaustiveness patch to its owning helper')

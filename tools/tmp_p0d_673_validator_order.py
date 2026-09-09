from pathlib import Path

path = Path("crates/runen-core-ir/src/interprocedural_validation.rs")
text = path.read_text()

old = '''            TypeKind::Scalar(ScalarType::Callable(interface)) => {
                validate_callable_interface(types, interface, &MirLocation::Program)?;
            }
            TypeKind::Scalar(_) | TypeKind::Struct(_) => {}
'''
new = '''            TypeKind::Scalar(ScalarType::Callable(_)) => {}
            TypeKind::Scalar(_) | TypeKind::Struct(_) => {}
'''
if text.count(old) != 1:
    raise SystemExit("callable initial-scan arm not found exactly once")
text = text.replace(old, new, 1)

old = '''        }
    }
    Ok(())
}

fn validate_function_declarations(
'''
new = '''        }
    }

    for index in 0..types.len() {
        let ty = TypeId(u32::try_from(index).expect("type index exceeds u32::MAX"));
        let definition = types
            .get(ty)
            .expect("type index was derived from the type-table length");
        if let TypeKind::Scalar(ScalarType::Callable(interface)) = &definition.kind {
            validate_callable_interface(types, interface, &MirLocation::Program)?;
        }
    }

    Ok(())
}

fn validate_function_declarations(
'''
if text.count(old) != 1:
    raise SystemExit("type-table completion anchor not found exactly once")
text = text.replace(old, new, 1)
path.write_text(text)

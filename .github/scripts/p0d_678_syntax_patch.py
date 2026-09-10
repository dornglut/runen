from pathlib import Path

ROOTS = [
    Path("crates/runen-syntax"),
    Path("crates/runen-hir"),
    Path("crates/runen-core-lowering"),
]

# The syntax-node rename is representation-neutral and mechanically required by
# the accepted neutral Call grammar. Restrict it to Rust code in the frontend/lowerer.
changed_kind_files = []
for root in ROOTS:
    for path in root.rglob("*.rs"):
        text = path.read_text()
        updated = text.replace("SyntaxKind::DirectCall", "SyntaxKind::Call")
        if updated != text:
            path.write_text(updated)
            changed_kind_files.append(str(path))

lib = Path("crates/runen-syntax/src/lib.rs")
text = lib.read_text()
assert text.count("    DirectCall,\n") == 1
assert text.count("41 => SyntaxKind::Call,") == 1, "mechanical SyntaxKind replacement must update raw mapping first"
text = text.replace("    DirectCall,\n", "    Call,\n", 1)
lib.write_text(text)

parser = Path("crates/runen-syntax/src/parser.rs")
text = parser.read_text()
text = text.replace("parse_direct_call_or_field_value_use", "parse_call_or_field_value_use")
text = text.replace("parse_direct_call", "parse_call")

old = '''    fn parse_type(&mut self) {
        self.builder.start_node(SyntaxKind::TypeRef.into());
        if self.eat(SyntaxKind::KwRaw) {
            self.parse_reference_referent_type();
        } else {
            if self.eat(SyntaxKind::Amp) {
                self.eat(SyntaxKind::KwMut);
            }
            self.parse_reference_referent_type();
        }
        self.builder.finish_node();
    }

'''
new = '''    fn parse_type(&mut self) {
        self.builder.start_node(SyntaxKind::TypeRef.into());
        if self.at(SyntaxKind::KwFn) {
            self.parse_function_type();
        } else if self.eat(SyntaxKind::KwRaw) {
            self.parse_reference_referent_type();
        } else {
            if self.eat(SyntaxKind::Amp) {
                self.eat(SyntaxKind::KwMut);
            }
            self.parse_reference_referent_type();
        }
        self.builder.finish_node();
    }

    fn parse_function_type(&mut self) {
        self.expect(SyntaxKind::KwFn, ExpectedSyntax::Type);
        if !self.expect(SyntaxKind::LParen, ExpectedSyntax::LeftParen) {
            return;
        }

        self.bump_trivia();
        let mut missing_close = false;
        while !self.at(SyntaxKind::RParen) && self.current().is_some() {
            if self.at(SyntaxKind::Arrow)
                || self.at(SyntaxKind::Semicolon)
                || self.at(SyntaxKind::RBrace)
                || self.at(SyntaxKind::LBrace)
                || self.at_any(TOP_LEVEL_STARTERS)
            {
                self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::RightParen));
                missing_close = true;
                break;
            }

            self.parse_type();
            if self.eat(SyntaxKind::Comma) {
                self.bump_trivia();
                continue;
            }
            if !self.at(SyntaxKind::RParen) {
                self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::CommaOrRightParen));
                self.recover_until(&[
                    SyntaxKind::Comma,
                    SyntaxKind::RParen,
                    SyntaxKind::Arrow,
                    SyntaxKind::Semicolon,
                    SyntaxKind::RBrace,
                    SyntaxKind::LBrace,
                    SyntaxKind::KwImport,
                    SyntaxKind::KwExport,
                    SyntaxKind::KwFn,
                    SyntaxKind::KwRecord,
                ]);
                self.eat(SyntaxKind::Comma);
            }
            self.bump_trivia();
        }

        if !missing_close {
            self.expect(SyntaxKind::RParen, ExpectedSyntax::RightParen);
        }
        if self.eat(SyntaxKind::Arrow) {
            self.parse_type();
        }
    }

'''
assert text.count(old) == 1
text = text.replace(old, new, 1)
parser.write_text(text)

# The old internal helper terminology must be gone from parser implementation.
assert "parse_direct_call" not in text
assert "SyntaxKind::DirectCall" not in text

# The neutral node rename must be complete in the authorized implementation crates.
for root in ROOTS:
    for path in root.rglob("*.rs"):
        assert "SyntaxKind::DirectCall" not in path.read_text(), path

# Reference referents and generic arguments deliberately remain narrower than Type.
parser_text = parser.read_text()
referent_helper = parser_text.split("const fn is_reference_referent_type_start", 1)[1]
referent_helper = referent_helper.split("const fn is_generic_type_argument_start", 1)[0]
assert "KwFn" not in referent_helper

TEST = r'''use runen_syntax::{Parse, SyntaxKind, parse_source};

fn parse(source: &str) -> Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

fn count(parsed: &Parse, kind: SyntaxKind) -> usize {
    parsed
        .syntax()
        .descendants()
        .filter(|node| node.kind() == kind)
        .count()
}

#[test]
fn call_kind_keeps_existing_numeric_slot_but_is_semantically_neutral() {
    assert_eq!(rowan::SyntaxKind::from(SyntaxKind::Call).0, 41);

    let parsed = parse(
        "fn use(f: fn(I64) -> I64) -> I64 { f(1); return dep::apply[I64](1); }",
    );
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count(&parsed, SyntaxKind::Call), 2);
    assert_eq!(count(&parsed, SyntaxKind::CallStatement), 1);
    assert_eq!(count(&parsed, SyntaxKind::GenericTypeArgumentList), 1);
}

#[test]
fn function_types_are_finite_recursive_type_refs_with_optional_result() {
    let source = "fn use(f: fn(I64, fn(Bool) -> I64,) -> Bool) -> fn(I64) { let g: fn(I64) = f; return g; }";
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(parsed.text(), source);

    let function_type_refs = parsed
        .syntax()
        .descendants()
        .filter(|node| {
            node.kind() == SyntaxKind::TypeRef
                && node
                    .children_with_tokens()
                    .filter_map(|element| element.into_token())
                    .any(|token| token.kind() == SyntaxKind::KwFn)
        })
        .count();
    assert_eq!(function_type_refs, 4);
}

#[test]
fn function_types_do_not_widen_reference_or_generic_argument_grammar() {
    for source in [
        "fn bad(value: &fn(I64)) {}",
        "fn bad(value: raw fn(I64)) {}",
        "fn use() { generic[fn(I64)](); }",
    ] {
        let parsed = parse(source);
        assert!(!parsed.errors().is_empty(), "source unexpectedly accepted: {source}");
        assert_eq!(parsed.text(), source);
    }
}

#[test]
fn arbitrary_postfix_callable_forms_remain_unrepresented() {
    for source in [
        "fn use(f: fn(I64) -> I64) -> I64 { return (f)(1); }",
        "fn use() -> I64 { return make()(1); }",
        "record R { handler: I64 } fn use(r: R) -> I64 { return r.handler(1); }",
    ] {
        let parsed = parse(source);
        assert!(!parsed.errors().is_empty(), "source unexpectedly accepted: {source}");
        assert_eq!(parsed.text(), source);
    }
}

#[test]
fn malformed_function_type_remains_lossless_and_recovers_to_body() {
    let source = "fn bad(f: fn(I64 -> Bool) { return true; }";
    let parsed = parse(source);
    assert!(!parsed.errors().is_empty());
    assert_eq!(parsed.text(), source);
    assert_eq!(count(&parsed, SyntaxKind::Body), 1);
}
'''

test_path = Path("crates/runen-syntax/tests/function_values.rs")
assert not test_path.exists()
test_path.write_text(TEST)

print("mechanically neutralized SyntaxKind in:")
for path in changed_kind_files:
    print(f"  {path}")
print("syntax function-type parser and focused tests staged")

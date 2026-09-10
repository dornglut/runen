from pathlib import Path

path = Path("crates/runen-syntax/src/parser.rs")
text = path.read_text()
old = '''        while !self.at(SyntaxKind::RParen) && self.current().is_some() {
            if self.at(SyntaxKind::Arrow)
                || self.at(SyntaxKind::Semicolon)
                || self.at(SyntaxKind::RBrace)
                || self.at(SyntaxKind::LBrace)
                || self.at_any(TOP_LEVEL_STARTERS)
            {
'''
new = '''        while !self.at(SyntaxKind::RParen) && self.current().is_some() {
            if self.at(SyntaxKind::Arrow)
                || self.at(SyntaxKind::Semicolon)
                || self.at(SyntaxKind::RBrace)
                || self.at(SyntaxKind::LBrace)
                || self.at(SyntaxKind::KwImport)
                || self.at(SyntaxKind::KwExport)
                || self.at(SyntaxKind::KwRecord)
                || (self.at(SyntaxKind::KwFn)
                    && self.peek_nontrivia(1) != Some(SyntaxKind::LParen))
            {
'''
assert text.count(old) == 1
path.write_text(text.replace(old, new, 1))
print("nested function-type KwFn is no longer mistaken for a top-level boundary")

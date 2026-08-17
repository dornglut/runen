use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn main() {
    let mut arguments = env::args().skip(1);
    match (arguments.next().as_deref(), arguments.next()) {
        (Some("validate"), None) => {
            if let Err(error) = validate() {
                eprintln!("validation failed: {error}");
                std::process::exit(1);
            }
        }
        _ => {
            eprintln!("usage: cargo validate");
            std::process::exit(2);
        }
    }
}

fn validate() -> Result<(), String> {
    let root = repository_root()?;
    let before = repository_status(&root)?;

    run_captured(
        &root,
        "Cargo metadata",
        "cargo",
        &["metadata", "--format-version", "1", "--locked", "--no-deps"],
    )?;
    validate_documentation(&root)?;
    run(
        &root,
        "formatting",
        "cargo",
        &["fmt", "--all", "--", "--check"],
    )?;
    run(
        &root,
        "locked workspace tests",
        "cargo",
        &["test", "--workspace", "--all-targets", "--locked"],
    )?;
    run(
        &root,
        "Clippy",
        "cargo",
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--locked",
            "--",
            "-D",
            "warnings",
        ],
    )?;
    run(&root, "diff hygiene", "git", &["diff", "--check"])?;

    let after = repository_status(&root)?;
    if after != before {
        return Err(format!(
            "validation changed repository state\nbefore:\n{}\nafter:\n{}",
            String::from_utf8_lossy(&before),
            String::from_utf8_lossy(&after)
        ));
    }

    println!("repository validation passed");
    Ok(())
}

fn validate_documentation(root: &Path) -> Result<(), String> {
    let mut files = Vec::new();
    collect_markdown_files(root, &mut files)?;

    let spec_root = fs::canonicalize(root.join("spec"))
        .map_err(|error| format!("failed to resolve spec directory: {error}"))?;

    for file in files {
        let content = fs::read_to_string(&file)
            .map_err(|error| format!("failed to read {}: {error}", file.display()))?;
        let is_spec = file.starts_with(root.join("spec"));

        if is_spec {
            validate_spec_content(root, &file, &content)?;
        }

        for target in markdown_link_targets(&content) {
            let Some(local_target) = local_markdown_target(&target) else {
                continue;
            };
            let parent = file
                .parent()
                .ok_or_else(|| format!("{} has no parent directory", file.display()))?;
            let candidate = parent.join(local_target);
            if !candidate.exists() {
                return Err(format!(
                    "broken Markdown link in {}: {target}",
                    file.strip_prefix(root).unwrap_or(&file).display()
                ));
            }

            if is_spec {
                let resolved = fs::canonicalize(&candidate).map_err(|error| {
                    format!(
                        "failed to resolve Markdown link {target} in {}: {error}",
                        file.display()
                    )
                })?;
                if !resolved.starts_with(&spec_root) {
                    return Err(format!(
                        "normative spec link escapes spec/: {} -> {target}",
                        file.strip_prefix(root).unwrap_or(&file).display()
                    ));
                }
            }
        }
    }

    println!("documentation validation passed");
    Ok(())
}

fn collect_markdown_files(directory: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let mut entries = fs::read_dir(directory)
        .map_err(|error| format!("failed to read {}: {error}", directory.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("failed to enumerate {}: {error}", directory.display()))?;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| format!("failed to inspect {}: {error}", path.display()))?;
        if file_type.is_dir() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name != ".git" && name != "target" {
                collect_markdown_files(&path, files)?;
            }
        } else if file_type.is_file()
            && path.extension().and_then(|extension| extension.to_str()) == Some("md")
        {
            files.push(path);
        }
    }

    Ok(())
}

fn validate_spec_content(root: &Path, file: &Path, content: &str) -> Result<(), String> {
    const FORBIDDEN: &[&str] = &[
        "P0-",
        "ROADMAP.md",
        "CONTRIBUTING.md",
        "AGENTS.md",
        "TESTING.md",
        "ARCHITECTURE.md",
        "docs/",
        "crates/",
        "tools/",
        "cargo validate",
        "runen-core-ir",
        "runen-reference",
    ];

    for marker in FORBIDDEN {
        if content.contains(marker) {
            return Err(format!(
                "normative spec contains repository/planning marker {marker:?}: {}",
                file.strip_prefix(root).unwrap_or(file).display()
            ));
        }
    }

    Ok(())
}

fn markdown_link_targets(content: &str) -> Vec<String> {
    let mut targets = Vec::new();
    let mut cursor = 0;

    while let Some(relative_start) = content[cursor..].find("](") {
        let start = cursor + relative_start + 2;
        let Some(relative_end) = content[start..].find(')') else {
            break;
        };
        let end = start + relative_end;
        let raw = content[start..end].trim();
        let target = if raw.starts_with('<') && raw.ends_with('>') {
            &raw[1..raw.len() - 1]
        } else {
            raw.split_whitespace().next().unwrap_or("")
        };
        if !target.is_empty() {
            targets.push(target.to_owned());
        }
        cursor = end + 1;
    }

    targets
}

fn local_markdown_target(target: &str) -> Option<&str> {
    if target.starts_with('#')
        || target.starts_with("http://")
        || target.starts_with("https://")
        || target.starts_with("mailto:")
        || target.starts_with("data:")
    {
        return None;
    }

    let path = target.split('#').next().unwrap_or("");
    (!path.is_empty() && !Path::new(path).is_absolute()).then_some(path)
}

fn repository_root() -> Result<PathBuf, String> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| "xtask manifest must live at <repository>/tools/xtask".to_owned())
}

fn repository_status(root: &Path) -> Result<Vec<u8>, String> {
    Ok(run_captured(
        root,
        "repository status",
        "git",
        &["status", "--porcelain=v1", "--untracked-files=all"],
    )?
    .stdout)
}

fn run(root: &Path, label: &str, program: &str, arguments: &[&str]) -> Result<(), String> {
    let status = Command::new(program)
        .args(arguments)
        .current_dir(root)
        .status()
        .map_err(|error| format!("failed to start {label}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "{label} failed with status {}",
            status
                .code()
                .map_or_else(|| "signal".to_owned(), |code| code.to_string())
        ))
    }
}

fn run_captured(
    root: &Path,
    label: &str,
    program: &str,
    arguments: &[&str],
) -> Result<Output, String> {
    let output = Command::new(program)
        .args(arguments)
        .current_dir(root)
        .output()
        .map_err(|error| format!("failed to start {label}: {error}"))?;
    if output.status.success() {
        Ok(output)
    } else {
        Err(format!(
            "{label} failed with status {}\n{}",
            output
                .status
                .code()
                .map_or_else(|| "signal".to_owned(), |code| code.to_string()),
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::{local_markdown_target, markdown_link_targets};

    #[test]
    fn extracts_inline_markdown_links() {
        assert_eq!(
            markdown_link_targets("[one](a.md) and [two](../b.md#section)"),
            vec!["a.md", "../b.md#section"]
        );
    }

    #[test]
    fn classifies_local_markdown_targets() {
        assert_eq!(local_markdown_target("a.md#section"), Some("a.md"));
        assert_eq!(local_markdown_target("#section"), None);
        assert_eq!(local_markdown_target("https://example.com"), None);
    }
}

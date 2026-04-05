//! Multi-file loading: resolve `import a.b` to paths, stitch sources (imports stripped), single parse + check.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::ast::Module;
use crate::parser::Parser;
use crate::SymError;

/// Options for loading a program from disk.
#[derive(Clone, Debug, Default)]
pub struct LoadOptions {
    /// If set, `stdlib/prelude.sym` (or this path + `/prelude.sym`) is not prepended.
    pub no_prelude: bool,
    /// Root for fallback resolution (`import x.y` tries `stdlib/x/y.sym` here). Default: cwd + `stdlib`.
    pub stdlib_root: Option<PathBuf>,
}

fn import_to_path(base: &Path, segments: &[String]) -> PathBuf {
    let mut p = base.to_path_buf();
    for (i, s) in segments.iter().enumerate() {
        if i + 1 == segments.len() {
            p.push(format!("{s}.sym"));
        } else {
            p.push(s);
        }
    }
    p
}

fn resolve_import_path(
    parent_file: &Path,
    segments: &[String],
    stdlib_root: Option<&Path>,
) -> Result<PathBuf, SymError> {
    let parent_dir = parent_file.parent().unwrap_or_else(|| Path::new("."));
    let rel = import_to_path(parent_dir, segments);
    if rel.is_file() {
        return rel.canonicalize().map_err(|e| SymError::Io(e.to_string()));
    }
    if let Some(root) = stdlib_root {
        let st = import_to_path(root, segments);
        if st.is_file() {
            return st.canonicalize().map_err(|e| SymError::Io(e.to_string()));
        }
    }
    Err(SymError::ImportNotFound {
        parent: parent_file.display().to_string(),
        segments: segments.join("."),
    })
}

/// Remove leading `import` / `module` lines so stitched files form one module.
fn strip_top_decl_lines(source: &str) -> String {
    source
        .lines()
        .filter(|l| {
            let t = l.trim_start();
            !(t.starts_with("import ") || t.starts_with("module "))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn visit_postorder(
    path: &Path,
    stdlib_root: Option<&Path>,
    seen: &mut HashSet<PathBuf>,
    order: &mut Vec<PathBuf>,
) -> Result<(), (SymError, Option<String>)> {
    let canon = path
        .canonicalize()
        .map_err(|e| (SymError::Io(e.to_string()), None))?;
    if !seen.insert(canon.clone()) {
        return Ok(());
    }
    let src = fs::read_to_string(path).map_err(|e| (SymError::Io(e.to_string()), None))?;
    let tokens = crate::lexer::lex(&src).map_err(|e| (SymError::Lex(e), None))?;
    let mut p = Parser::new(&src, tokens);
    let module = p.parse_module().map_err(|e| (SymError::Parse(e), None))?;
    for imp in &module.imports {
        let child = resolve_import_path(path, &imp.path, stdlib_root).map_err(|e| (e, None))?;
        visit_postorder(&child, stdlib_root, seen, order)?;
    }
    order.push(path.to_path_buf());
    Ok(())
}

/// Load entry file + dependencies + optional prelude, return a single module and stitched source (for diagnostics).
/// On failure, the optional string is the **stitched** source when failure happened after stitching (better line numbers).
pub fn load_and_check(
    entry: &Path,
    opts: &LoadOptions,
) -> Result<(Module, String), (SymError, Option<String>)> {
    let cwd = std::env::current_dir().map_err(|e| (SymError::Io(e.to_string()), None))?;
    let stdlib_root = opts
        .stdlib_root
        .clone()
        .unwrap_or_else(|| cwd.join("stdlib"));

    let mut order = Vec::new();
    let mut seen = HashSet::new();

    if !opts.no_prelude {
        let prelude = stdlib_root.join("prelude.sym");
        if prelude.is_file() {
            visit_postorder(&prelude, Some(&stdlib_root), &mut seen, &mut order)?;
        }
    }

    visit_postorder(entry, Some(&stdlib_root), &mut seen, &mut order)?;

    let mut stitched = String::new();
    for path in &order {
        let raw = fs::read_to_string(path).map_err(|e| (SymError::Io(e.to_string()), None))?;
        let cleaned = strip_top_decl_lines(&raw);
        stitched.push_str("# sym:file ");
        stitched.push_str(&path.display().to_string());
        stitched.push('\n');
        stitched.push_str(cleaned.trim());
        stitched.push_str("\n\n");
    }

    let module = match crate::parse_and_check(&stitched) {
        Ok(m) => m,
        Err(e) => return Err((e, Some(stitched))),
    };
    Ok((module, stitched))
}

//! Reserved compiler-provided calls (must not be redefined as Sym functions).

pub const NAMES: &[&str] = &[
    "println",
    "eprintln",
    "exit",
    "concat",
    "string_from_int",
    "strlen",
    "read_line",
    "assert",
    "parse_int",
    "env_get",
    "read_file",
    "write_file",
    "write_file_ok",
    "list_dir",
    "glob_files",
    "shell_exec",
    "trim",
    "starts_with",
    "substring",
    "index_of",
    "http_post",
    "http_post_sse_fold",
    "stdout_print",
    "json_string",
    "json_extract",
    "json_value",
];

pub fn is_reserved(name: &str) -> bool {
    NAMES.contains(&name)
}

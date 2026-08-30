use leptos::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptimizationPass {
    pub name: String,
    pub ir: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Cfg {
    pub function_name: String,
    pub dot_content: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompileResult {
    pub initial_ir: String,
    pub passes: Vec<OptimizationPass>,
    pub assembly: String,
    pub cfgs: Vec<Cfg>,
}

/// Maximum accepted source size in bytes. (Only referenced from the
/// SSR-compiled server function body.)
#[cfg(feature = "ssr")]
const MAX_CODE_LEN: usize = 100_000;
/// Maximum accepted length of the extra clang flags string.
const MAX_EXTRA_FLAGS_LEN: usize = 2_000;
/// Maximum accepted length of the custom opt pipeline string.
const MAX_PIPELINE_LEN: usize = 500;

pub fn validate_opt_level(opt_level: &str) -> Result<(), String> {
    match opt_level {
        "O0" | "O1" | "O2" | "O3" => Ok(()),
        other => Err(format!("unsupported optimization level: {:?}", other)),
    }
}

pub fn validate_language(language: &str) -> Result<(), String> {
    match language {
        "c" | "cpp" => Ok(()),
        other => Err(format!("unsupported language: {:?}", other)),
    }
}

/// Splits a whitespace-separated string of extra clang flags and rejects
/// anything that could redirect the compiler's output or smuggle shell
/// metacharacters into the process arguments. (Limitation: quoted arguments
/// containing spaces are not supported.)
pub fn validate_extra_flags(flags: &str) -> Result<Vec<String>, String> {
    if flags.len() > MAX_EXTRA_FLAGS_LEN {
        return Err(format!(
            "extra flags too long (max {} characters)",
            MAX_EXTRA_FLAGS_LEN
        ));
    }
    let mut out = Vec::new();
    for token in flags.split_whitespace() {
        if token.starts_with("-o") {
            return Err(format!(
                "flag {:?} is not allowed (the output file is managed by the server)",
                token
            ));
        }
        if token.contains('\\') || token.contains(';') {
            return Err(format!(
                "flag {:?} contains a forbidden character",
                token
            ));
        }
        out.push(token.to_string());
    }
    Ok(out)
}

/// Validates a custom opt pass pipeline. An empty string means "use the
/// default pipeline for the selected optimization level".
pub fn validate_pipeline(pipeline: &str) -> Result<String, String> {
    let trimmed = pipeline.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }
    if trimmed.len() > MAX_PIPELINE_LEN {
        return Err(format!(
            "custom pipeline too long (max {} characters)",
            MAX_PIPELINE_LEN
        ));
    }
    if trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | ',' | '<' | '>' | '(' | ')' | '_' | '+' | '-'))
    {
        Ok(trimmed.to_string())
    } else {
        Err(format!(
            "custom pipeline contains forbidden characters: {:?}",
            trimmed
        ))
    }
}

/// Resolves a tool binary name. If `LLVM_BIN_DIR` is set, the binary is
/// looked up inside that directory (with the platform executable suffix);
/// otherwise the bare name is returned and resolved through `PATH`.
#[cfg(feature = "ssr")]
fn tool_path(name: &str) -> String {
    match std::env::var("LLVM_BIN_DIR") {
        Ok(dir) if !dir.trim().is_empty() => {
            let path = std::path::Path::new(dir.trim())
                .join(format!("{}{}", name, std::env::consts::EXE_SUFFIX));
            path.to_string_lossy().into_owned()
        }
        _ => name.to_string(),
    }
}

#[server(CompileAndOptimize, "/api")]
pub async fn compile_and_optimize(
    code: String,
    opt_level: String,
    language: String,
    extra_flags: String,
    custom_pipeline: String,
) -> Result<CompileResult, ServerFnError> {
    use std::process::Command;
    use std::fs;
    use uuid::Uuid;

    if code.len() > MAX_CODE_LEN {
        return Err(ServerFnError::ServerError("Code too large".into()));
    }
    let flags = match validate_extra_flags(&extra_flags) {
        Ok(v) => v,
        Err(e) => return Err(ServerFnError::ServerError(e)),
    };
    let pipeline = match validate_pipeline(&custom_pipeline) {
        Ok(v) => v,
        Err(e) => return Err(ServerFnError::ServerError(e)),
    };
    if let Err(e) = validate_opt_level(&opt_level) {
        return Err(ServerFnError::ServerError(e));
    }
    if let Err(e) = validate_language(&language) {
        return Err(ServerFnError::ServerError(e));
    }

    let (driver_name, lang_flag, extension) = if language == "c" {
        ("clang", "c", "c")
    } else {
        ("clang++", "c++", "cpp")
    };
    let driver = tool_path(driver_name);

    let id = Uuid::new_v4().to_string();
    let temp_dir = std::env::temp_dir().join(format!("llvm-explorer-{}", id));
    match fs::create_dir_all(&temp_dir) {
        Ok(_) => {}
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    }

    let input_src = temp_dir.join(format!("input.{}", extension));
    match fs::write(&input_src, &code) {
        Ok(_) => {}
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    }

    let output_ll = temp_dir.join("output.ll");
    let output_s = temp_dir.join("output.s");

    let mut clang_ir_cmd = Command::new(&driver);
    clang_ir_cmd
        .args(["-S", "-emit-llvm", "-O0", "-x", lang_flag])
        .args(&flags)
        .arg(input_src.to_str().unwrap())
        .args(["-o", output_ll.to_str().unwrap()]);
    let clang_ir = match clang_ir_cmd.output() {
        Ok(v) => v,
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    };

    if !clang_ir.status.success() {
        let err = String::from_utf8_lossy(&clang_ir.stderr);
        let _ = fs::remove_dir_all(&temp_dir);
        return Err(ServerFnError::ServerError(format!("Clang failed: {}", err)));
    }

    let initial_ir = fs::read_to_string(&output_ll).unwrap_or_default();

    let mut clang_asm_cmd = Command::new(&driver);
    clang_asm_cmd
        .args(["-S", &format!("-{}", opt_level), "-x", lang_flag])
        .args(&flags)
        .arg(input_src.to_str().unwrap())
        .args(["-o", output_s.to_str().unwrap()]);
    let clang_asm = match clang_asm_cmd.output() {
        Ok(v) => v,
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    };

    let assembly = if clang_asm.status.success() {
        fs::read_to_string(&output_s).unwrap_or_default()
    } else {
        String::from_utf8_lossy(&clang_asm.stderr).into_owned()
    };

    let passes_spec = if pipeline.is_empty() {
        format!("default<{}>", opt_level)
    } else {
        pipeline.clone()
    };

    let opt_cmd = match Command::new(tool_path("opt"))
        .args([
            &format!("-passes={}", passes_spec),
            "-print-after-all",
            "-disable-output",
            output_ll.to_str().unwrap(),
        ])
        .output() {
            Ok(v) => v,
            Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
        };

    if !opt_cmd.status.success() {
        let err = String::from_utf8_lossy(&opt_cmd.stderr);
        let _ = fs::remove_dir_all(&temp_dir);
        return Err(ServerFnError::ServerError(format!("opt failed: {}", err)));
    }

    let opt_stderr = String::from_utf8_lossy(&opt_cmd.stderr);

    let mut passes = Vec::new();
    let mut current_pass_name = String::new();
    let mut current_ir = String::new();

    for line in opt_stderr.lines() {
        let stripped = line.strip_prefix("; ").unwrap_or(line);
        if stripped.starts_with("*** IR Dump After ") && stripped.ends_with(" ***") {
            if !current_pass_name.is_empty() {
                passes.push(OptimizationPass {
                    name: current_pass_name.clone(),
                    ir: current_ir.trim().to_string(),
                });
            }
            current_pass_name = stripped
                .trim_start_matches("*** IR Dump After ")
                .trim_end_matches(" ***")
                .to_string();
            current_ir.clear();
        } else {
            current_ir.push_str(line);
            current_ir.push('\n');
        }
    }

    if !current_pass_name.is_empty() {
        passes.push(OptimizationPass {
            name: current_pass_name,
            ir: current_ir.trim().to_string(),
        });
    }

    let _cfg_cmd = match Command::new(tool_path("opt"))
        .args([
            "-passes=dot-cfg",
            "-disable-output",
            output_ll.to_str().unwrap(),
        ])
        .current_dir(&temp_dir)
        .output() {
            Ok(v) => v,
            Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
        };

    let mut cfgs = Vec::new();
    if let Ok(entries) = fs::read_dir(&temp_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("dot") {
                let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                let function_name = file_name.trim_start_matches('.').trim_end_matches(".dot").to_string();
                if let Ok(dot_out) = Command::new(tool_path("dot"))
                    .args(["-Tsvg", path.to_str().unwrap()])
                    .output()
                {
                    if dot_out.status.success() {
                        cfgs.push(Cfg {
                            function_name,
                            dot_content: String::from_utf8_lossy(&dot_out.stdout).to_string(),
                        });
                    }
                }
            }
        }
    }

    let _ = fs::remove_dir_all(&temp_dir);

    Ok(CompileResult {
        initial_ir,
        passes,
        assembly,
        cfgs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opt_level_whitelist() {
        for lvl in ["O0", "O1", "O2", "O3"] {
            assert!(validate_opt_level(lvl).is_ok());
        }
        assert!(validate_opt_level("O4").is_err());
        assert!(validate_opt_level("o2").is_err());
        assert!(validate_opt_level("").is_err());
        assert!(validate_opt_level("-O2").is_err());
    }

    #[test]
    fn language_whitelist() {
        assert!(validate_language("c").is_ok());
        assert!(validate_language("cpp").is_ok());
        assert!(validate_language("rust").is_err());
        assert!(validate_language("").is_err());
    }

    #[test]
    fn extra_flags_split_and_reject() {
        assert_eq!(validate_extra_flags("").unwrap(), Vec::<String>::new());
        assert_eq!(
            validate_extra_flags(" -std=c++20   -march=native ").unwrap(),
            vec!["-std=c++20".to_string(), "-march=native".to_string()]
        );
        assert!(validate_extra_flags("-o evil").is_err());
        assert!(validate_extra_flags("-ofile.ll").is_err());
        assert!(validate_extra_flags("-DX=1;-DX=2").is_err());
        assert!(validate_extra_flags("-DA=\"back\\slash\"").is_err());
        assert!(validate_extra_flags(&" ".repeat(MAX_EXTRA_FLAGS_LEN + 1)).is_err());
    }

    #[test]
    fn pipeline_validation() {
        assert_eq!(validate_pipeline("").unwrap(), "");
        assert_eq!(
            validate_pipeline("  mem2reg,instcombine ").unwrap(),
            "mem2reg,instcombine"
        );
        assert_eq!(validate_pipeline("default<O2>").unwrap(), "default<O2>");
        assert_eq!(
            validate_pipeline("function(sroa),loop-vectorize").unwrap(),
            "function(sroa),loop-vectorize"
        );
        assert!(validate_pipeline("foo;bar").is_err());
        assert!(validate_pipeline(&"a".repeat(MAX_PIPELINE_LEN + 1)).is_err());
    }
}

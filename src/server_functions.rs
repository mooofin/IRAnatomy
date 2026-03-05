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

#[server(CompileAndOptimize, "/api")]
pub async fn compile_and_optimize(code: String, opt_level: String) -> Result<CompileResult, ServerFnError> {
    use std::process::Command;
    use std::fs;
    use uuid::Uuid;

    // Limit code size
    if code.len() > 100_000 {
        return Err(ServerFnError::ServerError("Code too large".into()));
    }

    let id = Uuid::new_v4().to_string();
    let temp_dir = std::env::temp_dir().join(format!("llvm-explorer-{}", id));
    match fs::create_dir_all(&temp_dir) {
        Ok(_) => {}
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    }

    let input_cpp = temp_dir.join("input.cpp");
    match fs::write(&input_cpp, &code) {
        Ok(_) => {}
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    }

    let output_ll = temp_dir.join("output.ll");
    let output_s = temp_dir.join("output.s");

    // 1. Generate initial LLVM IR (O0)
    let clang_ir = match Command::new("clang++")
        .args(["-S", "-emit-llvm", "-O0", input_cpp.to_str().unwrap(), "-o", output_ll.to_str().unwrap()])
        .output() {
            Ok(v) => v,
            Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
        };

    if !clang_ir.status.success() {
        let err = String::from_utf8_lossy(&clang_ir.stderr);
        let _ = fs::remove_dir_all(&temp_dir);
        return Err(ServerFnError::ServerError(format!("Clang failed: {}", err)));
    }

    let initial_ir = fs::read_to_string(&output_ll).unwrap_or_default();

    // 2. Generate Assembly
    let clang_asm = match Command::new("clang++")
        .args(["-S", &format!("-{}", opt_level), input_cpp.to_str().unwrap(), "-o", output_s.to_str().unwrap()])
        .output() {
            Ok(v) => v,
            Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
        };

    let assembly = if clang_asm.status.success() {
        fs::read_to_string(&output_s).unwrap_or_default()
    } else {
        String::from_utf8_lossy(&clang_asm.stderr).into_owned()
    };

    // 3. Run Opt and get print-after-all
    let opt_passes = format!("default<{}>", opt_level);
    let opt_cmd = match Command::new("opt")
        .args([
            &format!("-passes={}", opt_passes),
            "-print-after-all",
            "-disable-output",
            output_ll.to_str().unwrap(),
        ])
        .output() {
            Ok(v) => v,
            Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
        };

    // Opt prints `print-after-all` output to stderr
    let opt_stderr = String::from_utf8_lossy(&opt_cmd.stderr);

    // Parse stages
    let mut passes = Vec::new();
    let mut current_pass_name = String::new();
    let mut current_ir = String::new();

    for line in opt_stderr.lines() {
        // LLVM <15 uses "*** IR Dump After ...", LLVM 15+ prefixes with "; "
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

    // 4. Generate CFG
    let _cfg_cmd = match Command::new("opt")
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
                // Files are named like .foo.dot, so we extract foo
                let function_name = file_name.trim_start_matches('.').trim_end_matches(".dot").to_string();
                if let Ok(dot_out) = Command::new("dot")
                    .args(["-Tsvg", path.to_str().unwrap()])
                    .output()
                {
                    if dot_out.status.success() {
                        cfgs.push(Cfg {
                            function_name,
                            dot_content: String::from_utf8_lossy(&dot_out.stdout).to_string(), // now contains SVG
                        });
                    }
                }
            }
        }
    }

    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir);

    Ok(CompileResult {
        initial_ir,
        passes,
        assembly,
        cfgs,
    })
}

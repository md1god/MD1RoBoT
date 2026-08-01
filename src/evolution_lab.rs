use crate::protocol::Suggestion;
use crate::db::Phenotype;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

pub struct EvolutionLab;

impl EvolutionLab {
    pub fn run_experiment(project_dir: &str, suggestion: &Suggestion) -> Result<(bool, Option<Phenotype>, Vec<String>), String> {
        let temp_dir = format!("./temp_lab_{}", suggestion.id);
        copy_dir(project_dir, &temp_dir).map_err(|e| format!("Copy project failed: {e}"))?;

        let target = Path::new(&temp_dir).join(&suggestion.file_path);
        let content = fs::read_to_string(&target).map_err(|e| format!("Read file: {e}"))?;
        if let Some(_pos) = content.find(&suggestion.original_snippet) {
            let new_content = content.replace(&suggestion.original_snippet, &suggestion.new_snippet);
            fs::write(&target, new_content).map_err(|e| format!("Write modification: {e}"))?;
        } else {
            return Err("Original snippet not found".to_string());
        }

        let (success, phenotype, errors) = Self::build_and_test(&temp_dir, &suggestion.language);
        let _ = fs::remove_dir_all(&temp_dir);
        Ok((success, phenotype, errors))
    }

    fn build_and_test(dir: &str, language: &str) -> (bool, Option<Phenotype>, Vec<String>) {
        let start = Instant::now();
        let (build_ok, errors) = match language {
            "rust" => {
                let out = Command::new("cargo").args(["check"]).current_dir(dir).output();
                match out {
                    Ok(o) => (o.status.success(), if !o.status.success() { vec![String::from_utf8_lossy(&o.stderr).to_string()] } else { vec![] }),
                    Err(e) => (false, vec![e.to_string()]),
                }
            },
            "python" => {
                let out = Command::new("python3").args(["-m", "py_compile"]).arg(dir).output();
                match out {
                    Ok(o) => (o.status.success(), if !o.status.success() { vec![String::from_utf8_lossy(&o.stderr).to_string()] } else { vec![] }),
                    Err(e) => (false, vec![e.to_string()]),
                }
            },
            "javascript" => {
                let out = Command::new("node").args(["--check"]).arg(dir).output();
                match out {
                    Ok(o) => (o.status.success(), if !o.status.success() { vec![String::from_utf8_lossy(&o.stderr).to_string()] } else { vec![] }),
                    Err(e) => (false, vec![e.to_string()]),
                }
            },
            "c" | "cpp" => {
                let out = Command::new("gcc").args(["-fsyntax-only"]).arg(dir).output();
                match out {
                    Ok(o) => (o.status.success(), if !o.status.success() { vec![String::from_utf8_lossy(&o.stderr).to_string()] } else { vec![] }),
                    Err(e) => (false, vec![e.to_string()]),
                }
            },
            _ => {
                let out = Command::new("semgrep").args(["--config=auto", dir]).output();
                match out {
                    Ok(o) => {
                        let success = o.status.success();
                        let errors = if !success { vec![String::from_utf8_lossy(&o.stderr).to_string()] } else { vec![] };
                        (success, errors)
                    },
                    Err(e) => (false, vec![e.to_string()]),
                }
            }
        };

        let build_time = start.elapsed().as_millis() as u64;
        let phenotype = if build_ok {
            Some(Phenotype {
                search_speed_ms: 0,
                memory_usage_mb: 0,
                error_rate: if errors.is_empty() { 0.0 } else { 1.0 },
                build_time_ms: build_time,
            })
        } else { None };

        (build_ok, phenotype, errors)
    }
}

fn copy_dir(src: &str, dst: &str) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = Path::new(dst).join(entry.file_name());
        if ty.is_dir() {
            if entry.file_name() != "target" && entry.file_name() != "temp_lab" {
                copy_dir(&src_path.to_string_lossy(), &dst_path.to_string_lossy())?;
            }
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

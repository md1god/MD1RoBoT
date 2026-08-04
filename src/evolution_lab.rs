use crate::protocol::Suggestion;
use crate::db::Phenotype;
use std::process::{Command, Stdio};
use std::time::Instant;
use std::fs;
use std::path::Path;
use uuid::Uuid;

pub struct EvolutionLab;

impl EvolutionLab {
    /// تشغيل تجربة مباشرة في البيئة الحالية (بدون Docker) لتناسب Render.
    pub fn run_experiment(project_dir: &str, suggestion: &Suggestion) -> Result<(bool, Option<Phenotype>, Vec<String>), String> {
        // 1. نسخ المشروع إلى مجلد مؤقت لعزل التجربة
        let temp_host_dir = format!("./temp_lab_{}", suggestion.id);
        copy_dir(project_dir, &temp_host_dir).map_err(|e| format!("Copy project failed: {e}"))?;

        // 2. تطبيق الطفرة على الملف المستهدف داخل المجلد المؤقت
        let target_in_temp = Path::new(&temp_host_dir).join(&suggestion.file_path);
        
        // التأكد من وجود المجلدات الأب للملف إذا كان جديداً
        if let Some(parent) = target_in_temp.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Create parent dirs: {e}"))?;
        }

        let content = fs::read_to_string(&target_in_temp).unwrap_or_default();
        if !content.is_empty() && content.contains(&suggestion.original_snippet) {
            let new_content = content.replace(&suggestion.original_snippet, &suggestion.new_snippet);
            fs::write(&target_in_temp, new_content).map_err(|e| format!("Write mutation: {e}"))?;
        } else if suggestion.language != "shell" && suggestion.language != "asm" && !content.is_empty() {
            let _ = fs::remove_dir_all(&temp_host_dir);
            return Err("Original snippet not found in existing file".to_string());
        } else {
            // للغات الحرة أو الملفات الجديدة
            fs::write(&target_in_temp, &suggestion.new_snippet)
                .map_err(|e| format!("Write freeform mutation: {e}"))?;
        }

        // 3. إعداد أوامر البناء/التشغيل
        let (build_cmd, test_cmd) = Self::commands_for_language(&suggestion.language, &suggestion.file_path);

        // 4. تشغيل الأوامر محلياً
        let start = Instant::now();
        let (success, stdout, stderr) = Self::run_locally(
            &temp_host_dir,
            &build_cmd,
            &test_cmd,
        )?;

        let build_time = start.elapsed().as_millis() as u64;
        let phenotype = if success {
            Some(Phenotype {
                search_speed_ms: 0,
                memory_usage_mb: 0,
                error_rate: 0.0,
                build_time_ms: build_time,
            })
        } else {
            None
        };

        let errors = if !success {
            vec![format!("Execution output:\nSTDOUT: {}\nSTDERR: {}", stdout, stderr)]
        } else {
            vec![]
        };

        // 5. تنظيف
        let _ = fs::remove_dir_all(&temp_host_dir);

        Ok((success, phenotype, errors))
    }

    /// تحديد أوامر البناء والاختبار حسب اللغة.
    fn commands_for_language(language: &str, file_path: &str) -> (Vec<String>, Vec<String>) {
        match language {
            "rust" => (
                vec!["cargo check".into()],
                vec![],
            ),
            "python" => (
                vec![format!("python3 -m py_compile {}", file_path)],
                vec![],
            ),
            "javascript" => (
                vec![format!("node --check {}", file_path)],
                vec![],
            ),
            "c" | "cpp" => (
                vec![format!("gcc -fsyntax-only {}", file_path)],
                vec![],
            ),
            "shell" => (
                vec![format!("chmod +x {} && ./{}", file_path, file_path)],
                vec![],
            ),
            _ => (
                vec!["ls -l".into()], // أمر بسيط للتحقق من الوجود كخيار افتراضي
                vec![],
            ),
        }
    }

    /// تنفيذ الأوامر مباشرة في المجلد المؤقت.
    fn run_locally(
        work_dir: &str,
        build_cmds: &[String],
        test_cmds: &[String],
    ) -> Result<(bool, String, String), String> {
        let mut all_cmds = build_cmds.to_vec();
        all_cmds.extend_from_slice(test_cmds);
        let joined_cmds = all_cmds.join(" && ");

        let output = Command::new("sh")
            .arg("-c")
            .arg(&joined_cmds)
            .current_dir(work_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| format!("Local execution failed: {e}"))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok((output.status.success(), stdout, stderr))
    }
}

fn copy_dir(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let name = entry.file_name();
        
        // تجاهل المجلدات الكبيرة والمؤقتة
        if name == "target" || name == ".git" || name.to_string_lossy().starts_with("temp_lab_") {
            continue;
        }

        if ty.is_dir() {
            copy_dir(entry.path(), dst.as_ref().join(name))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(name))?;
        }
    }
    Ok(())
}

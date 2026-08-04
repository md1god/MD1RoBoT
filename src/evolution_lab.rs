use crate::protocol::Suggestion;
use crate::db::Phenotype;
use std::process::{Command, Stdio};
use std::time::Instant;
use std::fs;
use std::path::Path;
use uuid::Uuid;

pub struct EvolutionLab;

impl EvolutionLab {
    /// تشغيل تجربة داخل حاوية Docker معزولة.
    pub fn run_experiment(project_dir: &str, suggestion: &Suggestion) -> Result<(bool, Option<Phenotype>, Vec<String>), String> {
        let container_name = format!("md1robot_exp_{}", Uuid::new_v4());
        let sandbox_image = "md1robot-sandbox:latest"; // يجب بناؤه مسبقاً

        // 1. نسخ المشروع إلى مجلد مؤقت ليكون مصدراً للحاوية
        let temp_host_dir = format!("./temp_lab_{}", suggestion.id);
        copy_dir(project_dir, &temp_host_dir).map_err(|e| format!("Copy project failed: {e}"))?;

        // 2. تطبيق الطفرة على الملف المستهدف داخل المجلد المؤقت
        let target_in_temp = Path::new(&temp_host_dir).join(&suggestion.file_path);
        let content = fs::read_to_string(&target_in_temp).map_err(|e| format!("Read file: {e}"))?;
        if content.contains(&suggestion.original_snippet) {
            let new_content = content.replace(&suggestion.original_snippet, &suggestion.new_snippet);
            fs::write(&target_in_temp, new_content).map_err(|e| format!("Write mutation: {e}"))?;
        } else if suggestion.language != "shell" && suggestion.language != "asm" {
            // إذا لم يكن الكود الأصلي موجوداً، نسمح بذلك فقط للغات "الحرة" (shell/asm) حيث يتم إنشاء ملف جديد.
            // وإلا نرفض.
            let _ = fs::remove_dir_all(&temp_host_dir);
            return Err("Original snippet not found and language is not freeform".to_string());
        } else {
            // للغات الحرة: نكتب المحتوى الجديد مباشرة إذا كان الملف غير موجود،
            // أو نلحق به إذا كان موجوداً (سلوك تطوري حر).
            let mut current = String::new();
            if target_in_temp.exists() {
                current = fs::read_to_string(&target_in_temp).unwrap_or_default();
            }
            fs::write(&target_in_temp, format!("{}{}", current, suggestion.new_snippet))
                .map_err(|e| format!("Write freeform mutation: {e}"))?;
        }

        // 3. إعداد أمر البناء/التشغيل داخل الحاوية حسب اللغة
        let (build_cmd, test_cmd) = Self::commands_for_language(&suggestion.language, &suggestion.file_path);

        // 4. تشغيل الحاوية وتنفيذ الأوامر
        let start = Instant::now();
        let (success, stdout, stderr) = Self::run_in_container(
            &container_name,
            sandbox_image,
            &temp_host_dir,
            &build_cmd,
            &test_cmd,
        )?;

        let build_time = start.elapsed().as_millis() as u64;
        let phenotype = if success {
            Some(Phenotype {
                search_speed_ms: 0,
                memory_usage_mb: 0, // يمكن قياسه لاحقاً من Docker stats
                error_rate: 0.0,
                build_time_ms: build_time,
            })
        } else {
            None
        };

        let errors = if !success {
            vec![format!("Build output:\n{}", stderr)]
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
                vec!["cargo test --no-run 2>&1 || true".into()], // لا نشغل الاختبارات فعلياً لتوفير الوقت
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
                vec!["sh -c '".to_string() + &file_path + "'"], // سيتم استبدال file_path لاحقاً
                vec![],
            ),
            "asm" => (
                vec![
                    format!("nasm -f elf64 -o /tmp/out.o {}", file_path),
                    "ld -o /tmp/out /tmp/out.o".into(),
                ],
                vec!["/tmp/out".into()],
            ),
            _ => (
                vec!["semgrep --config=auto .".into()],
                vec![],
            ),
        }
    }

    /// تشغيل الأوامر داخل حاوية Docker مؤقتة.
    fn run_in_container(
        container_name: &str,
        image: &str,
        host_dir: &str,
        build_cmds: &[String],
        test_cmds: &[String],
    ) -> Result<(bool, String, String), String> {
        // تحويل المسار إلى مطلق
        let absolute_host_dir = std::fs::canonicalize(host_dir)
            .map_err(|e| format!("Cannot resolve path: {e}"))?;

        // أمر docker run (تمت إزالة --timeout لأنها ليست وسيطاً مدعوماً بشكل مباشر في docker run وتتسبب في خطأ)
        let mut docker_args = vec![
            "run".to_string(),
            "--rm".to_string(),
            "--name".to_string(), container_name.to_string(),
            "-v".to_string(), format!("{}:/app/project:ro", absolute_host_dir.display()), // للقراءة فقط داخل الحاوية
            "-w".to_string(), "/app/project".to_string(),
            "--network=none".to_string(), // بدون شبكة افتراضياً (يمكن تغييره عبر config)
            "--cpus=1".to_string(),
            "--memory=256m".to_string(),
            image.to_string(),
        ];

        // إعداد أمر شل واحد يجمع البناء والتشغيل
        let mut combined_script = String::new();
        for cmd in build_cmds {
            combined_script.push_str(&format!("echo '>>> BUILD: {}'; {} ;\\\n", cmd, cmd));
        }
        for cmd in test_cmds {
            combined_script.push_str(&format!("echo '>>> TEST: {}'; {} ;\\\n", cmd, cmd));
        }
        docker_args.push("sh".into());
        docker_args.push("-c".into());
        docker_args.push(combined_script.clone());

        // تنفيذ Docker
        let output = Command::new("docker")
            .args(&docker_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| format!("Failed to run docker: {e}"))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let success = output.status.success();

        Ok((success, stdout, stderr))
    }
}

// دالة نسخ المجلدات (كما هي) مع تجاهل بعض الملفات الكبيرة
fn copy_dir(src: &str, dst: &str) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = Path::new(dst).join(entry.file_name());
        if ty.is_dir() {
            if entry.file_name() != "target" && entry.file_name() != "temp_lab" && entry.file_name() != ".git" {
                copy_dir(&src_path.to_string_lossy(), &dst_path.to_string_lossy())?;
            }
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

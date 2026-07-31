use rusqlite::{params, Connection};
use serde_json::Value;
use sha2::{Sha256, Digest};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

// ========== هياكل ==========
#[derive(Clone)]
struct Genome {
    id: String,
    parent_id: Option<String>,
    generation: u64,
    code: String,
    fitness: f64,
}

struct Memory {
    db: Connection,
}

impl Memory {
    fn new(path: &str) -> Self {
        let db = Connection::open(path).unwrap();
        db.execute_batch("CREATE TABLE IF NOT EXISTS genomes (id TEXT PRIMARY KEY, parent_id TEXT, generation INTEGER, code TEXT, fitness REAL);").unwrap();
        Memory { db }
    }
    fn save(&self, g: &Genome) {
        self.db.execute("INSERT OR REPLACE INTO genomes VALUES (?1,?2,?3,?4,?5)", 
            params![g.id, g.parent_id, g.generation, g.code, g.fitness]).ok();
    }
    fn best_fitness(&self) -> f64 {
        self.db.query_row("SELECT MAX(fitness) FROM genomes", [], |r| r.get(0)).unwrap_or(0.0)
    }
}

// ========== لياقة حقيقية ==========
fn evaluate(code: &str, sandbox: &str) -> f64 {
    if Path::new(sandbox).exists() { fs::remove_dir_all(sandbox).ok(); }
    fs::create_dir_all(sandbox).unwrap();
    fs::write(format!("{}/main.rs", sandbox), code).unwrap();
    fs::write(format!("{}/Cargo.toml", sandbox), "[package]\nname=\"s\"\nversion=\"0.1.0\"\nedition=\"2021\"\n").unwrap();

    let build = Command::new("cargo").arg("build").current_dir(sandbox).stdout(Stdio::null()).stderr(Stdio::null()).status();
    if !build.map(|s| s.success()).unwrap_or(false) { return 0.1; }

    let start = std::time::Instant::now();
    let run = Command::new(format!("{}/target/debug/s", sandbox)).stdout(Stdio::null()).stderr(Stdio::null()).status();
    let dur = start.elapsed().as_secs_f64();

    if !run.map(|s| s.success()).unwrap_or(false) { return 0.3; }
    (1.0 / dur.max(0.001) * 10.0).min(1.0)
}

// ========== اتصال بـ Ollama ==========
fn ask_ollama(prompt: &str) -> String {
    let resp = ureq::post("http://localhost:11434/api/generate")
        .send_json(ureq::json!({
            "model": "qwen2.5:14b",
            "prompt": prompt,
            "stream": false
        }));
    match resp {
        Ok(r) => {
            let v: Value = r.into_json().unwrap_or_default();
            v["response"].as_str().unwrap_or("").to_string()
        }
        Err(_) => String::new()
    }
}

// ========== اقتراح طفرات ==========
fn propose_mutations(code: &str) -> Vec<String> {
    let prompt = format!("حسّن كود Rust التالي. أعد 3 نسخ مختلفة بين <CODE></CODE>.\n{}", code);
    let out = ask_ollama(&prompt);
    let mut suggestions = vec![];
    for part in out.split("<CODE>").skip(1) {
        if let Some(c) = part.split("</CODE>").next() {
            suggestions.push(c.trim().to_string());
        }
    }
    if suggestions.is_empty() {
        suggestions.push(code.replace("Genesis", "Gen"));
    }
    suggestions
}

// ========== Git auto-commit ==========
fn git_push() {
    Command::new("git").args(["add","."]).status().ok();
    Command::new("git").args(["commit","-m","تطور تلقائي"]).status().ok();
    Command::new("git").args(["push","origin","main"]).status().ok();
}

// ========== الرئيسية ==========
fn main() {
    println!("🌌 Genesis Core v8 – التشغيل الآلي الكامل");
    let mem = Memory::new("memory/genesis.db");
    let sandbox = "/tmp/sandbox_genesis";

    let mut current = Genome {
        id: Uuid::new_v4().to_string(),
        parent_id: None,
        generation: 0,
        code: "fn main() { println!(\"Genesis alive\"); }".to_string(),
        fitness: 0.0,
    };
    current.fitness = evaluate(&current.code, sandbox);
    mem.save(&current);

    loop {
        let candidates = propose_mutations(&current.code);
        let mut best_child: Option<Genome> = None;
        let mut best_fit = current.fitness;

        for code in candidates {
            let fit = evaluate(&code, sandbox);
            if fit > best_fit {
                best_fit = fit;
                best_child = Some(Genome {
                    id: Uuid::new_v4().to_string(),
                    parent_id: Some(current.id.clone()),
                    generation: current.generation + 1,
                    code,
                    fitness: fit,
                });
            }
        }

        if let Some(child) = best_child {
            println!("✨ تحسن! جيل {}: {:.2}", child.generation, child.fitness);
            mem.save(&child);
            current = child;

            fs::write("src/main.rs", &current.code).ok();
            fs::write("memory/best_fitness.txt", current.fitness.to_string()).ok();

            git_push();
        } else {
            println!("📛 لا تحسن هذا الجيل.");
        }

        std::thread::sleep(std::time::Duration::from_secs(30));
    }
}

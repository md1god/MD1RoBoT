use std::sync::Arc;
use std::io::Write;

mod api;
mod brain;
mod config_loader;
mod context_builder;
mod crazy;
mod db;
mod evolution;
mod evolution_lab;
mod goal_manager;
mod genome;
mod kreza;
mod memory_store;
mod model_router;
mod ollama_client;
mod planner;
mod protocol;
mod resource_governor;
mod seed;
mod workspace_tools;
mod independent_verifier;

use config_loader::AppConfig;
use db::Db;
use brain::Brain;
use goal_manager::GoalManager;
use evolution::EvolutionController;
use planner::Planner;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // تحميل الإعدادات
    let config = config_loader::load_config("config.toml");

    // قاعدة البيانات
    let db = Db::open("evolution.db")?;

    // المكونات الأساسية
    let brain = Brain::new(config.clone());
    let goal_manager = GoalManager::new(); // افترض وجوده
    let evo = EvolutionController::new(db.clone())?;

    let planner = Planner::new(brain, db.clone(), goal_manager, evo, config.clone());

    // طباعة رسالة بدء التشغيل
    println!("🧬 MD1RoBoT started. API server on http://0.0.0.0:8080");

    // تشغيل خادم API في خلفية غير متزامنة
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        api::start_api(db.clone(), planner, config).await
    })?;

    Ok(())
}

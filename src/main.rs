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
    let args: Vec<String> = std::env::args().collect();
    let run_once = args.contains(&"--run-once".to_string());

    let config = config_loader::load_config("config.toml");
    let db = Db::open("evolution.db")?;
    let brain = Brain::new(config.clone());
    let goal_manager = GoalManager::new();
    let evo = EvolutionController::new(db.clone())?;
    let mut planner = Planner::new(brain, db.clone(), goal_manager, evo, config.clone());

    if run_once {
        println!("Running one evolution cycle...");
        planner.run_cycle()?;
        println!("Cycle completed successfully.");
        return Ok(());
    }

    // وضع الخادم المستمر
    println!("🧬 MD1RoBoT API server on http://0.0.0.0:8080");
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        api::start_api(db.clone(), planner, config).await
    })?;

    Ok(())
}

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
    eprintln!("Starting MD1RoBoT...");
    let args: Vec<String> = std::env::args().collect();
    let run_once = args.contains(&"--run-once".to_string());

    eprintln!("Loading config...");
    let config = config_loader::load_config("config.toml");
    eprintln!("Config loaded successfully.");

    eprintln!("Opening database...");
    let db = Db::open("evolution.db")?;
    eprintln!("Database opened.");

    let brain = Brain::new(config.clone());
    let goal_manager = GoalManager::new();
    let evo = EvolutionController::new(db.clone(), "evolution.lock")?;
    let mut planner = Planner::new(brain, db.clone(), goal_manager, evo, config.clone());

    if run_once {
        eprintln!("Running one evolution cycle...");
        planner.run_cycle()?;
        eprintln!("Cycle completed successfully.");
        return Ok(());
    }

    eprintln!("Starting API server on port {}...", std::env::var("PORT").unwrap_or_else(|_| "8080".to_string()));
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        api::start_api(db.clone(), planner, config).await
    })?;

    Ok(())
}

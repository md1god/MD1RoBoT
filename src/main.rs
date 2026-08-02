mod notify;
mod ollama_client;
mod brain;
mod db;
mod search;
mod seed;
mod protocol;
mod context_builder;
mod evolution;
mod evolution_lab;
mod goal_manager;
mod planner;
mod crazy;
mod kreza;
mod genome;
mod model_router;
mod memory_store;
mod resource_governor;
mod workspace_tools;
mod independent_verifier;
mod config_loader;

use db::Db;
use brain::Brain;
use evolution::EvolutionController;
use planner::Planner;
use goal_manager::GoalManager;
use config_loader::AppConfig;

fn main() {
    println!("🌱 MD1RoBoT — Multi-Language Self-Evolution Engine");

    let config = AppConfig::load().expect("Failed to load config.toml");

    let db = Db::open("memory.db").expect("Failed to open database");
    let brain = Brain::new(config.clone());
    let goal_manager = GoalManager::new();

    let evo = EvolutionController::new(db.clone(), "./md1robot.lock")
        .expect("Failed to initialize EvolutionController");

    let mut planner = Planner::new(brain, db, goal_manager, evo, config.clone());

    for _ in 0..config.max_cycles {
        if !planner.evo.acquire_lock() {
            println!("⚠️ Evolution lock held, skipping cycle.");
            continue;
        }

        match planner.run_cycle() {
            Ok(()) => {}
            Err(e) => println!("⚠️ Error in evolution cycle: {e}"),
        }

        planner.evo.release_lock();
    }

    println!("✅ Evolution cycles complete. Current generation: {}", planner.evo.current_generation());
}

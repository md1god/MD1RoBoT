mod ollama_client;
mod brain;
mod db;
mod search;
mod seed;
mod protocol;
mod context_builder;      // كان context
mod evolution;           // لا تغيير
mod evolution_lab;       // كان lab
mod goals;               // كان goal_manager لكننا سنحتفظ بالاسم goals للملف goal_manager.rs؟ 
mod planner;
mod crazy;
mod kreza;
mod genome;
mod model_router;
mod memory_store;
mod resource_governor;
mod workspace_tools;
mod independent_verifier;

use db::Db;
use brain::Brain;
use evolution::EvolutionController;
use planner::Planner;
use goals::GoalManager;

fn main() {
    println!("🌱 MD1RoBoT — المحرك متعدد اللغات للتطور الذاتي");

    let db = Db::open("memory.db").expect("فشل فتح قاعدة البيانات");
    let brain = Brain::new();
    let goal_manager = GoalManager::new();

    let mut evo = EvolutionController::new(db.clone(), "./md1robot.lock")
        .expect("فشل تهيئة EvolutionController");

    let mut planner = Planner::new(brain, db, goal_manager, evo);

    let max_cycles = 5;
    for _ in 0..max_cycles {
        if !planner.evo.acquire_lock() {
            println!("⚠️ قفل التطور موجود، تخطي الدورة.");
            continue;
        }

        match planner.run_cycle() {
            Ok(()) => {}
            Err(e) => println!("⚠️ خطأ في دورة التطور: {e}"),
        }

        planner.evo.release_lock();
    }

    println!("✅ اكتملت دورات التطور. الجيل الحالي: {}", planner.evo.current_generation());
}

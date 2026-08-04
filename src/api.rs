use actix_web::{web, App, HttpServer, HttpResponse, middleware};
use actix_cors::Cors;
use std::sync::Mutex;
use crate::db::Db;
use crate::planner::Planner;
use crate::config_loader::AppConfig;

pub struct AppState {
    pub db: Db,
    pub planner: Mutex<Planner>,
    pub config: AppConfig,
}

async fn status(data: web::Data<AppState>) -> HttpResponse {
    let db = &data.db;
    let (gen, fitness, best_gen) = db.get_evolution_state().unwrap_or((0, 0.0, 0));
    let knowledge_count = db.knowledge_count();
    let theories = db.find_matching_theories(&[]);

    let status = serde_json::json!({
        "generation": gen,
        "best_fitness": fitness,
        "best_generation": best_gen,
        "knowledge_count": knowledge_count,
        "theories": theories.into_iter().map(|(id, stmt, conf, ev)| {
            serde_json::json!({ "id": id, "statement": stmt, "confidence": conf, "evidence": ev })
        }).collect::<Vec<_>>(),
    });

    HttpResponse::Ok().json(status)
}

async fn run_cycle(data: web::Data<AppState>) -> HttpResponse {
    let mut planner = data.planner.lock().unwrap();
    match planner.run_cycle() {
        Ok(()) => HttpResponse::Ok().json(serde_json::json!({"status": "cycle_completed"})),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e})),
    }
}

pub async fn start_api(db: Db, planner: Planner, config: AppConfig) -> std::io::Result<()> {
    let data = web::Data::new(AppState {
        db,
        planner: Mutex::new(planner),
        config,
    });

    let port = std::env::var("PORT").unwrap_or_else(|_| "10000".to_string());
    let bind_address = format!("0.0.0.0:{}", port);

    HttpServer::new(move || {
        let cors = Cors::permissive();
        App::new()
            .wrap(cors)
            .app_data(data.clone())
            .route("/api/status", web::get().to(status))
            .route("/api/run_cycle", web::post().to(run_cycle))
    })
    .bind(bind_address)?
    .run()
    .await
}

use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, State, Path},
    routing::{get, post},
    Json, Router,
    response::{IntoResponse,Response},
    http::StatusCode,
};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize}; 
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::sync::broadcast;
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    users: Arc<Mutex<HashMap<String, String>>>,
    jobs: Arc<Mutex<HashMap<String, Job>>>,
    tx: broadcast::Sender<String>,
}

#[derive(Deserialize)]
struct AuthPayload {
    username: String,
    password: String,
}

#[derive(Clone)]
struct Job {
    id: String,
    status: String,
}


#[derive(Serialize)]
struct JobStatusResponse {
    id: String,
    status: String,
}

//This enum lists everything that can go wrong in our API
enum AppError {
    Redis(redis::RedisError),
    LockError,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::Redis(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Redis Error: {}", e)),
            AppError::LockError => (StatusCode::INTERNAL_SERVER_ERROR,"Internal System Lock Error".to_string()),
        };
        (status, Json(serde_json::json!({ "error": error_message}))).into_response()
    }
}


impl From<redis::RedisError> for AppError {
    fn from(inner: redis::RedisError) -> Self {
        AppError::Redis(inner)
    }
}


//Main server
#[tokio::main]
async fn main() {
    println!("Server starting...");
    
    
    let (tx, _) = broadcast::channel(100);

    
    let state = AppState {
        users: Arc::new(Mutex::new(HashMap::new())),
        jobs: Arc::new(Mutex::new(HashMap::new())),
        tx,
    };

    
    tokio::spawn(start_worker(state.clone()));

    
    let app = Router::new()
        .route("/", get(root))
        .route("/signup", post(signup))
        .route("/login", post(login))
        .route("/createjob", post(create_job))
        .route("/job/:id", get(get_job_status)) 
        .route("/ws", get(ws_handler))
        .with_state(state);

    
    let addr = "0.0.0.0:3000";
    println!("Server listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind port 3000");

    axum::serve(listener, app).await.unwrap();
}

// Handlers  

async fn root() -> &'static str {
    "Backend is running!"
}

async fn signup(
    State(state): State<AppState>,
    Json(payload): Json<AuthPayload>,
) -> &'static str {
    let mut users = state.users.lock().unwrap();

    if users.contains_key(&payload.username) {
        return "User already exists";
    }

    users.insert(payload.username, payload.password);
    "Signup successful"
}

async fn login(
    State(state): State<AppState>,
    Json(payload): Json<AuthPayload>,
) -> &'static str {
    let users = state.users.lock().unwrap();

    match users.get(&payload.username) {
        Some(pass) if pass == &payload.password => "Login Successful",
        _ => "Invalid credentials",
    }
}


async fn create_job(State(state): State<AppState>) -> Result<String, AppError> {
    let job_id = Uuid::new_v4().to_string();

    let job = Job {
        id: job_id.clone(),
        status: "queued".to_string(),
    };

    {
        let mut jobs = state.jobs.lock().map_err(|_| AppError::LockError)?;
        jobs.insert(job_id.clone(), job);
    }

    let client = redis::Client::open("redis://redis:6379")?;
    let mut con = client.get_async_connection().await?;

    redis::cmd("PUBLISH")
        .arg("jobs")
        .arg(&job_id)
        .query_async::<_, ()>(&mut con)
        .await?;

    Ok(format!("Job created with id {}", job_id))
}

//job status handler
async fn get_job_status(
    Path(job_id): Path<String>, 
    State(state): State<AppState>,
) -> Json<JobStatusResponse> {
    let jobs = state.jobs.lock().unwrap();

    if let Some(job) = jobs.get(&job_id) {
        Json(JobStatusResponse {
            id: job.id.clone(),
            status: job.status.clone(),
        })
    } else {
        Json(JobStatusResponse {
            id: job_id,
            status: "not_found".to_string(),
        })
    }
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl axum::response::IntoResponse {

    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let mut rx = state.tx.subscribe();

    while let Ok(msg) = rx.recv().await {
        if socket.send(Message::Text(msg)).await.is_err() {
            break;
        }
    }
}

async fn start_worker(state: AppState) {
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    
    let client = match redis::Client::open("redis://redis:6379") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Worker failed to connect to Redis: {}", e);
            return;
        }
    };

    let con = match client.get_async_connection().await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Worker failed to get Redis connection: {}", e);
            return;
        }
    };

    let mut pubsub = con.into_pubsub();
    pubsub.subscribe("jobs").await.unwrap();

    println!("Worker listening for jobs....");

    while let Some(msg) = pubsub.on_message().next().await {
        let job_id: String = match msg.get_payload() {
            Ok(s) => s,
            Err(_) => continue,
        };

        let state_clone = state.clone();

        tokio::spawn(async move {
            println!("Processing job: {}", job_id);
            {
                let mut jobs = state_clone.jobs.lock().unwrap();
                if let Some(job) = jobs.get_mut(&job_id) {
                    job.status = "processing".to_string();
                    let _ = state_clone.tx.send(format!("Job {} is processing", job_id));
                }
            }

            // Simulate work
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;

            {
                let mut jobs = state_clone.jobs.lock().unwrap();
                if let Some(job) = jobs.get_mut(&job_id) {
                    job.status = "done".to_string();
                    let _ = state_clone.tx.send(format!("Job {} is done", job_id));
                }
            }

            println!("Job {} completed", job_id);
        });
    }
}

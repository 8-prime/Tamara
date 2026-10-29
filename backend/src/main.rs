mod api;
mod app;
mod archive;
mod auth;
mod db;
mod error;
mod models;
mod pdf;
mod state;
mod util;
mod validation;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    app::run().await
}

#[cfg(test)]
mod tests;

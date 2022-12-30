use std::fmt;
use std::process::{Command, Stdio};
use std::str;

use crate::RequireApiKey;
use actix_web::{
    get,
    web::{Query, ServiceConfig},
    HttpResponse, Responder,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Deserialize, Debug, Clone, ToSchema)]
pub(super) struct Password {
    word: String,
}
impl fmt::Display for Password {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(&self.word)?;
        Ok(())
    }
}

pub(super) fn configure() -> impl FnOnce(&mut ServiceConfig) {
    |config: &mut ServiceConfig| {
        config.service(crypto);
    }
}

#[derive(Serialize, Deserialize, Clone, ToSchema)]
pub(super) enum ErrorResponse {
    NotFound(String),
    Conflict(String),
    Unauthorized(String),
}

// async fn misc(word: String) -> String {}
async fn misc(word: String) -> String {
    let echofirst = Command::new("echo")
        .arg("-en")
        .arg(&word)
        .stdout(Stdio::piped()) // of which we will pipe the output.
        .spawn() // Once configured, we actually spawn the command...
        .unwrap(); // and assert everything went right.
    let base64first = Command::new("base64")
        .stdin(Stdio::from(echofirst.stdout.unwrap())) // of which we will pipe the output.
        .stdout(Stdio::piped())
        .spawn() // Once configured, we actually spawn the command...
        .unwrap(); // and assert everything went right.
    let sha1sum_child = Command::new("sha1sum")
        .stdin(Stdio::from(base64first.stdout.unwrap())) // Pipe through.
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let base64_child = Command::new("base64")
        .stdin(Stdio::from(sha1sum_child.stdout.unwrap())) // Pipe through.
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let cut_child = Command::new("cut")
        .arg("-c")
        .arg("45-")
        .stdin(Stdio::from(base64_child.stdout.unwrap())) // Pipe through.
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let output = cut_child.wait_with_output().unwrap();
    let result = str::from_utf8(&output.stdout).unwrap();
    result.to_string()
}

#[utoipa::path(
    responses(
        (status = 200, description = "Word burnt", body = String),
        (status = 404, description = "Word unburnt", body = ErrorResponse, example = json!(ErrorResponse::NotFound(String::from("id = 1"))))
    ),
    params(
        ("word", Query,description = "word to burn")
    ),
    security(
        ("api_key" = [])
    )
)]
#[get("/burn", wrap = "RequireApiKey")]
pub(super) async fn crypto(query: Query<Password>) -> impl Responder {
    println!("{query}");
    let ash = misc(query.to_string()).await;
    HttpResponse::Ok().body(ash)
}

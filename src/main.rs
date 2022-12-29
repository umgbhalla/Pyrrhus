// #[allow(dead_code)]
// #[allow(unused_variables)]
extern crate base64;
extern crate libucl;
use actix_web::{
    get, post,
    web::{self, Query},
    App, HttpRequest, HttpResponse, HttpServer, Responder,
};
use base64::encode;
use serde::Deserialize;
use std::env;
use std::fmt;
use std::process::{Command, Stdio};
use std::str;

#[derive(Deserialize, Debug)]
struct Password {
    word: String,
}

impl fmt::Display for Password {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(&self.word)?;
        Ok(())
    }
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

#[get("/")]
async fn health() -> impl Responder {
    HttpResponse::Ok().body("Services up")
}

#[post("/echo")]
async fn echo(req_body: String) -> impl Responder {
    HttpResponse::Ok().body(req_body)
}

#[get("/env")]
async fn environment() -> impl Responder {
    // Test env "TARGET" which defined when `docker run`, or `gcloud run deploy --set-env-vars`
    // Depend on your platform target. (See README.md)
    let test_target = match env::var("TARGET") {
        Ok(target) => format!("system running in {target} mode"),
        Err(_e) => "No TARGET env defined! , running local ?".to_owned(),
    };

    // Response with test_target
    HttpResponse::Ok().body(test_target)
}

#[get("/burn")]
async fn crypto(req: HttpRequest) -> impl Responder {
    let query = req.query_string();
    match Query::<Password>::from_query(query) {
        Ok(word) => {
            println!("{word}");
            let ash = misc(word.to_string()).await;
            HttpResponse::Ok().body(format!("{ash}"))
        }
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}

async fn get_server() -> std::io::Result<()> {
    let parser = libucl::Parser::new();

    let config = match parser.parse_file("test.conf") {
        Ok(conf) => conf,
        Err(err) => panic!("{:?}", err),
    };

    println!("{:?}", config.fetch("lol").and_then(|val| val.as_string()));
    println!(
        "{:?}",
        config
            .fetch_path("placki.duze")
            .and_then(|val| val.as_bool())
    );
    println!(
        "{:?}",
        config
            .fetch_path("placki.średnica")
            .and_then(|val| val.as_int())
    );
    println!(
        "{:?}",
        config
            .fetch_path("non.existent.path")
            .and_then(|val| val.as_string())
    );

    let port = match env::var("PORT") {
        Ok(targetport) => targetport,
        Err(_e) => "8080".to_string(),
    };
    HttpServer::new(|| {
        App::new()
            // "/"
            .service(health)
            .service(environment)
            .service(echo)
            .service(
                web::scope("/api").service(
                    web::scope("/v1")
                        // "/api/v1/burn?word=pass"
                        .service(crypto),
                ),
            )
    })
    .bind(format!("0.0.0.0:{}", port))?
    .run()
    .await
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    get_server().await
}

#[cfg(test)]
mod tests {
    use crate::health;
    use actix_web::App;

    #[actix_rt::test]
    async fn test() {
        let srv = actix_test::start(|| App::new().service(health));
        let req = srv.get("/");
        let response = req.send().await.unwrap();
        assert!(response.status().is_success());
    }
}

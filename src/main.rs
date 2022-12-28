#[allow(dead_code)]
#[allow(unused_variables)]
use actix_web::{
    get, post,
    web::{self, Query},
    App, HttpRequest, HttpResponse, HttpServer, Responder,
};
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
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Services up")
}

#[post("/echo")]
async fn echo(req_body: String) -> impl Responder {
    HttpResponse::Ok().body(req_body)
}

#[derive(Deserialize, Debug)]
struct Foo {
    id: String,
}

#[get("/search")]
async fn search(req: HttpRequest) -> impl Responder {
    let query = req.query_string();
    match Query::<Foo>::from_query(query) {
        Ok(foo) => HttpResponse::Ok().body(format!("Hi! {foo:?}")),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}

#[get("/bar")]
async fn bar() -> impl Responder {
    HttpResponse::Ok().body("Hello bar!")
}

async fn manual_hello() -> impl Responder {
    // Test env "TARGET" which defined when `docker run`, or `gcloud run deploy --set-env-vars`
    // Depend on your platform target. (See README.md)
    let test_target = match env::var("TARGET") {
        Ok(target) => format!("Hey {target}!"),
        Err(_e) => "No TARGET env defined!".to_owned(),
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
    HttpServer::new(|| {
        App::new()
            // "/"
            .service(hello)
            .service(search)
            .service(
                web::scope("/api/v1")
                    // "/api/v1/search?id=bar"
                    .service(search)
                    // "/api/v1/burn?id=bar"
                    .service(crypto)
                    // "/api/v1/bar"
                    .service(bar),
            )
            .route("/hey", web::get().to(manual_hello))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    get_server().await
}

#[cfg(test)]
mod tests {
    use crate::hello;
    use actix_web::App;

    #[actix_rt::test]
    async fn test() {
        let srv = actix_test::start(|| App::new().service(hello));

        let req = srv.get("/");
        let response = req.send().await.unwrap();
        assert!(response.status().is_success());
    }
}

// #[allow(dead_code)]
// #[allow(unused_variables)]
extern crate base64;
extern crate libucl;
use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    get,
    middleware::Logger,
    post, App, HttpResponse, HttpServer, Responder,
};
use apollohash::ErrorResponse;
use futures::future::LocalBoxFuture;

use std::{
    env,
    error::Error,
    future::{self, Ready},
    net::Ipv4Addr,
    str,
};
use utoipa::{
    openapi::security::{ApiKey, ApiKeyValue, SecurityScheme},
    Modify, OpenApi,
};
use utoipa_swagger_ui::SwaggerUi;

mod apollohash;

const API_KEY_NAME: &str = "pyrrhus_apikey";
const API_KEY: &str = "utoipa-rocks";

#[get("/")]
async fn root() -> impl Responder {
    HttpResponse::Ok().body("cr0nus")
}

#[get("/healthz")]
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

#[actix_web::main]
async fn main() -> Result<(), impl Error> {
    env_logger::init();

    #[derive(OpenApi)]
    #[openapi(
        paths(
            apollohash::crypto
        ),
        components(
            schemas(apollohash::Password,apollohash::ErrorResponse)
        ),
        tags(
            (name = "apollohash", description = "wip")
        ),
        modifiers(&SecurityAddon)
    )]
    struct ApiDoc;

    struct SecurityAddon;

    impl Modify for SecurityAddon {
        fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
            let components = openapi.components.as_mut().unwrap(); // we can unwrap safely since there already is components registered.
            components.add_security_scheme(
                "api_key",
                SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("pyrrhus_apikey"))),
            )
        }
    }

    // Make instance variable of ApiDoc so all worker threads gets the same instance.
    let openapi = ApiDoc::openapi();

    HttpServer::new(move || {
        // This factory closure is called on each worker thread independently.
        App::new()
            .wrap(Logger::default())
            .configure(apollohash::configure())
            .service(health)
            .service(root)
            .service(environment)
            .service(echo)
            // .service(
            //     web::scope("/api").service(
            //         web::scope("/v1")
            //             // "/api/v1/burn?word=pass"
            //             .service(crypto),
            //     ),
            // )
            .service(SwaggerUi::new("/docs/{_:.*}").url("/api-doc/openapi.json", openapi.clone()))
    })
    .bind((Ipv4Addr::UNSPECIFIED, 8080))?
    .run()
    .await
}

/// Require api key middlware will actually require valid api key
struct RequireApiKey;

impl<S> Transform<S, ServiceRequest> for RequireApiKey
where
    S: Service<
        ServiceRequest,
        Response = ServiceResponse<actix_web::body::BoxBody>,
        Error = actix_web::Error,
    >,
    S::Future: 'static,
{
    type Response = ServiceResponse<actix_web::body::BoxBody>;
    type Error = actix_web::Error;
    type Transform = ApiKeyMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        future::ready(Ok(ApiKeyMiddleware {
            service,
            log_only: false,
        }))
    }
}

/// Log api key midleware only logs about missing or invalid api keys
struct LogApiKey;

impl<S> Transform<S, ServiceRequest> for LogApiKey
where
    S: Service<
        ServiceRequest,
        Response = ServiceResponse<actix_web::body::BoxBody>,
        Error = actix_web::Error,
    >,
    S::Future: 'static,
{
    type Response = ServiceResponse<actix_web::body::BoxBody>;
    type Error = actix_web::Error;
    type Transform = ApiKeyMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        future::ready(Ok(ApiKeyMiddleware {
            service,
            log_only: true,
        }))
    }
}

struct ApiKeyMiddleware<S> {
    service: S,
    log_only: bool,
}

impl<S> Service<ServiceRequest> for ApiKeyMiddleware<S>
where
    S: Service<
        ServiceRequest,
        Response = ServiceResponse<actix_web::body::BoxBody>,
        Error = actix_web::Error,
    >,
    S::Future: 'static,
{
    type Response = ServiceResponse<actix_web::body::BoxBody>;
    type Error = actix_web::Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, actix_web::Error>>;

    fn poll_ready(
        &self,
        ctx: &mut core::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.service.poll_ready(ctx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let response = |req: ServiceRequest, response: HttpResponse| -> Self::Future {
            Box::pin(async { Ok(req.into_response(response)) })
        };

        match req.headers().get(API_KEY_NAME) {
            Some(key) if key != API_KEY => {
                if self.log_only {
                    log::debug!("Incorrect api api provided!!!")
                } else {
                    return response(
                        req,
                        HttpResponse::Unauthorized().json(ErrorResponse::Unauthorized(
                            String::from("incorrect api key"),
                        )),
                    );
                }
            }
            None => {
                if self.log_only {
                    log::debug!("Missing api key!!!")
                } else {
                    return response(
                        req,
                        HttpResponse::Unauthorized()
                            .json(ErrorResponse::Unauthorized(String::from("missing api key"))),
                    );
                }
            }
            _ => (), // just passthrough
        }

        if self.log_only {
            log::debug!("Performing operation")
        }

        let future = self.service.call(req);

        Box::pin(async move {
            let response = future.await?;

            Ok(response)
        })
    }
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

use route_recognizer;
use rusqlite::Connection;
use std::io;
use std::sync::mpsc::Sender;

use crate::api;
use crate::api::ApiChannel;
use crate::logger::{LogType, LoggingChannel};

mod instance;
mod tenant;
mod workload;

type Handler = fn(
    &mut tiny_http::Request,
    &route_recognizer::Params,
    &Connection,
    &Sender<ApiChannel>,
    &Sender<LoggingChannel>,
) -> Result<tiny_http::Response<io::Cursor<Vec<u8>>>, api::RikError>;

pub struct Router {
    routes: Vec<(tiny_http::Method, route_recognizer::Router<Handler>)>,
}

impl Router {
    pub fn new() -> Router {
        let mut get = route_recognizer::Router::<Handler>::new();
        let mut post = route_recognizer::Router::<Handler>::new();

        let base_path = "/api/v0";

        // GET
        get.add(&format!("{}/instances.list", base_path), instance::get);
        get.add(&format!("{}/tenants.list", base_path), tenant::get);
        get.add(&format!("{}/workloads.list", base_path), workload::get);
        // POST
        post.add(&format!("{}/instances.create", base_path), instance::create);
        post.add(&format!("{}/tenants.create", base_path), tenant::create);
        post.add(&format!("{}/workloads.create", base_path), workload::create);
        post.add(&format!("{}/instances.delete", base_path), instance::delete);
        post.add(&format!("{}/tenants.delete", base_path), tenant::delete);
        post.add(&format!("{}/workloads.delete", base_path), workload::delete);

        Router {
            routes: vec![
                ("GET".parse().unwrap(), get),
                ("POST".parse().unwrap(), post),
            ],
        }
    }

    pub fn handle(
        &self,
        request: &mut tiny_http::Request,
        connection: &Connection,
        internal_sender: &Sender<ApiChannel>,
        logger: &Sender<LoggingChannel>,
    ) -> Option<tiny_http::Response<io::Cursor<Vec<u8>>>> {
        self.routes
            .iter()
            .find(|&&(ref method, _)| method == request.method())
            .and_then(|&(_, ref routes)| {
                if let Ok(res) = routes.recognize(request.url()) {
                    Some(
                        res.handler()(request, &res.params(), connection, internal_sender, logger)
                            .unwrap_or_else(|error| {
                                logger
                                    .send(LoggingChannel {
                                        message: String::from(error.to_string()),
                                        log_type: LogType::Error,
                                    })
                                    .unwrap();
                                tiny_http::Response::from_string(error.to_string())
                                    .with_status_code(tiny_http::StatusCode::from(400))
                            }),
                    )
                } else {
                    None
                }
            })
    }
}

#[cfg(test)]
mod test {
    use crate::api::external::routes;
    use crate::api::ApiChannel;
    use crate::database::RikDataBase;
    use crate::logger;
    use crate::logger::Logger;
    use crate::tests::fixtures::{
        db_connection, mock_external_receiver, mock_internal_sender, mock_logger,
    };

    use rstest::rstest;
    use std::sync::mpsc::{Receiver, Sender};
    use tiny_http::Request;
    #[rstest]
    fn test_get_workloads(
        db_connection: std::sync::Arc<RikDataBase>,
        mock_internal_sender: Sender<ApiChannel>,
        mock_logger: (
            std::sync::mpsc::Sender<logger::LoggingChannel>,
            std::sync::mpsc::Receiver<logger::LoggingChannel>,
        ),
        _mock_external_receiver: Receiver<ApiChannel>,
    ) {
        let db_mock_external = db_connection.clone();

        let (mock_logging_sender, mock_logging_receiver) = mock_logger;

        let _logger = Logger::new(mock_logging_receiver, String::from("Main"));
        let router = routes::Router::new();
        let connection = db_mock_external.open().unwrap();

        let test_req = tiny_http::TestRequest::new()
            .with_method(tiny_http::Method::Get)
            .with_path("/api/v0/workloads.list");
        let mut req: Request = Request::from(test_req);

        if let Some(res) = router.handle(
            &mut req,
            &connection,
            &mock_internal_sender,
            &mock_logging_sender,
        ) {
            assert_eq!(res.status_code(), 200);
        }
    }
    #[rstest]
    fn test_create_workloads(
        db_connection: std::sync::Arc<RikDataBase>,
        mock_internal_sender: Sender<ApiChannel>,
        mock_logger: (
            std::sync::mpsc::Sender<logger::LoggingChannel>,
            std::sync::mpsc::Receiver<logger::LoggingChannel>,
        ),
        _mock_external_receiver: Receiver<ApiChannel>,
    ) {
        let db_mock_external = db_connection.clone();

        let (mock_logging_sender, mock_logging_receiver) = mock_logger;

        let _logger = Logger::new(mock_logging_receiver, String::from("Main"));
        let router = routes::Router::new();
        let connection = db_mock_external.open().unwrap();

        let test_req = tiny_http::TestRequest::new()
            .with_method(tiny_http::Method::Post)
            .with_path("/api/v0/workloads.create")
            .with_body("{\n  \"api_version\": \"v0\",\n  \"kind\": \"pods\",\n  \"name\": \"workload-name\",\n  \"spec\": {\n    \"containers\": [\n      {\n        \"name\": \"<name>\",\n        \"image\": \"<image>\",\n        \"env\": [\n          {\n            \"name\": \"key1\",\n            \"value\": \"value1\"\n          },\n           {\n            \"name\": \"key2\",\n            \"value\": \"value2\"\n          }\n        ],\n        \"ports\": {\n          \"port\": 80,\n          \"target_port\": 80,\n          \"protocol\": \"TCP\",\n          \"type\": \"clusterIP|nodePort|loadBalancer\"\n        }\n      }\n    ]\n  }\n}");
        let mut req: Request = Request::from(test_req);

        if let Some(res) = router.handle(
            &mut req,
            &connection,
            &mock_internal_sender,
            &mock_logging_sender,
        ) {
            assert_eq!(res.status_code(), 200);
        }
    }

    #[rstest]
    fn test_delete_workloads(
        db_connection: std::sync::Arc<RikDataBase>,
        mock_internal_sender: Sender<ApiChannel>,
        mock_logger: (
            std::sync::mpsc::Sender<logger::LoggingChannel>,
            std::sync::mpsc::Receiver<logger::LoggingChannel>,
        ),
        _mock_external_receiver: Receiver<ApiChannel>,
    ) {
        let db_mock_external = db_connection.clone();

        let (mock_logging_sender, mock_logging_receiver) = mock_logger;

        let _logger = Logger::new(mock_logging_receiver, String::from("Main"));
        let router = routes::Router::new();
        let connection = db_mock_external.open().unwrap();

        let test_req = tiny_http::TestRequest::new()
            .with_method(tiny_http::Method::Post)
            .with_path("/api/v0/workloads.delete")
            .with_body("{\n  \"api_version\": \"v0\",\n  \"kind\": \"pods\",\n  \"name\": \"workload-name\",\n  \"spec\": {\n    \"containers\": [\n      {\n        \"name\": \"<name>\",\n        \"image\": \"<image>\",\n        \"env\": [\n          {\n            \"name\": \"key1\",\n            \"value\": \"value1\"\n          },\n           {\n            \"name\": \"key2\",\n            \"value\": \"value2\"\n          }\n        ],\n        \"ports\": {\n          \"port\": 80,\n          \"target_port\": 80,\n          \"protocol\": \"TCP\",\n          \"type\": \"clusterIP|nodePort|loadBalancer\"\n        }\n      }\n    ]\n  }\n}");
        let mut req: Request = Request::from(test_req);

        if let Some(res) = router.handle(
            &mut req,
            &connection,
            &mock_internal_sender,
            &mock_logging_sender,
        ) {
            assert_eq!(res.status_code(), 204);
        }
    }

    #[rstest]
    fn test_get_instances(
        db_connection: std::sync::Arc<RikDataBase>,
        mock_internal_sender: Sender<ApiChannel>,
        mock_logger: (
            std::sync::mpsc::Sender<logger::LoggingChannel>,
            std::sync::mpsc::Receiver<logger::LoggingChannel>,
        ),
        _mock_external_receiver: Receiver<ApiChannel>,
    ) {
        let db_mock_external = db_connection.clone();

        let (mock_logging_sender, mock_logging_receiver) = mock_logger;

        let _logger = Logger::new(mock_logging_receiver, String::from("Main"));
        let router = routes::Router::new();
        let connection = db_mock_external.open().unwrap();

        let test_req = tiny_http::TestRequest::new()
            .with_method(tiny_http::Method::Get)
            .with_path("/api/v0/instances.create");
        let mut req: Request = Request::from(test_req);

        if let Some(res) = router.handle(
            &mut req,
            &connection,
            &mock_internal_sender,
            &mock_logging_sender,
        ) {
            assert_eq!(res.status_code(), 200);
        }
    }

    #[rstest]
    fn test_create_instances(
        db_connection: std::sync::Arc<RikDataBase>,
        mock_internal_sender: Sender<ApiChannel>,
        mock_logger: (
            std::sync::mpsc::Sender<logger::LoggingChannel>,
            std::sync::mpsc::Receiver<logger::LoggingChannel>,
        ),
        _mock_external_receiver: Receiver<ApiChannel>,
    ) {
        let db_mock_external = db_connection.clone();

        let (mock_logging_sender, mock_logging_receiver) = mock_logger;

        let _logger = Logger::new(mock_logging_receiver, String::from("Main"));
        let router = routes::Router::new();
        let connection = db_mock_external.open().unwrap();

        let test_req = tiny_http::TestRequest::new()
            .with_method(tiny_http::Method::Post)
            .with_path("/api/v0/instances.create")
            .with_body("{\n  \"api_version\": \"v0\",\n  \"kind\": \"pods\",\n  \"name\": \"workload-name\",\n  \"spec\": {\n    \"containers\": [\n      {\n        \"name\": \"<name>\",\n        \"image\": \"<image>\",\n        \"env\": [\n          {\n            \"name\": \"key1\",\n            \"value\": \"value1\"\n          },\n           {\n            \"name\": \"key2\",\n            \"value\": \"value2\"\n          }\n        ],\n        \"ports\": {\n          \"port\": 80,\n          \"target_port\": 80,\n          \"protocol\": \"TCP\",\n          \"type\": \"clusterIP|nodePort|loadBalancer\"\n        }\n      }\n    ]\n  }\n}");
        let mut req: Request = Request::from(test_req);

        if let Some(res) = router.handle(
            &mut req,
            &connection,
            &mock_internal_sender,
            &mock_logging_sender,
        ) {
            assert_eq!(res.status_code(), 200);
        }
    }

    #[rstest]
    fn test_delete_instances(
        db_connection: std::sync::Arc<RikDataBase>,
        mock_internal_sender: Sender<ApiChannel>,
        mock_logger: (
            std::sync::mpsc::Sender<logger::LoggingChannel>,
            std::sync::mpsc::Receiver<logger::LoggingChannel>,
        ),
        _mock_external_receiver: Receiver<ApiChannel>,
    ) {
        let db_mock_external = db_connection.clone();

        let (mock_logging_sender, mock_logging_receiver) = mock_logger;

        let _logger = Logger::new(mock_logging_receiver, String::from("Main"));
        let router = routes::Router::new();
        let connection = db_mock_external.open().unwrap();

        let test_req = tiny_http::TestRequest::new()
            .with_method(tiny_http::Method::Post)
            .with_path("/api/v0/instances.delete")
            .with_body("{\n  \"api_version\": \"v0\",\n  \"kind\": \"pods\",\n  \"name\": \"workload-name\",\n  \"spec\": {\n    \"containers\": [\n      {\n        \"name\": \"<name>\",\n        \"image\": \"<image>\",\n        \"env\": [\n          {\n            \"name\": \"key1\",\n            \"value\": \"value1\"\n          },\n           {\n            \"name\": \"key2\",\n            \"value\": \"value2\"\n          }\n        ],\n        \"ports\": {\n          \"port\": 80,\n          \"target_port\": 80,\n          \"protocol\": \"TCP\",\n          \"type\": \"clusterIP|nodePort|loadBalancer\"\n        }\n      }\n    ]\n  }\n}");
        let mut req: Request = Request::from(test_req);

        if let Some(res) = router.handle(
            &mut req,
            &connection,
            &mock_internal_sender,
            &mock_logging_sender,
        ) {
            assert_eq!(res.status_code(), 204);
        }
    }

    #[rstest]
    fn test_get_tenants(
        db_connection: std::sync::Arc<RikDataBase>,
        mock_internal_sender: Sender<ApiChannel>,
        mock_logger: (
            std::sync::mpsc::Sender<logger::LoggingChannel>,
            std::sync::mpsc::Receiver<logger::LoggingChannel>,
        ),
        _mock_external_receiver: Receiver<ApiChannel>,
    ) {
        let db_mock_external = db_connection.clone();

        let (mock_logging_sender, mock_logging_receiver) = mock_logger;

        let _logger = Logger::new(mock_logging_receiver, String::from("Main"));
        let router = routes::Router::new();
        let connection = db_mock_external.open().unwrap();

        let test_req = tiny_http::TestRequest::new()
            .with_method(tiny_http::Method::Get)
            .with_path("/api/v0/tenants.list");
        let mut req: Request = Request::from(test_req);

        if let Some(res) = router.handle(
            &mut req,
            &connection,
            &mock_internal_sender,
            &mock_logging_sender,
        ) {
            assert_eq!(res.status_code(), 200);
        }
    }

    #[rstest]
    fn test_create_tenants(
        db_connection: std::sync::Arc<RikDataBase>,
        mock_internal_sender: Sender<ApiChannel>,
        mock_logger: (
            std::sync::mpsc::Sender<logger::LoggingChannel>,
            std::sync::mpsc::Receiver<logger::LoggingChannel>,
        ),
        _mock_external_receiver: Receiver<ApiChannel>,
    ) {
        let db_mock_external = db_connection.clone();

        let (mock_logging_sender, mock_logging_receiver) = mock_logger;

        let _logger = Logger::new(mock_logging_receiver, String::from("Main"));
        let router = routes::Router::new();
        let connection = db_mock_external.open().unwrap();

        let test_req = tiny_http::TestRequest::new()
            .with_method(tiny_http::Method::Post)
            .with_path("/api/v0/tenants.create")
            .with_body("{\n  \"api_version\": \"v0\",\n  \"kind\": \"pods\",\n  \"name\": \"workload-name\",\n  \"spec\": {\n    \"containers\": [\n      {\n        \"name\": \"<name>\",\n        \"image\": \"<image>\",\n        \"env\": [\n          {\n            \"name\": \"key1\",\n            \"value\": \"value1\"\n          },\n           {\n            \"name\": \"key2\",\n            \"value\": \"value2\"\n          }\n        ],\n        \"ports\": {\n          \"port\": 80,\n          \"target_port\": 80,\n          \"protocol\": \"TCP\",\n          \"type\": \"clusterIP|nodePort|loadBalancer\"\n        }\n      }\n    ]\n  }\n}");
        let mut req: Request = Request::from(test_req);

        if let Some(res) = router.handle(
            &mut req,
            &connection,
            &mock_internal_sender,
            &mock_logging_sender,
        ) {
            assert_eq!(res.status_code(), 200);
        }
    }

    #[rstest]
    fn test_delete_tenants(
        db_connection: std::sync::Arc<RikDataBase>,
        mock_internal_sender: Sender<ApiChannel>,
        mock_logger: (
            std::sync::mpsc::Sender<logger::LoggingChannel>,
            std::sync::mpsc::Receiver<logger::LoggingChannel>,
        ),
        _mock_external_receiver: Receiver<ApiChannel>,
    ) {
        let db_mock_external = db_connection.clone();

        let (mock_logging_sender, mock_logging_receiver) = mock_logger;

        let _logger = Logger::new(mock_logging_receiver, String::from("Main"));
        let router = routes::Router::new();
        let connection = db_mock_external.open().unwrap();

        let test_req = tiny_http::TestRequest::new()
            .with_method(tiny_http::Method::Post)
            .with_path("/api/v0/tenants.delete")
            .with_body("{\n  \"api_version\": \"v0\",\n  \"kind\": \"pods\",\n  \"name\": \"workload-name\",\n  \"spec\": {\n    \"containers\": [\n      {\n        \"name\": \"<name>\",\n        \"image\": \"<image>\",\n        \"env\": [\n          {\n            \"name\": \"key1\",\n            \"value\": \"value1\"\n          },\n           {\n            \"name\": \"key2\",\n            \"value\": \"value2\"\n          }\n        ],\n        \"ports\": {\n          \"port\": 80,\n          \"target_port\": 80,\n          \"protocol\": \"TCP\",\n          \"type\": \"clusterIP|nodePort|loadBalancer\"\n        }\n      }\n    ]\n  }\n}");
        let mut req: Request = Request::from(test_req);

        if let Some(res) = router.handle(
            &mut req,
            &connection,
            &mock_internal_sender,
            &mock_logging_sender,
        ) {
            assert_eq!(res.status_code(), 204);
        }
    }
}

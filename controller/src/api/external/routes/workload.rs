use crate::api;
use crate::api::external::services::element::elements_set_right_name;
use crate::api::types::element::OnlyId;
use crate::api::{ApiChannel, CRUD};
use crate::database::RikRepository;
use crate::logger::{LogType, LoggingChannel};

use definition::workload::WorkloadDefinition;
use route_recognizer;
use rusqlite::Connection;
use std::io;
use std::str::FromStr;
use std::sync::mpsc::Sender;

pub fn get(
    _: &mut tiny_http::Request,
    _: &route_recognizer::Params,
    connection: &Connection,
    _: &Sender<ApiChannel>,
    logger: &Sender<LoggingChannel>,
) -> Result<tiny_http::Response<io::Cursor<Vec<u8>>>, api::RikError> {
    if let Ok(mut workloads) = RikRepository::find_all(connection, "/workload") {
        workloads = elements_set_right_name(workloads.clone());
        let workloads_json = serde_json::to_string(&workloads).unwrap();
        logger
            .send(LoggingChannel {
                message: String::from("Workloads found"),
                log_type: LogType::Log,
            })
            .unwrap();

        Ok(tiny_http::Response::from_string(workloads_json)
            .with_header(tiny_http::Header::from_str("Content-Type: application/json").unwrap())
            .with_status_code(tiny_http::StatusCode::from(200)))
    } else {
        Ok(tiny_http::Response::from_string("Cannot find workloads")
            .with_status_code(tiny_http::StatusCode::from(500)))
    }
}

pub fn create(
    req: &mut tiny_http::Request,
    _: &route_recognizer::Params,
    connection: &Connection,
    _: &Sender<ApiChannel>,
    logger: &Sender<LoggingChannel>,
) -> Result<tiny_http::Response<io::Cursor<Vec<u8>>>, api::RikError> {
    let mut content = String::new();
    req.as_reader().read_to_string(&mut content).unwrap();

    let workload: WorkloadDefinition = serde_json::from_str(&content)?;
    let namespace = "default";
    let name = format!(
        "/workload/{}/{}/{}",
        workload.kind, namespace, workload.name
    );

    // Check name is not used
    if let Ok(_) = RikRepository::check_duplicate_name(connection, &name) {
        logger
            .send(LoggingChannel {
                message: String::from("Name already used"),
                log_type: LogType::Warn,
            })
            .unwrap();
        return Ok(tiny_http::Response::from_string("Name already used")
            .with_status_code(tiny_http::StatusCode::from(404)));
    }

    if let Ok(inserted_id) = RikRepository::insert(
        connection,
        &name,
        &serde_json::to_string(&workload).unwrap(),
    ) {
        let workload_id: OnlyId = OnlyId { id: inserted_id };
        logger
            .send(LoggingChannel {
                message: String::from(format!("Workload {} successfully created", &workload_id.id)),
                log_type: LogType::Log,
            })
            .unwrap();
        Ok(
            tiny_http::Response::from_string(serde_json::to_string(&workload_id).unwrap())
                .with_header(tiny_http::Header::from_str("Content-Type: application/json").unwrap())
                .with_status_code(tiny_http::StatusCode::from(200)),
        )
    } else {
        logger
            .send(LoggingChannel {
                message: String::from("Cannot create workload"),
                log_type: LogType::Error,
            })
            .unwrap();
        Ok(tiny_http::Response::from_string("Cannot create workload")
            .with_status_code(tiny_http::StatusCode::from(500)))
    }
}

pub fn delete(
    req: &mut tiny_http::Request,
    _: &route_recognizer::Params,
    connection: &Connection,
    internal_sender: &Sender<ApiChannel>,
    logger: &Sender<LoggingChannel>,
) -> Result<tiny_http::Response<io::Cursor<Vec<u8>>>, api::RikError> {
    let mut content = String::new();
    req.as_reader().read_to_string(&mut content).unwrap();
    let OnlyId { id: delete_id } = serde_json::from_str(&content)?;

    if let Ok(workload) = RikRepository::find_one(connection, &delete_id, "/workload") {
        let definition: WorkloadDefinition = serde_json::from_value(workload.value).unwrap();
        internal_sender
            .send(ApiChannel {
                action: CRUD::Delete,
                workload_id: Some(delete_id),
                workload_definition: Some(definition),
                instance_id: None,
            })
            .unwrap();
        RikRepository::delete(connection, &workload.id).unwrap();

        logger
            .send(LoggingChannel {
                message: String::from("Delete workload"),
                log_type: LogType::Log,
            })
            .unwrap();
        Ok(tiny_http::Response::from_string("").with_status_code(tiny_http::StatusCode::from(204)))
    } else {
        logger
            .send(LoggingChannel {
                message: String::from(format!("Workload id {} not found", delete_id)),
                log_type: LogType::Error,
            })
            .unwrap();
        Ok(
            tiny_http::Response::from_string(format!("Workload id {} not found", delete_id))
                .with_status_code(tiny_http::StatusCode::from(404)),
        )
    }
}

#[cfg(test)]
mod test {
    use crate::api::external::Server;
    use crate::api::ApiChannel;
    use crate::database::{RikDataBase, RikRepository};
    use crate::logger::LoggingChannel;
    use crate::tests::fixtures::{
        db_connection, mock_external_receiver, mock_internal_sender, mock_logger,
    };
    use rstest::rstest;
    use std::sync::mpsc::{Receiver, Sender};

    #[rstest]
    fn test_create_workload(
        db_connection: std::sync::Arc<RikDataBase>,
        mock_internal_sender: Sender<ApiChannel>,
        mock_logger: Sender<LoggingChannel>,
        mock_external_receiver: Receiver<ApiChannel>,
    ) {
        let server = Server::new(mock_logger, mock_internal_sender, mock_external_receiver);

        let db_mock_external = db_connection.clone();
        server.run(db_mock_external);

        let request = tiny_http::TestRequest::new()
        .with_method(tiny_http::Method::Post)
        .with_path("/api/v0/workloads")
        .with_body("{\n  \"api_version\": \"v0\",\n  \"kind\": \"pods\",\n  \"name\": \"workload-name\",\n  \"spec\": {\n    \"containers\": [\n      {\n        \"name\": \"<name>\",\n        \"image\": \"<image>\",\n        \"env\": [\n          {\n            \"name\": \"key1\",\n            \"value\": \"value1\"\n          },\n           {\n            \"name\": \"key2\",\n            \"value\": \"value2\"\n          }\n        ],\n        \"ports\": {\n          \"port\": 80,\n          \"target_port\": 80,\n          \"protocol\": \"TCP\",\n          \"type\": \"clusterIP|nodePort|loadBalancer\"\n        }\n      }\n    ]\n  }\n}");

        // let response = server.handle(request.into());
        // assert_eq!(response.status_code(), StatusCode(200));

        // let ten_millis = std::time::Duration::from_millis(100);

        // std::thread::sleep(ten_millis);

        let connection = db_connection.open().unwrap();
        // create(
        //     &mut request.into(),
        //     route_recognizer,
        //     &connection,
        //     &mock_internal_sender,
        //     &mock_logger,
        // );
    }

    use std::io::Cursor;
    use tiny_http::{Method, Request, Response, StatusCode, TestRequest};
    #[rstest]
    fn test_example() {
        let mut request = TestRequest::new()
            .with_method(Method::Post)
            .with_path("/api/widgets")
            .with_body("42");
        struct TestServer {
            listener: tiny_http::Server,
        }
        let server = TestServer {
            listener: tiny_http::Server::http("127.0.0.1:6000").unwrap(),
        };
        impl TestServer {
            fn handle_request(&self, mut request: Request) -> Response<Cursor<Vec<u8>>> {
                let mut request_payload = String::new();
                request
                    .as_reader()
                    .read_to_string(&mut request_payload)
                    .unwrap();
                assert_eq!(request_payload, "42");
                Response::from_string("test")
            }
        }
        let response = server.handle_request(request.into());
        assert_eq!(response.status_code(), StatusCode(200));
    }
}

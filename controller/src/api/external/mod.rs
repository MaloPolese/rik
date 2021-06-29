mod routes;
mod services;

use crate::api::{ApiChannel, CRUD};
use crate::database::RickDataBase;
use crate::logger::{LogType, LoggingChannel};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::thread;
use tiny_http::{Request, Server as TinyServer};

use colored::Colorize;

#[derive(Debug)]
struct Cat {
    name: String,
    color: String,
}

pub struct Server {
    logger: Sender<LoggingChannel>,
    internal_sender: Sender<ApiChannel>,
    external_receiver: Receiver<ApiChannel>,
}

impl Server {
    pub fn new(
        logger_sender: Sender<LoggingChannel>,
        internal_sender: Sender<ApiChannel>,
        external_receiver: Receiver<ApiChannel>,
    ) -> Server {
        Server {
            logger: logger_sender,
            internal_sender,
            external_receiver,
        }
    }

    pub fn run(&self) {
        self.internal_sender
            .send(ApiChannel {
                action: CRUD::Delete,
                workload_id: Some(1),
                instance_id: Some(1),
            })
            .unwrap();
        self.run_server();
        self.listen_notification();
    }

    fn listen_notification(&self) {
        for notification in &self.external_receiver {
            println!("{}", notification);
        }
    }

    fn run_server(&self) {
        let host = String::from("127.0.0.1");
        let port = 5000;
        let server = TinyServer::http(format!("{}:{}", host, port)).unwrap();
        let server = Arc::new(server);
        let db = RickDataBase::new(String::from("rick"));
        db.init_tables().unwrap();

        let mut guards = Vec::with_capacity(4);

        for _ in 0..4 {
            let server = server.clone();
            let db = db.clone();
            let internal_sender = self.internal_sender.clone();
            let logger = self.logger.clone();

            let guard = thread::spawn(move || loop {
                let router = routes::Router::new();
                let connection = db.open().unwrap();

                let mut req: Request = server.recv().unwrap();

                if let Some(res) = router.handle(&mut req, &connection, &internal_sender, &logger) {
                    req.respond(res).unwrap();
                    continue;
                }
                logger
                    .send(LoggingChannel {
                        message: String::from("Route not found"),
                        log_type: LogType::Log,
                    })
                    .unwrap();
                req.respond(tiny_http::Response::empty(tiny_http::StatusCode::from(404)))
                    .unwrap();
            });

            guards.push(guard);
        }
        self.logger
            .send(LoggingChannel {
                message: format!(
                    "{}",
                    format!("Server running on http://{}:{}", host, port).green()
                ),
                log_type: LogType::Log,
            })
            .unwrap();
    }
}

use std::collections::VecDeque;
use url::{Url, Host};

use super::error::GraphError;

const DEFAULT_MAX_POOL_SIZE: usize = 50;

pub struct GraphDatabase;

impl GraphDatabase {
    pub fn driver(url: &str) -> Result<Driver, GraphError> {
        Driver::new(url)
    }
}

pub struct Driver {
    url: String,
    host: Host,
    port: u16,
    max_pool_size: usize,
    session_pool: VecDeque<Session>,
}

impl Driver {
    pub fn new(url: &str) -> Result<Self, GraphError> {
        let parsed = try!(Url::parse(url));
        if parsed.scheme != "bolt" {
            return Err(GraphError::new(format!("Unsupported URL scheme: {}", parsed.scheme), None))
        }

        let host = match parsed.host() {
            Some(host) => host.to_owned(),
            None => return Err(GraphError::new("Invalid url".to_owned(), None)),
        };

        let port = match parsed.port() {
            Some(port) => port,
            None => return Err(GraphError::new("Invalid url".to_owned(), None)),
        };

        Ok(Driver {
            url: url.to_owned(),
            host: host,
            port: port,
            max_pool_size: DEFAULT_MAX_POOL_SIZE,
            session_pool: VecDeque::new(),
        })
    }

    pub fn session(&mut self) -> Result<Session, GraphError> {
        loop {
            match self.session_pool.pop_front() {
                Some(session) => if session.healthy() {
                    session.connection().reset();
                    if session.healthy() {
                        return Ok(session)
                    }
                },
                None => {
                    return Session::new(self)
                }
            }
        }
    }

    pub fn recycle(&mut self, session: Session) {
        for (i, s) in self.session_pool.iter().enumerate() {
            if !s.healthy() {
                self.session_pool.remove(i);
            }
        }

        if session.healthy()
            && self.session_pool.len() < self.max_pool_size
            && !self.session_pool.iter().any(|s| s == &session) {

            self.session_pool.push_back(session);
        }
    }
}

#[derive(PartialEq)]
pub struct Session {
    healthy: bool,
}

impl Session {
    pub fn healthy(&self) -> bool {
        self.healthy
    }
}

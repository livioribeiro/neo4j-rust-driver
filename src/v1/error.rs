use std::convert::From;
use std::error::Error;
use std::fmt;
use url;

#[derive(Debug)]
pub struct GraphError {
    description: String,
    cause: Option<Box<Error>>,
}

impl GraphError {
    pub fn new(description: String, cause: Option<Box<Error>>) -> Self {
        GraphError {
            description: description,
            cause: cause,
        }
    }
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self.cause {
            Some(ref e) if self.description.is_empty() => e.fmt(f),
            Some(ref e) => write!(f, "{}: {}", self.description, e),
            None => write!(f, "{}", self.description),
        }
    }
}

impl Error for GraphError {
    fn description(&self) -> &str {
        &self.description
    }

    fn cause(&self) -> Option<&Error> {
        match self.cause {
            Some(ref e) => Some(&**e),
            None => None
        }
    }
}

impl From<url::ParseError> for GraphError {
    fn from(error: url::ParseError) -> Self {
        GraphError::new("Error".to_owned(), Some(Box::new(error)))
    }
}

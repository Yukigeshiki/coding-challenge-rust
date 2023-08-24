use std::fmt::Display;
use std::fmt::Formatter;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::{async_trait, Json};
use enum_iterator::{all, Sequence};
use rand::prelude::SliceRandom;
use reqwest::Client;
use serde::de;
use serde_json::{json, Value};

use crate::implement_json_display;

pub const CAT_API_URL: &str = "https://cat-fact.herokuapp.com/facts/random?animal_type=cat";
pub const DOG_API_URL: &str = "http://dog-api.kinduff.com/api/facts";

/// Type alias for a JSON response.
pub type Response = Json<Value>;

/// The animal request parameter.
#[derive(serde::Deserialize, serde::Serialize)]
pub struct Param {
    animal: String,
}

implement_json_display!(Param);

/// Returns a 200 OK JSON response and an animal fact payload.
fn respond_ok(fact: &str, animal: &str) -> (StatusCode, Response) {
    let value = json!({ "fact": fact, "animal": animal });
    tracing::info!("{value}");
    (StatusCode::OK, Json(value))
}

/// Returns a JSON response with an HTTP error code and an error message.
fn respond_error(code: StatusCode, error: &Error) -> (StatusCode, Response) {
    let value = json!({ "error": error.to_string() });
    tracing::error!("{value}");
    (code, Json(value))
}

#[tracing::instrument(
    name = "Fetching an animal fact",
    skip(client, params)
    fields(
        param = % params.0
    )
)]
pub async fn get_animal_fact(
    State(client): State<Client>,
    params: Query<Param>,
) -> (StatusCode, Response) {
    let mut param = params.0.animal.as_str();

    // choose an animal randomly if the animal param is "any"
    if param.to_lowercase() == "any" {
        let animals: Vec<&str> = all::<Animal>()
            .collect::<Vec<_>>()
            .iter()
            .map(Animal::as_str)
            .collect();
        param = animals.choose(&mut rand::thread_rng()).unwrap_or(&"dog");
    }

    // match on the animal param and respond with the appropriate fact or an error
    match param.try_into() {
        Ok(p) => match p {
            Animal::Cat => match Cat::get_fact(&client, CAT_API_URL).await {
                Ok(res) => respond_ok(&res.text, p.as_str()),
                Err(err) => respond_error(StatusCode::INTERNAL_SERVER_ERROR, &err),
            },
            Animal::Dog => match Dog::get_fact(&client, DOG_API_URL).await {
                Ok(res) => respond_ok(
                    res.facts.first().unwrap_or(&"Not available".to_string()),
                    p.as_str(),
                ),
                Err(err) => respond_error(StatusCode::INTERNAL_SERVER_ERROR, &err),
            },
        },
        Err(err) => respond_error(StatusCode::BAD_REQUEST, &err),
    }
}

/// The `Animal` enum.
#[derive(Debug, PartialEq, Sequence)]
pub enum Animal {
    Cat,
    Dog,
    // add as many more animals as you want!
}

/// Implements type conversion from an `Animal` enum to a string literal.
impl Animal {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Animal::Cat => "cat",
            Animal::Dog => "dog",
        }
    }
}

/// Implements type conversion from a string literal to an `Animal` enum.
impl TryFrom<&str> for Animal {
    type Error = Error;

    fn try_from(animal_param: &str) -> Result<Self, Self::Error> {
        match animal_param.to_lowercase().as_str() {
            "cat" => Ok(Self::Cat),
            "dog" => Ok(Self::Dog),
            other => Err(Error::ConvertToAnimalError(other.to_string())),
        }
    }
}

/// Provides a `get_fact` function for an animal API return struct.
#[async_trait]
trait GetFact {
    async fn get_fact(client: &Client, url: &str) -> Result<Self, Error>
    where
        for<'de> Self: Sized + de::Deserialize<'de>,
    {
        let res = client
            .get(url)
            .send()
            .await
            .map_err(|err| Error::ApiRequestError(err.to_string()))?;
        // check status first
        let status = res.status();
        if !status.is_success() {
            Err(Error::ApiResponseError(status.as_u16()))?;
        }
        let text = res
            .text()
            .await
            .map_err(|err| Error::ToTextError(err.to_string()))?;
        serde_json::from_str(&text).map_err(|err| Error::DeserializationError(err.to_string()))
    }
}

/// The cat API return type.
#[derive(serde::Deserialize)]
pub struct Cat {
    text: String,
}

impl GetFact for Cat {}

/// The dog API return type.
#[derive(serde::Deserialize)]
pub struct Dog {
    facts: Vec<String>,
}

impl GetFact for Dog {}

/// The Handler error types.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Error during Request to animal API: {0}")]
    ApiRequestError(String),

    #[error("Response from animal API returned error code: {0}")]
    ApiResponseError(u16),

    #[error("Error fetching text: {0}")]
    ToTextError(String),

    #[error("Error deserializing json string: {0}")]
    DeserializationError(String),

    #[error("'{0}' is not a supported animal.")]
    ConvertToAnimalError(String),
}

#[cfg(test)]
mod tests {
    use super::{Cat, Dog};
    use reqwest::Client;
    use wiremock::matchers::{any, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::GetFact;

    #[tokio::test]
    async fn test_cat_get_fact() {
        let mock_server = MockServer::start().await;

        Mock::given(any())
            .and(path("/facts/random"))
            .and(query_param("animal_type", "cat"))
            .and(method("GET"))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(r#"{"text": "fact"}"#, "application/json"),
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        let res = Cat::get_fact(
            &Client::new(),
            &format!("{}/{}", mock_server.uri(), "facts/random?animal_type=cat"),
        )
        .await
        .expect("Failed to get cat fact.");

        assert!(!res.text.is_empty());
    }

    #[tokio::test]
    async fn test_dog_get_fact() {
        let mock_server = MockServer::start().await;

        Mock::given(any())
            .and(path("/api/facts"))
            .and(method("GET"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_raw(r#"{"facts": ["fact"]}"#, "application/json"),
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        let res = Dog::get_fact(
            &Client::new(),
            &format!("{}/{}", mock_server.uri(), "api/facts"),
        )
        .await
        .expect("Failed to get dog fact.");

        assert!(!res.facts.first().expect("").is_empty());
    }
}

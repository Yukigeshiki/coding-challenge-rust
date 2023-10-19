use std::fmt::{Display, Formatter};

use axum::{
    async_trait,
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use enum_iterator::{all, Sequence};
use rand::prelude::SliceRandom;
use reqwest::Client;
use serde::de;
use serde_json::{json, Value};
use validator::{Validate, ValidationErrors};

use crate::implement_json_display;

const CAT_API_URL: &str = "https://cat-fact.herokuapp.com/facts/random?animal_type=cat";
const DOG_API_URL: &str = "http://dog-api.kinduff.com/api/facts";

/// Type alias for a JSON response.
pub type Response = Json<Value>;

/// The animal query parameter.
#[derive(serde::Deserialize, serde::Serialize, Validate)]
pub struct Param {
    #[validate(required, length(max = 24))]
    animal: Option<String>,
}

implement_json_display!(Param);

/// Returns a 200 OK JSON response and an animal fact payload.
fn respond_ok(fact: &str, animal: &str) -> (StatusCode, Response) {
    let value = json!({ "fact": fact, "animal": animal });
    tracing::info!("Success response payload: {value}");
    (StatusCode::OK, Json(value))
}

/// Returns a JSON response with an HTTP error code and an error message.
fn respond_error(code: StatusCode, err: &ErrorKind) -> (StatusCode, Response) {
    let value = json!({ "error": err.to_string() });
    tracing::error!("Fail response payload: {value}");
    (code, Json(value))
}

#[tracing::instrument(
    name = "Fetching an animal fact",
    skip(client, param)
    fields(
        param = % param.0
    )
)]
pub async fn get_animal_fact(
    State(client): State<Client>,
    param: Query<Param>,
) -> (StatusCode, Response) {
    // validate param
    if let Err(err) = param.0.validate() {
        return respond_error(StatusCode::BAD_REQUEST, &ErrorKind::Validation(err));
    };
    let animal = param.0.animal.unwrap(); // will always be Some(v) by this point
    let mut animal = animal.as_str();

    // choose an animal randomly if the animal param is "any"
    if animal.to_lowercase() == "any" {
        let animals: Vec<&str> = all::<Animal>()
            .collect::<Vec<_>>()
            .iter()
            .map(Animal::as_str)
            .collect();
        animal = animals.choose(&mut rand::thread_rng()).unwrap_or(&"dog");
    }

    // match on the animal and respond with the appropriate fact or an error
    match animal.try_into() {
        Ok(a) => match a {
            Animal::Cat => match Cat::get_fact(&client, CAT_API_URL).await {
                Ok(res) => respond_ok(&res.text, animal),
                Err(err) => respond_error(StatusCode::INTERNAL_SERVER_ERROR, &err),
            },
            Animal::Dog => match Dog::get_fact(&client, DOG_API_URL).await {
                Ok(res) => respond_ok(
                    res.facts.first().unwrap_or(&"Not available".to_string()),
                    animal,
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
    type Error = ErrorKind;

    fn try_from(animal_param: &str) -> Result<Self, Self::Error> {
        match animal_param.to_lowercase().as_str() {
            "cat" => Ok(Self::Cat),
            "dog" => Ok(Self::Dog),
            other => Err(ErrorKind::ConvertToAnimal(other.to_string())),
        }
    }
}

/// Provides a `get_fact` function for an animal API return struct.
#[async_trait]
trait GetFact {
    async fn get_fact(client: &Client, url: &str) -> Result<Self, ErrorKind>
    where
        for<'de> Self: Sized + de::Deserialize<'de>,
    {
        let res = client
            .get(url)
            .send()
            .await
            .map_err(|err| ErrorKind::ApiRequest(err.to_string()))?;
        // check status first
        let status = res.status();
        if !status.is_success() {
            Err(ErrorKind::ApiResponse(status.as_u16()))?;
        }
        let text = res
            .text()
            .await
            .map_err(|err| ErrorKind::ToText(err.to_string()))?;
        serde_json::from_str(&text).map_err(|err| ErrorKind::Deserialization(err.to_string()))
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
pub enum ErrorKind {
    #[error("{0}")]
    Validation(#[source] ValidationErrors),

    #[error("Error during Request to animal API: {0}")]
    ApiRequest(String),

    #[error("Response from animal API returned error code: {0}")]
    ApiResponse(u16),

    #[error("Error fetching text: {0}")]
    ToText(String),

    #[error("Error deserializing json string: {0}")]
    Deserialization(String),

    #[error("'{0}' is not a supported animal.")]
    ConvertToAnimal(String),
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

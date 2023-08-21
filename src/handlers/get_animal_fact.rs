use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use enum_iterator::{all, Sequence};
use rand::prelude::SliceRandom;
use reqwest::Client;
use serde_json::{json, Value};

pub const CAT_API_URL: &str = "https://cat-fact.herokuapp.com/facts/random?animal_type=cat";
pub const DOG_API_URL: &str = "http://dog-api.kinduff.com/api/facts";

/// Type alias for a JSON response.
pub type Response = Json<Value>;

/// The animal request parameter.
#[derive(serde::Deserialize)]
pub struct Param {
    animal: String,
}

/// Returns a 200 OK JSON response and an animal fact payload.
fn respond_ok(fact: &str, animal: &str) -> (StatusCode, Response) {
    let value = json!({ "fact": fact, "animal": animal });
    tracing::info!("{value}");
    (StatusCode::OK, Json(value))
}

/// Returns a JSON response with an HTTP error code and an error message.
fn respond_error(code: StatusCode, error: &str) -> (StatusCode, Response) {
    let value = json!({ "error": error });
    tracing::error!("{value}");
    (code, Json(value))
}

#[tracing::instrument(name = "Fetching an animal fact", skip(client, params))]
pub async fn get_animal_fact(
    State(client): State<Client>,
    params: Query<Param>,
) -> (StatusCode, Response) {
    let mut animal_param = params.0.animal.as_str();

    // choose an animal randomly if the animal param is "any"
    if animal_param.to_lowercase() == "any" {
        let animals: Vec<&str> = all::<Animal>()
            .collect::<Vec<_>>()
            .iter()
            .map(Animal::as_str)
            .collect();
        animal_param = animals.choose(&mut rand::thread_rng()).unwrap_or(&"dog");
    }

    // match on the animal param and respond with the appropriate fact or an error
    match animal_param.try_into() {
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
    type Error = String;

    fn try_from(animal_param: &str) -> Result<Self, Self::Error> {
        match animal_param.to_lowercase().as_str() {
            "cat" => Ok(Self::Cat),
            "dog" => Ok(Self::Dog),
            other => Err(format!("'{other}' is not a supported animal.")),
        }
    }
}

/// Implements a `get_fact` function for an animal API return struct.
macro_rules! implement_get_fact {
    ($t:ty) => {
        impl $t {
            async fn get_fact(client: &Client, url: &str) -> Result<$t, String> {
                let res = client
                    .get(url)
                    .send()
                    .await
                    .map_err(|err| format!("Error during Request to animal API: {err:?}"))?;
                // check status first
                let status = res.status().as_u16();
                if !res.status().is_success() {
                    Err(format!(
                        "Request to animal API failed with status code: {status}"
                    ))?;
                }
                let text = res
                    .text()
                    .await
                    .map_err(|err| format!("Error fetching text: {err:?}"))?;
                serde_json::from_str(&text)
                    .map_err(|err| format!("Error deserializing json string: {err:?}"))
            }
        }
    };
}

/// The cat API return type.
#[derive(serde::Deserialize)]
pub struct Cat {
    text: String,
}

implement_get_fact!(Cat);

/// The dog API return type.
#[derive(serde::Deserialize)]
pub struct Dog {
    facts: Vec<String>,
}

implement_get_fact!(Dog);

#[cfg(test)]
mod tests {
    use crate::handlers::{Cat, Dog, CAT_API_URL, DOG_API_URL};
    use reqwest::Client;

    #[tokio::test]
    async fn test_cat_get_fact() {
        let client = Client::new();
        let res = Cat::get_fact(&client, CAT_API_URL)
            .await
            .expect("Failed to get cat fact.");

        assert!(!res.text.is_empty());
    }

    #[tokio::test]
    async fn test_dog_get_fact() {
        let client = Client::new();
        let res = Dog::get_fact(&client, DOG_API_URL)
            .await
            .expect("Failed to get dog fact.");

        assert!(!res.facts.first().expect("").is_empty());
    }
}

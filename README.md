# coding-challenge-rust

[![build](https://github.com/Yukigeshiki/coding-challenge-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/Yukigeshiki/coding-challenge-rust/actions/workflows/ci.yml) [![build](https://github.com/Yukigeshiki/coding-challenge-rust/actions/workflows/build.yml/badge.svg)](https://github.com/Yukigeshiki/coding-challenge-rust/actions/workflows/build.yml)

Create an API server using Axum which will return random animal facts. The service should have the following:

- Receive the following inputs as arguments, environment variables, configuration, etc.

  - Port to start on

  - Type of fact to return with valid options

    - cat

    - dog

    - any (randomize the type on each request)

    - … assume more animals will be added in the future

  - Anything else you feel is needed

- Have the endpoint `/fact` return a random fact

  - Use the following APIs to retrieve a random cat or dog fact

    - Cat → https://alexwohlbruck.github.io/cat-facts/docs/endpoints/facts.html

    - Dog → https://kinduff.github.io/dog-api/

  - Return a response in this JSON format

```json
{
    "fact": "Three of the 12 dogs on the Titanic survived.",
    "animal": "dog"
}
```
- Assume this is a production-grade application. So create:

  - Tests

  - Logs

  - Handle errors

  - etc.


### To run the application:

```
cargo run
```

### To test the application:

```
cargo test
```

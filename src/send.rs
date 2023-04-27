// Licensed under the Open Software License version 3.0
use crate::{config::Endpoint, ds18b20::MeasuredTemperature, ups::UninterruptiblePowerSupplyData};
use reqwest::blocking::Client;
use serde::Serialize;
use std::time::Duration;

#[derive(Serialize)]
pub struct DataToSend {
    sensors: Vec<MeasuredTemperature>,
    upses: Vec<UninterruptiblePowerSupplyData>,
}

impl DataToSend {
    pub fn new(
        sensors: Vec<MeasuredTemperature>,
        upses: Vec<UninterruptiblePowerSupplyData>,
    ) -> Self {
        Self { sensors, upses }
    }
}

pub fn send_data<T>(
    client: &Client,
    json: &T,
    endpoint: &Endpoint,
    timeout: &Duration,
    ignore_connection_errors: &bool,
) where
    T: ?Sized + Serialize,
{
    // Send json to endpoint
    // With bearer token if available (use empty string if not)
    let result = client
        .post(&endpoint.url)
        .bearer_auth(endpoint.bearer_token.as_deref().unwrap_or(""))
        .json(json)
        .timeout(*timeout)
        .send();
    match result {
        Ok(response) => {
            if response.status().is_success() {
                // Pretty-print response object but only in debug mode
                // Used with httpbin to test the request
                #[cfg(debug_assertions)]
                {
                    let json: serde_json::Value = response.json().unwrap();
                    // Print only "json" field
                    log::trace!("{}", json["json"]);
                }
            } else {
                // Print response error with endpoint url
                log::warn!("Got {} response from {}", response.status(), endpoint.url);
            }
        }
        Err(error) => {
            // Ignore connection errors if specified
            if *ignore_connection_errors && error.is_connect() {
                return;
            }
            log::warn!("Connection failed: {}", error);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{Matcher::JsonString, Server};
    use std::time::Duration;

    #[test]
    fn test_send_data_without_token() {
        // Mock server
        let mut server = Server::new();
        // Prepare fake endpoint
        let mock = server
            .mock("POST", "/post-data")
            .match_body(JsonString(r#"[1, 2, 3, 4, 5]"#.to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"json": [1, 2, 3, 4, 5]}"#)
            .create();
        // Send data
        let client = Client::new();
        let endpoint = Endpoint {
            url: format!("{}{}", server.url(), "/post-data"),
            bearer_token: None,
        };
        let timeout = Duration::from_secs(5);
        let data = vec![1, 2, 3, 4, 5];
        send_data(&client, &data, &endpoint, &timeout, &false);
        // Assert that mock was called
        mock.assert();
    }

    #[test]
    fn test_send_data_with_bearer_token() {
        let mut server = Server::new();
        let mock = server
            .mock("POST", "/post-data")
            .match_body(JsonString(r#"[1, 2, 3, 4, 5]"#.to_string()))
            .match_header("Authorization", "Bearer token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"json": [1, 2, 3, 4, 5]}"#)
            .create();
        let bearer_token = Some("token".to_string());
        let client = Client::new();
        let endpoint = Endpoint {
            url: format!("{}{}", server.url(), "/post-data"),
            bearer_token,
        };
        let timeout = Duration::from_secs(5);
        let data = vec![1, 2, 3, 4, 5];
        send_data(&client, &data, &endpoint, &timeout, &false);
        mock.assert();
    }
}

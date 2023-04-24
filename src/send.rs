// Licensed under the Open Software License version 3.0
use crate::{config::Endpoint, ds18b20::MeasuredTemperature, ups::UninterruptiblePowerSupplyData};
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
    json: &T,
    endpoint: &Endpoint,
    timeout: &Duration,
    ignore_connection_errors: &bool,
) where
    T: ?Sized + Serialize,
{
    let client = reqwest::blocking::Client::new();
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
    use std::time::Duration;

    #[test]
    fn test_send_data() {
        let endpoint = Endpoint {
            url: "https://httpbin.org/post".to_string(),
            bearer_token: None,
        };
        let timeout = Duration::from_secs(5);
        let data = vec![1, 2, 3, 4, 5];
        send_data(&data, &endpoint, &timeout, &false);
    }
}

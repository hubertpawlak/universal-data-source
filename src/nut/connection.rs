// Licensed under the Open Software License version 3.0

// Original
pub use rups::tokio::Connection;

// ##########

// Mock implementation for testing
#[cfg(test)]
use rups::{ClientError, NutError, Variable};
#[cfg(test)]
pub struct MockConnection {}

#[cfg(test)]
impl MockConnection {
    pub async fn new(_: &rups::Config) -> Result<Self, ClientError> {
        Ok(Self {})
    }

    pub async fn get_var(
        &mut self,
        _: &str,
        variable_to_get: &str,
    ) -> Result<Variable, ClientError> {
        let value: Option<String> = match variable_to_get {
            "battery.charge" => Some(String::from("100")),
            "battery.charge.low" => Some(String::from("30")),
            "battery.runtime" => Some(String::from("15")),
            "battery.runtime.low" => Some(String::from("5")),
            _ => None,
        };
        if value.is_none() {
            return Err(ClientError::Nut(NutError::VarNotSupported));
        }
        Ok(Variable::Other((
            String::from(variable_to_get),
            value.unwrap(),
        )))
    }

    pub async fn get_server_version(&mut self) -> Result<String, ClientError> {
        Ok(String::from("Fake server 1.0"))
    }
}

// Licensed under the Open Software License version 3.0
use crate::config::NetworkUpsToolsClientConfig;
use rups::{blocking::Connection, Auth, ConfigBuilder};

pub struct NetworkUpsToolsClient {
    // Take ownership of the connection to use it
    connection: Option<Connection>,
    // Internal state
    server_id: String,
}

impl NetworkUpsToolsClient {
    // Easy to construct from deserialized config
    pub fn new(config: &NetworkUpsToolsClientConfig) -> Self {
        let server_id = config.get_server_id();
        // Prepare authentication details
        let auth: Option<Auth> = match (&config.username, &config.password) {
            (Some(username), Some(password)) => {
                Some(Auth::new(username.clone(), Some(password.clone())))
            }
            _ => None,
        };
        // Build config and return it
        let config = ConfigBuilder::new()
            .with_host(
                (
                    config.host.clone(),
                    config.port.unwrap_or(rups::DEFAULT_PORT),
                )
                    .try_into()
                    .unwrap_or_default(),
            )
            .with_auth(auth)
            .with_ssl(config.enable_tls.unwrap_or(false))
            .build();
        let connection = match Connection::new(&config) {
            Ok(connection) => Some(connection),
            Err(_) => None,
        };
        Self {
            connection,
            server_id,
        }
    }

    pub fn get_server_id(&self) -> String {
        self.server_id.clone()
    }

    pub fn take_connection(&mut self) -> Option<Connection> {
        self.connection.take()
    }

    pub fn give_connection(&mut self, connection: Connection) {
        self.connection = Some(connection);
    }
}

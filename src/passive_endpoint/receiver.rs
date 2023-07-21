// Licensed under the Open Software License version 3.0
use super::config::PassiveEndpointConfig;
use crate::{nut::sender::UninterruptiblePowerSupplyData, one_wire::sender::MeasuredTemperature};
use rocket::{get, http::Status, routes, serde::json::Json, Build, Rocket, State};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{broadcast, RwLock};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
struct ApiToken<'a>(&'a str);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
struct ApiResponse<T> {
    success: bool,
    error: Option<String>,
    data: Option<T>,
}

impl<T> ApiResponse<T> {
    fn new(data: Option<T>) -> Self {
        // If data is None, error is "not found"
        let error = match data.is_none() {
            true => Some(String::from("not found")),
            false => None,
        };
        Self {
            success: error.is_none(),
            error,
            data,
        }
    }
}

#[derive(Debug, Clone, Default)]
struct CachedData {
    // By category
    temperature_sensors: Arc<RwLock<Vec<MeasuredTemperature>>>,
    upses: Arc<RwLock<Vec<UninterruptiblePowerSupplyData>>>,
    // By category + hw.id
    temperature_sensors_by_hw_id: Arc<RwLock<HashMap<String, MeasuredTemperature>>>,
    upses_by_hw_id: Arc<RwLock<HashMap<String, UninterruptiblePowerSupplyData>>>,
}

impl CachedData {
    pub async fn get_temperature_sensors(&self) -> Vec<MeasuredTemperature> {
        self.temperature_sensors.read().await.clone()
    }

    pub async fn get_temperature_sensor_by_hw_id(&self, id: String) -> Option<MeasuredTemperature> {
        self.temperature_sensors_by_hw_id
            .read()
            .await
            .get(&id)
            .cloned()
    }

    pub async fn set_sensors(&self, sensors: Vec<MeasuredTemperature>) {
        *self.temperature_sensors.write().await = sensors.clone();
        let hash_map = &mut self.temperature_sensors_by_hw_id.write().await;
        hash_map.clear();
        for sensor in sensors {
            hash_map.insert(sensor.meta.hw.id.clone(), sensor);
        }
    }

    pub async fn get_upses(&self) -> Vec<UninterruptiblePowerSupplyData> {
        self.upses.read().await.clone()
    }

    pub async fn get_ups_by_hw_id(&self, id: String) -> Option<UninterruptiblePowerSupplyData> {
        self.upses_by_hw_id.read().await.get(&id).cloned()
    }

    pub async fn set_upses(&self, upses: Vec<UninterruptiblePowerSupplyData>) {
        *self.upses.write().await = upses.clone();
        let mut hash_map = self.upses_by_hw_id.write().await;
        hash_map.clear();
        for ups in upses {
            hash_map.insert(ups.meta.hw.id.clone(), ups);
        }
    }
}

async fn start_cache_updater_loop(
    mut shutdown_rx: broadcast::Receiver<()>,
    cache: Arc<CachedData>,
    mut one_wire_rx: broadcast::Receiver<Vec<MeasuredTemperature>>,
    mut ups_monitoring_rx: broadcast::Receiver<Vec<UninterruptiblePowerSupplyData>>,
) {
    loop {
        tokio::select! {
            Ok(value) = one_wire_rx.recv() => {
                tracing::trace!("{:?}", value);
                cache.set_sensors(value).await;
            }
            Ok(value) = ups_monitoring_rx.recv() => {
                tracing::trace!("{:?}", value);
                cache.set_upses(value).await;
            }
            _ = shutdown_rx.recv() => {
                tracing::trace!("Shutting down cache updater loop");
                break;
            }
        }
    }
}

#[get("/temperature")]
async fn get_temperature_sensors_route(
    cache: &State<Arc<CachedData>>,
) -> Json<ApiResponse<Vec<MeasuredTemperature>>> {
    Json(ApiResponse::new(Some(
        cache.get_temperature_sensors().await,
    )))
}

#[get("/temperature/<id>")]
async fn get_temperature_sensor_by_hw_id_route(
    cache: &State<Arc<CachedData>>,
    id: String,
) -> (Status, Json<ApiResponse<MeasuredTemperature>>) {
    let data = cache.get_temperature_sensor_by_hw_id(id).await;
    let data = ApiResponse::new(data);
    if !data.success {
        return (Status::NotFound, Json(data));
    }
    (Status::Ok, Json(data))
}

#[get("/ups")]
async fn get_upses_route(
    cache: &State<Arc<CachedData>>,
) -> Json<ApiResponse<Vec<UninterruptiblePowerSupplyData>>> {
    Json(ApiResponse::new(Some(cache.get_upses().await)))
}

#[get("/ups/<id>")]
async fn get_ups_by_hw_id_route(
    cache: &State<Arc<CachedData>>,
    id: String,
) -> (Status, Json<ApiResponse<UninterruptiblePowerSupplyData>>) {
    let data = cache.get_ups_by_hw_id(id).await;
    let data = ApiResponse::new(data);
    if !data.success {
        return (Status::NotFound, Json(data));
    }
    (Status::Ok, Json(data))
}

fn rocket(cache: Arc<CachedData>) -> Rocket<Build> {
    rocket::build().manage(cache).mount(
        "/",
        routes![
            get_temperature_sensors_route,
            get_temperature_sensor_by_hw_id_route,
            get_upses_route,
            get_ups_by_hw_id_route
        ],
    )
}

pub async fn start_passive_endpoint_loop(
    shutdown_rx: broadcast::Receiver<()>,
    config: PassiveEndpointConfig,
    one_wire_rx: broadcast::Receiver<Vec<MeasuredTemperature>>,
    ups_monitoring_rx: broadcast::Receiver<Vec<UninterruptiblePowerSupplyData>>,
) {
    // Check if module is enabled
    if !config.is_enabled() {
        tracing::trace!("Module is disabled");
        return;
    }

    let cache = Arc::new(CachedData::default());

    // Simple API that returns cached data as JSON
    tracing::trace!("Starting passive endpoint loop");
    let mut shutdown_rx_clone = shutdown_rx.resubscribe();
    let cache_arc_clone: Arc<CachedData> = cache.clone();
    let rocket_handle = tokio::spawn(async move {
        let prepared_rocket = rocket(cache_arc_clone)
            .configure(rocket::Config {
                port: config.get_port(),
                shutdown: rocket::config::Shutdown {
                    ctrlc: false,
                    ..Default::default()
                },
                ..Default::default()
            })
            .launch();

        tokio::select! {
            _ = prepared_rocket => {},
            _ = shutdown_rx_clone.recv() => {
                tracing::trace!("Aborting rocket");
            }
        }
    });

    // Cache updater
    let cache_updater_handle = tokio::spawn(async move {
        start_cache_updater_loop(shutdown_rx, cache, one_wire_rx, ups_monitoring_rx).await;
    });

    let _ = tokio::try_join!(rocket_handle, cache_updater_handle);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::types::Example;
    use rocket::{
        http::{ContentType, Status},
        local::asynchronous::Client,
        uri,
    };

    #[tokio::test]
    async fn test_get_sensors_empty_cache() {
        let cache = Arc::new(CachedData::default());
        let client = Client::tracked(rocket(cache.clone())).await.unwrap();

        let response = client
            .get(uri!(super::get_temperature_sensors_route))
            .dispatch()
            .await;
        // Basic checks
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.content_type(), Some(ContentType::JSON));
        // Inspect JSON response
        let response = response.into_string().await.unwrap();
        let response: ApiResponse<Vec<MeasuredTemperature>> =
            serde_json::from_str(&response).unwrap();
        assert!(response.success);
        assert!(response.error.is_none());
        assert_eq!(response.data.unwrap(), vec![]);
    }

    #[tokio::test]
    async fn test_get_sensors_with_updated_data() {
        let cache = Arc::new(CachedData::default());
        let client = Client::tracked(rocket(cache.clone())).await.unwrap();

        let response = client
            .get(uri!(super::get_temperature_sensors_route))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.content_type(), Some(ContentType::JSON));

        let sensors = vec![MeasuredTemperature::example()];
        cache.set_sensors(sensors.clone()).await;

        let response = client
            .get(uri!(super::get_temperature_sensors_route))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.content_type(), Some(ContentType::JSON));

        let response = response.into_string().await.unwrap();
        let response: ApiResponse<Vec<MeasuredTemperature>> =
            serde_json::from_str(&response).unwrap();
        assert!(response.success);
        assert!(response.error.is_none());
        assert_eq!(response.data.unwrap(), sensors);
    }

    #[tokio::test]
    async fn test_get_sensor_by_hw_id() {
        let cache = Arc::new(CachedData::default());
        let client = Client::tracked(rocket(cache.clone())).await.unwrap();

        let sensors = vec![MeasuredTemperature::example()];
        cache.set_sensors(sensors.clone()).await;

        let response = client
            .get(uri!(super::get_temperature_sensor_by_hw_id_route(
                sensors[0].meta.hw.id.clone()
            )))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.content_type(), Some(ContentType::JSON));

        let response = response.into_string().await.unwrap();
        let response: ApiResponse<MeasuredTemperature> = serde_json::from_str(&response).unwrap();
        assert!(response.success);
        assert!(response.error.is_none());
        assert!(response.data.is_some());
        assert_eq!(response.data.unwrap(), sensors[0]);
    }

    #[tokio::test]
    async fn test_get_sensor_by_hw_id_404() {
        let cache = Arc::new(CachedData::default());
        let client = Client::tracked(rocket(cache)).await.unwrap();

        let response = client
            .get(uri!(super::get_temperature_sensor_by_hw_id_route(
                String::from("non-existent-id")
            )))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::NotFound);
        assert_eq!(response.content_type(), Some(ContentType::JSON));

        let response = response.into_string().await.unwrap();
        let response: ApiResponse<MeasuredTemperature> = serde_json::from_str(&response).unwrap();
        assert!(!response.success);
        assert!(response.error.is_some());
        assert!(response.data.is_none());
    }

    #[tokio::test]
    async fn test_get_upses_empty_cache() {
        let cache = Arc::new(CachedData::default());
        let client = Client::tracked(rocket(cache.clone())).await.unwrap();

        let response = client.get(uri!(super::get_upses_route)).dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.content_type(), Some(ContentType::JSON));

        let response = response.into_string().await.unwrap();
        let response: ApiResponse<Vec<UninterruptiblePowerSupplyData>> =
            serde_json::from_str(&response).unwrap();
        assert!(response.success);
        assert!(response.error.is_none());
        assert_eq!(response.data.unwrap(), vec![]);
    }

    #[tokio::test]
    async fn test_get_upses_with_updated_data() {
        let cache = Arc::new(CachedData::default());
        let client = Client::tracked(rocket(cache.clone())).await.unwrap();

        let response = client.get(uri!(super::get_upses_route)).dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.content_type(), Some(ContentType::JSON));

        let upses = vec![UninterruptiblePowerSupplyData::example()];
        cache.set_upses(upses.clone()).await;

        let response = client.get(uri!(super::get_upses_route)).dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.content_type(), Some(ContentType::JSON));

        let response = response.into_string().await.unwrap();
        let response: ApiResponse<Vec<UninterruptiblePowerSupplyData>> =
            serde_json::from_str(&response).unwrap();
        assert!(response.success);
        assert!(response.error.is_none());
        assert_eq!(response.data.unwrap(), upses);
    }

    #[tokio::test]
    async fn test_get_ups_by_hw_id() {
        let cache = Arc::new(CachedData::default());
        let client = Client::tracked(rocket(cache.clone())).await.unwrap();

        let upses = vec![UninterruptiblePowerSupplyData::example()];
        cache.set_upses(upses.clone()).await;

        let response = client
            .get(uri!(super::get_ups_by_hw_id_route(
                upses[0].meta.hw.id.clone(),
            )))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.content_type(), Some(ContentType::JSON));

        let response = response.into_string().await.unwrap();
        let response: ApiResponse<UninterruptiblePowerSupplyData> =
            serde_json::from_str(&response).unwrap();
        assert!(response.success);
        assert!(response.error.is_none());
        assert!(response.data.is_some());
        assert_eq!(response.data.unwrap(), upses[0]);
    }

    #[tokio::test]
    async fn test_get_ups_by_hw_id_404() {
        let cache = Arc::new(CachedData::default());
        let client = Client::tracked(rocket(cache)).await.unwrap();

        let response = client
            .get(uri!(super::get_ups_by_hw_id_route(String::from(
                "non-existent-id"
            ))))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::NotFound);
        assert_eq!(response.content_type(), Some(ContentType::JSON));

        let response = response.into_string().await.unwrap();
        let response: ApiResponse<UninterruptiblePowerSupplyData> =
            serde_json::from_str(&response).unwrap();
        assert!(!response.success);
        assert!(response.error.is_some());
        assert!(response.data.is_none());
    }
}

# What is universal-data-source?
`universal-data-source` is a program for reading various measurements from many sources. It's designed to send data as JSON directly to [home-panel](https://github.com/hubertpawlak/home-panel) in a universal format. Feel free to use it as a source for your own projects.

# Supported sources
- 1-Wire temperature sensors
- Network UPS Tools

# Supported destinations
## Active data sender
Any HTTP(S) server that accepts JSON data in the following format:
```json
{
    "sensors": [
        {
            "meta": {
                "hw": {
                    "id": "28-00000a0b0c0d",
                    "hardware_type": "TemperatureSensor"
                },
                "source": {
                    "source_type": "OneWire"
                }
            },
            "temperature": 1.234,
            "resolution": 12
        }
    ],
    "upses": [
        {
            "meta": {
                "hw": {
                    "hardware_type": "UninterruptiblePowerSupply",
                    "id": "[ups]username@ups.lan:3493"
                },
                "source": {
                    "source_type": "NetworkUpsTools"
                }
            },
            "variables": {
                "battery.charge": "100",
                "battery.charge.low": "30",
                "battery.runtime": "1200",
                "battery.runtime.low": "180",
                "input.frequency": "50.0",
                "input.voltage": "233.0",
                "output.frequency": "50.0",
                "output.frequency.nominal": "50",
                "output.voltage": "238.0",
                "output.voltage.nominal": "230",
                "ups.load": "30",
                "ups.power": "147",
                "ups.power.nominal": "850",
                "ups.realpower": "108",
                "ups.status": "OL"
            }
        }
    ]
}
```
If a module is disabled, it simply returns an empty array for the corresponding key.

## Passive endpoint
You may send HTTP requests with or without authentication (depending on your configuration) to the following paths:
- `GET /temperature`
- `GET /temperature/<id>`
- `GET /ups`
- `GET /ups/<id>`

# How to use it?
1. Run `./universal-data-source` to generate a default configuration file. You can also specify a path to a custom configuration file using `UDS_RS_CONFIG_FILE` environment variable (ex. `UDS_RS_CONFIG_FILE=/etc/universal-data-source/config.toml universal-data-source`).
2. Edit the configuration file to your needs. Most of the settings are optional and have default values. See [Configuration](#configuration) section for more details.
3. Run `./universal-data-source` again to start the program. Remember to keep the `UDS_RS_CONFIG_FILE` environment variable set if you're using a custom configuration file.

# Configuration
## Environment variables
| key                | default                      | description                                                                                                                                        | required |
| ------------------ | ---------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- | -------- |
| UDS_RS_CONFIG_FILE | `./config.json`              | Path to the configuration file.                                                                                                                    | no       |
| RUST_LOG           | `universal_data_source=warn` | See [EnvFilter directives](https://docs.rs/tracing-subscriber/0.3.17/tracing_subscriber/filter/struct.EnvFilter.html#directives) for more details. | no       |

## All top-level options
The configuration file is written as a JSON object. See table below for a list of all available options. Missing modules are disabled by default.
| key                   | type                    | description                                                               | required |
| --------------------- | ----------------------- | ------------------------------------------------------------------------- | -------- |
| one_wire              | `OneWireConfig`         | 1-Wire temperature polling settings                                       | no       |
| ups_monitoring        | `UpsMonitoringConfig`   | Network UPS monitoring settings                                           | no       |
| active_data_sender    | `ActiveSenderConfig`    | Settings for periodical data sending using HTTP(S)                        | no       |
| passive_data_endpoint | `PassiveEndpointConfig` | Settings for passive HTTP endpoint (ideal for third-party control panels) | no       |


## Types explained
### `OneWireConfig`
| key       | type       | default             | description                     | required |
| --------- | ---------- | ------------------- | ------------------------------- | -------- |
| enabled   | `bool`     | false               | Whether to enable 1-Wire module | no       |
| base_path | `string`   | /sys/bus/w1/devices | Base path of 1-Wire devices     | no       |
| cooldown  | `Duration` | 5s                  | 1-Wire polling cooldown         | no       |

### `Duration`
| key   | type     | default | description | required |
| ----- | -------- | ------- | ----------- | -------- |
| secs  | `number` | 5       | seconds     | **yes**  |
| nanos | `number` | 0       | nanoseconds | **yes**  |


### `UpsMonitoringConfig`
| key      | type                            | default | description                             | required |
| -------- | ------------------------------- | ------- | --------------------------------------- | -------- |
| enabled  | `bool`                          | false   | Whether to enable UPS monitoring module | no       |
| servers  | `NetworkUpsToolsClientConfig[]` | []      | List of servers to query UPS data from  | no       |
| cooldown | `Duration`                      | 5s      | UPS polling cooldown                    | no       |

### `NetworkUpsToolsClientConfig`
| key        | type                                 | default   | description                                        | required |
| ---------- | ------------------------------------ | --------- | -------------------------------------------------- | -------- |
| host       | `string`                             | localhost | Hostname or IP address of Network UPS Tools server | **yes**  |
| port       | `number`                             | 3493      | Port of UPS server                                 | no       |
| enable_tls | `bool`                               | false     | Whether to enable TLS                              | no       |
| username   | `string`                             | username  | -                                                  | no       |
| password   | `string`                             | password  | -                                                  | no       |
| upses      | `UninterruptiblePowerSupplyConfig[]` | []        | List of UPSes to monitor                           | **yes**  |

### `UninterruptiblePowerSupplyConfig`
| key                  | type       | default                                   | description                | required |
| -------------------- | ---------- | ----------------------------------------- | -------------------------- | -------- |
| name                 | `string`   | -                                         | Name of the UPS            | **yes**  |
| variables_to_monitor | `string[]` | [variables_to_monitor](src/nut/client.rs) | List of variables to query | no       |

### `ActiveSenderConfig`
| key                      | type         | default | description                         | required |
| ------------------------ | ------------ | ------- | ----------------------------------- | -------- |
| enabled                  | `bool`       | false   | Whether to enable HTTP(S) sender    | no       |
| cooldown                 | `Duration`   | 5s      | HTTP(S) sender cooldown             | no       |
| ignore_connection_errors | `bool`       | false   | Whether to ignore connection errors | no       |
| endpoints                | `Endpoint[]` | []      | List of HTTP(S) endpoints           | no       |

### `Endpoint`
| key          | type     | default | description                               | required |
| ------------ | -------- | ------- | ----------------------------------------- | -------- |
| url          | `string` | -       | URL to which data will be sent            | **yes**  |
| bearer_token | `string` | -       | Bearer token to be sent with each request | no       |

# How to run it as a systemd service?
```bash 
# Create service account
useradd --system --home-dir /var/universal-data-source --shell /sbin/nologin --create-home --user-group universal-data-source
# Create service
systemctl edit --force --full universal-data-source
```
```ini
; Paste the following configuration
[Unit]
Description=sending universal measurements to HTTP endpoints
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
Environment="RUST_LOG=WARN"
#Environment="UDS_RS_CONFIG_FILE=config2.json"
ExecStart=/var/universal-data-source/universal-data-source
WorkingDirectory=/var/universal-data-source
Restart=no
User=universal-data-source
Group=universal-data-source
NoNewPrivileges=yes
PrivateTmp=yes
PrivateDevices=yes
DevicePolicy=closed
ProtectSystem=strict
ProtectHome=yes
ReadWritePaths=/var/universal-data-source
ProtectHostname=yes
ProtectControlGroups=yes
ProtectKernelModules=yes
ProtectKernelTunables=yes
RestrictAddressFamilies=AF_INET AF_INET6
RestrictNamespaces=yes
RestrictRealtime=yes
RestrictSUIDSGID=yes
MemoryDenyWriteExecute=yes
LockPersonality=yes
UMask=0077

[Install]
WantedBy=default.target
```
```bash
# Upload binary to /var/universal-data-source directory.
# Adjust permissions and ownership
chown universal-data-source:universal-data-source /var/universal-data-source/universal-data-source
chmod 740 /var/universal-data-source/universal-data-source
# Enable and start the service
systemctl enable --now universal-data-source.service
# Adjust configuration
nano /var/universal-data-source/config.json
# Restart service
systemctl restart universal-data-source.service
```

# How to build?
## Native compilation
1. Install Rust and Cargo (but you probably already have them installed). See [https://rustup.rs](https://rustup.rs) for more details.
2. Install OpenSSL development libraries. On Debian-based systems, run `sudo apt install libssl-dev`.
3. Clone this repository.
4. Run `cargo build --release` inside the repository.
5. The binary will be located at `target/release/universal-data-source`.

## Cross-compiling
If you want to cross-compile the binary, you can use the `cross` tool. It requires Docker to be installed.
1. Install `cross` using `cargo install cross`.
2. Run `cross build --release --target <target>` inside the repository.

# How to run tests?
Run `cargo test` inside the repository.

# How to contribute?
If you want to contribute, please fork this repository, create a new branch and submit a pull request. It will be reviewed and merged if it's a good fit. You may also create an issue if you find a bug or have a feature request.

# License
This project is licensed under the Open Software License version 3.0 - see the [LICENSE](LICENSE) file for details.

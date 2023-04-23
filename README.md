# What is universal-data-source?
`universal-data-source` is a program for reading various measurements from many sources. It's designed to send data as JSON directly to [home-panel](https://github.com/hubertpawlak/home-panel) in a universal format. Feel free to use it as a source for your own projects.

# Supported sources
- 1-Wire temperature sensors
- Network UPS Tools

# Supported destinations
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

# How to use it?
1. Run `./universal-data-source` to generate a default configuration file. You can also specify a path to a custom configuration file using `UDS_RS_CONFIG_FILE` environment variable (ex. `UDS_RS_CONFIG_FILE=/etc/universal-data-source/config.toml universal-data-source`).
2. Edit the configuration file to your needs. Most of the settings are optional and have default values. See [Configuration](#configuration) section for more details.
3. Run `./universal-data-source` again to start the program. Remember to keep the `UDS_RS_CONFIG_FILE` environment variable set if you're using a custom configuration file.

# Configuration
## Environment variables
| key                | default         | description                                                                        | required |
| ------------------ | --------------- | ---------------------------------------------------------------------------------- | -------- |
| UDS_RS_CONFIG_FILE | `./config.json` | Path to the configuration file.                                                    | no       |
| RUST_LOG           | `error`         | See [log::Level](https://docs.rs/log/latest/log/enum.Level.html) for more details. | no       |

## All top-level options
The configuration file is written as a JSON object. See table below for a list of all available options.
| key                   | type                            | default               | description                                   | required |
| --------------------- | ------------------------------- | --------------------- | --------------------------------------------- | -------- |
| endpoints             | `Endpoint[]`                    | -                     | List of endpoints to which data will be sent. | **yes**  |
| send_interval         | `Duration`                      | 5s                    | Interval between sending data to endpoints.   | no       |
| enable_one_wire       | `bool`                          | false                 | Enable 1-Wire temperature sensors.            | no       |
| one_wire_path_prefix  | `string`                        | `/sys/bus/w1/devices` | Path to 1-Wire sysfs directory.               | no       |
| enable_ups_monitoring | `bool`                          | false                 | Enable Network UPS Tools monitoring.          | no       |
| nut_connections       | `NetworkUpsToolsClientConfig[]` | []                    | List of NUT servers to connect to.            | **yes**  |

## Types explained
### `Endpoint`
| key          | type     | default | description                                | required |
| ------------ | -------- | ------- | ------------------------------------------ | -------- |
| url          | `string` | -       | URL to which data will be sent.            | **yes**  |
| bearer_token | `string` | -       | Bearer token to be sent with each request. | no       |

### `Duration`
| key   | type     | default | description | required |
| ----- | -------- | ------- | ----------- | -------- |
| secs  | `number` | 5       | seconds     | **yes**  |
| nanos | `number` | 0       | nanoseconds | **yes**  |

### `NetworkUpsToolsClientConfig`
| key        | type                           | default | description                                | required |
| ---------- | ------------------------------ | ------- | ------------------------------------------ | -------- |
| host       | `string`                       | -       | Hostname or IP address of the NUT server.  | **yes**  |
| port       | `number`                       | 3493    | Port on which the NUT server is listening. | no       |
| enable_tls | `bool`                         | false   | Enable strict TLS?                         | no       |
| username   | `string`                       | -       | Username for authentication.               | no       |
| password   | `string`                       | -       | Password for authentication.               | no       |
| upses      | `UniversalPowerSupplyConfig[]` | -       | List of UPSes to monitor.                  | **yes**  |

### `UniversalPowerSupplyConfig`
| key                  | type       | default                                                    | description                                                               | required |
| -------------------- | ---------- | ---------------------------------------------------------- | ------------------------------------------------------------------------- | -------- |
| name                 | `string`   | -                                                          | Name of the UPS.                                                          | **yes**  |
| variables_to_monitor | `string[]` | `DEFAULT_VARIABLES_TO_MONITOR` inside [ups.rs](src/ups.rs) | List of variables to monitor. If not specified, defaults to (src/ups.rs). | no       |

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

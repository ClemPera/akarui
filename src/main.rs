use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::modem::Modem;
use esp_idf_svc::hal::peripheral;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::mqtt::client::*;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::sys::EspError;
use esp_idf_svc::wifi::*;

use std::time::Duration;
use std::io::{BufReader, BufWriter, Read, Write};
use std::net::TcpStream;
use std::thread::sleep;
use sakura::{YeelightClient, Transition, PowerMode};

#[toml_cfg::toml_config]
pub struct Config {
    #[default("")]
    wifi_ssid: &'static str,
    #[default("")]
    wifi_pass: &'static str,
}

const WIFI_SSID: &str = CONFIG.wifi_ssid;
const WIFI_PASSWORD: &str = CONFIG.wifi_pass;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();
    let sys_loop = EspSystemEventLoop::take().unwrap();
    let nvs = EspDefaultNvsPartition::take().unwrap();

    let mut esp_wifi = EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs.clone()))?;
    let mut wifi = BlockingWifi::wrap(&mut esp_wifi, sys_loop.clone())?;

    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: WIFI_SSID.try_into().unwrap(),
        password: WIFI_PASSWORD.try_into().unwrap(),
        ..Default::default()
    }))?;

    wifi.start()?;
    log::info!("Wifi started");

    const MAX_RETRIES: u32 = 5;
    const RETRY_DELAY_MS: u64 = 3000;

    let mut retries = 0;
    loop {
        match wifi.connect() {
            Ok(_) => {
                log::info!("Wifi connected");
                match wifi.wait_netif_up() {
                    Ok(_) => {
                        log::info!("Wifi netif up");
                        break;
                    }
                    Err(e) => log::warn!("Netif failed to come up: {e}"),
                }
            }
            Err(e) => log::warn!("Wifi connection failed: {e}"),
        }

        retries += 1;
        if retries >= MAX_RETRIES {
            return Err("Wifi failed after max retries".into());
        }

        log::info!("Retrying in {RETRY_DELAY_MS}ms... ({retries}/{MAX_RETRIES})");
        std::thread::sleep(std::time::Duration::from_millis(RETRY_DELAY_MS));
    }

    light_toggle_with_lib();
    // light_toggle();

    log::info!("--EOP--");

    Ok(())
}

fn light_toggle_with_lib() {
    let bulb = YeelightClient::connect("192.168.1.171:55443").unwrap();
    bulb.toggle().unwrap();
}
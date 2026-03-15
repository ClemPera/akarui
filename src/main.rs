use esp_idf_svc::hal::adc::oneshot::config::AdcChannelConfig;
use esp_idf_svc::hal::adc::oneshot::{AdcChannelDriver, AdcDriver};
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::gpio::*;
use esp_idf_svc::hal::adc::attenuation::DB_11;

use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::*;

use std::time::Duration;
use std::thread::sleep;

use sakura::{YeelightClient, Transition};

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

    // Setup hardware
    let peripherals = Peripherals::take().unwrap();
    let mut button_pin = PinDriver::input(peripherals.pins.gpio20)?;
    let adc = AdcDriver::new(peripherals.adc1)?;

    let adc_config = AdcChannelConfig {
        attenuation: DB_11,
        ..Default::default()
    };

    let mut pot_pin = AdcChannelDriver::new(&adc, peripherals.pins.gpio4, &adc_config)?;

    button_pin.set_pull(Pull::Up)?;

    // WIFI
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
        sleep(std::time::Duration::from_millis(RETRY_DELAY_MS));
    }

    // Init light
    let bulb = YeelightClient::connect("192.168.1.171:55443").unwrap();

    // Main loop
    let mut old_brightness = 0;
    loop {
        // Button push
        match button_pin.get_level() {
            Level::High => log::info!("HIGH"),
            Level::Low  => {
                if let Err(e) = bulb.toggle() {
                    log::warn!("Toggle failed: {e}")
                }
                sleep(std::time::Duration::from_millis(300)); //Delay to not trigger multiple times
            },
        }

        //Potentiometer read:
        let pot_val = adc.read(&mut pot_pin)?;
        let mut brightness = ((pot_val as u64 * 100) / 3100) as u8;
        log::info!("{}", brightness);
        if brightness != old_brightness {
            if brightness <= 0 {
                brightness = 1;
            }

            bulb.set_brightness(brightness, Transition::Smooth(300)).unwrap();

            old_brightness = brightness;
        }

        sleep(std::time::Duration::from_millis(100));
    }

    log::warn!("--EOP--");

    Ok(())
}

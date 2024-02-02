//! This example test the RP Pico W on board LED.
//!
//! It does not work with the RP Pico board. See blinky.rs.

#![no_std]
#![no_main]

mod ui;

use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::i2c::{self, Config};
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver, InterruptHandler as USBInterruptHandler};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use {defmt_rtt as _, panic_probe as _};

use self::ui::{draw_ui_task, UiScreen, UiState};

bind_interrupts!(struct Irqs {
  USBCTRL_IRQ => USBInterruptHandler<USB>;
});

type Resource<R> = Mutex<ThreadModeRawMutex, Option<R>>;

const DISPLAY_FRAME_TIME: u64 = 1000 / 4;
static UI_STATE: Resource<UiState> = Resource::new(None);

#[embassy_executor::task]
async fn logger_task(driver: Driver<'static, USB>) {
  embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
  let p = embassy_rp::init(Default::default());
  let driver = Driver::new(p.USB, Irqs);

  spawner.spawn(logger_task(driver)).unwrap();

  let sda = p.PIN_14;
  let scl = p.PIN_15;

  // build the UI state
  {
    *(UI_STATE.lock().await) = Some(UiState::new(i2c::I2c::new_blocking(
      p.I2C1,
      scl,
      sda,
      Config::default(),
    )));
  }
  spawner.spawn(draw_ui_task(&UI_STATE)).unwrap();

  // switch between screens every 2 seconds
  let mut screen_ticker =
    embassy_time::Ticker::every(embassy_time::Duration::from_millis(2000));

  loop {
    screen_ticker.next().await;
    {
      let mut lock = UI_STATE.lock().await;
      let ui = lock.as_mut().unwrap();
      ui.screen = match ui.screen {
        UiScreen::HelloWorld => UiScreen::RustLogo,
        UiScreen::RustLogo => UiScreen::HelloWorld,
      };
    }
  }
}

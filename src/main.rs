//! This example test the RP Pico W on board LED.
//!
//! It does not work with the RP Pico board. See blinky.rs.

#![no_std]
#![no_main]

use core::fmt::Write;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::i2c::{self, Blocking, Config, I2c};
use embassy_rp::peripherals::I2C1;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver, InterruptHandler as USBInterruptHandler};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embedded_graphics::image::{Image, ImageRaw};
use embedded_graphics::mono_font::ascii::FONT_6X12;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::text::Text;
use embedded_graphics::{pixelcolor::BinaryColor, prelude::*};
use ssd1306::mode::BufferedGraphicsMode;
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
  USBCTRL_IRQ => USBInterruptHandler<USB>;
});

const DISPLAY_FRAME_TIME: u64 = 1000 / 4;

const RUST_LOGO_BYTES: &[u8] = include_bytes!("../assets/rust.raw");

type Display = Ssd1306<
  I2CInterface<I2c<'static, I2C1, Blocking>>,
  DisplaySize128x64,
  BufferedGraphicsMode<DisplaySize128x64>,
>;

type Resource<R> = Mutex<ThreadModeRawMutex, Option<R>>;

enum UiScreen {
  HelloWorld,
  RustLogo,
}

impl UiScreen {
  fn draw(&self, display: &mut Display) {
    display.clear_buffer();

    match self {
      UiScreen::HelloWorld => {
        display.clear_buffer();

        Text::new(
          "Hello, World!",
          Point::new(4, 12),
          MonoTextStyle::new(&FONT_6X12, BinaryColor::On),
        )
        .draw(display)
        .unwrap();
      }
      UiScreen::RustLogo => {
        display.clear_buffer();

        let raw: ImageRaw<BinaryColor> = ImageRaw::new(RUST_LOGO_BYTES, 64);
        let im = Image::new(&raw, Point::new(32, 0));

        im.draw(display).unwrap();
      }
    }

    display.flush().unwrap();
  }
}

struct UiState {
  display: Display,
  screen: UiScreen,
}

impl UiState {
  fn new(i2c: I2c<'static, I2C1, Blocking>) -> Self {
    let interface = I2CDisplayInterface::new(i2c);
    let mut display =
      Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    display.init().unwrap();

    Self {
      display,
      screen: UiScreen::HelloWorld,
    }
  }

  fn draw(&mut self) {
    self.screen.draw(&mut self.display);
  }
}

static UI_STATE: Resource<UiState> = Resource::new(None);

#[embassy_executor::task]
async fn logger_task(driver: Driver<'static, USB>) {
  embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
}

#[embassy_executor::task]
async fn draw_ui_task(ui: &'static Resource<UiState>) {
  let mut frame_ticker = embassy_time::Ticker::every(
    embassy_time::Duration::from_millis(DISPLAY_FRAME_TIME),
  );

  loop {
    {
      let mut ui = ui.lock().await;
      ui.as_mut().unwrap().draw();
    }

    frame_ticker.next().await;
  }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
  let p = embassy_rp::init(Default::default());
  let driver = Driver::new(p.USB, Irqs);

  spawner.spawn(logger_task(driver)).unwrap();

  let sda = p.PIN_14;
  let scl = p.PIN_15;

  let i2c = i2c::I2c::new_blocking(p.I2C1, scl, sda, Config::default());
  {
    *(UI_STATE.lock().await) = Some(UiState::new(i2c));
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

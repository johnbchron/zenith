//! This example test the RP Pico W on board LED.
//!
//! It does not work with the RP Pico board. See blinky.rs.

#![no_std]
#![no_main]

use core::fmt::Write;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::i2c::{self, Config};
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver, InterruptHandler as USBInterruptHandler};
use embedded_graphics::mono_font::ascii::FONT_6X12;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::text::Text;
use embedded_graphics::{pixelcolor::BinaryColor, prelude::*};
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
  USBCTRL_IRQ => USBInterruptHandler<USB>;
});

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

  let i2c = i2c::I2c::new_blocking(p.I2C1, scl, sda, Config::default());
  let interface = I2CDisplayInterface::new(i2c);
  let mut display =
    Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
      .into_buffered_graphics_mode();
  display.init().unwrap();

  let startup = embassy_time::Instant::now();

  loop {
    display.clear_buffer();

    let now = embassy_time::Instant::now();
    let elapsed = now - startup;

    // write the elapsed time to a string
    let mut string = heapless::String::<32>::new();
    write!(string, "uptime: {:?}", elapsed.as_millis() as f32 / 1000.0)
      .unwrap();

    Text::new(
      &string,
      Point::new(4, 12),
      MonoTextStyle::new(&FONT_6X12, BinaryColor::On),
    )
    .draw(&mut display)
    .unwrap();

    display.flush().unwrap();
  }
}

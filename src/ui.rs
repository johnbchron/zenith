use embassy_rp::i2c::{Blocking, I2c};
use embassy_rp::peripherals::I2C1;
use embedded_graphics::image::{Image, ImageRaw};
use embedded_graphics::mono_font::ascii::FONT_6X12;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::text::Text;
use ssd1306::mode::BufferedGraphicsMode;
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};

const RUST_LOGO_BYTES: &[u8] = include_bytes!("../assets/rust.raw");

type Display = Ssd1306<
  I2CInterface<I2c<'static, I2C1, Blocking>>,
  DisplaySize128x64,
  BufferedGraphicsMode<DisplaySize128x64>,
>;

pub enum UiScreen {
  HelloWorld,
  RustLogo,
}

impl UiScreen {
  pub fn draw(&self, display: &mut Display) {
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

pub struct UiState {
  display: Display,
  pub screen: UiScreen,
}

impl UiState {
  pub fn new(i2c: I2c<'static, I2C1, Blocking>) -> Self {
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

  pub fn draw(&mut self) {
    self.screen.draw(&mut self.display);
  }
}

#[embassy_executor::task]
pub async fn draw_ui_task(ui: &'static Resource<UiState>) {
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

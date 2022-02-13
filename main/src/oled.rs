use atmega_hal::{
    clock::MHz16,
    pac::TWI,
    port::{
        mode::{Input, PullUp},
        Pin, PD0, PD1,
    },
    I2c,
};
use ssd1306::{
    mode::{BufferedGraphicsMode, DisplayConfig},
    prelude::I2CInterface,
    rotation::DisplayRotation,
    size::DisplaySize128x32,
    I2CDisplayInterface, Ssd1306,
};

pub fn init_display(
    twi: TWI,
    sda: Pin<Input<PullUp>, PD1>,
    scl: Pin<Input<PullUp>, PD0>,
) -> Ssd1306<I2CInterface<I2c<MHz16>>, DisplaySize128x32, BufferedGraphicsMode<DisplaySize128x32>> {
    let i2c = I2c::<MHz16>::new(twi, sda, scl, 51200);
    let interface = I2CDisplayInterface::new(i2c);
    let mut display = Ssd1306::new(interface, DisplaySize128x32, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    display.init().ok();
    display
}

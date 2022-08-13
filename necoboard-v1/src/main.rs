#![no_std]
#![no_main]

use core::{
    cell::RefCell,
    sync::atomic::{AtomicUsize, Ordering},
};

use cortex_m::{
    delay::{self, Delay},
    interrupt::Mutex,
};
use cortex_m_rt::entry;
use defmt_rtt as _;
use embedded_graphics::{
    draw_target::DrawTarget,
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::Point,
    text::Text,
    Drawable,
};
use embedded_hal::spi::MODE_0;
use embedded_time::rate::*;
use hal::{
    gpio::{bank0::Gpio26, FloatingInput, FunctionSpi, Pin},
    Adc, Spi,
};
use heapless::{String, Vec};
use key_matrix::KeyMatrix;
use layout::{Layer, Layout};
use panic_probe as _;
use rp_pico::{
    hal::{self, prelude::*, timer::CountDown, usb::UsbBus, Timer},
    pac::{self, interrupt},
};
use rustkbd_core::{
    keyboard::{DeviceInfo, Key, Keyboard},
    split::{DummyConnection, SplitState},
};
use ssd1306::{
    mode::DisplayConfig, prelude::SPIInterface, rotation::DisplayRotation, size::DisplaySize128x64,
    Ssd1306,
};
use usb_device::class_prelude::UsbBusAllocator;

mod filter;
mod key_matrix;
mod layout;
mod switch_identifier;
mod uart_connection;

type KeyboardType = Keyboard<
    'static,
    2,
    UsbBus,
    KeyMatrix<Delay, Pin<Gpio26, FloatingInput>, 4, 4, 12>,
    DummyConnection,
    CountDown<'static>,
    Layer,
    Layout,
>;
static mut KEYBOARD: Mutex<RefCell<Option<KeyboardType>>> = Mutex::new(RefCell::new(None));

#[entry]
fn main() -> ! {
    // These variables must be static due to lifetime constraints
    static mut TIMER: Option<Timer> = None;
    static mut USB_BUS: Option<UsbBusAllocator<hal::usb::UsbBus>> = None;

    defmt::info!("Launching necoboard-petit EC!");

    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    // The single-cycle I/O block controls our GPIO pins
    let sio = hal::Sio::new(pac.SIO);
    let pins = rp_pico::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );
    // Set up the watchdog driver - needed by the clock setup code
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);
    // The default is to generate a 125 MHz system clock
    let clocks = hal::clocks::init_clocks_and_plls(
        rp_pico::XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();
    let mut delay = delay::Delay::new(core.SYST, clocks.system_clock.freq().integer());
    *TIMER = Some(Timer::new(pac.TIMER, &mut pac.RESETS));
    let adc = Adc::new(pac.ADC, &mut pac.RESETS);

    let usb_bus = UsbBusAllocator::new(hal::usb::UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));
    *USB_BUS = Some(usb_bus);

    delay.delay_ms(100);

    let spi = Spi::<_, _, 8>::new(pac.SPI1).init(
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
        16_000_000u32.Hz(),
        &MODE_0,
    );
    let _ = pins.gpio10.into_mode::<FunctionSpi>();
    let _ = pins.gpio11.into_mode::<FunctionSpi>();
    let interface = SPIInterface::new(
        spi,
        pins.gpio8.into_push_pull_output(),
        pins.gpio9.into_push_pull_output(),
    );
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate90)
        .into_buffered_graphics_mode();
    display.init().ok();

    let key_matrix = KeyMatrix::new(
        [
            pins.gpio15.into(),
            pins.gpio14.into(),
            pins.gpio13.into(),
            pins.gpio12.into(),
        ],
        [
            pins.gpio18.into(),
            pins.gpio19.into(),
            pins.gpio20.into(),
            pins.gpio21.into(),
        ],
        pins.gpio17.into(),
        pins.voltage_monitor.into(),
        pins.gpio28.into(),
        adc,
        pins.gpio26.into_floating_input(),
        delay,
    );

    let device_info = DeviceInfo {
        manufacturer: "necocen",
        vendor_id: 0x0c0d,
        product_id: 0x8030,
        product_name: "necoboard v1",
        serial_number: "17",
    };

    let keyboard = Keyboard::new(
        USB_BUS.as_ref().unwrap(),
        device_info,
        key_matrix,
        TIMER.as_ref().unwrap().count_down(),
        Layout::default(),
    );
    cortex_m::interrupt::free(|cs| unsafe {
        KEYBOARD.borrow(cs).replace(Some(keyboard));
    });

    unsafe {
        // Enable the USB interrupt
        pac::NVIC::unmask(hal::pac::Interrupt::USBCTRL_IRQ);
        pac::NVIC::unmask(hal::pac::Interrupt::UART0_IRQ);
    }
    // defmt のタイムスタンプを実装します
    // タイマを使えば、起動からの時間を表示したりできます
    static COUNT: AtomicUsize = AtomicUsize::new(0);
    defmt::timestamp!("{=usize}", {
        // NOTE(no-CAS) `timestamps` runs with interrupts disabled
        let n = COUNT.load(Ordering::Relaxed);
        COUNT.store(n + 1, Ordering::Relaxed);
        n
    });

    loop {
        cortex_m::interrupt::free(|cs| unsafe {
            let keyboard = KEYBOARD.borrow(cs).borrow();
            let keyboard = keyboard.as_ref().unwrap();
            keyboard.main_loop();
            draw_state(
                &mut display,
                keyboard.layer(),
                keyboard.keys(),
                keyboard.split_state(),
            );
            display.flush().ok();
        });
    }
}

#[allow(non_snake_case)]
#[interrupt]
fn USBCTRL_IRQ() {
    cortex_m::interrupt::free(|cs| unsafe {
        KEYBOARD
            .borrow(cs)
            .borrow()
            .as_ref()
            .map(Keyboard::usb_poll)
    });
}

#[allow(non_snake_case)]
#[interrupt]
fn UART0_IRQ() {
    cortex_m::interrupt::free(|cs| unsafe {
        KEYBOARD
            .borrow(cs)
            .borrow()
            .as_ref()
            .map(Keyboard::split_poll)
    });
    cortex_m::asm::sev();
}

fn draw_state(
    display: &mut impl DrawTarget<Color = BinaryColor>,
    layer: Layer,
    keys: Vec<Key, 6>,
    split: SplitState,
) {
    let char_style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
    display.clear(BinaryColor::Off).ok();

    // print pressed keys
    let mut string = String::<6>::new();
    keys.into_iter()
        .filter(|key| key.is_keyboard_key())
        .map(From::from)
        .for_each(|c| {
            string.push(c).ok();
        });
    Text::new(string.as_str(), Point::new(0, 10), char_style)
        .draw(display)
        .ok();

    // display "Receiver" or "Controller"
    let state = match split {
        SplitState::Undetermined => "Undetermined",
        SplitState::NotAvailable => "N/A",
        SplitState::Controller => "Controller",
        SplitState::Receiver => "Receiver",
    };
    Text::new(state, Point::new(0, 22), char_style)
        .draw(display)
        .ok();

    // display Layer
    let layer = match layer {
        Layer::Default => "Default",
        Layer::Lower => "Lower",
        Layer::Raise => "Raise",
    };
    Text::new(layer, Point::new(0, 34), char_style)
        .draw(display)
        .ok();
}

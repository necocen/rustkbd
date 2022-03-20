#![no_std]
#![no_main]
extern crate panic_halt;

use core::cell::RefCell;

use cortex_m::{
    delay::{self, Delay},
    interrupt::Mutex,
};
use cortex_m_rt::entry;
use embedded_hal::{digital::v2::InputPin, spi::MODE_0};
use embedded_time::rate::*;
use rp_pico::{
    hal::{
        self,
        gpio::{
            bank0::{Gpio0, Gpio1, Gpio4, Gpio5},
            Function, FunctionSpi, FunctionUart, Output, Pin, PushPull, Uart,
        },
        prelude::*,
        spi::Enabled,
        timer::CountDown,
        uart::{common_configs, UartPeripheral},
        usb::UsbBus,
        Spi, Timer,
    },
    pac::{self, interrupt, SPI0, UART0},
};
use rustkbd_core::keyboard::{DeviceInfo, Keyboard};
use rustkbd_rp_pico::{
    split_key_matrix::SplitKeyMatrix, split_layout::SplitLayout, ssd1306_display::Ssd1306Display,
    uart_connection::UartConnection,
};
use ssd1306::{
    mode::DisplayConfig, prelude::SPIInterface, rotation::DisplayRotation, size::DisplaySize128x64,
    Ssd1306,
};
use usb_device::class_prelude::UsbBusAllocator;

type KeyboardType = Keyboard<
    'static,
    3,
    UsbBus,
    SplitKeyMatrix<Delay, 2, 2>,
    Ssd1306Display<
        SPIInterface<
            Spi<Enabled, SPI0, 8>,
            Pin<Gpio4, Output<PushPull>>,
            Pin<Gpio5, Output<PushPull>>,
        >,
        DisplaySize128x64,
    >,
    UartConnection<UART0, (Pin<Gpio0, Function<Uart>>, Pin<Gpio1, Function<Uart>>)>,
    CountDown<'static>,
    SplitLayout,
>;
static mut KEYBOARD: Mutex<RefCell<Option<KeyboardType>>> = Mutex::new(RefCell::new(None));

#[entry]
fn main() -> ! {
    // These variables must be static due to lifetime constraints
    static mut TIMER: Option<Timer> = None;
    static mut USB_BUS: Option<UsbBusAllocator<hal::usb::UsbBus>> = None;

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

    let usb_bus = UsbBusAllocator::new(hal::usb::UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));
    *USB_BUS = Some(usb_bus);

    let uart_pins = (
        pins.gpio0.into_mode::<FunctionUart>(),
        pins.gpio1.into_mode::<FunctionUart>(),
    );
    let mut uart = UartPeripheral::new(pac.UART0, uart_pins, &mut pac.RESETS)
        .enable(common_configs::_9600_8_N_1, clocks.peripheral_clock.freq())
        .unwrap();
    uart.enable_rx_interrupt();
    let connection = UartConnection(uart);

    // なぜかここで待たないとディスプレイが点灯しない
    delay.delay_ms(100);

    let spi = Spi::<_, _, 8>::new(pac.SPI0).init(
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
        16_000_000u32.Hz(),
        &MODE_0,
    );
    let _ = pins.gpio6.into_mode::<FunctionSpi>();
    let _ = pins.gpio7.into_mode::<FunctionSpi>();
    let interface = SPIInterface::new(spi, pins.gpio4.into_mode(), pins.gpio5.into_mode());

    let mut ssd1306 = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    ssd1306.init().ok();
    let display = Ssd1306Display(ssd1306);

    let key_matrix = SplitKeyMatrix::new(
        [pins.gpio16.into(), pins.gpio17.into()],
        [pins.gpio14.into(), pins.gpio15.into()],
        delay,
        pins.gpio22.into_pull_up_input().is_low().unwrap(),
    );
    let layout = SplitLayout::default();
    let device_info = DeviceInfo {
        manufacturer: "necocen",
        vendor_id: 0x0c0d,
        product_id: 0x802f,
        product_name: "necoboard petit",
        serial_number: "17",
    };
    let keyboard = Keyboard::new(
        USB_BUS.as_ref().unwrap(),
        device_info,
        key_matrix,
        display,
        connection,
        TIMER.as_ref().unwrap().count_down(),
        layout,
    );
    cortex_m::interrupt::free(|cs| unsafe {
        KEYBOARD.borrow(cs).replace(Some(keyboard));
    });

    unsafe {
        // Enable the USB interrupt
        pac::NVIC::unmask(hal::pac::Interrupt::USBCTRL_IRQ);
        pac::NVIC::unmask(hal::pac::Interrupt::UART0_IRQ);
    }
    loop {
        cortex_m::interrupt::free(|cs| unsafe {
            KEYBOARD.borrow(cs).borrow().as_ref().unwrap().main_loop();
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
            .map(|keyboard| keyboard.usb_poll())
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
            .map(|keyboard| keyboard.split_poll())
    });
    cortex_m::asm::sev();
}
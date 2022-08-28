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
use defmt_rtt as _;
use embedded_graphics::{
    draw_target::DrawTarget,
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::Point,
    text::Text,
    Drawable,
};
use embedded_hal::digital::v2::InputPin;
use fugit::{ExtU64, MicrosDurationU32, RateExtU32};
use hal::{
    multicore::{Multicore, Stack},
    sio::Spinlock0,
    timer::Alarm,
    uart::Parity,
    I2C,
};
use heapless::String;
use key_matrix::KeyMatrix;
use panic_probe as _;
use rp_pico::{
    entry,
    hal::{
        self,
        gpio::{
            bank0::{Gpio0, Gpio1},
            Function, FunctionUart, Pin, Uart,
        },
        prelude::*,
        timer::CountDown,
        uart::{common_configs, UartPeripheral},
        usb::UsbBus,
        Timer,
    },
    pac::{self, interrupt, UART0},
};
use rustkbd::{
    keyboard::{Controller, KeyboardState},
    split::{SplitKeySwitches, SplitState},
    usb::{DeviceInfo, UsbCommunicator},
};
use split_layout::{Layer, SplitLayout};
use ssd1306::{
    mode::DisplayConfig, rotation::DisplayRotation, size::DisplaySize128x32, I2CDisplayInterface,
    Ssd1306,
};
use uart_connection::UartConnection;
use usb_device::class_prelude::UsbBusAllocator;

mod key_matrix;
mod split_layout;
mod uart_connection;

type KeyboardType = Controller<
    3,
    12,
    UsbCommunicator<'static, UsbBus>,
    SplitKeySwitches<
        2,
        12,
        UartConnection<UART0, (Pin<Gpio0, Function<Uart>>, Pin<Gpio1, Function<Uart>>)>,
        KeyMatrix<Delay, 2, 2>,
        CountDown<'static>,
    >,
    Layer,
    SplitLayout,
>;
static mut KEYBOARD: Mutex<RefCell<Option<KeyboardType>>> = Mutex::new(RefCell::new(None));
static mut ALARM0: Mutex<RefCell<Option<hal::timer::Alarm0>>> = Mutex::new(RefCell::new(None));
static mut ALARM1: Mutex<RefCell<Option<hal::timer::Alarm1>>> = Mutex::new(RefCell::new(None));
static mut CORE1_STACK: Stack<4096> = Stack::new();

const USB_SEND_INTERVAL: MicrosDurationU32 = MicrosDurationU32::micros(10_000);
const SWITCH_SCAN_INTERVAL: MicrosDurationU32 = MicrosDurationU32::micros(1_000);

#[entry]
fn main() -> ! {
    // These variables must be static due to lifetime constraints
    static mut TIMER: Option<Timer> = None;
    static mut USB_BUS: Option<UsbBusAllocator<hal::usb::UsbBus>> = None;

    defmt::info!("Launching necoboard-petit");

    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    // The single-cycle I/O block controls our GPIO pins
    let mut sio = hal::Sio::new(pac.SIO);
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
    let delay = delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());
    *TIMER = Some(Timer::new(pac.TIMER, &mut pac.RESETS));

    let mut alarm0 = TIMER.as_mut().unwrap().alarm_0().unwrap();
    alarm0.schedule(USB_SEND_INTERVAL).unwrap();
    alarm0.enable_interrupt();
    cortex_m::interrupt::free(|cs| unsafe {
        ALARM0.borrow(cs).replace(Some(alarm0));
    });
    let mut alarm1 = TIMER.as_mut().unwrap().alarm_1().unwrap();
    alarm1.schedule(SWITCH_SCAN_INTERVAL).unwrap();
    alarm1.enable_interrupt();
    cortex_m::interrupt::free(|cs| unsafe {
        ALARM1.borrow(cs).replace(Some(alarm1));
    });

    let mut mc = Multicore::new(&mut pac.PSM, &mut pac.PPB, &mut sio.fifo);
    let cores = mc.cores();
    let core1 = &mut cores[1];

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
    let mut uart_config = common_configs::_115200_8_N_1;
    uart_config.parity = Some(Parity::Even);
    let mut uart = UartPeripheral::new(pac.UART0, uart_pins, &mut pac.RESETS)
        .enable(uart_config, clocks.peripheral_clock.freq())
        .unwrap();
    uart.enable_rx_interrupt();
    let connection = UartConnection(uart);

    let i2c = I2C::i2c0(
        pac.I2C0,
        pins.gpio4.into_mode(),
        pins.gpio5.into_mode(),
        400u32.kHz(),
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
    );
    let interface = I2CDisplayInterface::new(i2c);
    let mut display = Ssd1306::new(interface, DisplaySize128x32, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    display.init().ok();

    let key_switches = SplitKeySwitches::new(
        KeyMatrix::new(
            [pins.gpio16.into(), pins.gpio17.into()],
            [pins.gpio14.into(), pins.gpio15.into()],
            delay,
        ),
        connection,
        TIMER.as_ref().unwrap().count_down(),
        10u64.millis(),
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
    let usb_communicator = UsbCommunicator::new(device_info, USB_BUS.as_ref().unwrap());
    let keyboard = Controller::new(usb_communicator, key_switches, layout);
    cortex_m::interrupt::free(|cs| unsafe {
        KEYBOARD.borrow(cs).replace(Some(keyboard));
    });

    unsafe {
        // Enable the USB interrupt
        pac::NVIC::unmask(hal::pac::Interrupt::USBCTRL_IRQ);
        pac::NVIC::unmask(hal::pac::Interrupt::UART0_IRQ);
        pac::NVIC::unmask(hal::pac::Interrupt::TIMER_IRQ_0);
        pac::NVIC::unmask(hal::pac::Interrupt::TIMER_IRQ_1);
    }

    static COUNT: AtomicUsize = AtomicUsize::new(0);
    defmt::timestamp!("{=usize}", {
        // NOTE(no-CAS) `timestamps` runs with interrupts disabled
        let n = COUNT.load(Ordering::Relaxed);
        COUNT.store(n + 1, Ordering::Relaxed);
        n
    });

    core1
        .spawn(unsafe { &mut CORE1_STACK.mem }, move || loop {
            let (state, split_state) = cortex_m::interrupt::free(|cs| unsafe {
                let _lock = Spinlock0::claim();
                let keyboard = KEYBOARD.borrow(cs).borrow();
                let keyboard = keyboard.as_ref().unwrap();
                (keyboard.get_state(), keyboard.key_switches.state())
            });
            draw_state(&mut display, state, split_state);
            display.flush().ok();
        })
        .unwrap();

    loop {
        cortex_m::asm::wfi();
    }
}

fn draw_state<const RO: usize>(
    display: &mut impl DrawTarget<Color = BinaryColor>,
    state: KeyboardState<Layer, RO>,
    split_state: SplitState,
) {
    let char_style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
    display.clear(BinaryColor::Off).ok();

    // print pressed keys
    let mut string = String::<6>::new();
    state
        .keys
        .into_iter()
        .filter(|key| key.is_keyboard_key())
        .map(From::from)
        .for_each(|c| {
            string.push(c).ok();
        });
    Text::new(string.as_str(), Point::new(0, 10), char_style)
        .draw(display)
        .ok();

    // display "Receiver" or "Controller"
    let split = match split_state {
        SplitState::Undetermined => "Undetermined",
        SplitState::Controller => "Controller",
        SplitState::Receiver => "Receiver",
    };
    Text::new(split, Point::new(0, 20), char_style)
        .draw(display)
        .ok();

    // display Layer
    let layer = match state.layer {
        Layer::Default => "Default",
        Layer::Lower => "Lower",
        Layer::Raise => "Raise",
    };
    Text::new(layer, Point::new(0, 30), char_style)
        .draw(display)
        .ok();
}

#[allow(non_snake_case)]
#[interrupt]
fn USBCTRL_IRQ() {
    cortex_m::interrupt::free(|cs| unsafe {
        let _lock = Spinlock0::claim();
        KEYBOARD
            .borrow(cs)
            .borrow_mut()
            .as_mut()
            .map(|keyboard| keyboard.communicator.poll())
    });
}

#[allow(non_snake_case)]
#[interrupt]
fn UART0_IRQ() {
    cortex_m::interrupt::free(|cs| unsafe {
        let _lock = Spinlock0::claim();
        KEYBOARD
            .borrow(cs)
            .borrow()
            .as_ref()
            .map(|keyboard| keyboard.key_switches.poll())
    });
    cortex_m::asm::sev();
}

#[allow(non_snake_case)]
#[interrupt]
fn TIMER_IRQ_0() {
    cortex_m::interrupt::free(|cs| unsafe {
        let _lock = Spinlock0::claim();
        let mut alarm = ALARM0.borrow(cs).borrow_mut();
        let alarm = alarm.as_mut().unwrap();
        alarm.clear_interrupt();
        alarm.schedule(USB_SEND_INTERVAL).unwrap();
        alarm.enable_interrupt();
        if let Some(Err(e)) = KEYBOARD
            .borrow(cs)
            .borrow()
            .as_ref()
            .map(Controller::send_keys)
        {
            defmt::warn!("UsbError: {}", defmt::Debug2Format(&e));
        }
    });
}

#[allow(non_snake_case)]
#[interrupt]
fn TIMER_IRQ_1() {
    cortex_m::interrupt::free(|cs| unsafe {
        let _lock = Spinlock0::claim();
        let mut alarm = ALARM1.borrow(cs).borrow_mut();
        let alarm = alarm.as_mut().unwrap();
        alarm.clear_interrupt();
        alarm.schedule(SWITCH_SCAN_INTERVAL).unwrap();
        alarm.enable_interrupt();
        KEYBOARD
            .borrow(cs)
            .borrow()
            .as_ref()
            .map(Controller::main_loop)
    });
}

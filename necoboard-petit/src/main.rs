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
use embedded_hal::{digital::v2::InputPin, spi::MODE_0};
use embedded_time::{duration::Extensions, rate::*};
use hal::{
    multicore::{Multicore, Stack},
    sio::Spinlock0,
    timer::Alarm,
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
            Function, FunctionSpi, FunctionUart, Pin, Uart,
        },
        prelude::*,
        timer::CountDown,
        uart::{common_configs, UartPeripheral},
        usb::UsbBus,
        Spi, Timer,
    },
    pac::{self, interrupt, UART0},
};
use rustkbd_core::{
    keyboard::{Controller, KeyboardState},
    split::{SplitKeySwitches, SplitState},
    usb::{DeviceInfo, UsbCommunicator},
};
use split_layout::{Layer, SplitLayout};
use ssd1306::{
    mode::DisplayConfig, prelude::SPIInterface, rotation::DisplayRotation, size::DisplaySize128x64,
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
static mut ALARM: Mutex<RefCell<Option<hal::timer::Alarm0>>> = Mutex::new(RefCell::new(None));
static mut CORE1_STACK: Stack<4096> = Stack::new();

const USB_SEND_INTERVAL_MICROS: u32 = 10_000;

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
    let mut delay = delay::Delay::new(core.SYST, clocks.system_clock.freq().integer());
    *TIMER = Some(Timer::new(pac.TIMER, &mut pac.RESETS));

    let mut alarm = TIMER.as_mut().unwrap().alarm_0().unwrap();
    alarm
        .schedule(USB_SEND_INTERVAL_MICROS.microseconds())
        .unwrap();
    alarm.enable_interrupt();
    cortex_m::interrupt::free(|cs| unsafe {
        ALARM.borrow(cs).replace(Some(alarm));
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
    let interface = SPIInterface::new(
        spi,
        pins.gpio4.into_push_pull_output(),
        pins.gpio5.into_push_pull_output(),
    );
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate90)
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
        cortex_m::interrupt::free(|cs| unsafe {
            let _lock = Spinlock0::claim();
            KEYBOARD
                .borrow(cs)
                .borrow()
                .as_ref()
                .map(Controller::main_loop);
        });
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
    Text::new(split, Point::new(0, 22), char_style)
        .draw(display)
        .ok();

    // display Layer
    let layer = match state.layer {
        Layer::Default => "Default",
        Layer::Lower => "Lower",
        Layer::Raise => "Raise",
    };
    Text::new(layer, Point::new(0, 34), char_style)
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
        let mut alarm = ALARM.borrow(cs).borrow_mut();
        let alarm = alarm.as_mut().unwrap();
        alarm.clear_interrupt();
        alarm
            .schedule(USB_SEND_INTERVAL_MICROS.microseconds())
            .unwrap();
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

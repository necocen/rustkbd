use core::{
    cell::RefCell,
    mem::{transmute_copy, MaybeUninit},
};

use cortex_m::prelude::_embedded_hal_adc_OneShot;
use embedded_hal::{adc::Channel, blocking::delay::DelayUs, digital::v2::OutputPin};
use rp2040_hal::adc::Adc;
use rp_pico::hal::gpio::DynPin;
use rustkbd::{keyboard::KeySwitches, Vec};

use crate::{buffer::Buffer, kalman_filter::KalmanFilter, switch_identifier::KeySwitchIdentifier};

pub struct KeyMatrix<
    D: DelayUs<u16>,
    P: Channel<Adc, ID = u8>,
    const ROWS: usize,
    const CSELS: usize,
    const COLS: usize,
> {
    rows: RefCell<[DynPin; ROWS]>,
    mux_selectors: RefCell<[DynPin; CSELS]>,
    mux_enabled: RefCell<DynPin>,
    opa_shutdown: RefCell<DynPin>,
    rst_charge: RefCell<DynPin>,
    adc: RefCell<Adc>,
    adc_pin: RefCell<P>,
    delay: RefCell<D>,
    filters: [[KalmanFilter; COLS]; ROWS],
    buffers: RefCell<[[Buffer<3>; COLS]; ROWS]>,
}

impl<
        D: DelayUs<u16>,
        P: Channel<Adc, ID = u8>,
        const ROWS: usize,
        const CSELS: usize,
        const COLS: usize,
    > KeyMatrix<D, P, ROWS, CSELS, COLS>
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        mut rows: [DynPin; ROWS],
        mut mux_selectors: [DynPin; CSELS],
        mut mux_enabled: DynPin,
        mut opa_shutdown: DynPin,
        mut rst_charge: DynPin,
        adc: Adc,
        adc_pin: P,
        delay: D,
    ) -> KeyMatrix<D, P, ROWS, CSELS, COLS> {
        for pin in rows.iter_mut() {
            pin.into_push_pull_output()
        }
        for pin in mux_selectors.iter_mut() {
            pin.into_push_pull_output()
        }
        mux_enabled.into_push_pull_output();
        mux_enabled.set_high().ok();
        opa_shutdown.into_push_pull_output();
        opa_shutdown.set_low().ok();
        rst_charge.into_push_pull_output();
        rst_charge.set_high().ok();

        let mut filters: [[MaybeUninit<KalmanFilter>; COLS]; ROWS] =
            unsafe { MaybeUninit::uninit().assume_init() };
        for slot in filters.iter_mut() {
            for slot in slot.iter_mut() {
                *slot = MaybeUninit::new(KalmanFilter::new());
            }
        }

        let mut buffers: [[MaybeUninit<Buffer<3>>; COLS]; ROWS] =
            unsafe { MaybeUninit::uninit().assume_init() };
        for slot in buffers.iter_mut() {
            for slot in slot.iter_mut() {
                *slot = MaybeUninit::new(Buffer::new());
            }
        }

        KeyMatrix {
            rows: RefCell::new(rows),
            mux_selectors: RefCell::new(mux_selectors),
            mux_enabled: RefCell::new(mux_enabled),
            opa_shutdown: RefCell::new(opa_shutdown),
            rst_charge: RefCell::new(rst_charge),
            adc: RefCell::new(adc),
            adc_pin: RefCell::new(adc_pin),
            delay: RefCell::new(delay),
            filters: unsafe { transmute_copy::<_, [[KalmanFilter; COLS]; ROWS]>(&filters) },
            buffers: RefCell::new(unsafe {
                transmute_copy::<_, [[Buffer<3>; COLS]; ROWS]>(&buffers)
            }),
        }
    }
}

impl<
        D: DelayUs<u16>,
        P: Channel<Adc, ID = u8>,
        const ROWS: usize,
        const CSELS: usize,
        const COLS: usize,
    > KeySwitches<2, 12> for KeyMatrix<D, P, ROWS, CSELS, COLS>
{
    type Identifier = KeySwitchIdentifier;

    fn scan(&self) -> Vec<Self::Identifier, 12> {
        let mut keys = Vec::<Self::Identifier, 12>::new();
        let mut rows = self.rows.borrow_mut();
        let mut delay = self.delay.borrow_mut();
        let mut csels = self.mux_selectors.borrow_mut();
        let mut rst_charge = self.rst_charge.borrow_mut();
        let mut adc = self.adc.borrow_mut();
        let mut adc_pin = self.adc_pin.borrow_mut();
        let mut buffers = self.buffers.borrow_mut();

        // opa_shutdownとmux_enabledは実際はHi/Loが逆
        self.opa_shutdown.borrow_mut().set_high().ok();
        self.mux_enabled.borrow_mut().set_low().ok();

        delay.delay_us(10);

        for col in 0..COLS {
            // マルチプレクサの設定
            self.mux_enabled.borrow_mut().set_high().ok();
            for sel in 0..CSELS {
                csels[sel].set_state((col & (1 << sel) != 0).into()).ok();
            }
            self.mux_enabled.borrow_mut().set_low().ok();
            delay.delay_us(10);

            for row in 0..ROWS {
                rst_charge.set_low().ok();
                delay.delay_us(50);
                rows[row].set_high().unwrap();
                delay.delay_us(10);

                let val: u16 = adc.read(&mut *adc_pin).unwrap_or(0);
                delay.delay_us(10);
                // if col == 0 && row == 0 {
                //     defmt::debug!("{}", val);
                // }
                let val = self.filters[row][col].predict(val.into());
                if buffers[row][col].update(val > 40.0) {
                    let key_identifier = KeySwitchIdentifier {
                        row: row as u8,
                        col: col as u8,
                    };
                    keys.push(key_identifier).ok();
                }

                rows[row].set_low().unwrap();
                rst_charge.set_high().ok();
                delay.delay_us(5);
            }
        }

        self.mux_enabled.borrow_mut().set_high().ok();
        self.opa_shutdown.borrow_mut().set_low().ok();

        keys
    }
}

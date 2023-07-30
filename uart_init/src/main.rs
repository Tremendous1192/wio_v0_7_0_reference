//! Wio terminal 以外に必要な準備は下記のURLを参照してください
//! https://github.com/Tremendous1192/rust-wio-terminal-exercise/tree/main/ch06_p184_01_uart
//! 組込みRustのおまじない
#![no_std] // 必須アトリビュート
#![no_main] // 必須アトリビュート
use panic_halt as _; // 必須クレート
use wio::prelude::*; // ほぼ必須
use wio_terminal as wio; // 必須クレート

#[wio_terminal::entry] // 必須アトリビュート
fn main() -> ! {
    // 初期化
    // 必須インスタンス
    let mut peripherals = wio::pac::Peripherals::take().unwrap();
    //let core = wio::pac::CorePeripherals::take().unwrap();
    let mut clocks = wio::hal::clock::GenericClockController::with_external_32kosc(
        peripherals.GCLK,
        &mut peripherals.MCLK,
        &mut peripherals.OSC32KCTRL,
        &mut peripherals.OSCCTRL,
        &mut peripherals.NVMCTRL,
    );
    //let mut delay = wio::hal::delay::Delay::new(core.SYST, &mut clocks);
    let sets = wio::Pins::new(peripherals.PORT).split();

    // UARTドライバオブジェクト
    let mut serial: wio::HalUart = sets.uart.init(
        &mut clocks,
        9600.Hz(),
        peripherals.SERCOM2,
        &mut peripherals.MCLK,
    );
    // ここまで 初期化

    // Tera Term に「hello world」と出力する
    for c in b"hello world\n".iter() {
        nb::block!(serial.write(*c)).unwrap();
    }

    // 組込みはloop必須
    loop {}
    // ここまでloop処理
}
// ここまでmain関数

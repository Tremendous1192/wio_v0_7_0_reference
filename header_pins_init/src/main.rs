//! 正面右側の Grove ピンに差し込んだブザーを鳴らすプログラム
//! 必要なものは下記URLを参照してください
//! https://github.com/Tremendous1192/rust-wio-terminal-exercise/tree/main/ex_grove_02
//! 組込みRustのおまじない
#![no_std] // 必須アトリビュート
#![no_main] // 必須アトリビュート
use panic_halt as _; // 必須クレート
use wio::prelude::*; // ほぼ必須
use wio_terminal as wio; // 必須クレート

#[wio_terminal::entry] // 必須アトリビュート
fn main() -> ! {
    // 初期化
    // 必須
    let mut peripherals = wio::pac::Peripherals::take().unwrap();
    let core = wio::pac::CorePeripherals::take().unwrap();
    let mut clocks = wio::hal::clock::GenericClockController::with_external_32kosc(
        peripherals.GCLK,
        &mut peripherals.MCLK,
        &mut peripherals.OSC32KCTRL,
        &mut peripherals.OSCCTRL,
        &mut peripherals.NVMCTRL,
    );
    let mut delay = wio::hal::delay::Delay::new(core.SYST, &mut clocks);
    let mut sets: wio::Sets = wio::Pins::new(peripherals.PORT).split();

    // 正面右側の Grove ピンの送信側インスタンス
    let mut a0_d0 = sets.header_pins.a0_d0.into_push_pull_output();
    a0_d0.set_low().unwrap();
    // 受信は sets.header_pins.a1_d1 のようだ

    // ここまで 初期化

    // 正面右側につないだブザーを鳴らす
    delay.delay_ms(100u8);
    a0_d0.toggle().ok();
    delay.delay_ms(100u8);
    a0_d0.toggle().ok();

    // 組込みはloop必須
    loop {}
    // ここまでloop処理
}
// ここまでmain関数
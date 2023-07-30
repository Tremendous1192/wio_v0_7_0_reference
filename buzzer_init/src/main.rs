//! 組込みRustのおまじない
#![no_std] // 必須アトリビュート
#![no_main] // 必須アトリビュート
use panic_halt as _; // 必須クレート
use wio::prelude::*; // ほぼ必須
use wio_terminal as wio; // 必須クレート

use wio::hal::pwm::Channel; // ブザー

#[wio::entry] // 必須アトリビュート
fn main() -> ! {
    // 初期化
    // 必須インスタンス
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
    let sets = wio::Pins::new(peripherals.PORT).split();

    // ブザー
    // Tcc0Pwm<BuzzerCtrlId, BuzzerCtrlMode>
    let mut buzzer = sets
        .buzzer
        .init(&mut clocks, peripherals.TCC0, &mut peripherals.MCLK);
    // ここまで 初期化

    // ド の周波数を設定する
    let freq_c = 261_u32;
    buzzer.set_period(freq_c.Hz());
    buzzer.set_duty(Channel::_4, buzzer.get_max_duty() / 2);

    // ブザーを1秒間鳴らす
    buzzer.enable(Channel::_4);
    delay.delay_ms(1_000_u16);
    buzzer.disable(Channel::_4);
    
    // 組込みはloop必須
    loop {}
    // ここまでloop処理
}
// ここまでmain関数

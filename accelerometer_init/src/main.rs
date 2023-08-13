//! 組込みRustのおまじない
#![no_std] // 必須アトリビュート
#![no_main] // 必須アトリビュート
use panic_halt as _; // 必須クレート
use wio::prelude::*; // ほぼ必須
use wio_terminal as wio; // 必須クレート

// 描画
use eg::pixelcolor::Rgb565;
use eg::prelude::*;
use eg::primitives::{PrimitiveStyleBuilder, Rectangle};
use embedded_graphics as eg;

use ryu; // float型を文字列に変換する

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

    // LCDディスプレイ
    let (mut display, _backlight) = sets
        .display
        .init(
            &mut clocks,
            peripherals.SERCOM7,
            &mut peripherals.MCLK,
            58.MHz(),
            &mut delay,
        )
        .unwrap();

    // 画面を黒色で塗りつぶす
    let style = PrimitiveStyleBuilder::new()
        .fill_color(Rgb565::BLACK)
        .build();
    let backdrop =
        Rectangle::with_corners(Point::new(0, 0), Point::new(320, 240)).into_styled(style);
    backdrop.draw(&mut display).unwrap();
    // 文字を描画する部分だけ黒く塗りつぶす
    let mini_back_drop =
        Rectangle::with_corners(Point::new(0, 0), Point::new(150, 60)).into_styled(style);

    // 加速度計
    // lis3dh::Lis3dh
    let mut lis3dh =
        sets.accelerometer
            .init(&mut clocks, peripherals.SERCOM4, &mut peripherals.MCLK);
    // ここまで 初期化

    // 組込みはloop必須
    loop {
        // 加速度
        // micromath::vector::F32x3
        let vec = lis3dh.accel_norm().unwrap();
        let mut buffer = ryu::Buffer::new();

        // 加速度を画面に表示する
        let style = eg::mono_font::MonoTextStyle::new(
            &eg::mono_font::ascii::FONT_10X20,
            eg::pixelcolor::Rgb565::WHITE,
        );
        eg::text::Text::new(
            buffer.format(vec.x),
            eg::prelude::Point::new(15_i32, 15_i32),
            style,
        )
        .draw(&mut display)
        .unwrap();
        eg::text::Text::new(
            buffer.format(vec.y),
            eg::prelude::Point::new(15_i32, 35_i32),
            style,
        )
        .draw(&mut display)
        .unwrap();
        eg::text::Text::new(
            buffer.format(vec.z),
            eg::prelude::Point::new(15_i32, 50_i32),
            style,
        )
        .draw(&mut display)
        .unwrap();

        // 30fps
        delay.delay_ms(33_u16);
        mini_back_drop.draw(&mut display).unwrap();
    }
    // ここまでloop処理
}
// ここまでmain関数

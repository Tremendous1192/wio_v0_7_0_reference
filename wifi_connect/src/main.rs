//! Wifiルーターに接続するプログラム.
//! https://github.com/atsamd-rs/atsamd/blob/0820f0df58eb8705ddfa6533ed76953d18e6b992/boards/wio_terminal/examples/wifi_connect.rs
//!
//! Note 1: Wio Terminal の Wi-Fi ファームウェアアップデートが必要である
//! https://wiki.seeedstudio.com/Wio-Terminal-Network-Overview/
//!
//! Note 2: 下記のコマンドで上手く起動するようになる.理由は不明
//! cargo hf2 --vid 0x2886 --pid 0x002d --release
//!
//! 参考になった Issue と Post
//! wio-terminal Wi-Fi examples broken by #542 #628
//! https://github.com/atsamd-rs/atsamd/issues/628
//! https://github.com/atsamd-rs/atsamd/issues/628#issuecomment-1337554363
//!
//! 組込みRustのおまじない
#![no_std] // 必須アトリビュート
#![no_main] // 必須アトリビュート
use panic_halt as _; // 必須クレート
use wio::prelude::*; // ほぼ必須
use wio_terminal as wio; // 必須クレート

// データのやり取り
use core::fmt::Write;
use cortex_m::interrupt::free as disable_interrupts;
use heapless::String;

// 描画
use eg::mono_font::{ascii::FONT_6X12, MonoTextStyle};
use eg::pixelcolor::Rgb565;
use eg::prelude::*;
use eg::text::{Baseline, Text};
use embedded_graphics as eg;

// Wi-Fi
use wio::hal::clock::GenericClockController;
use wio::hal::delay::Delay;
use wio::wifi_prelude::*;
use wio::wifi_rpcs as rpc;
use wio::wifi_singleton;
use wio::wifi_types::Security;
// Wi-Fiシングルトンと割り込み処理を生成するマクロ
// WIFI: Option<Wifi> = Some(Wifi::init(略));
wifi_singleton!(WIFI);

#[wio::entry]
fn main() -> ! {
    // 初期化
    // 必須インスタンス
    let mut peripherals = wio::pac::Peripherals::take().unwrap();
    let mut core = wio::pac::CorePeripherals::take().unwrap();
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
    clear(&mut display);
    let mut textbuffer = String::<256_usize>::new();

    // 内蔵LED
    let mut user_led = sets.user_led.into_push_pull_output();
    user_led.set_low().unwrap();
    delay.delay_ms(200_u16);

    // wifi ペリフェラル
    let nvic = &mut core.NVIC;
    disable_interrupts(|cs| unsafe {
        wifi_init(
            cs,
            sets.wifi,
            peripherals.SERCOM0,
            &mut clocks,
            &mut peripherals.MCLK,
            &mut delay,
        );
        if let Some(wifi) = WIFI.as_mut() {
            wifi.enable(cs, nvic);
        }
    });

    // バージョン番号を表示する
    let version = unsafe {
        WIFI.as_mut()
            .map(|wifi| wifi.blocking_rpc(rpc::GetVersion {}).unwrap())
            .unwrap()
    };
    writeln!(textbuffer, "fw: {}", version).unwrap();
    write(
        &mut display,
        textbuffer.as_str(),
        Point::new(320 - (3 + version.len() * 12) as i32, 3),
    );
    textbuffer.truncate(0);

    // mac 番号を表示する
    let mac = unsafe {
        WIFI.as_mut()
            .map(|wifi| wifi.blocking_rpc(rpc::GetMacAddress {}).unwrap())
            .unwrap()
    };
    writeln!(textbuffer, "mac: {}", mac).unwrap();
    write(&mut display, textbuffer.as_str(), Point::new(3, 3));
    textbuffer.truncate(0);

    // Wi-Fi ルーターに接続する
    // Notice!
    // You do NOT have to share your network name and that password.
    // If you want to use this code, you must save this in private repository.
    let ip_info = unsafe {
        WIFI.as_mut()
            .map(|wifi| {
                wifi.connect_to_ap(
                    &mut delay,
                    "NETWORK_NAME",
                    "PASSWORD_HERE",
                    Security::WPA2_SECURITY | Security::AES_ENABLED,
                )
                .unwrap()
            })
            .unwrap()
    };

    // 接続成功の証でLEDを点灯する
    user_led.set_high().ok();

    // Wi-Fi接続の情報を画面に表示する
    writeln!(textbuffer, "ip = {}", ip_info.ip).unwrap();
    write(&mut display, textbuffer.as_str(), Point::new(3, 30));
    textbuffer.truncate(0);
    writeln!(textbuffer, "netmask = {}", ip_info.netmask).unwrap();
    write(&mut display, textbuffer.as_str(), Point::new(3, 42));
    textbuffer.truncate(0);
    writeln!(textbuffer, "gateway = {}", ip_info.gateway).unwrap();
    write(&mut display, textbuffer.as_str(), Point::new(3, 54));
    textbuffer.truncate(0);

    loop {
        user_led.toggle().ok();
        delay.delay_ms(200u8);
    }
}

// 画面を初期化する
fn clear(display: &mut wio::LCD) {
    display.clear(Rgb565::BLACK).ok().unwrap();
}

// 文字を描画する
fn write<'a, T: Into<&'a str>>(display: &mut wio::LCD, text: T, pos: Point) {
    Text::with_baseline(
        text.into(),
        pos,
        MonoTextStyle::new(&FONT_6X12, Rgb565::WHITE),
        Baseline::Top,
    )
    .draw(display)
    .ok()
    .unwrap();
}

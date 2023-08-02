//! wifi 接続可能な端末を表示するプログラム.
//! https://github.com/atsamd-rs/atsamd/blob/0820f0df58eb8705ddfa6533ed76953d18e6b992/boards/wio_terminal/examples/wifi_scan.rs
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
use heapless::String;

// 描画
use eg::mono_font::{ascii::FONT_6X12, MonoTextStyle};
use eg::pixelcolor::Rgb565;
use eg::prelude::*;
use eg::primitives::{PrimitiveStyleBuilder, Rectangle};
use eg::text::{Baseline, Text};
use embedded_graphics as eg;

// Wi-Fi
use wio::hal::clock::GenericClockController;
use wio::hal::delay::Delay;
use wio::wifi_prelude::*;
use wio::wifi_rpcs as rpc;
use wio::wifi_singleton;
// Wi-Fiシングルトンと割り込み処理を生成するマクロ
// WIFI: Option<Wifi> = Some(Wifi::init(略));
wifi_singleton!(WIFI);

// 非同期処理
use cortex_m::interrupt::free as disable_interrupts;

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

    // 組込みはloop必須
    loop {
        user_led.set_low().ok();
        // スキャンを開始する
        unsafe {
            WIFI.as_mut()
                .map(|wifi| wifi.blocking_rpc(rpc::ScanStart {}).unwrap())
                .unwrap()
        };
        // スキャン完了まで待機する
        loop {
            let scanning = unsafe {
                WIFI.as_mut()
                    .map(|wifi| wifi.blocking_rpc(rpc::IsScanning {}).unwrap())
                    .unwrap()
            };
            if !scanning {
                break;
            }
        }

        // アクセスポイント番号を取得する
        let num = unsafe {
            WIFI.as_mut()
                .map(|wifi| wifi.blocking_rpc(rpc::ScanGetNumAPs {}).unwrap())
                .unwrap()
        };
        // 接続可能範囲にあるアクセスポイントを取得する(?)
        let aps = unsafe {
            WIFI.as_mut()
                .map(|wifi| {
                    wifi.blocking_rpc(rpc::ScanGetAP::<generic_array::typenum::consts::U16>::new())
                })
                .unwrap()
        };
        user_led.set_high().ok();

        writeln!(textbuffer, "{:?} APs", num).unwrap();
        write_with_clear(&mut display, textbuffer.as_str(), 3, Point::new(170, 3));
        textbuffer.truncate(0);

        // 各アクセスポイントの情報を画面に表示する
        for (i, ap) in aps.unwrap().0.iter().enumerate() {
            if i >= num as usize {
                break;
            }
            // SSID
            writeln!(textbuffer, "{:?}", ap.ssid).unwrap();
            write_with_clear(
                &mut display,
                textbuffer.as_str(),
                (150 / 6) as i32,
                Point::new(3, 30 + i as i32 * 12),
            );
            textbuffer.truncate(0);

            // BSSID
            writeln!(textbuffer, "{:?}", ap.bssid).unwrap();
            write_with_clear(
                &mut display,
                textbuffer.as_str(),
                18,
                Point::new(150, 30 + i as i32 * 12),
            );
            textbuffer.truncate(0);

            // RSSI
            writeln!(textbuffer, "{:?}", ap.rssi).unwrap();
            write_with_clear(
                &mut display,
                textbuffer.as_str(),
                4,
                Point::new(290, 30 + i as i32 * 12),
            );
            textbuffer.truncate(0);

            // 表示内容を更新する
            if ap.band as u8 == 1 {
                write_with_clear(&mut display, "5G", 3, Point::new(132, 30 + i as i32 * 12));
                textbuffer.truncate(0);
            }
        }
    }
    // ここまでloop処理
}
// ここまでmain関数

// 画面を初期化する
fn clear(display: &mut wio::LCD) {
    let style = PrimitiveStyleBuilder::new()
        .fill_color(Rgb565::BLACK)
        .build();
    let backdrop =
        Rectangle::with_corners(Point::new(0, 0), Point::new(320, 320)).into_styled(style);
    backdrop.draw(display).ok().unwrap();
}

// 文字を記入する(初回)
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

// 前回の表示を削除して新しい情報を表示する
fn write_with_clear<'a, T: Into<&'a str>>(
    display: &mut wio::LCD,
    text: T,
    num_clear: i32,
    pos: Point,
) {
    let style = PrimitiveStyleBuilder::new()
        .fill_color(Rgb565::BLACK)
        .build();
    Rectangle::with_corners(pos, Point::new(pos.x + (6 * num_clear), pos.y + 12))
        .into_styled(style)
        .draw(display)
        .ok()
        .unwrap();

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

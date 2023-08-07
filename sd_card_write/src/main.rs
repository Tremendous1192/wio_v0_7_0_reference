//! 組込みRustのおまじない
#![no_std] // 必須アトリビュート
#![no_main] // 必須アトリビュート
use panic_halt as _; // 必須クレート
use wio::prelude::*; // ほぼ必須
use wio_terminal as wio; // 必須クレート

// SD カード
use embedded_sdmmc::{TimeSource, Timestamp, VolumeIdx};
//use wio::SDCardController;

// SDカード制御インスタンスの引数には TimeSource トレイトの実装が必要
struct Clock;
impl TimeSource for Clock {
    fn get_timestamp(&self) -> Timestamp {
        Timestamp {
            year_since_1970: 0,
            zero_indexed_month: 0,
            zero_indexed_day: 0,
            hours: 0,
            minutes: 0,
            seconds: 0,
        }
    }
}

// 描画
use eg::mono_font::{ascii::FONT_9X15, MonoTextStyle};
use eg::pixelcolor::Rgb565;
use eg::prelude::*;
use eg::text::{Baseline, Text};
use embedded_graphics as eg;

use core::fmt::Write;

use heapless::String;

#[wio::entry] // 必須アトリビュート
fn main() -> ! {
    // 挿入したSDカードに保存されている「RTEST.TXT」ファイルの文字列を読み込むプログラム
    // ファイル名は5文字までしか読み取れないことに注意

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
    // 黒で塗りつぶす
    eg::primitives::Rectangle::with_corners(Point::new(0, 0), Point::new(320, 240))
        .into_styled(
            eg::primitives::PrimitiveStyleBuilder::new()
                .fill_color(Rgb565::BLACK)
                .build(),
        )
        .draw(&mut display)
        .ok()
        .unwrap();

    // SDカード制御
    let (mut cont, _sd_present) = sets
        .sd_card
        .init(
            &mut clocks,
            peripherals.SERCOM6,
            &mut peripherals.MCLK,
            Clock, // TimeSource トレイト
        )
        .unwrap();
    // ここまで 初期化

    // 読み取りまでの待ち時間
    delay.delay_ms(1000_u16);

    let style = MonoTextStyle::new(&FONT_9X15, Rgb565::WHITE);
    match cont.device().init() {
        // SDカードと通信できている場合
        Ok(_) => {
            // SDカードとの通信間隔の設定
            cont.set_baud(20.MHz());

            // ルートディレクトリに移動する
            let mut volume: embedded_sdmmc::Volume = cont.get_volume(VolumeIdx(0)).unwrap();
            let dir = cont.open_root_dir(&volume).unwrap();

            // ファイルへの書き込み
            // ファイルを開く
            let mut my_file = cont
                .open_file_in_dir(
                    &mut volume,
                    &dir,
                    "WTEST.TXT",
                    embedded_sdmmc::Mode::ReadWriteCreateOrAppend,
                )
                .unwrap();

            // ファイルにデータを書き込む
            let message = b"Hi!\n";
            let _ = cont.write(&mut volume, &mut my_file, &message[..]);

            //ファイルを閉じる
            cont.close_file(&volume, my_file).unwrap();
            // ここまで ファイルへの書き込み

            // 待ち時間
            delay.delay_ms(500_u16);

            // ファイルの読み込み
            // ファイルを開く
            let mut my_file = cont
                .open_file_in_dir(
                    &mut volume,
                    &dir,
                    "WTEST.TXT",
                    embedded_sdmmc::Mode::ReadOnly,
                )
                .unwrap();

            // ファイル内のデータを読み込む
            while !my_file.eof() {
                let mut buffer = [0u8; 128];
                let num_read = cont.read(&volume, &mut my_file, &mut buffer).unwrap();
                let mut sentence = String::<128_usize>::new();
                for b in &buffer[0..num_read] {
                    write!(sentence, "{}", *b as char).unwrap();
                }
                Text::with_baseline(sentence.as_str(), Point::new(4, 2), style, Baseline::Top)
                    .draw(&mut display)
                    .ok()
                    .unwrap();
            }

            //ファイルを閉じる
            cont.close_file(&volume, my_file).unwrap();
            // ここまで ファイルの読み込み

            // ルートディレクトリを開放する
            cont.close_dir(&volume, dir);
        }
        Err(e) => {
            // SDカードと接続できない場合何かのメッセージを表示する
            let mut data = String::<128_usize>::new();
            writeln!(data, "Error!: {:?}", e).unwrap();
            Text::with_baseline(data.as_str(), Point::new(4, 2), style, Baseline::Top)
                .draw(&mut display)
                .ok()
                .unwrap();
        }
    }

    // 組込みはloop必須
    loop {}
    // ここまでloop処理
}
// ここまでmain関数

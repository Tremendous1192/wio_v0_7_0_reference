//! 組込みRustのおまじない
#![no_std] // 必須アトリビュート
#![no_main] // 必須アトリビュート
use panic_halt as _; // 必須クレート
use wio::prelude::*; // ほぼ必須
use wio_terminal as wio; // 必須クレート

// SD カード
use embedded_sdmmc::{TimeSource, Timestamp, VolumeIdx};
use wio::SDCardController;

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

    // the ILI9341-based LCD display.
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
            // SDカードと接続できない場合エラーを返す
            cont.set_baud(20.MHz());

            // SDカードとの通信に成功したことと容量を画面に表示する
            let mut data = String::<128_usize>::new();
            write!(data, "OK! ").unwrap();
            match cont.device().card_size_bytes() {
                Ok(size) => writeln!(data, "{}Mb", size / 1024 / 1024).unwrap(),
                Err(e) => writeln!(data, "Err: {:?}", e).unwrap(),
            }
            Text::with_baseline(data.as_str(), Point::new(4, 2), style, Baseline::Top)
                .draw(&mut display)
                .ok()
                .unwrap();

            // SDカード内部のファイル名を画面に表示する関数
            if let Err(e) = print_contents(&mut cont, &mut display) {
                // エラーが出た場合何かのメッセージを表示する
                let mut data = String::<128_usize>::new();
                writeln!(data, "Err: {:?}", e).unwrap();
                Text::with_baseline(data.as_str(), Point::new(4, 20), style, Baseline::Top)
                    .draw(&mut display)
                    .ok()
                    .unwrap();
            }
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

// SDカード内部のファイル名を画面に表示する
// ただし戻り値はResult型
fn print_contents(
    cont: &mut SDCardController<Clock>,
    lcd: &mut wio::LCD,
) -> Result<(), embedded_sdmmc::Error<embedded_sdmmc::SdMmcError>> {
    let style = MonoTextStyle::new(&FONT_9X15, Rgb565::WHITE);

    // volume (or partition) の取得. おそらくメモリ番地(?)の取得
    let volume: embedded_sdmmc::Volume = cont.get_volume(VolumeIdx(0)).unwrap();
    // フォルダーを開く
    let dir = cont.open_root_dir(&volume).unwrap();

    // 序数
    let mut count = 0;
    // 戻り値
    let out = cont.iterate_dir(&volume, &dir, |ent| {
        // ファイル名を 取得する
        let mut data = String::<128_usize>::new();
        writeln!(data, "{} - {:?}", ent.name, ent.attributes).unwrap();
        // ファイル名を画面に表示する
        Text::with_baseline(
            data.as_str(),
            Point::new(4, 20 + count * 16),
            style,
            Baseline::Top,
        )
        .draw(lcd)
        .ok()
        .unwrap();
        // 序数を 1 進める
        count += 1;
    });
    // フォルダの所有権を手放す
    cont.close_dir(&volume, dir);
    out
}

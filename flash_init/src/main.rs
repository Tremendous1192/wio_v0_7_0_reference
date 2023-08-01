//! QSPIを初期化するプログラム
//! ポインタのようなものか(?)
//! https://github.com/atsamd-rs/atsamd/blob/0820f0df58eb8705ddfa6533ed76953d18e6b992/boards/wio_terminal/examples/qspi.rs
//! 組込みRustのおまじない
#![no_std] // 必須アトリビュート
#![no_main] // 必須アトリビュート
use panic_halt as _; // 必須クレート
use wio::prelude::*; // ほぼ必須
use wio_terminal as wio; // 必須クレート

// やり取りするデータの格納先
use bitfield::bitfield;
use heapless::String;

// QSPI
use wio::hal::qspi::{self, Command};

// 状態の保存先 1
bitfield! {
    struct Status1(u8);
    impl Debug;
    pub busy, _: 0;
    pub write_en, _: 1;
    pub block_protect, _: 4, 2;
    pub tb_protect, _: 5;
    pub sector_block_protect, _: 6;
    pub srp, _ : 7;
}

// 状態の保存先 2
bitfield! {
    struct Status2(u8);
    impl Debug;
    pub sr_lock, _: 0;
    pub quad_en, _: 1;
    pub lock_bits, _: 5, 3;
    pub cmp_protect, _: 6;
    pub suspend, _: 7;
}

/// Wait for the write-in-progress and suspended write/erase.
fn wait_ready(flash: &mut qspi::Qspi<qspi::OneShot>) {
    while Status1(flash_status(flash, Command::ReadStatus)).busy() {}
    while Status2(flash_status(flash, Command::ReadStatus2)).suspend() {}
}

/// Returns the contents of the status register indicated by cmd.
fn flash_status(flash: &mut qspi::Qspi<qspi::OneShot>, cmd: Command) -> u8 {
    let mut out = [0u8; 1];
    flash.read_command(cmd, &mut out).ok().unwrap();
    out[0]
}

// 描画
use core::fmt::Write;
use eg::mono_font::{ascii::FONT_6X12, MonoTextStyle};
use eg::pixelcolor::Rgb565;
use eg::prelude::*;
use eg::primitives::{PrimitiveStyleBuilder, Rectangle};
use eg::text::{Baseline, Text};
use embedded_graphics as eg;

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
    let (display, _backlight) = sets
        .display
        .init(
            &mut clocks,
            peripherals.SERCOM7,
            &mut peripherals.MCLK,
            58.MHz(),
            &mut delay,
        )
        .unwrap();
    // ロガー
    let mut terminal = Terminal::new(display);
    // 何のバッファ(?)
    let mut textbuffer = String::<256_usize>::new();

    // 内蔵LED
    let mut user_led = sets.user_led.into_push_pull_output();
    user_led.set_high().unwrap();

    // QSPI Flash pins (uses SERCOM4)
    let mut flash = sets.flash.init(&mut peripherals.MCLK, peripherals.QSPI);

    // QSPIの初期化
    delay.delay_ms(15u8);
    wait_ready(&mut flash); // 他の処理が終わるまで待機
    flash.run_command(Command::EnableReset).unwrap();
    flash.run_command(Command::Reset).unwrap();
    delay.delay_ms(15u8);

    // 通信速度(?)の設定
    // 120MHz / (3-1) = 60mhz
    // w25q can do frequencies of up to 80MHz
    flash.set_clk_divider(3);

    // Quad SPI mode の起動.
    if !Status2(flash_status(&mut flash, Command::ReadStatus2)).quad_en() {
        wait_ready(&mut flash);
        flash.run_command(Command::WriteEnable).unwrap();
        flash
            .write_command(Command::WriteStatus, &[0x00, 0x02])
            .unwrap();
    }

    // One shot mode
    // チップの初期化(?)
    wait_ready(&mut flash);
    flash.run_command(Command::WriteEnable).unwrap();
    terminal.write_str("erasing chip, please wait...");
    flash.erase_command(Command::EraseChip, 0x0).unwrap();
    wait_ready(&mut flash);
    terminal.write_str("DONE.\n");

    // メモリの最初の4バイトの情報を描画する
    let mut read_buf = [0u8; 4];
    flash.read_memory(0, &mut read_buf);
    writeln!(textbuffer, "post-erase read value: {:?}\n", read_buf).unwrap();
    terminal.write_str(textbuffer.as_str());
    textbuffer.truncate(0);

    // 0番アドレスに4バイトの情報を書き込む
    let write_buf = [0x0, 0xff, 0xaa, 0x11];
    wait_ready(&mut flash);
    flash.run_command(Command::WriteEnable).unwrap();
    flash.write_memory(0, &write_buf);
    writeln!(textbuffer, "Wrote {:?} to address 0.\n", write_buf).unwrap();
    terminal.write_str(textbuffer.as_str());
    textbuffer.truncate(0);

    // 0番アドレスから4バイトの情報を読み取る
    let mut read_buf = [0u8; 4];
    wait_ready(&mut flash);
    flash.read_memory(0, &mut read_buf);
    writeln!(textbuffer, "post-write read value: {:?}\n", read_buf).unwrap();
    terminal.write_str(textbuffer.as_str());
    textbuffer.truncate(0);

    // 読み込んだデータと書き込んだはずのデータが異なる場合
    // エラー処理
    if read_buf != write_buf {
        loop {
            user_led.toggle().ok();
            delay.delay_ms(200u8);
        }
    }
    // ここまで One shot mode

    // XIP mode
    // 0x800番アドレスに4バイトの情報を書き込む
    let write_buf = [0x1, 0xaa, 0xce, 0x4];
    wait_ready(&mut flash);
    flash.run_command(Command::WriteEnable).unwrap();
    flash.write_memory(0x800, &write_buf);
    writeln!(textbuffer, "Wrote {:?} to address 0x800.\n", write_buf).unwrap();
    terminal.write_str(textbuffer.as_str());
    textbuffer.truncate(0);

    // XIPモードに切り替えて0x800番アドレスから4バイトの情報を読み取る
    let flash = flash.into_xip();
    let mut read_buf = [0u8; 4];
    unsafe {
        core::ptr::copy(
            (0x04000000 + 0x800) as *mut u8,
            read_buf.as_mut_ptr(),
            read_buf.len(),
        );
    }
    writeln!(textbuffer, "XIP read value: {:?}\n", read_buf).unwrap();
    terminal.write_str(textbuffer.as_str());
    textbuffer.truncate(0);

    // 読み込んだデータと書き込んだはずのデータが異なる場合
    // エラー処理
    if read_buf != write_buf {
        loop {
            user_led.toggle().ok();
            delay.delay_ms(200u8);
        }
    }

    // One shot modeに切り替えて0x800番アドレスから4バイトの情報を読み取る
    let mut flash = flash.into_oneshot();
    let mut read_buf = [0u8; 4];
    wait_ready(&mut flash);
    flash.read_memory(0x800, &mut read_buf);
    writeln!(textbuffer, "post-XIP read value: {:?}\n", read_buf).unwrap();
    terminal.write_str(textbuffer.as_str());
    textbuffer.truncate(0);

    // 読み込んだデータと書き込んだはずのデータが異なる場合
    // エラー処理
    if read_buf != write_buf {
        loop {
            user_led.toggle().ok();
            delay.delay_ms(200u8);
        }
    }

    // 全ての処理が完了したので通信待機状態に移る
    user_led.set_low().unwrap();
    loop {
        cortex_m::asm::wfi();
    }
}

/// ログを画面に表示する構造体
struct Terminal<'a> {
    text_style: MonoTextStyle<'a, Rgb565>,
    cursor: Point,
    display: wio::LCD,
}
impl<'a> Terminal<'a> {
    pub fn new(mut display: wio::LCD) -> Self {
        // Clear the screen.
        let style = PrimitiveStyleBuilder::new()
            .fill_color(Rgb565::BLACK)
            .build();
        let backdrop =
            Rectangle::with_corners(Point::new(0, 0), Point::new(320, 320)).into_styled(style);
        backdrop.draw(&mut display).ok().unwrap();

        Self {
            text_style: MonoTextStyle::new(&FONT_6X12, Rgb565::WHITE),
            cursor: Point::new(0, 0),
            display,
        }
    }

    pub fn write_str(&mut self, str: &str) {
        for character in str.chars() {
            self.write_character(character);
        }
    }

    pub fn write_character(&mut self, c: char) {
        if self.cursor.x >= 320 || c == '\n' {
            self.cursor = Point::new(0, self.cursor.y + FONT_6X12.character_size.height as i32);
        }
        if self.cursor.y >= 240 {
            // Clear the screen.
            let style = PrimitiveStyleBuilder::new()
                .fill_color(Rgb565::BLACK)
                .build();
            let backdrop =
                Rectangle::with_corners(Point::new(0, 0), Point::new(320, 320)).into_styled(style);
            backdrop.draw(&mut self.display).ok().unwrap();
            self.cursor = Point::new(0, 0);
        }

        if c != '\n' {
            let mut buf = [0u8; 8];
            Text::with_baseline(
                c.encode_utf8(&mut buf),
                self.cursor,
                self.text_style,
                Baseline::Top,
            )
            .draw(&mut self.display)
            .ok()
            .unwrap();

            self.cursor.x += (FONT_6X12.character_size.width + FONT_6X12.character_spacing) as i32;
        }
    }
}

//! 組込みRustのおまじない
#![no_std] // 必須アトリビュート
#![no_main] // 必須アトリビュート
use panic_halt as _; // 必須クレート
use wio::prelude::*; // ほぼ必須
use wio_terminal as wio; // 必須クレート

// 非同期
use heapless::spsc::Queue;
use wio::pac::interrupt;

// マイク
use wio::hal::adc::InterruptAdc;
type ConversionMode = wio::hal::adc::FreeRunning;
#[interrupt]
fn ADC1_RESRDY() {
    unsafe {
        let ctx = CTX.as_mut().unwrap();
        let mut producer = ctx.samples.split().0;
        if let Some(sample) = ctx.adc.service_interrupt_ready() {
            producer.enqueue_unchecked(sample);
        }
    }
}
struct Ctx {
    adc: InterruptAdc<wio::pac::ADC1, ConversionMode>,
    samples: Queue<u16, 8_usize>,
}
static mut CTX: Option<Ctx> = None;

// 描画
use eg::pixelcolor::Rgb565;
use eg::prelude::*;
use eg::primitives::{PrimitiveStyleBuilder, Rectangle};
use embedded_graphics as eg;

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

    // 内臓マイク
    // (Adc<ADC1>, MicOutput)
    let (mut microphone_adc, mut microphone_pin) = {
        let (adc, pin) = sets
            .microphone
            .init(peripherals.ADC1, &mut clocks, &mut peripherals.MCLK);
        let interrupt_adc: InterruptAdc<_, ConversionMode> = InterruptAdc::from(adc);
        (interrupt_adc, pin)
    };
    microphone_adc.start_conversion(&mut microphone_pin);
    unsafe {
        CTX = Some(Ctx {
            adc: microphone_adc,
            samples: Queue::new(),
        });
    }
    let mut consumer = unsafe { CTX.as_mut().unwrap().samples.split().1 };
    unsafe {
        cortex_m::peripheral::NVIC::unmask(interrupt::ADC1_RESRDY);
    }
    // ここまで 初期化

    // マイクで拾った音の振幅の和を計算する
    let mut min = core::f32::INFINITY;
    let mut max = core::f32::NEG_INFINITY;
    let mut sum = 0_f32;
    let count_max = 83333; // 実効サンプリングレート 83.333[kSPS]
    for _count in 0..count_max {
        let value = loop {
            if let Some(value) = consumer.dequeue() {
                break value as f32;
            }
        };
        if value < min {
            min = value;
        }
        if max < value {
            max = value
        }
        sum += value;
    }

    // マイクで拾った音の大きさを画面に表示する
    let mut buffer = ryu::Buffer::new();
    let style = eg::mono_font::MonoTextStyle::new(
        &eg::mono_font::ascii::FONT_10X20,
        eg::pixelcolor::Rgb565::WHITE,
    );
    eg::text::Text::new(
        buffer.format(sum),
        eg::prelude::Point::new(15_i32, 15_i32),
        style,
    )
    .draw(&mut display)
    .unwrap();

    // 組込みはloop必須
    loop {}
    // ここまでloop処理
}
// ここまでmain関数

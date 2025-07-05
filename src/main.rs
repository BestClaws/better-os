#![no_std]
#![no_main]

extern crate alloc;

use defmt::info;

use embassy_executor::Spawner;
use embassy_time::Timer;

mod system;
mod tasks;
mod util;


#[esp_hal_embassy::main]
async fn main(spawner: Spawner) -> ! {
        system::kernel::start::start(spawner);
    
    loop {
        Timer::after_millis(1000).await;
    }
    
    
        
}





// let rng = esp_hal::rng::Rng::new(peripherals.RNG);
    //
    // let timer1 = TimerGroup::new(peripherals.TIMG0);
    // let init = esp_wifi::init(
    //     timer1.timer0,
    //     rng.clone(),
    //     peripherals.RADIO_CLK,
    // ).unwrap();
    //
    // let connector = BleConnector::new(&init, peripherals.BT);
    // let controller: ExternalController<_, 20> = ExternalController::new(connector);
    //
    //



    //
    //
    //
    //
    //
    //
  
    //
    //
    // let text_style = MonoTextStyleBuilder::new()
    //     .font(&FONT_8X13_BOLD)
    //     .text_color(BinaryColor::On)
    //     .build();
    //
    //
    // let mut str = String::from("");
    //
    //
    // loop {
    //
    //     ra.wait_for_falling_edge().await;
    //     embassy_time::Timer::after_millis(1).await;
    //
    //
    //
    //     display.flush().await.unwrap();
    //
    //
    //
    //     if ra.is_low() && rb.is_high() {
    //         str = String::from("cw");
    //     } else if ra.is_low() && rb.is_low(){
    //
    //         str = String::from("ccw");
    //     }
    //
    //     display.clear_buffer();
    //     Text::with_baseline(str.as_str(), Point::zero(), text_style, Baseline::Top)
    //         .draw(&mut display)
    //         .unwrap();
    //
    //     embassy_time::Timer::after_millis(5).await;
    //
    //
    // }
    //
    //
    //
    //     display.clear_buffer();
    //     Text::with_baseline(str.as_str(), Point::zero(), text_style, Baseline::Top)
    //         .draw(&mut display)
    //         .unwrap();
    //
    //     display.flush().await.unwrap();
    //
    //
    //     embassy_time::Timer::after_millis(1000).await;
    // }
    //
    //
    //
    //











// fn fill_bw<D>(display: &mut D, color: BinaryColor) -> Result<(), D::Error>
// where
//     D: DrawTarget<Color = BinaryColor>,
// {
//     let style = PrimitiveStyle::with_fill(color);
//
//     Rectangle::new(Point::zero(), display.bounding_box().size)
//         .into_styled(style)
//         .draw(display)?;
//
//     Ok(())
// }


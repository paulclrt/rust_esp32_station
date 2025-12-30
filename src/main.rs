#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

use esp_hal::gpio;
use esp_hal::timer::timg::TimerGroup;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_hal::rng;

use w::mk_static;

esp_bootloader_esp_idf::esp_app_desc!();


#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {

    // initialize peripherals: clock, watchdog, etc
    let peripherals = esp_hal::init(
        esp_hal::Config::default()
            .with_cpu_clock(
                esp_hal::clock::CpuClock::max()
            )
        );
    
    esp_alloc::heap_allocator!(#[unsafe(link_section = ".dram2_uninit")] size: 98767);

    // initilize timer for embassy (rtos) framework
    let timer0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timer0.timer0);

    // use esp_hal::interrupt::software::SoftwareInterruptControl;
    // let software_interrupt = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    // esp_rtos::start_second_core(
    //     software_interrupt.software_interrupt0,
    //     software_interrupt.software_interrupt1,
    //     || {}, // Second core's main function.
    // );

    let radio_init = mk_static!(
        esp_radio::Controller<'static>,
        esp_radio::init().expect("Failed to initialize wifi/ble controller")
        );
    let rng = rng::Rng::new();
    let stack = w::wifi::start_wifi(radio_init, peripherals.WIFI, rng, &spawner).await;

    let web_app = w::web::WebApp::default();
    for id in 0..w::web::WEB_TASK_POOL_SIZE {
        spawner.must_spawn(w::web::web_task(
            id,
            stack,
            web_app.router,
            web_app.config,
        ));
    }


    let led = gpio::Output::new(
            peripherals.GPIO2,
            gpio::Level::Low,
            gpio::OutputConfig::default()
        );
    spawner.spawn(blink_task(led)).unwrap();


    loop {
        let _ = Timer::after(Duration::from_secs(1)).await;
    }

}


#[embassy_executor::task]
async fn blink_task(mut led: gpio::Output<'static>) {
    loop {
        led.toggle();
        let _ = Timer::after(Duration::from_millis(300)).await;
    }
}


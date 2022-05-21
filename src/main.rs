#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![feature(panic_info_message)]
#![test_runner(annex::test::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use alloc::{borrow::ToOwned, format};
use annex::{
    cmos,
    gui::{self, colour, Draw},
    hpet,
    task::{executor::Executor, Task},
    threading,
    utils::font::Font,
};
use log::info;
use x86_64::{
    instructions::{self, interrupts},
    PhysAddr, VirtAddr,
};

mod panic;

bootloader::entry_point!(entry_point);
fn entry_point(info: &'static mut bootloader::BootInfo) -> ! {
    annex::logger::init();

    let framebuffer = info.framebuffer.as_mut().unwrap();
    let rsdp_address = PhysAddr::new(info.rsdp_addr.into_option().unwrap());
    let physical_memory_offset = VirtAddr::new(info.physical_memory_offset.into_option().unwrap());
    let memory_regions = &info.memory_regions;
    annex::init(
        framebuffer,
        rsdp_address,
        physical_memory_offset,
        memory_regions,
    );

    info!("starting kernel");

    // Run the tests if we're running under the test harness
    #[cfg(test)]
    test_main();

    //memory::display_page_table();

    threading::scheduler::with_scheduler(|s| {
        s.set_idle_thread(task_idle, 4096);
        s.add_paused_thread("async", task_async_executor, 4096);
        s.add_paused_thread("screen", task_screen_update, 4096);
        s.add_paused_thread("clock", task_clock, 4096);
        s.set_active(true);
    });

    loop {
        threading::sleep(threading::Deadline::relative(1_000_000_000));
    }
}

fn task_idle() -> ! {
    interrupts::enable();
    loop {
        instructions::hlt();
        threading::yield_now();
    }
}

fn task_screen_update() -> ! {
    interrupts::enable();
    gui::screen::SCREEN
        .try_get()
        .unwrap()
        .lock()
        .clear(colour::GREY);

    let initial = gui::Coordinates::new(0, 0, 300, 150);
    let window = gui::new_window("DVD".to_owned(), initial);
    window.lock().clear(colour::BLUE);

    let screen_width = gui::screen::SCREEN.try_get().unwrap().lock().width();
    let screen_height = gui::screen::SCREEN.try_get().unwrap().lock().height();

    let mut moving_right = true;
    let mut moving_down = true;
    let horizontal_velocity = 3;
    let vertical_velocity = 3;

    let alpha = 0.4;
    let mut last_smoothed_frame_time = 0.;

    let frame_interval = 33_000_000;
    let mut next_frame_start_time = hpet::nanoseconds();

    loop {
        gui::screen::SCREEN.try_get().unwrap().lock().render();
        let mut win = window.lock();

        if moving_right {
            win.coordinates.x += horizontal_velocity;
            if win.coordinates.x + win.width() as isize >= screen_width as isize {
                moving_right = false;
                win.coordinates.x -= horizontal_velocity;
            }
        } else {
            win.coordinates.x -= horizontal_velocity;
            if win.coordinates.x < 0 {
                moving_right = true;
                win.coordinates.x += horizontal_velocity;
            }
        }

        if moving_down {
            win.coordinates.y += vertical_velocity;
            if win.coordinates.y + win.height() as isize >= screen_height as isize {
                moving_down = false;
                win.coordinates.y -= vertical_velocity;
            }
        } else {
            win.coordinates.y -= vertical_velocity;
            // The 20 here is the height of the title block
            if win.coordinates.y < 20 {
                moving_down = true;
                win.coordinates.y += vertical_velocity;
            }
        }

        let elapsed = (hpet::nanoseconds() - next_frame_start_time) as f64 / 1e6;
        last_smoothed_frame_time = alpha * elapsed + (1. - alpha) * last_smoothed_frame_time;

        let current_time = hpet::nanoseconds();
        while next_frame_start_time < current_time {
            next_frame_start_time += frame_interval;
        }
        threading::sleep(threading::Deadline::absolute(next_frame_start_time));
    }
}

fn task_clock() -> ! {
    interrupts::enable();

    let initial = gui::Coordinates::new(60, 50, 300, 150);
    let window = gui::new_window("Clock".to_owned(), initial);
    window.lock().clear(colour::GREEN);

    let font = Font::new(
        noto_sans_mono_bitmap::FontWeight::Regular,
        noto_sans_mono_bitmap::BitmapHeight::Size14,
        colour::TextColour::new(colour::BLACK, colour::GREEN),
    );

    loop {
        let time = cmos::RTC.try_get().unwrap().time();
        let time_string = format!("{}", time.format("%Y/%m/%d %H:%M:%S"));
        font.write(&mut *window.lock(), 10, 10, &time_string);
        threading::sleep(threading::Deadline::relative(100_000_000));
    }
}

fn task_async_executor() -> ! {
    interrupts::enable();

    let mut executor = Executor::new();
    executor.spawn(Task::new(annex::user::shell::run()));

    executor.run();
}

#[test_case]
fn trivial_assertion() {
    assert_eq!(1, 1);
}

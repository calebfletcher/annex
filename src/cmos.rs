use conquer_once::noblock::OnceCell;
use x86_64::instructions::port::Port;

const CMOS_ADDR: u16 = 0x70;
const CMOS_DATA: u16 = 0x71;

const REG_RTC_SECONDS: u8 = 0x00;
const REG_RTC_MINUTES: u8 = 0x02;
const REG_RTC_HOURS: u8 = 0x04;
//const REG_RTC_WEEKDAY: u8 = 0x06;
const REG_RTC_DAYOFMONTH: u8 = 0x07;
const REG_RTC_MONTH: u8 = 0x08;
const REG_RTC_YEAR: u8 = 0x09;
//
//const REG_RTC_STATUSA: u8 = 0x0A;
const REG_RTC_STATUSB: u8 = 0x0B;

fn read(addr: u8) -> u8 {
    let mut cmos_addr: Port<u8> = Port::new(CMOS_ADDR);
    let mut cmos_data: Port<u8> = Port::new(CMOS_DATA);

    unsafe {
        cmos_addr.write(1 << 7 | addr);
        cmos_data.read()
    }
}

#[allow(dead_code)]
fn write(addr: u8, data: u8) {
    let mut cmos_addr: Port<u8> = Port::new(CMOS_ADDR);
    let mut cmos_data: Port<u8> = Port::new(CMOS_DATA);

    unsafe {
        cmos_addr.write(1 << 7 | addr);
        cmos_data.write(data);
    }
}

pub static RTC: OnceCell<Rtc> = OnceCell::uninit();

pub struct Rtc {
    century_register: u8,
}

impl Rtc {
    pub fn new(century_register: u8) -> Self {
        Self { century_register }
    }

    pub fn time(&self) -> chrono::DateTime<chrono::Utc> {
        let status_b = read(REG_RTC_STATUSB);
        let uses_24_hour = (status_b >> 1) & 1 == 1;
        let uses_binary = (status_b >> 2) & 1 == 1;

        let normalise = |value| {
            if uses_binary {
                value
            } else {
                bcd_to_binary(value)
            }
        };

        let decode_hour = |hour| {
            if uses_24_hour {
                bcd_to_binary(hour)
            } else {
                let masked_hour = bcd_to_binary(hour & 0x7F) % 12;
                if (hour >> 7) == 1_u8 {
                    // PM
                    masked_hour + 12
                } else {
                    // AM
                    masked_hour
                }
            }
        };

        let second = normalise(read(REG_RTC_SECONDS)) as u32;
        let minute = normalise(read(REG_RTC_MINUTES)) as u32;
        let hour = decode_hour(read(REG_RTC_HOURS)) as u32;
        let day = normalise(read(REG_RTC_DAYOFMONTH)) as u32;
        let month = normalise(read(REG_RTC_MONTH)) as u32;
        let year = normalise(read(REG_RTC_YEAR));
        let century = normalise(read(self.century_register));
        let year = century as i32 * 100 + year as i32;

        let date = chrono::NaiveDate::from_ymd(year, month, day);
        let time = chrono::NaiveTime::from_hms(hour, minute, second);
        let datetime = chrono::NaiveDateTime::new(date, time);
        chrono::DateTime::from_utc(datetime, chrono::Utc)
    }
}

fn bcd_to_binary(value: u8) -> u8 {
    10 * (value >> 4) + (value & 0xf)
}

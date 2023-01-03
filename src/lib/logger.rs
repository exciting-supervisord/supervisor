use libc::{localtime_r, strftime, tm};
use std::fmt;
use std::mem::{MaybeUninit};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(PartialEq, PartialOrd)]
pub enum LogLevel {
    Crit = 0,
    Warn = 1,
    Info = 2,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                LogLevel::Info => "INFO",
                LogLevel::Warn => "WARN",
                LogLevel::Crit => "CRIT",
            }
        )
    }
}

pub struct Logger(pub LogLevel);

impl Logger {
    fn get_epoch_time() -> (i64, i64) {
        let now = SystemTime::now();
        let since_the_epoch = now.duration_since(UNIX_EPOCH).expect("Time went backwards");

        let millis = since_the_epoch.as_millis();

        ((millis / 1000) as i64, (millis % 1000) as i64)
    }

    fn get_formated_timestamp() -> String {
        let (seconds, millis) = Self::get_epoch_time();

        let mut datetime = unsafe { MaybeUninit::<tm>::zeroed().assume_init() };
        unsafe { localtime_r(&seconds, &mut datetime) };

        let mut buf: [u8; 64] = [0; 64];
        let length = unsafe { strftime(
                buf.as_mut_ptr() as *mut i8,
                64,
                "%Y-%m-%d %H:%M:%S.".as_ptr() as *const i8,
                &datetime,
        ) };
        if length == 0 {
            panic!("strftime returned 0 (exceeded max byte)");
        }

        let buf = buf.into_iter().take(length).collect();
        let mut timestamp = unsafe { String::from_utf8_unchecked(buf) };
        timestamp.push_str(&format!("{:03}", millis));

        timestamp
    }

    fn log(&self, level: LogLevel, message: &str) {
        if self.0 >= level {
            println!("{} {level} {message}", Self::get_formated_timestamp());
        }
    }

    pub fn info(&self, message: &str) {
        self.log(LogLevel::Info, message);
    }

    pub fn warn(&self, message: &str) {
        self.log(LogLevel::Warn, message);
    }

    pub fn crit(&self, message: &str) {
        self.log(LogLevel::Crit, message);
    }
}

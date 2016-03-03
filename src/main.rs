use std::io::{self, Read};
use std::fs::{self, File};
use std::path::PathBuf;
use std::time::Duration;
use std::str::FromStr;
use std::fmt;
use std::num;
use std::string::ParseError;
use std::error::Error;

const POWER_PATH: &'static str = "/sys/class/power_supply";

fn main() {
    let batteries: Vec<PowerSupply> =
        fs::read_dir(POWER_PATH).expect("can't list batteries")
        .map(|entry| PowerSupply::new(entry.unwrap().path()))
        .filter(|ps| ps.is_battery().expect("can't list batteries"))
        .collect();

    for bat in batteries.iter() {
        println!("{}: {}% ({})",
                 bat.name(),
                 bat.percent().expect("Could not read battery"),
                 bat.status().expect("Could not read battery"));
    }
    let runtime = format_time(combined_runtime(&batteries));
    println!("Estimated runtime (all batteries): {}", runtime);
}

#[derive(Debug)]
struct PowerError;

impl Error for PowerError {
    fn description(&self) -> &str {
        "PowerError"
    }

    fn cause(&self) -> Option<&Error> {
        None
    }
}

impl fmt::Display for PowerError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(formatter, "{}", self.description())
    }
}

impl From<num::ParseFloatError> for PowerError {
    fn from(_: num::ParseFloatError) -> PowerError {
        PowerError
    }
}

impl From<num::ParseIntError> for PowerError {
    fn from(_: num::ParseIntError) -> PowerError {
        PowerError
    }
}

impl From<ParseError> for PowerError {
    fn from(_: ParseError) -> PowerError {
        PowerError
    }
}

impl From<io::Error> for PowerError {
    fn from(_: io::Error) -> PowerError {
        PowerError
    }
}

#[derive(Debug, PartialEq, Eq)]
struct PowerSupply {
    base_path: PathBuf,
}

impl PowerSupply {
    fn new(path: PathBuf) -> PowerSupply {
        PowerSupply { base_path: path }
    }

    fn read_prop<T>(&self, prop_name: &str) -> Result<T, PowerError>
        where T: FromStr,
              <T as FromStr>::Err: fmt::Debug,
              PowerError: From<<T as FromStr>::Err>,
    {
        let type_path = self.base_path.join(prop_name);
        let mut prop = String::new();
        let mut f = try!(File::open(type_path));
        try!(f.read_to_string(&mut prop));
        Ok(try!(prop.trim_right().to_string().parse::<T>()))
    }

    fn name(&self) -> &str {
        self.base_path.file_name().unwrap().to_str().unwrap()
    }

    fn is_battery(&self) -> Result<bool, PowerError> {
        Ok(try!(self.read_prop::<String>("type")) == "Battery")
    }

    fn percent(&self) -> Result<i8, PowerError> {
        self.read_prop::<i8>("capacity")
    }

    fn status(&self) -> Result<String, PowerError> {
        self.read_prop::<String>("status")
    }
}

fn combined_runtime(batteries: &[PowerSupply]) -> Duration {
    let total_energy = batteries.iter()
        .map(|b| b.read_prop::<f64>("energy_now").expect("Could not read battery"))
        .fold(0f64, |a, b| a + b); // µW*h
    let total_power = batteries.iter()
        .map(|b| b.read_prop::<f64>("power_now").expect("Could not read battery"))
        .fold(0f64, |a, b| a + b) as f64; // µW*h/h
    let runtime = total_energy / total_power; // hours
    let runtime_ms = runtime * 60f64 * 60f64 * 1000f64;
    Duration::from_millis(runtime_ms as u64)
}

fn format_time(d: Duration) -> String {
    let mut seconds = d.as_secs();
    let hours = seconds / 3600;
    seconds -= hours * 3600;
    let minutes = seconds / 60;

    format!("{}h{}m", hours, minutes)
}

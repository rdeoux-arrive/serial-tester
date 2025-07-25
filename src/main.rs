#[cfg(unix)]
mod posix;
mod tap;

use core::fmt::{self, Display, Formatter};
use std::{
    io::{Error, ErrorKind},
    thread::sleep,
    time::{Duration, Instant},
};

use clap::Parser;
use colored::{ColoredString, Colorize};
use serialport::{ClearBuffer, SerialPort};
use tap::Tap;

#[derive(Parser)]
#[command(version, author, about)]
struct Args {
    /// Path to the first serial port
    pub first: String,
    /// Path to the second serial port
    pub second: String,
}

const BAUD_RATES: [u32; 4] = [9_600, 19_200, 38_400, 115_200];

fn wait<P, E>(mut predicate: P, timeout: Duration) -> Result<bool, E>
where
    P: FnMut() -> Result<bool, E>,
{
    let timeout = Instant::now() + timeout;
    while Instant::now() < timeout {
        if predicate()? {
            return Ok(true);
        }
        sleep(Duration::from_millis(1));
    }
    Ok(false)
}

fn test_pin<F, G>(mut set: F, mut get: G) -> serialport::Result<()>
where
    F: FnMut(bool) -> serialport::Result<()>,
    G: FnMut() -> serialport::Result<bool>,
{
    set(false)?;
    if wait(|| get().map(|level| !level), Duration::from_millis(100))? {
        set(true)?;
        if wait(get, Duration::from_millis(100))? {
            Ok(())
        } else {
            Err(Error::other("stayed low").into())
        }
    } else {
        Err(Error::other("stayed high").into())
    }
}

#[allow(clippy::struct_excessive_bools)]
struct Pins {
    data_terminal_ready: bool,
    data_set_ready: bool,
    request_to_send: bool,
    clear_to_send: bool,
}

impl Display for Pins {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        fn state_of(pin: bool) -> ColoredString {
            if pin { "up".green() } else { "down".red() }
        }

        let dtlr = state_of(self.data_terminal_ready);
        let dstr = state_of(self.data_set_ready);
        let rts = state_of(self.request_to_send);
        let cts = state_of(self.clear_to_send);
        write!(f, "DTR {dtlr}, DSR {dstr}, RTS {rts}, CTS {cts}")
    }
}

fn test_transmit<S: SerialPort>(
    pins: &Pins,
    first: &mut S,
    second: &mut S,
) -> Result<(), serialport::Error> {
    // Define the pins
    first.write_data_terminal_ready(pins.data_terminal_ready)?;
    second.write_data_terminal_ready(pins.data_set_ready)?;
    first.write_request_to_send(pins.request_to_send)?;
    second.write_request_to_send(pins.clear_to_send)?;

    // Wait for the pins to be ready
    wait(
        || -> serialport::Result<bool> {
            Ok(second.read_data_set_ready()? == pins.data_terminal_ready
                && first.read_data_set_ready()? == pins.data_set_ready
                && second.read_clear_to_send()? == pins.request_to_send
                && first.read_clear_to_send()? == pins.clear_to_send)
        },
        Duration::from_millis(100),
    )?;

    // Send a pattern
    let pattern: Vec<_> = (u8::MIN..=u8::MAX).collect();
    first.write_all(&pattern)?;

    // Wait for the input end to receive at least N bytes
    let ready = wait(
        || second.bytes_to_read().map(|i| i as usize >= pattern.len()),
        Duration::from_millis(500),
    )?;

    if !ready {
        return Err(Error::from(ErrorKind::TimedOut).into());
    }

    // Read back
    let size = second.bytes_to_read().unwrap_or_default();
    let mut buf = vec![0; size as usize];
    second.read_exact(&mut buf)?;

    // Compare
    if buf == pattern {
        Ok(())
    } else {
        Err(Error::other(format!(
            "content mismatched: “{}” != “{}”",
            buf.escape_ascii(),
            pattern.escape_ascii()
        ))
        .into())
    }
}

fn main() {
    let args = Args::parse();

    let mut tap = Tap::new(134);

    let first = serialport::new(&args.first, 9600).open_native();
    tap.result(format!("open {:?}", args.first), first.as_ref());
    #[cfg(unix)]
    let first = first.map(posix::FixedTTYPort);

    let second = serialport::new(&args.second, 9600).open_native();
    tap.result(format!("open {:?}", args.second), second.as_ref());
    #[cfg(unix)]
    let second = second.map(posix::FixedTTYPort);

    let mut first = first;
    let mut second = second;

    if let (Ok(first), Ok(second)) = (&mut first, &mut second) {
        tap.result(
            "test RTS → CTS",
            test_pin(
                |level| first.write_request_to_send(level),
                || second.read_clear_to_send(),
            ),
        );
    } else {
        tap.skip("test RTS → CTS");
    }

    if let (Ok(first), Ok(second)) = (&mut first, &mut second) {
        tap.result(
            "test CTS ← RTS",
            test_pin(
                |level| second.write_request_to_send(level),
                || first.read_clear_to_send(),
            ),
        );
    } else {
        tap.skip("test CTS ← RTS");
    }

    if let (Ok(first), Ok(second)) = (&mut first, &mut second) {
        tap.result(
            "test DTR → DSR",
            test_pin(
                |level| first.write_data_terminal_ready(level),
                || second.read_data_set_ready(),
            ),
        );
    } else {
        tap.skip("test DTR → DSR");
    }

    if let (Ok(first), Ok(second)) = (&mut first, &mut second) {
        tap.result(
            "test DSR ← DTR",
            test_pin(
                |level| second.write_data_terminal_ready(level),
                || first.read_data_set_ready(),
            ),
        );
    } else {
        tap.skip("test DSR ← DTR");
    }

    for baud_rate in BAUD_RATES {
        if let (Ok(first), Ok(second)) = (&mut first, &mut second) {
            for port in [first, second] {
                port.set_baud_rate(baud_rate)
                    .expect("failed to set the baudrate");
                port.clear(ClearBuffer::All)
                    .expect("failed to clear buffers");
            }

            sleep(Duration::from_millis(10));
        }

        for pins in 0..=0xf {
            let pins = Pins {
                data_terminal_ready: pins & 1 != 0,
                data_set_ready: pins & 2 != 0,
                request_to_send: pins & 4 != 0,
                clear_to_send: pins & 8 != 0,
            };
            let description = format!("send data at {baud_rate} bps ({pins})");
            if let (Ok(first), Ok(second)) = (&mut first, &mut second) {
                tap.result(description, test_transmit(&pins, first, second));
            } else {
                tap.skip(description);
            }

            let description = format!("receive data at {baud_rate} bps ({pins})");
            if let (Ok(first), Ok(second)) = (&mut first, &mut second) {
                tap.result(description, test_transmit(&pins, second, first));
            } else {
                tap.skip(description);
            }
        }
    }
}

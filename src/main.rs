#[cfg(unix)]
mod posix;
mod tap;

use std::{
    io::{Error, ErrorKind},
    thread::sleep,
    time::{Duration, Instant},
};

use clap::Parser;
use serialport::{ClearBuffer, SerialPort};
use tap::Tap;

/// Test a serial wire
#[derive(Parser)]
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
            Err(Error::new(ErrorKind::Other, "stayed low").into())
        }
    } else {
        Err(Error::new(ErrorKind::Other, "stayed high").into())
    }
}

fn test_transmit<S: SerialPort>(first: &mut S, second: &mut S) -> Result<(), serialport::Error> {
    // Send a pattern
    const PATTERN: &[u8] = b"1234567890";
    first.write_all(PATTERN)?;

    // Wait for the input end to receive at least N bytes
    let ready = wait(
        || second.bytes_to_read().map(|i| i as usize >= PATTERN.len()),
        Duration::from_millis(100),
    )?;

    if !ready {
        return Err(Error::from(ErrorKind::TimedOut).into());
    }

    // Read back
    let size = second.bytes_to_read().unwrap_or_default();
    let mut buf = vec![0; size as usize];
    second.read_exact(&mut buf)?;

    // Compare
    if buf == PATTERN {
        Ok(())
    } else {
        Err(Error::new(
            ErrorKind::Other,
            format!(
                "content mismatched: “{}” != “{}”",
                buf.escape_ascii(),
                PATTERN.escape_ascii()
            ),
        )
        .into())
    }
}

fn main() {
    let args = Args::parse();

    let mut tap = Tap::new(14);

    let first = serialport::new(&args.first, 9600).open_native();
    tap.result(format!("open {:?}", args.first), first.as_ref());

    let second = serialport::new(&args.second, 9600).open_native();
    tap.result(format!("open {:?}", args.second), second.as_ref());

    if let (Ok(first), Ok(second)) = (first, second) {
        #[cfg(unix)]
        let mut first = posix::FixedTTYPort(first);
        #[cfg(windows)]
        let mut first = first;
        #[cfg(unix)]
        let mut second = posix::FixedTTYPort(second);
        #[cfg(windows)]
        let mut second = second;

        tap.result(
            "test RTS → CTS",
            test_pin(
                |level| first.write_request_to_send(level),
                || second.read_clear_to_send(),
            ),
        );

        tap.result(
            "test CTS ← RTS",
            test_pin(
                |level| second.write_request_to_send(level),
                || first.read_clear_to_send(),
            ),
        );

        tap.result(
            "test DTR → DSR",
            test_pin(
                |level| first.write_data_terminal_ready(level),
                || second.read_data_set_ready(),
            ),
        );

        tap.result(
            "test DSR ← DTR",
            test_pin(
                |level| second.write_data_terminal_ready(level),
                || first.read_data_set_ready(),
            ),
        );

        for baud_rate in BAUD_RATES {
            for port in [&mut first, &mut second] {
                port.set_baud_rate(baud_rate)
                    .expect("failed to set the baudrate");
                port.clear(ClearBuffer::All)
                    .expect("failed to clear buffers");
            }

            sleep(Duration::from_millis(10));

            tap.result(
                format!("send data at {baud_rate} bps..."),
                test_transmit(&mut first, &mut second),
            );

            tap.result(
                format!("receive data at {baud_rate} bps..."),
                test_transmit(&mut second, &mut first),
            );
        }
    } else {
        tap.skip("test RTS → CTS");
        tap.skip("test CTS ← RTS");
        tap.skip("test DTR → DSR");
        tap.skip("test DSR ← DTR");
        for baud_rate in BAUD_RATES {
            tap.skip(format!("send data at {baud_rate} bps..."));
            tap.skip(format!("receive data at {baud_rate} bps..."));
        }
    }
}

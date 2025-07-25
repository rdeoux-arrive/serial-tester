use std::{
    ffi::c_int,
    io::{Error, Read, Write},
    os::unix::prelude::AsRawFd,
    time::Duration,
};

use nix::{
    ioctl_read_bad,
    libc::{TIOCM_CD, TIOCM_CTS, TIOCM_DSR, TIOCM_RI, TIOCMGET},
};
use serialport::{ClearBuffer, DataBits, FlowControl, Parity, Result, SerialPort, StopBits};

ioctl_read_bad!(tiocmget, TIOCMGET, c_int);

pub struct FixedTTYPort(pub serialport::TTYPort);

impl FixedTTYPort {
    fn read_pin(&mut self, pin: c_int) -> Result<bool> {
        let mut status = 0;
        unsafe { tiocmget(self.0.as_raw_fd(), &raw mut status) }
            .map(|_| status & pin == pin)
            .map_err(|err| Error::from(err).into())
    }
}

impl Write for FixedTTYPort {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush()
    }
}

impl Read for FixedTTYPort {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(buf)
    }
}

impl SerialPort for FixedTTYPort {
    fn name(&self) -> Option<String> {
        self.0.name()
    }

    fn baud_rate(&self) -> Result<u32> {
        self.0.baud_rate()
    }

    fn data_bits(&self) -> Result<DataBits> {
        self.0.data_bits()
    }

    fn flow_control(&self) -> Result<FlowControl> {
        self.0.flow_control()
    }

    fn parity(&self) -> Result<Parity> {
        self.0.parity()
    }

    fn stop_bits(&self) -> Result<StopBits> {
        self.0.stop_bits()
    }

    fn timeout(&self) -> Duration {
        self.0.timeout()
    }

    fn set_baud_rate(&mut self, baud_rate: u32) -> Result<()> {
        self.0.set_baud_rate(baud_rate)
    }

    fn set_data_bits(&mut self, data_bits: DataBits) -> Result<()> {
        self.0.set_data_bits(data_bits)
    }

    fn set_flow_control(&mut self, flow_control: FlowControl) -> Result<()> {
        self.0.set_flow_control(flow_control)
    }

    fn set_parity(&mut self, parity: Parity) -> Result<()> {
        self.0.set_parity(parity)
    }

    fn set_stop_bits(&mut self, stop_bits: StopBits) -> Result<()> {
        self.0.set_stop_bits(stop_bits)
    }

    fn set_timeout(&mut self, timeout: Duration) -> Result<()> {
        self.0.set_timeout(timeout)
    }

    fn write_request_to_send(&mut self, level: bool) -> Result<()> {
        self.0.write_request_to_send(level)
    }

    fn write_data_terminal_ready(&mut self, level: bool) -> Result<()> {
        self.0.write_data_terminal_ready(level)
    }

    fn read_clear_to_send(&mut self) -> Result<bool> {
        self.read_pin(TIOCM_CTS)
    }

    fn read_data_set_ready(&mut self) -> Result<bool> {
        self.read_pin(TIOCM_DSR)
    }

    fn read_ring_indicator(&mut self) -> Result<bool> {
        self.read_pin(TIOCM_RI)
    }

    fn read_carrier_detect(&mut self) -> Result<bool> {
        self.read_pin(TIOCM_CD)
    }

    fn bytes_to_read(&self) -> Result<u32> {
        self.0.bytes_to_read()
    }

    fn bytes_to_write(&self) -> Result<u32> {
        self.0.bytes_to_write()
    }

    fn clear(&self, buffer_to_clear: ClearBuffer) -> Result<()> {
        self.0.clear(buffer_to_clear)
    }

    fn try_clone(&self) -> Result<Box<dyn SerialPort>> {
        self.0.try_clone()
    }

    fn set_break(&self) -> Result<()> {
        self.0.set_break()
    }

    fn clear_break(&self) -> Result<()> {
        self.0.clear_break()
    }
}

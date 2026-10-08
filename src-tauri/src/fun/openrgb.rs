//! A small client for OpenRGB's SDK server (the free app that controls RGB from most brands):
//! count the devices, read how many LEDs each has, switch them to direct control and set
//! colors. Protocol version 0 — every OpenRGB release speaks it.

use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    time::Duration,
};

const MAGIC: &[u8; 4] = b"ORGB";
const REQUEST_CONTROLLER_COUNT: u32 = 0;
const REQUEST_CONTROLLER_DATA: u32 = 1;
const SET_CLIENT_NAME: u32 = 50;
const UPDATE_LEDS: u32 = 1050;
const SET_CUSTOM_MODE: u32 = 1100;
/// More than any real device has; a bigger number means a damaged reply.
const MAX_LEDS: usize = 4096;

pub fn header(device: u32, packet: u32, size: u32) -> [u8; 16] {
    let mut h = [0u8; 16];
    h[..4].copy_from_slice(MAGIC);
    h[4..8].copy_from_slice(&device.to_le_bytes());
    h[8..12].copy_from_slice(&packet.to_le_bytes());
    h[12..16].copy_from_slice(&size.to_le_bytes());
    h
}

/// The body of an UPDATELEDS packet: every LED the same color.
pub fn update_leds(leds: usize, (r, g, b): (u8, u8, u8)) -> Vec<u8> {
    let size = 4 + 2 + 4 * leds;
    let mut body = Vec::with_capacity(size);
    body.extend_from_slice(&(size as u32).to_le_bytes());
    body.extend_from_slice(&(leds as u16).to_le_bytes());
    for _ in 0..leds {
        body.extend_from_slice(&[r, g, b, 0]);
    }
    body
}

/// Reads a controller description (protocol 0) far enough to find its LED count.
struct Cursor<'a> {
    data: &'a [u8],
    at: usize,
}

impl Cursor<'_> {
    fn take(&mut self, n: usize) -> Option<&[u8]> {
        let slice = self.data.get(self.at..self.at.checked_add(n)?)?;
        self.at += n;
        Some(slice)
    }
    fn u16(&mut self) -> Option<u16> {
        Some(u16::from_le_bytes(self.take(2)?.try_into().ok()?))
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    fn string(&mut self) -> Option<()> {
        let len = self.u16()? as usize;
        self.take(len).map(|_| ())
    }
}

/// How many LEDs a controller has, from its REQUEST_CONTROLLER_DATA reply.
pub fn led_count(data: &[u8]) -> Option<usize> {
    let mut c = Cursor { data, at: 0 };
    c.u32()?; // total size
    c.u32()?; // device type
    for _ in 0..5 {
        c.string()?; // name, description, version, serial, location
    }
    let modes = c.u16()?;
    c.u32()?; // active mode
    for _ in 0..modes {
        c.string()?;
        for _ in 0..9 {
            c.u32()?; // value, flags, speed min/max, colors min/max, speed, direction, color mode
        }
        let colors = c.u16()? as usize;
        c.take(colors * 4)?;
    }
    let zones = c.u16()?;
    for _ in 0..zones {
        c.string()?;
        for _ in 0..4 {
            c.u32()?; // type, leds min/max/count
        }
        let matrix = c.u16()? as usize;
        c.take(matrix)?;
    }
    let leds = c.u16()? as usize;
    for _ in 0..leds {
        c.string()?;
        c.u32()?;
    }
    let colors = c.u16()? as usize;
    (colors <= MAX_LEDS).then_some(colors)
}

pub struct Client {
    stream: TcpStream,
    /// LED count per device.
    pub devices: Vec<usize>,
}

impl Client {
    pub fn connect(port: u16) -> std::io::Result<Self> {
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let stream = TcpStream::connect_timeout(&addr, Duration::from_millis(500))?;
        stream.set_read_timeout(Some(Duration::from_secs(2)))?;
        stream.set_write_timeout(Some(Duration::from_secs(2)))?;
        stream.set_nodelay(true)?;
        let mut client = Self {
            stream,
            devices: Vec::new(),
        };
        client.send(0, SET_CLIENT_NAME, b"Overline\0")?;
        client.send(0, REQUEST_CONTROLLER_COUNT, &[])?;
        let count = u32::from_le_bytes(
            client.reply(REQUEST_CONTROLLER_COUNT)?[..4]
                .try_into()
                .unwrap_or_default(),
        );
        for device in 0..count.min(64) {
            client.send(device, REQUEST_CONTROLLER_DATA, &[])?;
            let data = client.reply(REQUEST_CONTROLLER_DATA)?;
            let leds = led_count(&data).unwrap_or(0);
            client.devices.push(leds);
            // Direct control, so our colors show instead of the device's own effect.
            client.send(device, SET_CUSTOM_MODE, &[])?;
        }
        Ok(client)
    }

    fn send(&mut self, device: u32, packet: u32, body: &[u8]) -> std::io::Result<()> {
        let mut out = header(device, packet, body.len() as u32).to_vec();
        out.extend_from_slice(body);
        self.stream.write_all(&out)
    }

    fn reply(&mut self, expect: u32) -> std::io::Result<Vec<u8>> {
        let bad = || std::io::Error::new(std::io::ErrorKind::InvalidData, "not an OpenRGB reply");
        let mut h = [0u8; 16];
        self.stream.read_exact(&mut h)?;
        if &h[..4] != MAGIC || u32::from_le_bytes(h[8..12].try_into().map_err(|_| bad())?) != expect {
            return Err(bad());
        }
        let size = u32::from_le_bytes(h[12..16].try_into().map_err(|_| bad())?) as usize;
        if size > 4 * 1024 * 1024 {
            return Err(bad());
        }
        let mut body = vec![0u8; size];
        self.stream.read_exact(&mut body)?;
        if body.len() < 4 && expect == REQUEST_CONTROLLER_COUNT {
            return Err(bad());
        }
        Ok(body)
    }

    /// Every LED on every device to one color.
    pub fn set_all(&mut self, color: (u8, u8, u8)) -> std::io::Result<()> {
        for (device, leds) in self.devices.clone().into_iter().enumerate() {
            if leds > 0 {
                self.send(device as u32, UPDATE_LEDS, &update_leds(leds, color))?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn string(out: &mut Vec<u8>, s: &str) {
        out.extend_from_slice(&((s.len() + 1) as u16).to_le_bytes());
        out.extend_from_slice(s.as_bytes());
        out.push(0);
    }

    /// A controller as OpenRGB describes it: 2 modes, 1 zone with a matrix, 3 LEDs.
    fn sample() -> Vec<u8> {
        let mut d = Vec::new();
        d.extend_from_slice(&0u32.to_le_bytes()); // size (not checked)
        d.extend_from_slice(&2i32.to_le_bytes()); // type
        for s in ["Keyboard", "A keyboard", "1.0", "SN1", "HID"] {
            string(&mut d, s);
        }
        d.extend_from_slice(&2u16.to_le_bytes());
        d.extend_from_slice(&0i32.to_le_bytes());
        for (name, colors) in [("Direct", 0u16), ("Static", 1)] {
            string(&mut d, name);
            for _ in 0..9 {
                d.extend_from_slice(&0u32.to_le_bytes());
            }
            d.extend_from_slice(&colors.to_le_bytes());
            d.extend(std::iter::repeat_n(0u8, colors as usize * 4));
        }
        d.extend_from_slice(&1u16.to_le_bytes());
        string(&mut d, "Keys");
        for _ in 0..4 {
            d.extend_from_slice(&0u32.to_le_bytes());
        }
        d.extend_from_slice(&16u16.to_le_bytes()); // matrix: height, width, 2 cells
        d.extend(std::iter::repeat_n(0u8, 16));
        d.extend_from_slice(&3u16.to_le_bytes());
        for led in ["A", "B", "C"] {
            string(&mut d, led);
            d.extend_from_slice(&0u32.to_le_bytes());
        }
        d.extend_from_slice(&3u16.to_le_bytes());
        d.extend(std::iter::repeat_n(0u8, 12));
        d
    }

    #[test]
    fn reads_led_counts() {
        assert_eq!(led_count(&sample()), Some(3));
        assert_eq!(led_count(&sample()[..40]), None);
    }

    #[test]
    fn builds_packets() {
        let h = header(2, UPDATE_LEDS, 14);
        assert_eq!(&h[..4], b"ORGB");
        assert_eq!(u32::from_le_bytes(h[4..8].try_into().unwrap()), 2);
        assert_eq!(u32::from_le_bytes(h[8..12].try_into().unwrap()), 1050);
        let body = update_leds(2, (255, 10, 5));
        assert_eq!(body.len(), 14);
        assert_eq!(u32::from_le_bytes(body[..4].try_into().unwrap()), 14);
        assert_eq!(u16::from_le_bytes(body[4..6].try_into().unwrap()), 2);
        assert_eq!(&body[6..10], &[255, 10, 5, 0]);
    }
}

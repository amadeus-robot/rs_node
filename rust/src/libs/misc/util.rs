use reqwest::Client;
use serde_json::Value;
use std::fmt::Write;
use std::process::Command;
use std::str;

pub struct Util;

impl Util {
    /// Hexdump like in Elixir version
    pub fn hexdump(data: &[u8]) -> String {
        data.chunks(16)
            .enumerate()
            .map(|(index, chunk)| {
                let address = index * 16;
                let offset_str = format!("{:08X}", address);

                let mut hex_bytes = String::new();
                for byte in chunk {
                    write!(&mut hex_bytes, "{:02X} ", byte).unwrap();
                }
                // pad to 16*3 = 48 chars
                while hex_bytes.len() < 48 {
                    hex_bytes.push(' ');
                }

                let ascii: String = chunk
                    .iter()
                    .map(|&b| {
                        if (32..=126).contains(&b) {
                            b as char
                        } else {
                            '.'
                        }
                    })
                    .collect();

                format!("{offset_str}  {hex_bytes} {ascii}")
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Keep only ascii safe chars
    pub fn ascii(s: &str) -> String {
        s.chars()
            .filter(|&c| {
                c == ' '
                    || (c >= '!' && c <= '@')
                    || (c >= '[' && c <= '_')
                    || c.is_ascii_alphanumeric()
                    || (c >= '{' && c <= '~')
            })
            .collect()
    }

    pub fn is_ascii(s: &str) -> bool {
        s == Self::ascii(s)
    }

    pub fn alphanumeric(s: &str) -> String {
        s.chars().filter(|c| c.is_ascii_alphanumeric()).collect()
    }

    pub fn is_alphanumeric(s: &str) -> bool {
        s == Self::alphanumeric(s)
    }

    pub fn ascii_dash_underscore(s: &str) -> String {
        s.chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
            .collect()
    }

    pub fn alphanumeric_hostname(s: &str) -> String {
        s.chars()
            .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-')
            .collect()
    }

    pub fn sext(path: &str) -> String {
        let ext = std::path::Path::new(path)
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        format!(".{}", Self::alphanumeric(&ext))
    }

    pub fn url(base: &str) -> String {
        base.trim_end_matches('/').to_string()
    }

    pub fn url_with_path(base: &str, path: &str) -> String {
        format!("{}{}", base.trim_end_matches('/'), path)
    }

    pub fn url_to_ws(base: &str, path: &str) -> String {
        let mut url = format!("{}{}", base.trim_end_matches('/'), path);
        url = url.replace("https://", "wss://");
        url = url.replace("http://", "ws://");
        url
    }

    /// HTTP GET returning JSON
    pub async fn get_json(url: &str) -> reqwest::Result<Value> {
        let client = Client::new();
        let resp = client.get(url).send().await?.text().await?;
        let json: Value = serde_json::from_str(&resp).unwrap();
        Ok(json)
    }

    /// HTTP POST returning JSON
    pub async fn post_json(url: &str, body: &Value) -> reqwest::Result<Value> {
        let client = Client::new();
        let json = client
            .post(url)
            .json(body)
            .send()
            .await?
            .json::<Value>()
            .await?;
        Ok(json)
    }

    /// Run b3sum and return base32
    // pub fn b3sum(path: &str) -> String {
    //     let output = Command::new("b3sum")
    //         .arg("--no-names")
    //         .arg("--raw")
    //         .arg(path)
    //         .output()
    //         .expect("failed to run b3sum");

    //     data_encoding::BASE32_NOPAD
    //         .encode(&output.stdout)
    //         .to_lowercase()
    // }

    /// Pad a bitstring (here Vec<u8>) to full bytes if needed
    pub fn pad_bitstring_to_bytes(bits: &[u8], bit_len: usize) -> Vec<u8> {
        let mut out = bits.to_vec();
        let pad = (8 - (bit_len % 8)) % 8;
        if pad > 0 {
            // already byte aligned since we store bytes, no-op in Rust
        }
        out
    }

    pub fn set_bit(buf: &mut [u8], i: usize) {
        let byte_index = i / 8;
        let bit_index = i % 8;
        buf[byte_index] |= 1 << (7 - bit_index);
    }

    pub fn get_bit(buf: &[u8], i: usize) -> bool {
        let byte_index = i / 8;
        let bit_index = i % 8;
        (buf[byte_index] & (1 << (7 - bit_index))) != 0
    }

    pub fn index_of<T: PartialEq>(list: &[T], key: &T) -> Option<usize> {
        list.iter().position(|x| x == key)
    }

    pub fn verify_time_sync() -> bool {
        let output = Command::new("timedatectl")
            .arg("status")
            .output()
            .expect("failed to run timedatectl");
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout.contains("System clock synchronized: yes")
    }
}

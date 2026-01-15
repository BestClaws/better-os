use crate::libs::bluetooth::types::{DeviceName, MAX_DEVICE_NAME_LEN};

/// Extract a human-readable device name from BLE advertisement data.
pub(crate) fn parse_device_name(data: &[u8]) -> Option<DeviceName> {
    let mut offset = 0;
    let mut fallback: Option<DeviceName> = None;

    while offset < data.len() {
        let len = data[offset] as usize;
        if len == 0 {
            break;
        }

        let segment_end = offset + 1 + len;
        if segment_end > data.len() {
            break;
        }

        let ty = data[offset + 1];
        if matches!(ty, 0x08 | 0x09) {
            let value = &data[(offset + 2)..segment_end];
            if let Ok(s) = core::str::from_utf8(value) {
                let mut name = DeviceName::with_capacity(MAX_DEVICE_NAME_LEN);
                for ch in s.chars() {
                    let ch_len = ch.len_utf8();
                    if name.len() + ch_len > MAX_DEVICE_NAME_LEN {
                        break;
                    }
                    name.push(ch);
                }

                if !name.is_empty() {
                    if ty == 0x09 {
                        return Some(name);
                    }
                    fallback = Some(name);
                }
            }
        }

        offset = segment_end;
    }

    fallback
}

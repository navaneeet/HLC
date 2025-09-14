pub fn encode(data: &[u8]) -> Vec<u8> {
    let mut encoded = Vec::with_capacity(data.len());
    let mut i = 0usize;
    while i < data.len() {
        if data[i] == 0 {
            let mut count = 0usize;
            while i + count < data.len() && data[i + count] == 0 && count < 255 {
                count += 1;
            }
            encoded.push(0);
            encoded.push(count as u8);
            i += count;
        } else {
            encoded.push(data[i]);
            i += 1;
        }
    }
    encoded
}

pub fn decode(data: &[u8]) -> Vec<u8> {
    let mut decoded = Vec::with_capacity(data.len() * 2);
    let mut i = 0usize;
    while i < data.len() {
        if data[i] == 0 {
            if i + 1 < data.len() {
                let count = data[i + 1] as usize;
                decoded.extend(std::iter::repeat(0u8).take(count));
                i += 2;
            } else {
                i += 1;
            }
        } else {
            decoded.push(data[i]);
            i += 1;
        }
    }
    decoded
}


pub fn encode(data: &[u8]) -> Vec<u8> {
    if data.is_empty() { return vec![]; }
    let mut encoded = Vec::with_capacity(data.len());
    encoded.push(data[0]);
    for i in 1..data.len() {
        encoded.push(data[i].wrapping_sub(data[i - 1]));
    }
    encoded
}

pub fn decode(data: &[u8]) -> Vec<u8> {
    if data.is_empty() { return vec![]; }
    let mut decoded = Vec::with_capacity(data.len());
    decoded.push(data[0]);
    for i in 1..data.len() {
        let value = data[i].wrapping_add(decoded[i - 1]);
        decoded.push(value);
    }
    decoded
}


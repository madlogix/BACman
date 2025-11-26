
fn main() {
    // Device 5050 name: 0x7515004241436E6574344A2064657669636520353035303F
    let hex = "7515004241436E6574344A2064657669636520353035303F";
    let bytes: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i+2], 16).unwrap())
        .collect();
    
    println\!("Bytes: {:?}", bytes);
    println\!("Tag: 0x{:02X}", bytes[0]);
    println\!("Length: {}", bytes[1]);
    
    // Extract UTF-16 string
    let string_data = &bytes[2..2+21];
    println\!("String bytes: {:?}", string_data);
    
    // Check if UTF-16
    let mut utf16_pairs = Vec::new();
    for i in (0..string_data.len()).step_by(2) {
        if i + 1 < string_data.len() {
            let pair = ((string_data[i] as u16) << 8) | (string_data[i+1] as u16);
            utf16_pairs.push(pair);
        }
    }
    
    let text = String::from_utf16(&utf16_pairs).unwrap();
    println\!("Decoded text: {}", text);
}


// Smart Switch Key & App Memory Integration Tests

use minkey::smart_switch::SmartSwitchTable;

#[test]
fn test_smart_switch_empty_table() {
    let table = SmartSwitchTable::new();
    assert!(table.map.is_empty());
    let bin = table.to_binary();
    // 2 bytes for count = 0
    assert_eq!(bin, vec![0x00, 0x00]);

    let restored = SmartSwitchTable::from_binary(&bin);
    assert!(restored.map.is_empty());
}

#[test]
fn test_smart_switch_roundtrip() {
    let mut table = SmartSwitchTable::new();
    // app1: Code.exe -> Vietnamese (1) + Unicode (0) -> 0x01
    table.update("Code.exe", 1, 0);
    // app2: WindowsTerminal.exe -> English (0) + VNI (2) -> 0 | (2 << 1) = 4
    table.update("WindowsTerminal.exe", 0, 2);

    let bin = table.to_binary();
    let restored = SmartSwitchTable::from_binary(&bin);

    assert_eq!(restored.map.len(), 2);
    assert_eq!(restored.map.get("Code.exe"), Some(&0x01));
    assert_eq!(restored.map.get("WindowsTerminal.exe"), Some(&0x04));
}

#[test]
fn test_smart_switch_openkey_binary_compatibility() {
    // Manually construct binary format produced by OpenKey C++:
    // count: 2 (0x02, 0x00)
    // entry 1: len: 9 ("excel.exe"), val: 0x05 (Vietnamese 1, VNI Win 2 => 1 | (2 << 1) = 5)
    // entry 2: len: 10 ("chrome.exe"), val: 0x00 (English 0, Unicode 0 => 0)
    let mut raw_bytes = Vec::new();
    raw_bytes.extend_from_slice(&2u16.to_le_bytes());

    let app1 = "excel.exe";
    raw_bytes.push(app1.len() as u8);
    raw_bytes.extend_from_slice(app1.as_bytes());
    raw_bytes.push(5);

    let app2 = "chrome.exe";
    raw_bytes.push(app2.len() as u8);
    raw_bytes.extend_from_slice(app2.as_bytes());
    raw_bytes.push(0);

    let table = SmartSwitchTable::from_binary(&raw_bytes);
    assert_eq!(table.map.len(), 2);

    let val1 = *table.map.get("excel.exe").unwrap();
    assert_eq!(val1 & 0x01, 1); // Vietnamese
    assert_eq!((val1 >> 1) & 0x07, 2); // VNI Windows

    let val2 = *table.map.get("chrome.exe").unwrap();
    assert_eq!(val2 & 0x01, 0); // English
    assert_eq!((val2 >> 1) & 0x07, 0); // Unicode
}

#[test]
fn test_get_app_input_method_status_caching() {
    let mut table = SmartSwitchTable::new();
    
    // First query for an unknown app -> should insert current and return -1
    let status = table.get_app_input_method_status("notepad.exe", 0x03);
    assert_eq!(status, -1);
    assert_eq!(table.map.get("notepad.exe"), Some(&0x03));

    // Subsequent query -> should return cached value 3
    let status2 = table.get_app_input_method_status("notepad.exe", 0x00);
    assert_eq!(status2, 3);

    // Case-insensitive query
    let status3 = table.get_app_input_method_status("NotePad.EXE", 0x00);
    assert_eq!(status3, 3);

    // Update app
    table.update("notepad.exe", 0, 1); // English (0) + TCVN3 (1) => 0 | (1 << 1) = 2
    let status4 = table.get_app_input_method_status("notepad.exe", 0x00);
    assert_eq!(status4, 2);
}

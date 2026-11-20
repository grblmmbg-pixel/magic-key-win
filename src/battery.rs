use windows::core::*;
use windows::Devices::Bluetooth::GenericAttributeProfile::*;
use windows::Devices::Bluetooth::*;
use windows::Storage::Streams::DataReader;

pub fn get_battery_level(bluetooth_address: u64) -> Result<u8> {
    let ble_device = BluetoothLEDevice::FromBluetoothAddressAsync(bluetooth_address)?.join()?;

    let battery_service_uuid = GUID::from_u128(0x0000180F_0000_1000_8000_00805F9B34FB_u128);
    let battery_char_uuid = GUID::from_u128(0x00002A19_0000_1000_8000_00805F9B34FB_u128);

    let services_res = ble_device.GetGattServicesForUuidAsync(battery_service_uuid)?.join()?;
    let status = services_res.Status()?;
    println!("GattServices status: {:?}", status);

    if status != GattCommunicationStatus::Success {
        return Err(Error::new(
            HRESULT(0x80004005u32 as i32),
            "GATT Battery Service not accessible",
        ));
    }

    let services = services_res.Services()?;
    if services.Size()? == 0 {
        return Err(Error::new(HRESULT(-1), "Battery Service not found"));
    }

    let service = services.GetAt(0)?;
    let chars_res = service.GetCharacteristicsForUuidAsync(battery_char_uuid)?.join()?;

    if chars_res.Status()? != GattCommunicationStatus::Success {
        return Err(Error::new(
            HRESULT(-1),
            "GATT Battery Characteristic not accessible",
        ));
    }

    let chars = chars_res.Characteristics()?;
    if chars.Size()? == 0 {
        return Err(Error::new(HRESULT(-1), "Battery Characteristic not found"));
    }

    let ch = chars.GetAt(0)?;
    let val_res = ch.ReadValueAsync()?.join()?;

    if val_res.Status()? != GattCommunicationStatus::Success {
        return Err(Error::new(
            HRESULT(-1),
            "Failed to read battery characteristic value",
        ));
    }

    let buffer = val_res.Value()?;
    let reader = DataReader::FromBuffer(&buffer)?;
    let len = reader.UnconsumedBufferLength()?;
    if len > 0 {
        let percent = reader.ReadByte()?;
        Ok(percent)
    } else {
        Err(Error::new(HRESULT(-1), "Empty battery buffer"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_battery_query() {
        let addr = 0xD4DCCD83823Bu64;
        match get_battery_level(addr) {
            Ok(level) => println!("\n>>> SUCCESS: Magic Keyboard Battery Level: {}% <<<\n", level),
            Err(e) => println!("\nBattery query note (check status): {:?}\n", e),
        }
    }
}

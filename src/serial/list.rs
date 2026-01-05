use anyhow::Result;
use serialport::SerialPortType;

pub fn run() -> Result<()> {
    let ports = serialport::available_ports()?;
    if ports.is_empty() {
        println!("No serial ports found.");
        return Ok(());
    }

    println!("Available serial ports:");
    for p in ports {
        println!("  {}", p.port_name);
        match p.port_type {
            SerialPortType::UsbPort(info) => {
                if let Some(product) = info.product {
                    println!("    Product: {}", product);
                }
                if let Some(manufacturer) = info.manufacturer {
                    println!("    Manufacturer: {}", manufacturer);
                }
            }
            SerialPortType::PciPort => {
                println!("    Type: PCI");
            }
            SerialPortType::BluetoothPort => {
                println!("    Type: Bluetooth");
            }
            SerialPortType::Unknown => {
                println!("    Type: Unknown");
            }
        }
    }
    Ok(())
}

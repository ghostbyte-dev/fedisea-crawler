use crate::models::Nodeinfo;
use std::fs::File;
use std::io::Write;

pub fn save_data(instance: String, nodeinfo: Nodeinfo, file: &mut File) {
    writeln!(
        file,
        "instance: {}, {}: {}",
        instance, nodeinfo.software.name, nodeinfo.software.version
    )
    .expect("Failed to save to file");
}

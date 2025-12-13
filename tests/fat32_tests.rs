use fat32_rs::{
    blockdev::MemDevice,
    dir::parse_dir_entry,
    fat::FatTable,
    fs::Fat32,
    bpb::BiosParameterBlock,
};

fn boot_sector_template() -> [u8; 512] {
    let mut sector = [0u8; 512];
    sector[11..13].copy_from_slice(&512u16.to_le_bytes());
    sector[13] = 1; // sectors per cluster
    sector[14..16].copy_from_slice(&1u16.to_le_bytes()); // reserved
    sector[16] = 1; // number of FATs
    sector[36..40].copy_from_slice(&1u32.to_le_bytes()); // sectors per FAT
    sector[44..48].copy_from_slice(&2u32.to_le_bytes()); // root cluster
    sector
}

fn make_dir_entry(name: &str, ext: &str, attr: u8, cluster: u32, size: u32) -> [u8; 32] {
    let mut entry = [0u8; 32];
    let mut name_buf = [b' '; 8];
    let mut ext_buf = [b' '; 3];
    for (i, b) in name.as_bytes().iter().take(8).enumerate() {
        name_buf[i] = *b;
    }
    for (i, b) in ext.as_bytes().iter().take(3).enumerate() {
        ext_buf[i] = *b;
    }
    entry[0..8].copy_from_slice(&name_buf);
    entry[8..11].copy_from_slice(&ext_buf);
    entry[11] = attr;
    entry[20..22].copy_from_slice(&((cluster >> 16) as u16).to_le_bytes());
    entry[26..28].copy_from_slice(&(cluster as u16).to_le_bytes());
    entry[28..32].copy_from_slice(&size.to_le_bytes());
    entry
}

#[test]
fn parse_bpb() {
    let sector = boot_sector_template();
    let bpb = BiosParameterBlock::parse(&sector).unwrap();
    assert_eq!(bpb.bytes_per_sector, 512);
    assert_eq!(bpb.sectors_per_cluster, 1);
    assert_eq!(bpb.reserved_sector_count, 1);
    assert_eq!(bpb.num_fats, 1);
    assert_eq!(bpb.sectors_per_fat, 1);
    assert_eq!(bpb.root_cluster, 2);
}

#[test]
fn next_cluster_reads_fat() {
    let bpb = BiosParameterBlock::parse(&boot_sector_template()).unwrap();
    // Layout: sector0 boot, sector1 FAT, sector2 data
    let mut image = vec![0u8; 512 * 3];
    image[0..512].copy_from_slice(&boot_sector_template());
    // FAT sector
    let fat = &mut image[512..1024];
    // cluster0/1 reserved
    fat[0..4].copy_from_slice(&0x0FFFFFF8u32.to_le_bytes());
    fat[4..8].copy_from_slice(&0x0FFFFFFFu32.to_le_bytes());
    // cluster2 points to 3
    fat[8..12].copy_from_slice(&3u32.to_le_bytes());
    // cluster3 EOC
    fat[12..16].copy_from_slice(&0x0FFFFFFFu32.to_le_bytes());

    let dev = MemDevice::new(512, image).unwrap();
    let fat_table = FatTable { device: dev, bpb, lba_start: 0 };
    assert_eq!(fat_table.next_cluster(2).unwrap(), Some(3));
    assert_eq!(fat_table.next_cluster(3).unwrap(), None);
}

#[test]
fn parse_directory_entry() {
    let entry = make_dir_entry("HELLO", "TXT", 0x20, 5, 42);
    let parsed = parse_dir_entry(&entry).unwrap();
    assert_eq!(parsed.name, "HELLO.TXT");
    assert_eq!(parsed.cluster, 5);
    assert_eq!(parsed.size, 42);
}

fn mini_image() -> MemDevice {
    // sectors: [boot][fat][cluster2 root][cluster3 file]
    let mut image = vec![0u8; 512 * 4];
    image[0..512].copy_from_slice(&boot_sector_template());
    let fat = &mut image[512..1024];
    fat[0..4].copy_from_slice(&0x0FFFFFF8u32.to_le_bytes());
    fat[4..8].copy_from_slice(&0x0FFFFFFFu32.to_le_bytes());
    fat[8..12].copy_from_slice(&3u32.to_le_bytes()); // root -> cluster3 for file
    fat[12..16].copy_from_slice(&0x0FFFFFFFu32.to_le_bytes()); // cluster3 EOC

    // root dir entry for FILE.TXT at cluster3, size 5
    let root = &mut image[1024..1536];
    root[0..32].copy_from_slice(&make_dir_entry("FILE", "TXT", 0x20, 3, 5));
    root[32] = 0; // end marker

    // cluster3 data
    let data = &mut image[1536..2048];
    data[0..5].copy_from_slice(b"hello");

    MemDevice::new(512, image).unwrap()
}

#[test]
fn resolve_path_and_ls() {
    let dev = mini_image();
    let mut fs = Fat32::mount(dev, 0).unwrap();
    let list = fs.ls(None).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].name, "FILE.TXT");
    fs.cd("/").unwrap();
    assert_eq!(fs.pwd(), "/");
}

#[test]
fn read_file_content() {
    let dev = mini_image();
    let fs = Fat32::mount(dev, 0).unwrap();
    let data = fs.read_file("/FILE.TXT").unwrap();
    assert_eq!(data, b"hello");
}

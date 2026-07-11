// Auto-extracted from lib.rs first mod tests block
mod tests {
    use super::*;

    #[test]
    fn test_binary_storage_tpch_sf1_load() {
        use sqlrustgo_storage::BinaryTableStorage;

        let bin_dir = std::path::PathBuf::from("/tmp/tpch-sf1-bin");
        if !bin_dir.exists() {
            println!("SKIP: /tmp/tpch-sf1-bin not found (run tbl2bin first)");
            return;
        }

        let storage = BinaryTableStorage::new_with_data(bin_dir).expect("load .bin files");
        let counts: Vec<(&str, usize)> = vec![
            ("region", 5),
            ("nation", 25),
            ("customer", 150_000),
            ("supplier", 10_000),
            ("part", 200_000),
            ("partsupp", 800_000),
            ("orders", 1_500_000),
            ("lineitem", 6_001_215),
        ];

        for (table, expected) in counts {
            let rows = storage.scan(table).expect(table);
            assert_eq!(rows.len(), expected, "table {} row count", table);
            println!("  {}: {} rows OK", table, rows.len());
        }
        println!("BinaryTableStorage loaded all 8 TPC-H tables correctly");
    }

    // Test Packet serialization - roundtrip
    #[test]
    fn test_packet_roundtrip() {
        let pkt = Packet {
            length: 5,
            sequence: 3,
            payload: vec![1, 2, 3, 4, 5],
        };
        let mut buf = Vec::new();
        pkt.write_to(&mut buf).unwrap();

        // Verify header: 3 bytes length + 1 byte sequence
        assert_eq!(buf.len(), 4 + 5);
        assert_eq!(buf[0], 5); // length byte 0
        assert_eq!(buf[1], 0); // length byte 1
        assert_eq!(buf[2], 0); // length byte 2
        assert_eq!(buf[3], 3); // sequence

        let mut reader = std::io::Cursor::new(&buf);
        let read_pkt = Packet::read_from(&mut reader).unwrap();
        assert_eq!(read_pkt.length, pkt.length);
        assert_eq!(read_pkt.sequence, pkt.sequence);
        assert_eq!(read_pkt.payload, pkt.payload);
    }

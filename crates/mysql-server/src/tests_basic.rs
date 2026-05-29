// Auto-extracted from lib.rs first mod tests block
mod tests {
    use super::*;

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

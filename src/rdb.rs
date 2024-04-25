pub mod rdb_writer {
    use crate::types::toml_to_string::string_from_toml_value;
    use core::result::Result;
    use crc64::crc64;
    use regex::Regex;
    use std::io::{self, BufRead, Write};
    use toml::value::Array as TomlArray;
    use toml::{Table, Value};

    const TABLE_NAME_REGEX: &str = r"^\[[^\s\[\]]+\]";
    const STRING_TYPECODE: u8 = b'\x00';
    const SET_TYPECODE: u8 = b'\x02';
    const HASH_TYPECODE: u8 = b'\x04';

    // First 2 bits of the length-encoding bytes are reserved for
    // one of these constants, telling Redis how many bytes in total
    // will be representing the length of the coming value
    const RDB_6BITLEN: u8 = 0;
    const RDB_14BITLEN: u8 = 1;
    const RDB_32BITLEN: u8 = 0x80;
    const RDB_64BITLEN: u8 = 0x81;

    struct RedisHash<'a> {
        key: &'a String,
        value: &'a Table,
        crc: u64,
    }

    struct RedisSet<'a> {
        key: &'a String,
        value: &'a TomlArray,
        crc: u64,
    }

    struct RedisString<'a> {
        key: &'a String,
        value: &'a String,
        crc: u64,
    }

    trait RedisWriter {
        fn write_bytes(&mut self, buf_writer: &mut impl Write);
    }

    impl RedisWriter for RedisHash<'_> {
        fn write_bytes(&mut self, buf_writer: &mut impl Write) {
            let mut table_bytes = [&[HASH_TYPECODE], &encode_string(self.key)[..]].concat();
            let mut table_length_encoding: Vec<u8> =
                encode_length(self.value.len().try_into().unwrap());
            table_bytes.append(&mut table_length_encoding);

            for (key, val) in self.value.into_iter() {
                table_bytes.append(&mut encode_string(key));
                table_bytes.append(&mut encode_string(&string_from_toml_value(val)));
            }
            self.crc = checksum_write(buf_writer, &table_bytes, self.crc);
        }
    }

    impl RedisWriter for RedisString<'_> {
        fn write_bytes(&mut self, buf_writer: &mut impl Write) {
            self.crc = checksum_write(
                buf_writer,
                &[
                    &[STRING_TYPECODE],
                    &encode_string(self.key)[..],
                    &encode_string(self.value)[..],
                ]
                .concat(),
                self.crc,
            );
        }
    }

    impl RedisWriter for RedisSet<'_> {
        fn write_bytes(&mut self, buf_writer: &mut impl Write) {
            let mut set_bytes: Vec<u8> = [&[SET_TYPECODE], &encode_string(self.key)[..]].concat();
            let mut set_length_encoding: Vec<u8> =
                encode_length(self.value.len().try_into().unwrap());
            set_bytes.append(&mut set_length_encoding);

            for val in self.value {
                set_bytes.append(&mut encode_string(&string_from_toml_value(val)));
            }
            self.crc = checksum_write(buf_writer, &set_bytes, self.crc);
        }
    }

    fn encode_length(length: u64) -> Vec<u8> {
        if length < (1 << 6) {
            Vec::<u8>::from([u8::try_from(length).unwrap() | (RDB_6BITLEN << 6)])
        } else if length < (1 << 14) {
            Vec::<u8>::from([
                u8::try_from(length >> 8).unwrap() | (RDB_14BITLEN << 6),
                u8::try_from(length & 0xFF).unwrap(),
            ])
        } else if length < u64::from(u32::MAX) {
            [
                &[RDB_32BITLEN],
                &u32::try_from(length).unwrap().to_be_bytes()[..],
            ]
            .concat()
        } else {
            return [&[RDB_64BITLEN], &length.to_be_bytes()[..]].concat();
        }
    }

    fn checksum_write(buf_writer: &mut impl Write, bytes: &[u8], start_crc: u64) -> u64 {
        let _ = buf_writer.write(bytes);
        crc64(start_crc, bytes)
    }

    fn encode_string(value: &String) -> Vec<u8> {
        return [
            encode_length(value.len().try_into().unwrap()),
            value.as_bytes().to_vec(),
        ]
        .concat();
    }

    fn write_to_rdb_bytes_from_string(
        buf_writer: &mut impl Write,
        key_value_string: String,
        crc: u64,
    ) -> u64 {
        let table: Table = key_value_string
            .parse::<Table>()
            .expect(crate::INVALID_TOML_ERROR);
        let key = table.keys().next().expect(crate::INVALID_TOML_ERROR);

        match table.get(key).expect(crate::INVALID_TOML_ERROR) {
            Value::Array(array_val) => {
                let mut set = RedisSet {
                    key,
                    value: array_val,
                    crc,
                };
                set.write_bytes(buf_writer);
                set.crc
            }
            Value::Table(table_val) => {
                let mut hash = RedisHash {
                    key,
                    value: table_val,
                    crc,
                };
                hash.write_bytes(buf_writer);
                hash.crc
            }
            other_val => {
                let mut redis_str = RedisString {
                    key,
                    value: &string_from_toml_value(other_val),
                    crc,
                };
                redis_str.write_bytes(buf_writer);
                redis_str.crc
            }
        }
    }

    pub fn rdb_from_buffer<R: io::Read>(
        buf_reader: &mut io::BufReader<R>,
        buf_writer: &mut impl Write,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut crc: u64 = 0;
        crc = header(buf_writer, crc);
        let table_name_regex: Regex = Regex::new(TABLE_NAME_REGEX).unwrap();
        let mut current_table_contents: String = String::new();

        for line in buf_reader.lines() {
            let line_str: String = line.expect(crate::INVALID_TOML_ERROR);
            if !current_table_contents.is_empty() {
                if line_str.is_empty() {
                    crc = write_to_rdb_bytes_from_string(buf_writer, current_table_contents, crc);
                    current_table_contents = String::new();
                } else {
                    current_table_contents.push('\n');
                    current_table_contents.push_str(&line_str);
                }
            } else if !line_str.is_empty() {
                if table_name_regex.is_match(&line_str) {
                    current_table_contents.push_str(&line_str);
                } else {
                    crc = write_to_rdb_bytes_from_string(buf_writer, line_str, crc);
                }
            }
        }
        if !current_table_contents.is_empty() {
            crc = write_to_rdb_bytes_from_string(buf_writer, current_table_contents, crc);
        }
        end_of_file(buf_writer, crc);
        Ok(())
    }

    fn header(buf_writer: &mut impl Write, crc: u64) -> u64 {
        checksum_write(
            buf_writer,
            &[
                &format!(
                    "REDIS{:04}",
                    crate::REDIS_VERSION
                        .get()
                        .expect("Redis version is not set")
                )
                .into_bytes()[..],
                b"\xfe",
                b"\x00", // ID of the database = 0
            ]
            .concat(),
            crc,
        )
    }

    fn end_of_file(buf_writer: &mut impl Write, crc: u64) {
        let final_checksum = checksum_write(buf_writer, b"\xff", crc).to_le_bytes();
        let _ = buf_writer.write(&final_checksum);
    }
}

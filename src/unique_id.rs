use chrono::{DateTime, TimeZone, Utc};

///
/// Implementation for the snowflake ID. See: https://en.wikipedia.org/wiki/Snowflake_ID
pub struct SnowflakeId(i64);

const fn and_with(value: i64, num_bits: u32) -> i64 {
    value & (2i64.pow(num_bits) - 1)
}

impl SnowflakeId {
    ///
    /// Snoflake ID timestamp start from this point in time
    const EPOCH_START_MS: i64 = 1288834974657;

    const SEQUENCE_BITS: u32 = 12;

    pub const fn new(machine_id: u16, timestamp: DateTime<Utc>, sequence: u16) -> Self {
        let timestamp_ms = timestamp.timestamp_millis() - Self::EPOCH_START_MS;
        let timestamp_ms = and_with(timestamp_ms, 42) << 22;

        let machine_id = and_with(machine_id as i64, 10) << 12;
        let sequence = and_with(sequence as i64, Self::SEQUENCE_BITS);

        Self(timestamp_ms | machine_id | sequence)
    }

    pub const fn get(&self) -> i64 {
        self.0
    }

    pub const fn machine_id(&self) -> u16 {
        let machine_id = and_with(self.0 >> 12, 10);
        // we know the number of bits is 10 so it fits in a u16
        machine_id as u16
    }

    pub fn timestamp(&self) -> DateTime<Utc> {
        let timestamp = and_with(self.0 >> 22, 42);

        let timestamp_millis = Self::EPOCH_START_MS + timestamp;
        DateTime::from_timestamp_millis(timestamp_millis)
            .expect("timestamp millis is always in range because we know the number of bits")
    }

    pub const fn sequence(&self) -> u16 {
        let sequence = and_with(self.0, Self::SEQUENCE_BITS);
        // we know the number of bits is 12 so it fits in a u16
        sequence as u16
    }
}

pub struct SnowflakeIdGenerator {
    machine_id: u16,
    sequence: u16,
}

impl SnowflakeIdGenerator {
    pub fn new(machine_id: u16, sequence: u16) -> Self {
        Self {
            machine_id,
            sequence,
        }
    }

    pub fn generate(&mut self) -> SnowflakeId {
        let timestamp = Utc::now();
        let id = SnowflakeId::new(self.machine_id, timestamp, self.sequence);
        self.sequence += 1;
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_attributes() {
        let id = SnowflakeId(1541815603606036480);
        assert_eq!(id.timestamp().timestamp_millis(), 1656432460105);
        assert_eq!(id.sequence(), 0);
        assert_eq!(id.machine_id(), 378);
    }

    #[test]
    fn test_id_new() {
        let timestamp = DateTime::from_timestamp_millis(1656432460105).unwrap();
        let id = SnowflakeId::new(378, timestamp, 0);
        assert_eq!(id.get(), 1541815603606036480);
    }
}

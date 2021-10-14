use crate::payload::common::AsciiStr;
use crate::payload::{Error, Serialize};
use arbitrary::{size_hint, Arbitrary, Unstructured};
use rand::Rng;
use std::io::Write;

const PARTITIONS: [&str; 4] = ["eu", "eu2", "ap1", "us1"];
const STAGES: [&str; 4] = ["production", "performance", "noprod", "staging"];
const CONTAINER_TYPES: [&str; 1] = ["ingress"];
const EVENT_TYPES: [&str; 1] = ["service"];
const SYSTEM_IDS: [&str; 4] = ["one", "two", "three", "four"];
const SERVICES: [&str; 7] = [
    "tablet",
    "phone",
    "phone2",
    "laptop",
    "desktop",
    "monitor",
    "bigger-monitor",
];
const MESSAGES: [&str; 5] = [
"Es war ein Mann im Lande Uz, der hieß Hiob. Derselbe war schlecht und recht, gottesfürchtig und mied das Böse.",
"Und zeugte sieben Söhne und drei Töchter;",
"und seines Viehs waren siebentausend Schafe, dreitausend Kamele, fünfhundert Joch Rinder und fünfhundert Eselinnen, und er hatte viel Gesinde; und er war herrlicher denn alle, die gegen Morgen wohnten.",
"Und seine Söhne gingen und machten ein Mahl, ein jeglicher in seinem Hause auf seinen Tag, und sandten hin und luden ihre drei Schwestern, mit ihnen zu essen und zu trinken",
"Und wenn die Tage des Mahls um waren, sandte Hiob hin und heiligte sie und machte sich des Morgens früh auf und opferte Brandopfer nach ihrer aller Zahl; denn Hiob gedachte: Meine Söhne möchten gesündigt und Gott abgesagt haben in ihrem Herzen. Also tat Hiob allezeit.",
    ];

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Attrs {
    #[serde(rename = "systemid")]
    pub system_id: String,
    pub stage: String,
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(rename = "c2cService")]
    pub c2c_service: String,
    #[serde(rename = "c2cPartition")]
    pub c2c_partition: String,
    #[serde(rename = "c2cStage")]
    pub c2c_stage: String, // same as
    #[serde(rename = "c2cContainerType")]
    pub c2c_container_type: String,
    pub aws_account: String,
}

impl<'a> Arbitrary<'a> for Attrs {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        let choice: u8 = u.arbitrary()?;
        let system_id = SYSTEM_IDS[(choice as usize) % SYSTEM_IDS.len()].to_string();
        let partition = PARTITIONS[(choice as usize) % PARTITIONS.len()].to_string();
        let event_type = EVENT_TYPES[(choice as usize) % EVENT_TYPES.len()].to_string();
        let stage = STAGES[(choice as usize) % STAGES.len()].to_string();
        let service = SERVICES[(choice as usize) % SERVICES.len()].to_string();
        let container = CONTAINER_TYPES[(choice as usize) % CONTAINER_TYPES.len()].to_string();
        let aws_account = "verymodelofthemodernmajor".to_string();

        let attrs = Attrs {
            system_id,
            stage: stage.clone(),
            event_type,
            c2c_service: service,
            c2c_partition: partition,
            c2c_stage: stage,
            c2c_container_type: container,
            aws_account,
        };
        Ok(attrs)
    }

    fn size_hint(depth: usize) -> (usize, Option<usize>) {
        size_hint::recursion_guard(depth, |depth| {
            size_hint::and_all(&[<AsciiStr as Arbitrary>::size_hint(depth)])
        })
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Event {
    pub timestamp: f64,
    attrs: Attrs,
}

impl<'a> Arbitrary<'a> for Event {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        let event = Event {
            timestamp: 1606215269.333915,
            attrs: u.arbitrary()?,
        };
        Ok(event)
    }

    fn size_hint(depth: usize) -> (usize, Option<usize>) {
        size_hint::recursion_guard(depth, |depth| {
            size_hint::and_all(&[
                <f64 as Arbitrary>::size_hint(depth),
                <Attrs as Arbitrary>::size_hint(depth),
            ])
        })
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Member {
    pub event: Event,
    pub time: f64,
    pub host: String,
    pub index: String,
    pub message: String,
}

impl<'a> Arbitrary<'a> for Member {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        let choice: u8 = u.arbitrary()?;
        let host = SYSTEM_IDS[(choice as usize) % SYSTEM_IDS.len()].to_string();
        let index = PARTITIONS[(choice as usize) % PARTITIONS.len()].to_string();
        let message = MESSAGES[(choice as usize) % MESSAGES.len()].to_string();
        let member = Member {
            event: u.arbitrary()?,
            time: 1606215269.333915,
            host,
            index,
            message,
        };
        Ok(member)
    }

    fn size_hint(depth: usize) -> (usize, Option<usize>) {
        size_hint::recursion_guard(depth, |depth| {
            size_hint::and_all(&[
                <Attrs as Arbitrary>::size_hint(depth),
                <f64 as Arbitrary>::size_hint(depth),
                <AsciiStr as Arbitrary>::size_hint(depth),
                <AsciiStr as Arbitrary>::size_hint(depth),
                <AsciiStr as Arbitrary>::size_hint(depth),
            ])
        })
    }
}

#[derive(Debug, Default)]
pub struct SplunkHec {}

impl Serialize for SplunkHec {
    fn to_bytes<W, R>(&self, mut rng: R, max_bytes: usize, writer: &mut W) -> Result<(), Error>
    where
        R: Rng + Sized,
        W: Write,
    {
        let mut entropy: Vec<u8> = vec![0; max_bytes];
        rng.fill_bytes(&mut entropy);
        let mut unstructured = Unstructured::new(&entropy);

        let mut bytes_remaining = max_bytes;
        while let Ok(member) = unstructured.arbitrary::<Member>() {
            let encoding = serde_json::to_string(&member)?;
            let line_length = encoding.len() + 1; // add one for the newline
            match bytes_remaining.checked_sub(line_length) {
                Some(remainder) => {
                    writeln!(writer, "{}", encoding)?;
                    bytes_remaining = remainder;
                }
                None => break,
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use quickcheck::{QuickCheck, TestResult};
    use rand::rngs::SmallRng;
    use rand::SeedableRng;

    use super::Member;
    use crate::payload::{Serialize, SplunkHec};

    // We want to be sure that the serialized size of the payload does not
    // exceed `max_bytes`.
    #[test]
    fn payload_not_exceed_max_bytes() {
        fn inner(seed: u64, max_bytes: u16) -> TestResult {
            let max_bytes = max_bytes as usize;
            let rng = SmallRng::seed_from_u64(seed);
            let hec = SplunkHec::default();

            let mut bytes = Vec::with_capacity(max_bytes);
            hec.to_bytes(rng, max_bytes, &mut bytes).unwrap();
            assert!(bytes.len() <= max_bytes);

            TestResult::passed()
        }
        QuickCheck::new()
            .tests(1_000)
            .quickcheck(inner as fn(u64, u16) -> TestResult);
    }

    // We want to know that every payload produced by this type actually
    // deserializes as splunk's hec, is not truncated etc.
    #[test]
    fn every_payload_deserializes() {
        fn inner(seed: u64, max_bytes: u16) -> TestResult {
            let max_bytes = max_bytes as usize;
            let rng = SmallRng::seed_from_u64(seed);
            let hec = SplunkHec::default();

            let mut bytes: Vec<u8> = Vec::with_capacity(max_bytes);
            hec.to_bytes(rng, max_bytes, &mut bytes).unwrap();

            let payload = std::str::from_utf8(&bytes).unwrap();
            for msg in payload.lines() {
                let _members: Member = serde_json::from_str(msg).unwrap();
            }

            TestResult::passed()
        }
        QuickCheck::new()
            .tests(1_000_000)
            .quickcheck(inner as fn(u64, u16) -> TestResult);
    }
}